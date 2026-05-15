use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Debug, Clone, serde::Serialize)]
pub struct SharedFile {
    pub token: String,
    pub file_name: String,
    pub file_path: String,
    pub file_size: u64,
}

pub struct FileManager {
    files: Arc<Mutex<HashMap<String, SharedFile>>>,
}

impl FileManager {
    pub fn new() -> Self {
        FileManager {
            files: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn add_file(&self, path: PathBuf) -> SharedFile {
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        let file_path = path.to_string_lossy().to_string();
        let file_size = std::fs::metadata(&path)
            .map(|m| m.len())
            .unwrap_or(0);
        let token = Uuid::new_v4().to_string();
        let shared_file = SharedFile {
            token: token.clone(),
            file_name,
            file_path,
            file_size,
        };
        self.files
            .lock()
            .unwrap()
            .insert(token.clone(), shared_file.clone());
        shared_file
    }

    pub fn remove_file(&self, token: &str) -> bool {
        self.files.lock().unwrap().remove(token).is_some()
    }

    pub fn get_file(&self, token: &str) -> Option<SharedFile> {
        self.files.lock().unwrap().get(token).cloned()
    }

    pub fn list_files(&self) -> Vec<SharedFile> {
        self.files
            .lock()
            .unwrap()
            .values()
            .cloned()
            .collect()
    }

    pub fn is_empty(&self) -> bool {
        self.files.lock().unwrap().is_empty()
    }
}
