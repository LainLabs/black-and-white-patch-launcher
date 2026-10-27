//! Copying the patch into a game folder and removing it again, compatible with the decomp's setup program
//! (src/setup/Install.cpp): the same files, the same manifest `BWDecomp\Installed.txt`, the same rules. A folder
//! installed by BWDecompSetup.exe is updated or removed by the launcher and the other way round.
//!
//! Manifest: `# Black & White decompilation <version>`, then one line per file: `<sha256> R|K <path>`. R files are
//! replaced on update; K files (BWDecomp.ini) are installed only when missing and removed only when unchanged.
//! The header keeps the word "decompilation" although the launcher says "patch": it is the setup program's format.

use std::io::Write as _;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Context, Error, Result};
use crate::game::{file_sha256, sha256_hex};
use crate::paths::is_safe_relative_path;

pub const DATA_FOLDER: &str = "BWDecomp";
pub const MANIFEST_PATH: &str = r"BWDecomp\Installed.txt";
/// BWDecompSetup's uninstaller; a manifest listing it came from the setup program.
pub const LEGACY_UNINSTALLER: &str = r"BWDecomp\Uninstall.exe";

/// The player's settings: installed only when missing, removed only when unchanged. scripts/sync-payload.ps1 marks
/// the same file in the bundled patch; downloaded patches use this.
pub fn is_keep_file(path: &str) -> bool {
    path.eq_ignore_ascii_case("BWDecomp.ini")
}
/// Written by BWDecomp.dll, not installed; removed with it.
const GENERATED_FILES: &[&str] = &["BWDecomp.log", "BWDecomp.directx.csv"];
const MANIFEST_HEADER: &str = "# Black & White decompilation ";
const LARGEST_INSTALLED_FILE: u64 = 256 << 20;

/// One file to install.
pub struct InstallFile<'a> {
    pub path: &'a str,
    pub keep: bool,
    pub data: &'a [u8],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestEntry {
    pub path: String,
    pub sha256: String,
    pub keep: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Manifest {
    /// Empty when the manifest does not say.
    pub version: String,
    pub entries: Vec<ManifestEntry>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveResult {
    /// Keep files the player changed, relative to the folder.
    pub kept: Vec<String>,
}

fn find<'a>(entries: &'a [ManifestEntry], path: &str) -> Option<&'a ManifestEntry> {
    entries.iter().find(|e| e.path.eq_ignore_ascii_case(path))
}

fn file_matches(file: &Path, sha256: &str) -> bool {
    file_sha256(file, LARGEST_INSTALLED_FILE).is_some_and(|actual| actual == sha256)
}

fn clear_read_only(path: &Path) {
    if let Ok(meta) = std::fs::metadata(path) {
        let mut perms = meta.permissions();
        if perms.readonly() {
            #[allow(clippy::permissions_set_readonly_false)]
            perms.set_readonly(false);
            let _ = std::fs::set_permissions(path, perms);
        }
    }
}

/// Writes `data` next to `target` and moves it over `target`, so a failure never leaves a half-written file.
pub fn write_replacing(target: &Path, data: &[u8]) -> Result<()> {
    let mut temporary = target.as_os_str().to_owned();
    temporary.push(".setup-new");
    let temporary = PathBuf::from(temporary);
    let written = (|| {
        let mut file = std::fs::File::create(&temporary)?;
        file.write_all(data)?;
        file.sync_all()
    })();
    if let Err(e) = written {
        let _ = std::fs::remove_file(&temporary);
        return Err(e).context("Cannot write", &temporary);
    }
    clear_read_only(target);
    if let Err(e) = move_replacing(&temporary, target) {
        let _ = std::fs::remove_file(&temporary);
        return Err(e).context("Cannot replace", target);
    }
    Ok(())
}

#[cfg(windows)]
fn move_replacing(from: &Path, to: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH};
    let wide = |p: &Path| p.as_os_str().encode_wide().chain(std::iter::once(0)).collect::<Vec<u16>>();
    let ok = unsafe {
        MoveFileExW(wide(from).as_ptr(), wide(to).as_ptr(), MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH)
    };
    if ok == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// Deletes `file`; Ok when it is gone (or never existed).
fn delete(file: &Path) -> std::io::Result<()> {
    clear_read_only(file);
    match std::fs::remove_file(file) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}

fn write_manifest(folder: &Path, manifest: &Manifest) -> Result<()> {
    let mut text = format!("{MANIFEST_HEADER}{}\r\n", manifest.version);
    text.push_str("# Files setup installed: SHA-256, R (replaced on update) or K (kept when changed), path.\r\n");
    for entry in &manifest.entries {
        text.push_str(&format!("{} {} {}\r\n", entry.sha256, if entry.keep { 'K' } else { 'R' }, entry.path));
    }
    let path = folder.join(MANIFEST_PATH);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    write_replacing(&path, text.as_bytes())
}

/// The manifest in `folder`; None when there is none (not installed, or installed by hand).
pub fn read_manifest(folder: &Path) -> Option<Manifest> {
    let text = std::fs::read(folder.join(MANIFEST_PATH)).ok()?;
    let text = String::from_utf8_lossy(&text);
    let mut manifest = Manifest::default();
    for line in text.lines() {
        let line = line.trim_end_matches('\r');
        if let Some(version) = line.strip_prefix(MANIFEST_HEADER) {
            manifest.version = version.to_string();
            continue;
        }
        let bytes = line.as_bytes();
        let is_hex = |s: &str| s.len() == 64 && s.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c));
        if bytes.len() < 68 || !line.get(..64).is_some_and(is_hex) || bytes[64] != b' ' || bytes[66] != b' ' {
            continue;
        }
        let keep = match bytes[65] {
            b'K' => true,
            b'R' => false,
            _ => continue,
        };
        let path = &line[67..];
        if is_safe_relative_path(path) {
            manifest.entries.push(ManifestEntry { path: path.to_string(), sha256: line[..64].to_string(), keep });
        }
    }
    Some(manifest)
}

/// Writes `files` into `folder` and records them in the manifest under `version`. Files a previous version installed
/// and this one does not are removed (unless the player changed a Keep file).
pub fn install_files(folder: &Path, files: &[InstallFile], version: &str) -> Result<()> {
    if !folder.is_dir() {
        return Err(Error::msg(format!("The folder {} does not exist.", folder.display())));
    }
    let previous = read_manifest(folder).unwrap_or_default();
    let mut manifest = Manifest { version: version.to_string(), entries: Vec::new() };

    let result = (|| -> Result<()> {
        for file in files {
            if !is_safe_relative_path(file.path) {
                return Err(Error::msg(format!("Invalid file name in the payload: {}", file.path)));
            }
            let target = folder.join(file.path);
            if file.keep && target.exists() {
                if let Some(entry) = find(&previous.entries, file.path) {
                    manifest.entries.push(entry.clone());
                }
                continue;
            }
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).context("Cannot create the folder", parent)?;
            }
            write_replacing(&target, file.data)?;
            manifest.entries.push(ManifestEntry {
                path: file.path.to_string(),
                sha256: sha256_hex(file.data),
                keep: file.keep,
            });
        }
        // Files an earlier version installed and this one does not.
        for entry in &previous.entries {
            if files.iter().any(|f| f.path.eq_ignore_ascii_case(&entry.path)) {
                continue;
            }
            let target = folder.join(&entry.path);
            if (entry.keep && target.exists() && !file_matches(&target, &entry.sha256)) || delete(&target).is_err() {
                manifest.entries.push(entry.clone());
            }
        }
        Ok(())
    })();

    if let Err(error) = result {
        // The manifest still lists every file this or the previous version wrote.
        for entry in &previous.entries {
            if find(&manifest.entries, &entry.path).is_none() {
                manifest.entries.push(entry.clone());
            }
        }
        let _ = write_manifest(folder, &manifest);
        return Err(error);
    }
    write_manifest(folder, &manifest)
}

/// Removes what the manifest lists, the files BWDecomp.dll writes and the data folder if it is then empty.
pub fn remove_files(folder: &Path) -> Result<RemoveResult> {
    let manifest = read_manifest(folder).ok_or_else(|| {
        Error::msg(format!(
            "The patch is not installed in {} (there is no {MANIFEST_PATH}).",
            folder.display()
        ))
    })?;
    let mut result = RemoveResult::default();
    for (i, entry) in manifest.entries.iter().enumerate() {
        let target = folder.join(&entry.path);
        if !target.exists() {
            continue;
        }
        if entry.keep && !file_matches(&target, &entry.sha256) {
            result.kept.push(entry.path.clone());
            continue;
        }
        if let Err(e) = delete(&target) {
            // Leave a manifest listing what is still there, so removal can be retried.
            let remaining = Manifest { version: manifest.version.clone(), entries: manifest.entries[i..].to_vec() };
            let _ = write_manifest(folder, &remaining);
            return Err(e).context("Cannot delete", &target);
        }
    }
    for generated in GENERATED_FILES {
        let _ = delete(&folder.join(generated));
    }
    delete(&folder.join(MANIFEST_PATH)).context("Cannot delete", &folder.join(MANIFEST_PATH))?;

    // Now empty folders, deepest first (remove_dir fails, as intended, unless empty).
    let mut folders = vec![folder.join(DATA_FOLDER)];
    for entry in &manifest.entries {
        let mut parent = Path::new(&entry.path).parent();
        while let Some(p) = parent.filter(|p| !p.as_os_str().is_empty()) {
            folders.push(folder.join(p));
            parent = p.parent();
        }
    }
    folders.sort_by_key(|p| std::cmp::Reverse(p.as_os_str().len()));
    for path in folders {
        let _ = std::fs::remove_dir(path);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files<'a>(dll: &'a [u8], ini: &'a [u8]) -> Vec<InstallFile<'a>> {
        vec![
            InstallFile { path: "BWDecomp.dll", keep: false, data: dll },
            InstallFile { path: "BWDecomp.ini", keep: true, data: ini },
            InstallFile { path: r"BWDecomp\README.txt", keep: false, data: b"readme" },
        ]
    }

    #[test]
    fn install_update_remove() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path();
        install_files(folder, &files(b"dll 1", b"ini 1"), "0.1.0").unwrap();
        assert_eq!(std::fs::read(folder.join("BWDecomp.dll")).unwrap(), b"dll 1");
        let manifest = read_manifest(folder).unwrap();
        assert_eq!(manifest.version, "0.1.0");
        assert_eq!(manifest.entries.len(), 3);

        // The player changes the settings; an update replaces the DLL and keeps them.
        std::fs::write(folder.join("BWDecomp.ini"), b"mine").unwrap();
        install_files(folder, &files(b"dll 2", b"ini 2"), "0.2.0").unwrap();
        assert_eq!(std::fs::read(folder.join("BWDecomp.dll")).unwrap(), b"dll 2");
        assert_eq!(std::fs::read(folder.join("BWDecomp.ini")).unwrap(), b"mine");

        std::fs::write(folder.join("BWDecomp.log"), b"log").unwrap();
        let removed = remove_files(folder).unwrap();
        assert_eq!(removed.kept, vec!["BWDecomp.ini".to_string()]);
        assert!(!folder.join("BWDecomp.dll").exists());
        assert!(!folder.join("BWDecomp.log").exists());
        assert!(!folder.join(DATA_FOLDER).exists());
        assert!(folder.join("BWDecomp.ini").exists());
    }

    #[test]
    fn unchanged_keep_file_is_removed() {
        let dir = tempfile::tempdir().unwrap();
        install_files(dir.path(), &files(b"dll", b"ini"), "1").unwrap();
        assert!(remove_files(dir.path()).unwrap().kept.is_empty());
        assert!(!dir.path().join("BWDecomp.ini").exists());
    }

    #[test]
    fn takes_over_a_setup_installation() {
        // A manifest as BWDecompSetup.exe writes it, with its uninstaller.
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path();
        std::fs::create_dir_all(folder.join(DATA_FOLDER)).unwrap();
        std::fs::write(folder.join(LEGACY_UNINSTALLER), b"uninstaller").unwrap();
        let manifest = format!(
            "# Black & White decompilation 0.11.0\r\n# comment\r\n{} R {}\r\n",
            sha256_hex(b"uninstaller"),
            LEGACY_UNINSTALLER
        );
        std::fs::write(folder.join(MANIFEST_PATH), manifest).unwrap();
        assert_eq!(read_manifest(folder).unwrap().entries[0].path, LEGACY_UNINSTALLER);

        install_files(folder, &files(b"dll", b"ini"), "0.12.0").unwrap();
        assert!(!folder.join(LEGACY_UNINSTALLER).exists());
        assert!(find(&read_manifest(folder).unwrap().entries, LEGACY_UNINSTALLER).is_none());
    }

    #[test]
    fn manifest_paths_outside_the_folder_are_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let outside = dir.path().join("outside.txt");
        std::fs::write(&outside, b"x").unwrap();
        let folder = dir.path().join("game");
        std::fs::create_dir_all(folder.join(DATA_FOLDER)).unwrap();
        let manifest = format!("# Black & White decompilation 1\r\n{} R ..\\outside.txt\r\n", sha256_hex(b"x"));
        std::fs::write(folder.join(MANIFEST_PATH), manifest).unwrap();
        assert!(read_manifest(&folder).unwrap().entries.is_empty());
        remove_files(&folder).unwrap();
        assert!(outside.exists());
    }
}
