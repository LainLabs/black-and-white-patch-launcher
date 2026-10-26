//! Update checks against GitHub releases. A launcher release carries the decompilation it installs, so one update
//! brings both. The installer asset is verified against the release's SHA256SUMS file before it runs.
//!
//! Release assets expected (written by scripts/package.ps1): `BWLauncher-<version>-Setup.exe` and
//! `BWLauncher-<version>-SHA256SUMS.txt`. The repository may also publish other releases (the decompilation's own
//! zips), so the newest release carrying a launcher setup program is used, and its version is read from the asset
//! name rather than the tag.

use std::io::Read as _;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::game::sha256_hex;
use crate::paths::state_dir;

const LARGEST_DOWNLOAD: u64 = 512 << 20;

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
    pub current_version: String,
    pub latest_version: String,
    pub newer: bool,
    pub name: String,
    pub notes: String,
    pub page_url: String,
    pub published_at: String,
    pub installer: Option<Asset>,
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

/// The launcher version in `BWLauncher-<version>-Setup.exe`.
fn setup_version(asset_name: &str) -> Option<semver::Version> {
    let version = asset_name.strip_prefix("BWLauncher-")?.strip_suffix("-Setup.exe")?;
    semver::Version::parse(version).ok()
}

/// The newest published, non-prerelease release that carries a launcher setup program, with that program's version.
fn pick_release(releases: &[GitHubRelease]) -> Option<(&GitHubRelease, semver::Version)> {
    releases
        .iter()
        .filter(|r| !r.draft && !r.prerelease)
        .filter_map(|r| r.assets.iter().filter_map(|a| setup_version(&a.name)).max().map(|v| (r, v)))
        .max_by(|a, b| a.1.cmp(&b.1))
}

pub fn check(repo: &str) -> Result<UpdateInfo> {
    if !is_valid_repo(repo) {
        return Err(Error::msg("Set the update repository as owner/name (GitHub)."));
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
    let (release, latest) =
        pick_release(&releases).ok_or_else(|| Error::msg(format!("{repo} has no launcher release yet.")))?;
    let current = env!("CARGO_PKG_VERSION").to_string();
    let newer = semver::Version::parse(&current).is_ok_and(|c| latest > c);
    let asset = |name: String| {
        release
            .assets
            .iter()
            .find(|a| a.name == name)
            .map(|a| Asset { name: a.name.clone(), url: a.browser_download_url.clone(), size: a.size })
    };
    Ok(UpdateInfo {
        current_version: current,
        newer,
        name: release.name.clone().filter(|n| !n.is_empty()).unwrap_or_else(|| release.tag_name.clone()),
        notes: release.body.clone().unwrap_or_default(),
        page_url: release.html_url.clone(),
        published_at: release.published_at.clone().unwrap_or_default(),
        installer: asset(format!("BWLauncher-{latest}-Setup.exe")),
        sums: asset(format!("BWLauncher-{latest}-SHA256SUMS.txt")),
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

/// Downloads the installer, checks it against SHA256SUMS and returns its path.
pub fn download_installer(info: &UpdateInfo) -> Result<PathBuf> {
    let installer =
        info.installer.as_ref().ok_or_else(|| Error::msg("This release has no launcher setup program."))?;
    let sums =
        info.sums.as_ref().ok_or_else(|| Error::msg("This release has no SHA256SUMS file to check it with."))?;
    if installer.name.contains(['\\', '/', ':']) {
        return Err(Error::msg("Invalid asset name."));
    }
    let sums_text = String::from_utf8_lossy(&download(&sums.url, 1 << 20)?).into_owned();
    let expected = sums_text
        .lines()
        .filter_map(|l| l.split_once(' '))
        .find(|(_, name)| name.trim().trim_start_matches('*') == installer.name)
        .map(|(hash, _)| hash.trim().to_lowercase())
        .ok_or_else(|| Error::msg("SHA256SUMS does not list the setup program."))?;
    let data = download(&installer.url, LARGEST_DOWNLOAD)?;
    if sha256_hex(&data) != expected {
        return Err(Error::msg("The downloaded setup program does not match its SHA-256 sum; it was not run."));
    }
    let dir = state_dir().join("updates");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(&installer.name);
    std::fs::write(&path, data)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::{is_valid_repo, pick_release, GitHubRelease};

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
            // A decompilation release in the same repository: higher tag, no launcher setup.
            release("v0.12.0", false, false, &["BWDecomp-0.12.0.zip"]),
            release("v0.3.0-beta", false, true, &["BWLauncher-0.3.0-Setup.exe"]),
            release("v0.2.0", true, false, &["BWLauncher-0.2.5-Setup.exe"]),
            release("launcher-0.2.0", false, false, &["BWLauncher-0.2.0-Setup.exe", "BWLauncher-0.2.0-SHA256SUMS.txt"]),
            release("launcher-0.1.0", false, false, &["BWLauncher-0.1.0-Setup.exe"]),
        ];
        let (picked, version) = pick_release(&releases).unwrap();
        assert_eq!(picked.tag_name, "launcher-0.2.0");
        assert_eq!(version.to_string(), "0.2.0");
        assert!(pick_release(&releases[..1]).is_none());
    }

    #[test]
    fn repos() {
        assert!(is_valid_repo("konstantinev/BlackWhiteLauncher"));
        for bad in ["", "x", "a/b/c", "a/", "/b", "a b/c", "a/b?x=1"] {
            assert!(!is_valid_repo(bad), "{bad}");
        }
    }
}
