//! Commands the UI calls (src/lib/api.ts mirrors these signatures). Slow work (hashing, file copies, network)
//! runs on a blocking thread so the window stays responsive.

use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::AppHandle;

use crate::elevate::{self, Request};
use crate::error::{Error, Result};
use crate::{addons, display, game, install, launch, payload, registry, settings, state, updates};

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Result<T> + Send + 'static) -> Result<T> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| Error::msg(format!("Task failed: {e}")))?
}

fn folder_path(folder: &str) -> Result<PathBuf> {
    let path = PathBuf::from(folder.trim());
    if !path.is_absolute() {
        return Err(Error::msg("Choose a full folder path."));
    }
    Ok(path)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    launcher_version: &'static str,
    decomp_version: &'static str,
    decomp_commit: &'static str,
    has_payload: bool,
    state_dir: String,
    addons_dir: String,
    elevated: bool,
}

#[tauri::command]
pub fn app_info() -> AppInfo {
    AppInfo {
        launcher_version: env!("CARGO_PKG_VERSION"),
        decomp_version: payload::DECOMP_VERSION,
        decomp_commit: payload::DECOMP_COMMIT,
        has_payload: payload::has_payload(),
        state_dir: crate::paths::state_dir().display().to_string(),
        addons_dir: addons::addons_dir().display().to_string(),
        elevated: elevate::is_elevated(),
    }
}

#[tauri::command]
pub fn get_state() -> state::LauncherState {
    state::load()
}

#[tauri::command]
pub fn set_state(value: state::LauncherState) -> Result<()> {
    if !value.update_repo.is_empty() && !updates::is_valid_repo(&value.update_repo) {
        return Err(Error::msg("The update repository must look like owner/name."));
    }
    state::save(&value)
}

#[tauri::command]
pub async fn detect_game_folder() -> Result<Option<String>> {
    blocking(|| {
        let remembered = state::load().game_folder;
        Ok(game::find_game_folder(remembered.as_deref()).map(|p| p.display().to_string()))
    })
    .await
}

#[tauri::command]
pub async fn check_game(folder: String) -> Result<game::GameCheck> {
    let folder = folder_path(&folder)?;
    blocking(move || Ok(game::check_executable(&folder))).await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchStatus {
    /// "notInstalled", "installed", "outdated" (another version), "byHand" (files without a manifest).
    state: &'static str,
    installed_version: String,
    bundled_version: &'static str,
    /// Installed by the decomp's BWDecompSetup.exe (its uninstaller is listed); the launcher takes it over.
    from_setup: bool,
    needs_rights: bool,
    running: bool,
}

#[tauri::command]
pub async fn patch_status(folder: String) -> Result<PatchStatus> {
    let folder = folder_path(&folder)?;
    blocking(move || {
        let manifest = install::read_manifest(&folder);
        let (state, installed_version, from_setup) = match &manifest {
            Some(m) => (
                if m.version == payload::DECOMP_VERSION { "installed" } else { "outdated" },
                m.version.clone(),
                m.entries.iter().any(|e| e.path.eq_ignore_ascii_case(install::LEGACY_UNINSTALLER)),
            ),
            None if folder.join("BWDecomp.dll").is_file() => ("byHand", String::new(), false),
            None => ("notInstalled", String::new(), false),
        };
        Ok(PatchStatus {
            state,
            installed_version,
            bundled_version: payload::DECOMP_VERSION,
            from_setup,
            needs_rights: elevate::install_needs_rights(&folder),
            running: game::is_game_running(&folder),
        })
    })
    .await
}

fn ensure_not_running(folder: &Path) -> Result<()> {
    if game::is_game_running(folder) {
        return Err(Error::msg("Close Black & White first: it is running from this folder."));
    }
    Ok(())
}

/// Installs or updates the patch and remembers the folder.
#[tauri::command]
pub async fn install_patch(folder: String) -> Result<()> {
    let folder = folder_path(&folder)?;
    blocking(move || {
        if !payload::has_payload() {
            return Err(Error::msg("This launcher was built without the decompilation (no payload)."));
        }
        if !game::holds_game(&folder) {
            return Err(Error::msg(format!("There is no runblack.exe in {}.", folder.display())));
        }
        ensure_not_running(&folder)?;
        let had_setup_uninstaller = folder.join(install::LEGACY_UNINSTALLER).exists();
        let rights = elevate::install_needs_rights(&folder);
        elevate::execute_with_rights(&Request::Install { folder: folder.clone() }, rights)?;
        // BWDecompSetup's uninstaller is gone now: its Settings > Apps entry would point at nothing.
        if had_setup_uninstaller
            && !folder.join(install::LEGACY_UNINSTALLER).exists()
            && registry::legacy_install_location().is_some_and(|l| crate::paths::same_path(&l, &folder))
        {
            registry::remove_legacy_uninstall_entry();
        }
        registry::ensure_game_dir(&folder)?;
        let mut s = state::load();
        s.game_folder = Some(folder);
        state::save(&s)
    })
    .await
}

#[tauri::command]
pub async fn remove_patch(folder: String) -> Result<install::RemoveResult> {
    let folder = folder_path(&folder)?;
    blocking(move || {
        ensure_not_running(&folder)?;
        for addon in addons::list(Some(&folder), payload::DECOMP_VERSION).into_iter().filter(|a| a.enabled) {
            addons::disable(&folder, &addon.manifest.id)?;
        }
        let rights = elevate::install_needs_rights(&folder);
        let data = elevate::execute_with_rights(&Request::Remove { folder: folder.clone() }, rights)?;
        if registry::legacy_install_location().is_some_and(|l| crate::paths::same_path(&l, &folder)) {
            registry::remove_legacy_uninstall_entry();
        }
        Ok(serde_json::from_value(data).unwrap_or_default())
    })
    .await
}

#[tauri::command]
pub async fn read_settings(folder: String) -> Result<settings::SettingsSnapshot> {
    let folder = folder_path(&folder)?;
    blocking(move || Ok(settings::read(&folder))).await
}

#[tauri::command]
pub async fn write_settings(folder: String, change: settings::SettingsChange) -> Result<()> {
    let folder = folder_path(&folder)?;
    blocking(move || settings::write(&folder, &change)).await
}

#[tauri::command]
pub fn display_modes() -> display::DisplayModes {
    display::display_modes()
}

#[tauri::command]
pub async fn launch_game(app: AppHandle, folder: String) -> Result<()> {
    let folder = folder_path(&folder)?;
    let arguments = state::load().game_arguments;
    blocking(move || launch::launch(app, folder, arguments)).await
}

#[tauri::command]
pub async fn game_running(folder: String) -> Result<bool> {
    let folder = folder_path(&folder)?;
    blocking(move || Ok(game::is_game_running(&folder))).await
}

#[tauri::command]
pub fn read_log_tail(folder: String, lines: usize) -> Result<String> {
    Ok(launch::log_tail(&folder_path(&folder)?, lines.min(2000)))
}

/// Opens a folder in Explorer (created first for Screenshots and Textures\Replace, which the game creates lazily).
#[tauri::command]
pub fn open_folder(path: String, create: bool) -> Result<()> {
    let path = PathBuf::from(path);
    if create {
        std::fs::create_dir_all(&path).map_err(|e| Error::msg(format!("Cannot create {}: {e}", path.display())))?;
    }
    if !path.exists() {
        return Err(Error::msg(format!("{} does not exist.", path.display())));
    }
    std::process::Command::new("explorer.exe").arg(&path).spawn()?;
    Ok(())
}

#[tauri::command]
pub async fn list_addons(folder: Option<String>) -> Result<Vec<addons::AddonInfo>> {
    blocking(move || Ok(addons::list(folder.as_deref().map(Path::new), payload::DECOMP_VERSION))).await
}

#[tauri::command]
pub async fn import_addon(source: String) -> Result<String> {
    blocking(move || addons::import(Path::new(&source))).await
}

#[tauri::command]
pub async fn delete_addon(id: String) -> Result<()> {
    blocking(move || addons::delete(&id)).await
}

#[tauri::command]
pub async fn set_addon_enabled(folder: String, id: String, enabled: bool) -> Result<()> {
    let folder = folder_path(&folder)?;
    blocking(move || {
        ensure_not_running(&folder)?;
        if enabled {
            addons::enable(&folder, &id, payload::DECOMP_VERSION)
        } else {
            addons::disable(&folder, &id)
        }
    })
    .await
}

#[tauri::command]
pub async fn check_updates(repo: String) -> Result<updates::UpdateInfo> {
    blocking(move || updates::check(repo.trim())).await
}

/// Downloads and verifies the new launcher's setup program, starts it and closes the launcher.
#[tauri::command]
pub async fn install_update(app: AppHandle, info: updates::UpdateInfo) -> Result<()> {
    let installer = blocking(move || updates::download_installer(&info)).await?;
    std::process::Command::new(&installer)
        .spawn()
        .map_err(|e| Error::msg(format!("Cannot start {}: {e}", installer.display())))?;
    app.exit(0);
    Ok(())
}
