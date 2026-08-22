use std::path::PathBuf;

const EXTS: &[&str] = &["jpg", "jpeg", "png", "webp", "gif", "tiff", "tif", "bmp"];

pub fn scan_images(root: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in walkdir::WalkDir::new(root).follow_links(false) {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let ext = entry
            .path()
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        if EXTS.contains(&ext.as_str()) {
            out.push(entry.path().to_path_buf());
        }
    }
    out
}