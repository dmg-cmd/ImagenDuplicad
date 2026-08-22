use crate::hasher;
use std::path::{Path, PathBuf};

const THUMB_SIZE: u32 = 320;

pub fn cache_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("imagen-duplicada-thumbs");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

pub fn thumb_path(path: &Path) -> PathBuf {
    let key = hasher::sha256_bytes(path.to_string_lossy().as_bytes());
    cache_dir().join(format!("{}.png", key))
}

pub fn generate_thumbnail(path: &Path) -> Option<PathBuf> {
    let out = thumb_path(path);
    if out.exists() {
        return Some(out);
    }
    let img = image::ImageReader::open(path).ok()?.decode().ok()?;
    img.thumbnail(THUMB_SIZE, THUMB_SIZE).save(&out).ok()?;
    Some(out)
}
