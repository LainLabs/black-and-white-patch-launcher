//! Update checks against GitHub releases. A launcher release carries the decompilation it installs, so one update
//! brings both. The installer asset is verified against the release's SHA256SUMS file before it runs.
//!
//! Release assets expected (written by scripts/package.ps1): `BWLauncher-<version>-Setup.exe` and
//! `BWLauncher-<version>-SHA256SUMS.txt`.

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

pub fn check(repo: &str) -> Result<UpdateInfo> {
    if !is_valid_repo(repo) {
        return Err(Error::msg("Set the update repository as owner/name (GitHub)."));
    }
    let url = format!("https://api.github.com/repos/{repo}/releases/latest");
    let release: GitHubRelease = agent()
        .get(&url)
        .set("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| match e {
            ureq::Error::Status(404, _) => Error::msg(format!("{repo} has no published releases.")),
            other => Error::msg(format!("Cannot reach GitHub: {other}")),
        })?
        .into_json()
        .map_err(|e| Error::msg(format!("Unexpected answer from GitHub: {e}")))?;
    let current = env!("CARGO_PKG_VERSION").to_string();
    let latest = release.tag_name.trim_start_matches('v').to_string();
    let newer = match (semver::Version::parse(&latest), semver::Version::parse(&current)) {
        (Ok(l), Ok(c)) => l > c,
        _ => false,
    };
    let asset = |suffix: &str| {
        release
            .assets
            .iter()
            .find(|a| a.name.starts_with("BWLauncher-") && a.name.ends_with(suffix))
            .map(|a| Asset { name: a.name.clone(), url: a.browser_download_url.clone(), size: a.size })
    };
    Ok(UpdateInfo {
        current_version: current,
        newer,
        name: release.name.clone().unwrap_or_else(|| release.tag_name.clone()),
        notes: release.body.clone().unwrap_or_default(),
        page_url: release.html_url.clone(),
        published_at: release.published_at.clone().unwrap_or_default(),
        installer: asset("-Setup.exe"),
        sums: asset("-SHA256SUMS.txt"),
        latest_version: latest,
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
    use super::is_valid_repo;

    #[test]
    fn repos() {
        assert!(is_valid_repo("konstantinev/BlackWhiteLauncher"));
        for bad in ["", "x", "a/b/c", "a/", "/b", "a b/c", "a/b?x=1"] {
            assert!(!is_valid_repo(bad), "{bad}");
        }
    }
}
