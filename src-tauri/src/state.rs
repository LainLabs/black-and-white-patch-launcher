//! The launcher's own settings: %APPDATA%\BWLauncher\launcher.json.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::paths::state_dir;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LauncherState {
    /// The game folder the patch was installed into; None until the first-run setup is done.
    pub game_folder: Option<PathBuf>,
    /// What the launcher window does when the game starts: "keep", "minimize" or "close".
    pub on_play: String,
    /// GitHub repository ("owner/name") whose releases are checked for updates; empty = not configured.
    pub update_repo: String,
    pub check_updates_on_start: bool,
    /// Extra arguments passed to runblack.exe.
    pub game_arguments: Vec<String>,
}

impl Default for LauncherState {
    fn default() -> Self {
        Self {
            game_folder: None,
            on_play: "minimize".into(),
            update_repo: option_env!("BW_LAUNCHER_UPDATE_REPO").unwrap_or_default().into(),
            check_updates_on_start: true,
            game_arguments: Vec::new(),
        }
    }
}

fn file() -> PathBuf {
    state_dir().join("launcher.json")
}

pub fn load() -> LauncherState {
    std::fs::read(file()).ok().and_then(|bytes| serde_json::from_slice(&bytes).ok()).unwrap_or_default()
}

pub fn save(state: &LauncherState) -> Result<()> {
    std::fs::create_dir_all(state_dir())?;
    let path = file();
    let temporary = path.with_extension("json.new");
    std::fs::write(&temporary, serde_json::to_vec_pretty(state)?)?;
    std::fs::rename(temporary, path)?;
    Ok(())
}
