//! Update checks against GitHub releases, for two products in two repositories:
//!
//! - the **launcher** (`state::DEFAULT_LAUNCHER_REPO`): `BWLauncher-<version>-Setup.exe`, written by
//!   scripts/package.ps1, is downloaded and run;
//! - the **patch** (`state::DEFAULT_PATCH_REPO`): `BWDecomp-<version>.zip`, written by the patch's package.ps1, is
//!   downloaded and installed into the game folder, as the bundled patch is (`Request::InstallZip`).
//!
//! Either download is checked against `<prefix><version>-SHA256SUMS.txt` from the same release first. A repository
//! may publish other files and releases, so the newest non-draft, non-prerelease release carrying the product's file
//! is used, and its version is read from that file's name rather than the tag.

use std::io::Read as _;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::game::sha256_hex;
use crate::install;
use crate::paths::{is_safe_relative_path, state_dir};

const LARGEST_DOWNLOAD: u64 = 512 << 20;
const LARGEST_PATCH_FILE: u64 = 64 << 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Product {
    Launcher,
    Patch,
}

impl Product {
    fn prefix(self) -> &'static str {
        match self {
            Product::Launcher => "BWLauncher-",
            Product::Patch => "BWDecomp-",
        }
    }

    /// What follows the version in the name of the file that is downloaded.
    fn suffix(self) -> &'static str {
        match self {
            Product::Launcher => "-Setup.exe",
            Product::Patch => ".zip",
        }
    }

    fn noun(self) -> &'static str {
        match self {
            Product::Launcher => "launcher",
            Product::Patch => "patch",
        }
    }

    fn file_name(self, version: &semver::Version) -> String {
        format!("{}{version}{}", self.prefix(), self.suffix())
    }

    fn sums_name(self, version: &semver::Version) -> String {
        format!("{}{version}-SHA256SUMS.txt", self.prefix())
    }

    /// The version in this product's download name, e.g. 0.11.1 in `BWDecomp-0.11.1.zip`. Only plain versions
    /// count, so `BWDecomp-0.11.1-Release.zip` (the patch's combined download) is not mistaken for one.
    fn version_in(self, asset_name: &str) -> Option<semver::Version> {
        let version = asset_name.strip_prefix(self.prefix())?.strip_suffix(self.suffix())?;
        semver::Version::parse(version).ok().filter(|v| v.pre.is_empty() && v.build.is_empty())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    pub name: String,
    pub url: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub product: Product,
    pub current_version: String,
    pub latest_version: String,
    pub newer: bool,
    pub name: String,
    pub notes: String,
    pub page_url: String,
    pub published_at: String,
    /// The setup program (launcher) or zip (patch).
    pub download: Option<Asset>,
    pub sums: Option<Asset>,
}

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    body: Option<String>,
    html_url: String,
    #[serde(default)]
    published_at: Option<String>,
    #[serde(default)]
    assets: Vec<GitHubAsset>,
}

#[derive(Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
    size: u64,
}

pub fn is_valid_repo(repo: &str) -> bool {
    let ok = |s: &str| {
        !s.is_empty() && s.len() <= 100 && s.bytes().all(|c| c.is_ascii_alphanumeric() || b"-_.".contains(&c))
    };
    matches!(repo.split_once('/'), Some((owner, name)) if ok(owner) && ok(name))
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent(concat!("BWLauncher/", env!("CARGO_PKG_VERSION")))
        .build()
}

/// The newest published, non-prerelease release that carries `product`'s download, with that download's version.
fn pick_release(releases: &[GitHubRelease], product: Product) -> Option<(&GitHubRelease, semver::Version)> {
    releases
        .iter()
        .filter(|r| !r.draft && !r.prerelease)
        .filter_map(|r| r.assets.iter().filter_map(|a| product.version_in(&a.name)).max().map(|v| (r, v)))
        .max_by(|a, b| a.1.cmp(&b.1))
}

/// The newest `product` release in `repo`, compared with `current` (the version in use).
pub fn check(repo: &str, product: Product, current: &str) -> Result<UpdateInfo> {
    if !is_valid_repo(repo) {
        return Err(Error::msg(format!("Set the {} repository as owner/name (GitHub).", product.noun())));
    }
    let url = format!("https://api.github.com/repos/{repo}/releases?per_page=30");
    let releases: Vec<GitHubRelease> = agent()
        .get(&url)
        .set("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| match e {
            // GitHub answers 404 for private repositories as well as missing ones.
            ureq::Error::Status(404, _) => {
                Error::msg(format!("{repo} was not found on GitHub, or it is private (updates need a public repository)."))
            }
            other => Error::msg(format!("Cannot reach GitHub: {other}")),
        })?
        .into_json()
        .map_err(|e| Error::msg(format!("Unexpected answer from GitHub: {e}")))?;
    let (release, latest) = pick_release(&releases, product)
        .ok_or_else(|| Error::msg(format!("{repo} has no {} release yet.", product.noun())))?;
    let newer = semver::Version::parse(current).map_or(true, |c| latest > c);
    let asset = |name: String| {
        release
            .assets
            .iter()
            .find(|a| a.name == name)
            .map(|a| Asset { name: a.name.clone(), url: a.browser_download_url.clone(), size: a.size })
    };
    Ok(UpdateInfo {
        product,
        current_version: current.to_string(),
        newer,
        name: release.name.clone().filter(|n| !n.is_empty()).unwrap_or_else(|| release.tag_name.clone()),
        notes: release.body.clone().unwrap_or_default(),
        page_url: release.html_url.clone(),
        published_at: release.published_at.clone().unwrap_or_default(),
        download: asset(product.file_name(&latest)),
        sums: asset(product.sums_name(&latest)),
        latest_version: latest.to_string(),
    })
}

fn download(url: &str, largest: u64) -> Result<Vec<u8>> {
    if !url.starts_with("https://github.com/") {
        return Err(Error::msg(format!("Refusing to download from {url}")));
    }
    let response = agent().get(url).call().map_err(|e| Error::msg(format!("Download failed: {e}")))?;
    let mut data = Vec::new();
    response.into_reader().take(largest + 1).read_to_end(&mut data)?;
    if data.len() as u64 > largest {
        return Err(Error::msg("The download is larger than expected."));
    }
    Ok(data)
}

/// The hash `sums` (a SHA256SUMS file: `<hash> *<name>` lines) gives for `name`.
fn expected_sum(sums: &str, name: &str) -> Option<String> {
    sums.lines()
        .filter_map(|l| l.split_once(' '))
        .find(|(_, n)| n.trim().trim_start_matches('*') == name)
        .map(|(hash, _)| hash.trim().to_lowercase())
}

/// Downloads the release's file, checks it against SHA256SUMS and saves it in the launcher's updates folder.
fn download_verified(info: &UpdateInfo) -> Result<PathBuf> {
    let noun = info.product.noun();
    let file = info.download.as_ref().ok_or_else(|| Error::msg(format!("This release has no {noun} download.")))?;
    let sums =
        info.sums.as_ref().ok_or_else(|| Error::msg("This release has no SHA256SUMS file to check the download with."))?;
    if file.name.contains(['\\', '/', ':']) {
        return Err(Error::msg("Invalid asset name."));
    }
    let sums_text = String::from_utf8_lossy(&download(&sums.url, 1 << 20)?).into_owned();
    let expected = expected_sum(&sums_text, &file.name)
        .ok_or_else(|| Error::msg(format!("SHA256SUMS does not list {}.", file.name)))?;
    let data = download(&file.url, LARGEST_DOWNLOAD)?;
    if sha256_hex(&data) != expected {
        return Err(Error::msg(format!("{} does not match its SHA-256 sum; it was not used.", file.name)));
    }
    let dir = state_dir().join("updates");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(&file.name);
    std::fs::write(&path, data)?;
    Ok(path)
}

/// The new launcher's setup program, verified.
pub fn download_installer(info: &UpdateInfo) -> Result<PathBuf> {
    if info.product != Product::Launcher {
        return Err(Error::msg("This is not a launcher release."));
    }
    download_verified(info)
}

/// The new patch's zip, verified and checked to hold a patch of the release's version.
pub fn download_patch(info: &UpdateInfo) -> Result<PathBuf> {
    if info.product != Product::Patch {
        return Err(Error::msg("This is not a patch release."));
    }
    let zip = download_verified(info)?;
    patch_files(&std::fs::read(&zip)?, &info.latest_version)?;
    Ok(zip)
}

/// The files in a patch zip (`BWDecomp-<version>/<path>`, the layout its setup program installs), as game-folder
/// paths with "\" and their contents. Refuses anything that is not a whole patch of `version`.
pub fn patch_files(zip: &[u8], version: &str) -> Result<Vec<(String, Vec<u8>)>> {
    let top = format!("BWDecomp-{version}/");
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(zip))
        .map_err(|e| Error::msg(format!("The patch download is not a valid zip: {e}")))?;
    let mut files = Vec::new();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| Error::msg(format!("Damaged patch zip: {e}")))?;
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().replace('\\', "/");
        let Some(relative) = name.strip_prefix(&top) else {
            return Err(Error::msg(format!("The patch zip has {name} outside {top}.")));
        };
        let path = relative.replace('/', "\\");
        if !is_safe_relative_path(&path) {
            return Err(Error::msg(format!("The patch zip has an invalid file name: {name}")));
        }
        if entry.size() > LARGEST_PATCH_FILE {
            return Err(Error::msg(format!("{name} in the patch zip is too large.")));
        }
        let mut data = Vec::new();
        (&mut entry).take(LARGEST_PATCH_FILE + 1).read_to_end(&mut data)?;
        files.push((path, data));
    }
    for required in ["BWLaunch.exe", "BWDecomp.dll", "BWDecomp.ini"] {
        if !files.iter().any(|(p, _)| p.eq_ignore_ascii_case(required)) {
            return Err(Error::msg(format!("The patch zip has no {required}.")));
        }
    }
    Ok(files)
}

/// Installs a downloaded patch zip into `folder` (in the elevated copy too). BWDecomp.ini is a Keep file, as in the
/// bundled patch: installed when missing, never overwritten.
pub fn install_patch_zip(folder: &std::path::Path, zip: &std::path::Path, version: &str) -> Result<()> {
    let files = patch_files(&std::fs::read(zip)?, version)?;
    let list: Vec<install::InstallFile> = files
        .iter()
        .map(|(path, data)| install::InstallFile { path, keep: install::is_keep_file(path), data })
        .collect();
    install::install_files(folder, &list, version)
}

#[cfg(test)]
mod tests {
    use std::io::Write as _;

    use super::{expected_sum, is_valid_repo, patch_files, pick_release, GitHubRelease, Product};

    fn release(tag: &str, draft: bool, prerelease: bool, assets: &[&str]) -> GitHubRelease {
        serde_json::from_value(serde_json::json!({
            "tag_name": tag, "draft": draft, "prerelease": prerelease, "html_url": "https://github.com/o/r",
            "assets": assets.iter().map(|n| serde_json::json!({
                "name": n, "browser_download_url": format!("https://github.com/o/r/{n}"), "size": 1
            })).collect::<Vec<_>>(),
        }))
        .unwrap()
    }

    #[test]
    fn picks_newest_launcher_release() {
        let releases = [
            // A patch release: higher tag, no launcher setup.
            release("v0.12.0", false, false, &["BWDecomp-0.12.0.zip"]),
            release("v0.3.0-beta", false, true, &["BWLauncher-0.3.0-Setup.exe"]),
            release("v0.2.0", true, false, &["BWLauncher-0.2.5-Setup.exe"]),
            release("launcher-0.2.0", false, false, &["BWLauncher-0.2.0-Setup.exe", "BWLauncher-0.2.0-SHA256SUMS.txt"]),
            release("launcher-0.1.0", false, false, &["BWLauncher-0.1.0-Setup.exe"]),
        ];
        let (picked, version) = pick_release(&releases, Product::Launcher).unwrap();
        assert_eq!(picked.tag_name, "launcher-0.2.0");
        assert_eq!(version.to_string(), "0.2.0");
        assert!(pick_release(&releases[..1], Product::Launcher).is_none());
    }

    #[test]
    fn picks_newest_patch_release() {
        // As the patch tool publishes them: setup, zip, combined zip and sums.
        let releases = [
            release("launcher-v0.3.0", false, false, &["BWLauncher-0.3.0-Setup.exe"]),
            release("v0.11.1", false, false, &[
                "BWDecomp-0.11.1-Release.zip", "BWDecomp-0.11.1-Setup.exe", "BWDecomp-0.11.1.zip",
                "BWDecomp-0.11.1-SHA256SUMS.txt",
            ]),
            release("v0.11.0", false, false, &["BWDecomp-0.11.0.zip"]),
        ];
        let (picked, version) = pick_release(&releases, Product::Patch).unwrap();
        assert_eq!(picked.tag_name, "v0.11.1");
        assert_eq!(version.to_string(), "0.11.1");
        assert_eq!(Product::Patch.file_name(&version), "BWDecomp-0.11.1.zip");
        assert_eq!(Product::Patch.sums_name(&version), "BWDecomp-0.11.1-SHA256SUMS.txt");
        // Only the combined download: no plain zip to install.
        assert!(pick_release(&[release("v1", false, false, &["BWDecomp-1.0.0-Release.zip"])], Product::Patch).is_none());
    }

    #[test]
    fn sums() {
        let text = "aa11 *BWDecomp-0.11.1-Setup.exe\r\nBB22 *BWDecomp-0.11.1.zip\r\n";
        assert_eq!(expected_sum(text, "BWDecomp-0.11.1.zip").as_deref(), Some("bb22"));
        assert_eq!(expected_sum(text, "BWDecomp-0.11.1-Release.zip"), None);
    }

    fn zip_of(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        for (name, data) in entries {
            writer.start_file(*name, zip::write::SimpleFileOptions::default()).unwrap();
            writer.write_all(data).unwrap();
        }
        writer.finish().unwrap().into_inner()
    }

    #[test]
    fn patch_zip() {
        let good = zip_of(&[
            ("BWDecomp-1.2.3/BWLaunch.exe", b"exe"),
            ("BWDecomp-1.2.3/BWDecomp.dll", b"dll"),
            ("BWDecomp-1.2.3/BWDecomp.ini", b"ini"),
            ("BWDecomp-1.2.3/BWDecomp/FIXES.md", b"md"),
        ]);
        let files = patch_files(&good, "1.2.3").unwrap();
        assert!(files.iter().any(|(p, d)| p == r"BWDecomp\FIXES.md" && d == b"md"));
        // Another version's zip, a file outside the folder, a missing DLL.
        assert!(patch_files(&good, "1.2.4").is_err());
        let outside = zip_of(&[("BWDecomp-1.2.3/BWLaunch.exe", b"exe"), ("evil.dll", b"x")]);
        assert!(patch_files(&outside, "1.2.3").is_err());
        let partial = zip_of(&[("BWDecomp-1.2.3/BWLaunch.exe", b"exe"), ("BWDecomp-1.2.3/BWDecomp.ini", b"ini")]);
        assert!(patch_files(&partial, "1.2.3").is_err());
    }

    #[test]
    fn repos() {
        assert!(is_valid_repo("konstantinev/BlackWhiteLauncher"));
        for bad in ["", "x", "a/b/c", "a/", "/b", "a b/c", "a/b?x=1"] {
            assert!(!is_valid_repo(bad), "{bad}");
        }
    }
}
