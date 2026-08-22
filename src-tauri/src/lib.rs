mod commands;
mod hasher;
mod matcher;
mod metadata;
mod reader;
mod scanner;
mod thumbnails;

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Serialize, Clone)]
pub struct ImageInfo {
    pub path: String,
    pub file_name: String,
    pub dir: String,
    pub size_bytes: u64,
    pub modified: Option<String>,
    pub date_taken: Option<String>,
    pub camera: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub hash: String,
    pub thumbnail: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct DupGroup {
    pub confidence: String,
    pub images: Vec<ImageInfo>,
    pub total_size: u64,
}

#[derive(Serialize, Clone)]
pub struct ScanProgress {
    pub phase: String,
    pub done: usize,
    pub total: usize,
    pub detail: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct ScanResult {
    pub groups: Vec<DupGroup>,
    pub skipped: usize,
}

#[derive(Deserialize)]
pub struct DeleteRequest {
    pub paths: Vec<String>,
}

#[derive(Clone)]
pub struct CancelToken {
    flag: Arc<AtomicBool>,
}

impl CancelToken {
    pub fn new() -> Self {
        Self {
            flag: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.flag.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::Relaxed)
    }

    pub fn reset(&self) {
        self.flag.store(false, Ordering::Relaxed);
    }
}

pub static CANCEL_TOKEN: once_cell::sync::Lazy<CancelToken> =
    once_cell::sync::Lazy::new(CancelToken::new);

#[cfg(desktop)]
mod desktop {
    use crate::commands;

    pub fn run() {
        tauri::Builder::default()
            .plugin(tauri_plugin_dialog::init())
            .invoke_handler(tauri::generate_handler![
                commands::scan_folder,
                commands::cancel_scan,
                commands::preview,
                commands::delete_to_trash,
                commands::delete_permanent,
            ])
            .run(tauri::generate_context!())
            .expect("error al ejecutar la app");
    }
}

pub fn run() {
    #[cfg(desktop)]
    desktop::run();
}
