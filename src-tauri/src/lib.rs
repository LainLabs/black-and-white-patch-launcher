//! Black & White Launcher: installs the decompilation into the player's game, edits its settings and starts it.
//! See AGENTS.md for how the pieces fit and how they map to the decomp repository.

pub mod addons;
pub mod commands;
pub mod display;
pub mod elevate;
pub mod error;
pub mod fsops;
pub mod game;
pub mod ini;
pub mod install;
pub mod launch;
pub mod paths;
pub mod payload;
pub mod registry;
pub mod settings;
pub mod state;
pub mod updates;

use tauri::Manager;

/// Without the UI: `--play` starts the game from the remembered folder (for a "Play" shortcut). Returns the exit
/// code, or None to open the window.
pub fn run_headless(args: &[String]) -> Option<i32> {
    match args.get(1).map(String::as_str) {
        Some("--elevated") if args.len() == 4 => {
            Some(elevate::elevated_main(std::path::Path::new(&args[2]), std::path::Path::new(&args[3])))
        }
        Some("--play") => {
            let folder = state::load().game_folder?;
            let mut command = std::process::Command::new(folder.join(game::LAUNCH_EXE));
            command.current_dir(&folder);
            let arguments = state::load().game_arguments;
            if !arguments.is_empty() {
                command.arg("--").args(arguments);
            }
            Some(if command.spawn().is_ok() { 0 } else { 1 })
        }
        _ => None,
    }
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::get_state,
            commands::set_state,
            commands::detect_game_folder,
            commands::check_game,
            commands::patch_status,
            commands::install_patch,
            commands::remove_patch,
            commands::read_settings,
            commands::write_settings,
            commands::display_modes,
            commands::launch_game,
            commands::game_running,
            commands::read_log_tail,
            commands::open_folder,
            commands::list_addons,
            commands::import_addon,
            commands::delete_addon,
            commands::set_addon_enabled,
            commands::check_updates,
            commands::install_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running the launcher");
}
