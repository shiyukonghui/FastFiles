use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use rand::Rng;

#[derive(Debug, Clone, serde::Serialize)]
pub struct SharedFile {
    pub code: String,
    pub file_name: String,
    pub file_path: String,
    pub file_size: u64,
}

fn generate_code() -> String {
    const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut rng = rand::thread_rng();
    (0..4)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
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
        let code = generate_code();
        let shared_file = SharedFile {
            code: code.clone(),
            file_name,
            file_path,
            file_size,
        };
        self.files
            .lock()
            .unwrap()
            .insert(code.clone(), shared_file.clone());
        shared_file
    }

    pub fn remove_file(&self, code: &str) -> bool {
        self.files.lock().unwrap().remove(code).is_some()
    }

    pub fn get_file(&self, code: &str) -> Option<SharedFile> {
        self.files.lock().unwrap().get(code).cloned()
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
