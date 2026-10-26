//! Addons: extra content the launcher deploys into a game folder and takes out again.
//!
//! An addon is a folder in %APPDATA%\BWLauncher\addons\<id>\ with an `addon.json`:
//! ```json
//! { "id": "hd-skies", "name": "HD skies", "version": "1.0.0", "author": "...", "description": "...",
//!   "kind": "texture-pack", "requires": { "decomp": ">=0.10.0" } }
//! ```
//! Kinds (see KINDS; add new ones there and in the UI's addon kind table):
//! - `texture-pack`: images in `textures\`, copied to `<game>\Textures\Replace\<id>\` (docs/TEXTURES.md in the decomp).
//! - `preset`: `"settings": [{ "section": "Display", "key": "AntiAliasing", "value": "8" }]`, written to
//!   BWDecomp.ini when enabled and put back when disabled (unless the player changed them since).
//!
//! What each enabled addon wrote is recorded in %APPDATA%\BWLauncher\deployed.json, per game folder, so disabling
//! removes exactly that.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::fsops::{self, FileOp};
use crate::ini::{self, IniValue};
use crate::paths::{is_safe_relative_path, state_dir};
use crate::settings;

pub const MANIFEST: &str = "addon.json";

/// Addon kinds this launcher can deploy.
pub const KINDS: &[&str] = &["texture-pack", "preset"];

const TEXTURE_EXTENSIONS: &[&str] = &["png", "dds", "bmp", "jpg", "jpeg", "tif", "tiff"];
const LARGEST_ADDON_FILE: u64 = 256 << 20;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddonManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub description: String,
    pub kind: String,
    #[serde(default)]
    pub homepage: String,
    #[serde(default)]
    pub requires: Requires,
    /// For `preset`.
    #[serde(default)]
    pub settings: Vec<IniValue>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Requires {
    /// Semver range of the patch, e.g. ">=0.10.0".
    #[serde(default)]
    pub decomp: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddonInfo {
    pub manifest: AddonManifest,
    pub path: String,
    pub enabled: bool,
    /// Why it cannot be enabled; empty when it can.
    pub problem: String,
    pub file_count: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Deployment {
    version: String,
    /// Files written, relative to the game folder.
    files: Vec<String>,
    /// Settings changed: what was there before (None = absent) and what the addon set.
    settings: Vec<SettingChange>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct SettingChange {
    section: String,
    key: String,
    before: Option<String>,
    after: String,
}

type Deployed = BTreeMap<String, BTreeMap<String, Deployment>>;

pub fn addons_dir() -> PathBuf {
    state_dir().join("addons")
}

fn deployed_file() -> PathBuf {
    state_dir().join("deployed.json")
}

fn folder_key(folder: &Path) -> String {
    crate::paths::normalize(folder).to_string_lossy().to_lowercase()
}

fn load_deployed() -> Deployed {
    std::fs::read(deployed_file()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

fn save_deployed(deployed: &Deployed) -> Result<()> {
    std::fs::create_dir_all(state_dir())?;
    std::fs::write(deployed_file(), serde_json::to_vec_pretty(deployed)?)?;
    Ok(())
}

/// Lower-case letters, digits, "-" and "_": the id is a folder name in two places.
pub fn is_valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-' || c == b'_')
}

fn read_manifest(dir: &Path) -> Result<AddonManifest> {
    let path = dir.join(MANIFEST);
    let bytes = std::fs::read(&path).map_err(|e| Error::msg(format!("Cannot read {}: {e}", path.display())))?;
    let manifest: AddonManifest =
        serde_json::from_slice(&bytes).map_err(|e| Error::msg(format!("{} is not a valid addon manifest: {e}", path.display())))?;
    if !is_valid_id(&manifest.id) {
        return Err(Error::msg(format!(
            "Addon id \"{}\" must be 1-64 lower-case letters, digits, '-' or '_'.",
            manifest.id
        )));
    }
    Ok(manifest)
}

/// Files below `dir`, relative to it with "\" separators.
fn files_below(dir: &Path) -> Vec<String> {
    fn walk(base: &Path, dir: &Path, out: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(kind) = entry.file_type() else { continue };
            if kind.is_dir() {
                walk(base, &path, out);
            } else if kind.is_file() {
                if let Ok(rel) = path.strip_prefix(base) {
                    out.push(rel.to_string_lossy().replace('/', "\\"));
                }
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out);
    out.sort();
    out
}

fn texture_files(addon: &Path) -> Vec<String> {
    files_below(&addon.join("textures"))
        .into_iter()
        .filter(|f| {
            Path::new(f)
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| TEXTURE_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
        })
        .collect()
}

fn problem(manifest: &AddonManifest, decomp_version: &str) -> String {
    if !KINDS.contains(&manifest.kind.as_str()) {
        return format!("This addon is a \"{}\", which needs a newer launcher.", manifest.kind);
    }
    if let (Some(range), false) = (&manifest.requires.decomp, decomp_version.is_empty()) {
        match (semver::VersionReq::parse(range), semver::Version::parse(decomp_version)) {
            (Ok(req), Ok(version)) if !req.matches(&version) => {
                return format!("Needs patch {range}; this is {decomp_version}.");
            }
            (Err(_), _) => return format!("Its required version \"{range}\" is not a version range."),
            _ => {}
        }
    }
    String::new()
}

pub fn list(folder: Option<&Path>, decomp_version: &str) -> Vec<AddonInfo> {
    let deployed = load_deployed();
    let here = folder.and_then(|f| deployed.get(&folder_key(f)));
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(addons_dir()) else { return out };
    for entry in entries.flatten() {
        let dir = entry.path();
        let Ok(manifest) = read_manifest(&dir) else { continue };
        if dir.file_name().and_then(|n| n.to_str()) != Some(manifest.id.as_str()) {
            continue;
        }
        let file_count = match manifest.kind.as_str() {
            "texture-pack" => texture_files(&dir).len(),
            "preset" => manifest.settings.len(),
            _ => 0,
        };
        out.push(AddonInfo {
            enabled: here.is_some_and(|d| d.contains_key(&manifest.id)),
            problem: problem(&manifest, decomp_version),
            path: dir.display().to_string(),
            file_count,
            manifest,
        });
    }
    out.sort_by(|a, b| a.manifest.name.to_lowercase().cmp(&b.manifest.name.to_lowercase()));
    out
}

/// Copies an addon (a folder with addon.json, or a .zip of one) into the addons folder. Returns its id.
pub fn import(source: &Path) -> Result<String> {
    if source.is_dir() {
        return import_folder(source);
    }
    if !source.extension().is_some_and(|e| e.eq_ignore_ascii_case("zip")) {
        return Err(Error::msg("Choose an addon folder (with addon.json) or a .zip of one."));
    }
    let staging = tempdir_in_state("import")?;
    let result = extract_zip(source, &staging).and_then(|()| {
        let dir = find_manifest_dir(&staging).ok_or_else(|| Error::msg("The zip has no addon.json."))?;
        import_folder(&dir)
    });
    let _ = std::fs::remove_dir_all(&staging);
    result
}

fn import_folder(dir: &Path) -> Result<String> {
    let manifest = read_manifest(dir)?;
    let target = addons_dir().join(&manifest.id);
    if crate::paths::same_path(dir, &target) {
        return Ok(manifest.id);
    }
    if target.exists() {
        std::fs::remove_dir_all(&target)?;
    }
    copy_tree(dir, &target)?;
    Ok(manifest.id)
}

/// Deletes an addon from the addons folder; it must not be enabled in any game folder.
pub fn delete(id: &str) -> Result<()> {
    if !is_valid_id(id) {
        return Err(Error::msg("Invalid addon id."));
    }
    if load_deployed().values().any(|d| d.contains_key(id)) {
        return Err(Error::msg("Disable the addon before removing it."));
    }
    std::fs::remove_dir_all(addons_dir().join(id))?;
    Ok(())
}

fn tempdir_in_state(name: &str) -> Result<PathBuf> {
    let dir = state_dir().join(name).join(format!(
        "{}-{}",
        std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos()
    ));
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn find_manifest_dir(dir: &Path) -> Option<PathBuf> {
    if dir.join(MANIFEST).is_file() {
        return Some(dir.to_path_buf());
    }
    // A zip of the addon folder itself: one level down.
    std::fs::read_dir(dir).ok()?.flatten().map(|e| e.path()).find(|p| p.join(MANIFEST).is_file())
}

fn extract_zip(zip: &Path, into: &Path) -> Result<()> {
    let file = std::fs::File::open(zip)?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| Error::msg(format!("Cannot read {}: {e}", zip.display())))?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| Error::msg(format!("Damaged zip: {e}")))?;
        // enclosed_name rejects absolute paths and "..": nothing is written outside `into`.
        let Some(relative) = entry.enclosed_name() else { continue };
        if entry.size() > LARGEST_ADDON_FILE {
            return Err(Error::msg(format!("{} is too large.", relative.display())));
        }
        let target = into.join(relative);
        if entry.is_dir() {
            std::fs::create_dir_all(&target)?;
            continue;
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut out = std::fs::File::create(&target)?;
        std::io::copy(&mut (&mut entry).take(LARGEST_ADDON_FILE + 1), &mut out)?;
    }
    Ok(())
}

use std::io::Read as _;

fn copy_tree(from: &Path, to: &Path) -> Result<()> {
    std::fs::create_dir_all(to)?;
    for relative in files_below(from) {
        let target = to.join(&relative);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(from.join(&relative), target)?;
    }
    Ok(())
}

/// Deploys an addon into `folder`.
pub fn enable(folder: &Path, id: &str, decomp_version: &str) -> Result<()> {
    if !is_valid_id(id) {
        return Err(Error::msg("Invalid addon id."));
    }
    let dir = addons_dir().join(id);
    let manifest = read_manifest(&dir)?;
    let why = problem(&manifest, decomp_version);
    if !why.is_empty() {
        return Err(Error::msg(why));
    }
    let mut deployed = load_deployed();
    if deployed.get(&folder_key(folder)).is_some_and(|d| d.contains_key(id)) {
        disable(folder, id)?;
        deployed = load_deployed();
    }
    let mut deployment = Deployment { version: manifest.version.clone(), ..Default::default() };
    match manifest.kind.as_str() {
        "texture-pack" => {
            let mut ops = Vec::new();
            for file in texture_files(&dir) {
                let to = format!(r"Textures\Replace\{id}\{file}");
                if !is_safe_relative_path(&to) {
                    continue;
                }
                ops.push(FileOp::Copy { from: dir.join("textures").join(&file), to: to.clone() });
                deployment.files.push(to);
            }
            if ops.is_empty() {
                return Err(Error::msg("This texture pack has no images in its textures folder."));
            }
            fsops::apply_with_rights(folder, ops)?;
        }
        "preset" => {
            let current = settings::read_ini_text(folder);
            let values = ini::parse(&current);
            let mut text = current.clone();
            for s in &manifest.settings {
                deployment.settings.push(SettingChange {
                    section: s.section.clone(),
                    key: s.key.clone(),
                    before: ini::get(&values, &s.section, &s.key).map(str::to_string),
                    after: s.value.clone(),
                });
                text = ini::set(&text, &s.section, &s.key, &s.value);
            }
            settings::write_ini_text(folder, &text)?;
        }
        other => return Err(Error::msg(format!("Unknown addon kind {other}."))),
    }
    deployed.entry(folder_key(folder)).or_default().insert(id.to_string(), deployment);
    save_deployed(&deployed)
}

/// Takes an addon out of `folder`: removes the files it wrote and puts back the settings it changed.
pub fn disable(folder: &Path, id: &str) -> Result<()> {
    let mut deployed = load_deployed();
    let key = folder_key(folder);
    let Some(deployment) = deployed.get(&key).and_then(|d| d.get(id)).cloned() else {
        return Ok(());
    };
    if !deployment.files.is_empty() {
        let mut ops: Vec<FileOp> = deployment.files.iter().map(|f| FileOp::Delete { path: f.clone() }).collect();
        let mut dirs: Vec<String> = Vec::new();
        for file in &deployment.files {
            let mut parent = Path::new(file).parent();
            while let Some(p) = parent.filter(|p| p.starts_with(format!(r"Textures\Replace\{id}"))) {
                dirs.push(p.to_string_lossy().into_owned());
                parent = p.parent();
            }
        }
        dirs.sort_by_key(|d| std::cmp::Reverse(d.len()));
        dirs.dedup();
        ops.extend(dirs.into_iter().map(|path| FileOp::RemoveDirIfEmpty { path }));
        fsops::apply_with_rights(folder, ops)?;
    }
    if !deployment.settings.is_empty() {
        let mut text = settings::read_ini_text(folder);
        let values = ini::parse(&text);
        let defaults = crate::payload::default_ini().map(ini::parse).unwrap_or_default();
        for change in &deployment.settings {
            // Only what still has the addon's value: a setting the player changed since stays.
            if ini::get(&values, &change.section, &change.key) != Some(change.after.as_str()) {
                continue;
            }
            let restore = change
                .before
                .clone()
                .or_else(|| ini::get(&defaults, &change.section, &change.key).map(str::to_string));
            if let Some(value) = restore {
                text = ini::set(&text, &change.section, &change.key, &value);
            }
        }
        settings::write_ini_text(folder, &text)?;
    }
    if let Some(d) = deployed.get_mut(&key) {
        d.remove(id);
    }
    save_deployed(&deployed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids() {
        assert!(is_valid_id("hd-skies_2"));
        for bad in ["", "Caps", "a b", "..", "a\\b", "a/b", &"x".repeat(65)] {
            assert!(!is_valid_id(bad), "{bad}");
        }
    }

    /// Deploys and removes both kinds in a scratch game folder, with launcher state in a scratch %APPDATA%.
    #[test]
    fn enable_and_disable() {
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("APPDATA", dir.path().join("appdata"));
        let game = dir.path().join("game");
        std::fs::create_dir_all(&game).unwrap();
        std::fs::write(game.join("BWDecomp.ini"), "; c\r\n[Display]\r\nAntiAliasing=0\r\n").unwrap();

        let source = dir.path().join("src");
        std::fs::create_dir_all(source.join(r"pack\textures\data")).unwrap();
        std::fs::write(source.join(r"pack\textures\data\sky.png"), b"png").unwrap();
        std::fs::write(source.join(r"pack\textures\notes.txt"), b"ignored").unwrap();
        std::fs::write(source.join(r"pack\addon.json"), r#"{"id":"pack","name":"Pack","version":"1","kind":"texture-pack"}"#).unwrap();
        std::fs::create_dir_all(source.join("preset")).unwrap();
        std::fs::write(
            source.join(r"preset\addon.json"),
            r#"{"id":"preset","name":"P","version":"1","kind":"preset","settings":[
                {"section":"Display","key":"AntiAliasing","value":"8"},{"section":"Display","key":"VSync","value":"0"}]}"#,
        )
        .unwrap();
        assert_eq!(import(&source.join("pack")).unwrap(), "pack");
        assert_eq!(import(&source.join("preset")).unwrap(), "preset");
        assert_eq!(list(Some(&game), "0.11.0").len(), 2);

        enable(&game, "pack", "0.11.0").unwrap();
        let png = game.join(r"Textures\Replace\pack\data\sky.png");
        assert!(png.exists());
        assert!(!game.join(r"Textures\Replace\pack\notes.txt").exists());
        assert!(delete("pack").is_err(), "enabled addons cannot be deleted");
        disable(&game, "pack").unwrap();
        assert!(!game.join(r"Textures\Replace\pack").exists());

        enable(&game, "preset", "0.11.0").unwrap();
        let text = std::fs::read_to_string(game.join("BWDecomp.ini")).unwrap();
        assert!(text.starts_with("; c\r\n") && text.contains("AntiAliasing=8") && text.contains("VSync=0"));
        disable(&game, "preset").unwrap();
        let values = ini::parse(&std::fs::read_to_string(game.join("BWDecomp.ini")).unwrap());
        assert_eq!(ini::get(&values, "Display", "AntiAliasing"), Some("0"));

        delete("pack").unwrap();
        assert_eq!(list(Some(&game), "0.11.0").len(), 1);
    }

    #[test]
    fn compatibility() {
        let mut m: AddonManifest = serde_json::from_str(
            r#"{"id":"x","name":"X","version":"1","kind":"texture-pack","requires":{"decomp":">=0.10.0"}}"#,
        )
        .unwrap();
        assert_eq!(problem(&m, "0.11.0"), "");
        assert!(problem(&m, "0.9.0").contains("Needs"));
        m.kind = "script".into();
        assert!(problem(&m, "0.11.0").contains("newer launcher"));
    }
}
