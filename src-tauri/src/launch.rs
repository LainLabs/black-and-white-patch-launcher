//! Starting the patched game: BWLaunch.exe (from the decomp) checks runblack.exe, starts it suspended and loads
//! BWDecomp.dll into it, then exits. The launcher then follows runblack.exe until the game ends.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::error::{Error, Result};
use crate::game::{self, LAUNCH_EXE};

pub const LOG_FILE: &str = "BWDecomp.log";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameEvent {
    /// "started", "failed" (BWLaunch.exe reported an error) or "exited".
    pub state: &'static str,
    pub launcher_exit_code: Option<i32>,
    pub log_tail: String,
}

/// The last `lines` lines of BWDecomp.log.
pub fn log_tail(folder: &Path, lines: usize) -> String {
    let Ok(bytes) = std::fs::read(folder.join(LOG_FILE)) else {
        return String::new();
    };
    let text = String::from_utf8_lossy(&bytes);
    let all: Vec<&str> = text.lines().collect();
    all[all.len().saturating_sub(lines)..].join("\n")
}

/// Starts the game and reports on the `game` event: started, then exited or failed.
pub fn launch(app: AppHandle, folder: PathBuf, arguments: Vec<String>) -> Result<()> {
    let launcher = folder.join(LAUNCH_EXE);
    if !launcher.is_file() {
        return Err(Error::msg(format!("{} is missing: install the patch first.", launcher.display())));
    }
    if game::is_game_running(&folder) {
        return Err(Error::msg("Black & White is already running from this folder."));
    }
    let mut command = std::process::Command::new(&launcher);
    command.current_dir(&folder);
    if !arguments.is_empty() {
        command.arg("--").args(&arguments);
    }
    let mut child = command.spawn().map_err(|e| Error::msg(format!("Cannot start {}: {e}", launcher.display())))?;

    std::thread::spawn(move || {
        let code = child.wait().ok().and_then(|status| status.code());
        if code != Some(0) {
            let _ = app.emit(
                "game",
                GameEvent { state: "failed", launcher_exit_code: code, log_tail: log_tail(&folder, 40) },
            );
            return;
        }
        let _ = app.emit("game", GameEvent { state: "started", launcher_exit_code: code, log_tail: String::new() });
        // The game process appears as BWLaunch.exe resumes it; wait for it to end.
        std::thread::sleep(Duration::from_secs(2));
        while game::is_game_running(&folder) {
            std::thread::sleep(Duration::from_secs(2));
        }
        let _ = app.emit("game", GameEvent { state: "exited", launcher_exit_code: code, log_tail: log_tail(&folder, 40) });
    });
    Ok(())
}
