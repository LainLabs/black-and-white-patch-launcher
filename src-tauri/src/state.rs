//! The launcher's own settings: %APPDATA%\BWLauncher\launcher.json.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::paths::state_dir;

/// Where launcher releases are published. A build can name another with BW_LAUNCHER_UPDATE_REPO.
pub const DEFAULT_LAUNCHER_REPO: &str = match option_env!("BW_LAUNCHER_UPDATE_REPO") {
    Some(repo) => repo,
    None => "konstantine-v/black-and-white-patch-launcher",
};

/// Where patch releases are published. A build can name another with BW_PATCH_UPDATE_REPO.
pub const DEFAULT_PATCH_REPO: &str = match option_env!("BW_PATCH_UPDATE_REPO") {
    Some(repo) => repo,
    None => "konstantine-v/black-and-white-patch-tool",
};

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
    if state.launcher_repo.is_empty() {
        state.launcher_repo = DEFAULT_LAUNCHER_REPO.into();
    }
    if state.patch_repo.is_empty() {
        state.patch_repo = DEFAULT_PATCH_REPO.into();
    }
    state
}

pub fn save(state: &LauncherState) -> Result<()> {
    std::fs::create_dir_all(state_dir())?;
    let path = file();
    let temporary = path.with_extension("json.new");
    std::fs::write(&temporary, serde_json::to_vec_pretty(state)?)?;
    std::fs::rename(temporary, path)?;
    Ok(())
}
