use std::sync::{Arc, Mutex};
use tauri::State;
use tauri_plugin_dialog::DialogExt;
use serde::Serialize;

use crate::file_manager::{FileManager, SharedFile};
use crate::http_server::HttpFileServer;

#[derive(Serialize)]
pub struct ServerInfo {
    pub ip: String,
    pub port: u16,
    pub running: bool,
    pub base_url: String,
    pub ips: Vec<String>,
    pub base_urls: Vec<String>,
}

pub struct AppState {
    pub file_manager: Arc<Mutex<FileManager>>,
    pub http_server: Mutex<Option<HttpFileServer>>,
    pub lan_ips: Mutex<Vec<String>>,
}

#[tauri::command]
pub async fn share_file(
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<SharedFile, String> {
    let file_path = app_handle
        .dialog()
        .file()
        .blocking_pick_file();

    let file = file_path.ok_or_else(|| "用户取消了选择".to_string())?;
    let path = file.into_path().map_err(|e| e.to_string())?;

    let shared_file = state.file_manager.lock().unwrap().add_file(path);

    let need_start = {
        let server = state.http_server.lock().unwrap();
        server.is_none()
    };

    if need_start {
        let ips = crate::network::detect_lan_ips();
        *state.lan_ips.lock().unwrap() = ips;
        let new_server = HttpFileServer::start(state.file_manager.clone()).await?;
        *state.http_server.lock().unwrap() = Some(new_server);
    }

    Ok(shared_file)
}

#[tauri::command]
pub fn get_shared_files(state: State<'_, AppState>) -> Vec<SharedFile> {
    state.file_manager.lock().unwrap().list_files()
}

#[tauri::command]
pub fn delete_shared_file(state: State<'_, AppState>, code: String) -> Result<(), String> {
    let removed = state.file_manager.lock().unwrap().remove_file(&code);
    if !removed {
        return Err("文件不存在".to_string());
    }

    if state.file_manager.lock().unwrap().is_empty() {
        let _ = state.http_server.lock().unwrap().take();
    }

    Ok(())
}

#[tauri::command]
pub fn get_server_info(state: State<'_, AppState>) -> Result<ServerInfo, String> {
    let server = state.http_server.lock().map_err(|e| e.to_string())?;

    match server.as_ref() {
        Some(s) => {
            let ips = state.lan_ips.lock().map_err(|e| e.to_string())?.clone();
            let port = s.port();
            let first_ip = ips.first().cloned().unwrap_or_default();
            let base_urls: Vec<String> = ips.iter().map(|ip| format!("http://{}:{}", ip, port)).collect();
            let first_base_url = base_urls.first().cloned().unwrap_or_default();
            Ok(ServerInfo {
                ip: first_ip,
                port,
                running: true,
                base_url: first_base_url,
                ips,
                base_urls,
            })
        }
        None => Ok(ServerInfo {
            ip: String::new(),
            port: 0,
            running: false,
            base_url: String::new(),
            ips: vec![],
            base_urls: vec![],
        }),
    }
}
