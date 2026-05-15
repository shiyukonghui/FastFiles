mod network;
mod file_manager;
pub mod http_server;
mod commands;

use std::sync::{Arc, Mutex};
use commands::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_state = AppState {
        file_manager: Arc::new(Mutex::new(crate::file_manager::FileManager::new())),
        http_server: Mutex::new(None),
        lan_ips: Mutex::new(Vec::new()),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::share_file,
            commands::get_shared_files,
            commands::delete_shared_file,
            commands::get_server_info,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
