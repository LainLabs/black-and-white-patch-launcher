//! The registry values the launcher reads and writes, all per user except the machine-wide GameDir it reads.
//! Sources: the decomp's src/setup/System.cpp (keys and GameDir rule) and docs/FIXES.md (BWSetup values).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use winreg::enums::{RegType, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY};
use winreg::{RegKey, RegValue};

use crate::error::{Error, Result};

const GAME_KEY: &str = r"Software\Lionhead Studios Ltd\Black & White";
const SETTINGS_KEY: &str = r"Software\Lionhead Studios Ltd\Black & White\BWSetup";
/// Written by the decomp's BWDecompSetup.exe; the launcher takes such installations over.
const LEGACY_UNINSTALL_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\BlackWhiteDecomp";

/// The game's own settings the launcher edits. The game stores them as DWORDs (checked on an installed game:
/// ScreenW/ScreenH/detailidx/maxdetailidx and the volumes are REG_DWORD); ScreenD is the colour depth (16 or 32).
pub const GAME_SETTINGS: &[&str] =
    &["ScreenW", "ScreenH", "ScreenD", "detailidx", "maxdetailidx", "AudioSampleMasterVolume", "AudioMusicMasterVolume"];

fn read_string(root: &RegKey, key: &str, value: &str) -> Option<String> {
    root.open_subkey(key).ok()?.get_value::<String, _>(value).ok().filter(|s| !s.is_empty())
}

pub fn legacy_install_location() -> Option<PathBuf> {
    read_string(&RegKey::predef(HKEY_CURRENT_USER), LEGACY_UNINSTALL_KEY, "InstallLocation").map(PathBuf::from)
}

/// Removes BWDecompSetup's Settings > Apps entry once its uninstaller is gone (the launcher replaced it).
pub fn remove_legacy_uninstall_entry() {
    let _ = RegKey::predef(HKEY_CURRENT_USER).delete_subkey_all(LEGACY_UNINSTALL_KEY);
}

/// GameDir for the current user, then for the machine (the game is 32-bit, so its machine key is in the 32-bit view).
pub fn game_dir_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    out.extend(read_string(&RegKey::predef(HKEY_CURRENT_USER), GAME_KEY, "GameDir").map(PathBuf::from));
    if let Ok(key) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey_with_flags(GAME_KEY, KEY_READ | KEY_WOW64_32KEY) {
        if let Ok(dir) = key.get_value::<String, _>("GameDir") {
            if !dir.is_empty() {
                out.push(PathBuf::from(dir));
            }
        }
    }
    out
}

/// Sets GameDir when the game has none: without it profiles and saves go to \dev\black (EnsureGameDir in setup).
pub fn ensure_game_dir(folder: &Path) -> Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if read_string(&hkcu, GAME_KEY, "GameDir").is_some() {
        return Ok(());
    }
    let (key, _) = hkcu.create_subkey(GAME_KEY).map_err(|e| Error::msg(format!("Cannot write the registry: {e}")))?;
    key.set_value("GameDir", &folder.display().to_string())
        .map_err(|e| Error::msg(format!("Cannot write GameDir: {e}")))
}

/// The values of GAME_SETTINGS that exist (and are numbers).
pub fn read_game_settings() -> BTreeMap<String, u32> {
    let mut out = BTreeMap::new();
    if let Ok(key) = RegKey::predef(HKEY_CURRENT_USER).open_subkey(SETTINGS_KEY) {
        for name in GAME_SETTINGS {
            if let Ok(value) = key.get_value::<u32, _>(name) {
                out.insert((*name).to_string(), value);
            } else if let Ok(text) = key.get_value::<String, _>(name) {
                if let Ok(value) = text.trim().parse() {
                    out.insert((*name).to_string(), value);
                }
            }
        }
    }
    out
}

/// Writes game settings, keeping the type of a value that exists as a string; new values are DWORDs as the game
/// writes them.
pub fn write_game_settings(values: &BTreeMap<String, u32>) -> Result<()> {
    let (key, _) = RegKey::predef(HKEY_CURRENT_USER)
        .create_subkey(SETTINGS_KEY)
        .map_err(|e| Error::msg(format!("Cannot write the registry: {e}")))?;
    for (name, value) in values {
        if !GAME_SETTINGS.contains(&name.as_str()) {
            return Err(Error::msg(format!("Unknown game setting {name}")));
        }
        let existing = key.get_raw_value(name).ok();
        let result = match existing {
            Some(RegValue { vtype: RegType::REG_SZ, .. }) => key.set_value(name, &value.to_string()),
            _ => key.set_value(name, value),
        };
        result.map_err(|e| Error::msg(format!("Cannot write {name}: {e}")))?;
    }
    Ok(())
}
