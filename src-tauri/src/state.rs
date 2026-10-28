//! The launcher's own settings: %APPDATA%\BWLauncher\launcher.json.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::paths::state_dir;

/// Where launcher and patch releases are published: updates.ini at the repository root, read by build.rs.
pub const DEFAULT_LAUNCHER_REPO: &str = env!("BW_LAUNCHER_REPO");
pub const DEFAULT_PATCH_REPO: &str = env!("BW_PATCH_REPO");
/// Earlier defaults (comma-separated), which a saved setting follows to the current default.
const PREVIOUS_LAUNCHER_REPOS: &str = env!("BW_LAUNCHER_PREVIOUS_REPOS");
const PREVIOUS_PATCH_REPOS: &str = env!("BW_PATCH_PREVIOUS_REPOS");

/// The repository to use for a saved setting: the default when it is empty or names an earlier default.
fn resolve_repo(saved: &str, default: &str, previous: &str) -> String {
    let old = previous.split(',').any(|p| !p.is_empty() && p.eq_ignore_ascii_case(saved));
    if saved.is_empty() || old { default.into() } else { saved.into() }
}

/// The setting to save: empty for the default, so a later launcher with another default moves along with it.
fn saved_repo(repo: &str, default: &str) -> String {
    if repo.eq_ignore_ascii_case(default) { String::new() } else { repo.into() }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LauncherState {
    /// The game folder the patch was installed into; None until the first-run setup is done.
    pub game_folder: Option<PathBuf>,
    /// What the launcher window does when the game starts: "keep", "minimize" or "close".
    pub on_play: String,
    /// GitHub repositories ("owner/name") checked for new launchers and new patches; empty means the default.
    /// (0.1.x saved a single `updateRepo`, which is ignored.)
    pub launcher_repo: String,
    pub patch_repo: String,
    pub check_updates_on_start: bool,
    /// Extra arguments passed to runblack.exe.
    pub game_arguments: Vec<String>,
}

impl Default for LauncherState {
    fn default() -> Self {
        Self {
            game_folder: None,
            on_play: "minimize".into(),
            launcher_repo: DEFAULT_LAUNCHER_REPO.into(),
            patch_repo: DEFAULT_PATCH_REPO.into(),
            check_updates_on_start: true,
            game_arguments: Vec::new(),
        }
    }
}

fn file() -> PathBuf {
    state_dir().join("launcher.json")
}

pub fn load() -> LauncherState {
    let mut state: LauncherState =
        std::fs::read(file()).ok().and_then(|bytes| serde_json::from_slice(&bytes).ok()).unwrap_or_default();
    state.launcher_repo = resolve_repo(&state.launcher_repo, DEFAULT_LAUNCHER_REPO, PREVIOUS_LAUNCHER_REPOS);
    state.patch_repo = resolve_repo(&state.patch_repo, DEFAULT_PATCH_REPO, PREVIOUS_PATCH_REPOS);
    state
}

pub fn save(state: &LauncherState) -> Result<()> {
    let mut state = state.clone();
    state.launcher_repo = saved_repo(&state.launcher_repo, DEFAULT_LAUNCHER_REPO);
    state.patch_repo = saved_repo(&state.patch_repo, DEFAULT_PATCH_REPO);
    std::fs::create_dir_all(state_dir())?;
    let path = file();
    let temporary = path.with_extension("json.new");
    std::fs::write(&temporary, serde_json::to_vec_pretty(&state)?)?;
    std::fs::rename(temporary, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repos_follow_the_default() {
        let previous = "old/launcher,Older/Launcher";
        assert_eq!(resolve_repo("", "new/launcher", previous), "new/launcher");
        assert_eq!(resolve_repo("old/launcher", "new/launcher", previous), "new/launcher");
        assert_eq!(resolve_repo("older/launcher", "new/launcher", previous), "new/launcher");
        assert_eq!(resolve_repo("mine/launcher", "new/launcher", previous), "mine/launcher");
        assert_eq!(resolve_repo("mine/launcher", "new/launcher", ""), "mine/launcher");
        assert_eq!(saved_repo("New/Launcher", "new/launcher"), "");
        assert_eq!(saved_repo("mine/launcher", "new/launcher"), "mine/launcher");
    }

    #[test]
    fn defaults_come_from_updates_ini() {
        assert!(crate::updates::is_valid_repo(DEFAULT_LAUNCHER_REPO));
        assert!(crate::updates::is_valid_repo(DEFAULT_PATCH_REPO));
    }
}
