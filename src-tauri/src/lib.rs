pub mod config;
pub mod error;
pub mod install;
pub mod launch;
pub mod releases;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            releases::list_versions,
            install::install_version,
            install::get_installed_versions,
            install::remove_version,
            launch::launch_version,
            launch::get_running_versions,
            launch::open_game_folder,
            launch::open_log
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
