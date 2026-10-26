//! The settings page: BWDecomp.ini in the game folder (read by BWDecomp.dll, src/core/Config.cpp in the decomp) and
//! the game's own values in the registry.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::fsops::{self, FileOp};
use crate::ini::{self, IniValue};
use crate::{payload, registry};

pub const INI_FILE: &str = "BWDecomp.ini";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsSnapshot {
    /// What BWDecomp.ini says (the shipped defaults when the game folder has none).
    pub ini: Vec<IniValue>,
    pub ini_exists: bool,
    /// The settings as shipped with the bundled patch.
    pub defaults: Vec<IniValue>,
    /// The game's own settings (HKCU ...\Black & White\BWSetup).
    pub game: BTreeMap<String, u32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsChange {
    #[serde(default)]
    pub ini: Vec<IniValue>,
    #[serde(default)]
    pub game: BTreeMap<String, u32>,
}

/// BWDecomp.ini of the game folder, or the shipped one when there is none.
pub fn read_ini_text(folder: &Path) -> String {
    std::fs::read(folder.join(INI_FILE))
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .unwrap_or_else(|_| payload::default_ini().unwrap_or_default().to_string())
}

/// Writes BWDecomp.ini, with administrator rights when the folder needs them.
pub fn write_ini_text(folder: &Path, text: &str) -> Result<()> {
    let staged = fsops::stage(text.as_bytes())?;
    let result = fsops::apply_with_rights(folder, vec![FileOp::Copy { from: staged.clone(), to: INI_FILE.into() }]);
    let _ = std::fs::remove_file(staged);
    result
}

pub fn read(folder: &Path) -> SettingsSnapshot {
    SettingsSnapshot {
        ini: ini::parse(&read_ini_text(folder)),
        ini_exists: folder.join(INI_FILE).is_file(),
        defaults: payload::default_ini().map(ini::parse).unwrap_or_default(),
        game: registry::read_game_settings(),
    }
}

pub fn write(folder: &Path, change: &SettingsChange) -> Result<()> {
    if !change.ini.is_empty() {
        let mut text = read_ini_text(folder);
        for value in &change.ini {
            text = ini::set(&text, &value.section, &value.key, value.value.trim());
        }
        write_ini_text(folder, &text)?;
    }
    if !change.game.is_empty() {
        registry::write_game_settings(&change.game)?;
    }
    Ok(())
}
