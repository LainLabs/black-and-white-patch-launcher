// No console window in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let Some(code) = bw_launcher_lib::run_headless(&args) {
        std::process::exit(code);
    }
    bw_launcher_lib::run();
}
