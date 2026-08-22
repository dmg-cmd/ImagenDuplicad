use crate::metadata;
use crate::reader;
use crate::scanner;
use crate::thumbnails;
use crate::{DeleteRequest, DupGroup, ImageInfo, ScanProgress, ScanResult, CANCEL_TOKEN};
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::UNIX_EPOCH;
use tauri::Emitter;

const PROGRESS_BATCH: usize = 50;

fn build_fs_info(path: &Path) -> Option<(String, String, u64, Option<String>)> {
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let dir_str = path
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    let meta = fs::metadata(path).ok()?;
    let size_bytes = meta.len();
    let modified = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| {
            let secs = d.as_secs() as i64;
            chrono::DateTime::from_timestamp(secs, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_default()
        })
        .filter(|s| !s.is_empty());
    Some((file_name, dir_str, size_bytes, modified))
}

fn emit_progress(app: &tauri::AppHandle, phase: &str, done: usize, total: usize, detail: Option<&str>) {
    app.emit(
        "scan-progress",
        ScanProgress {
            phase: phase.to_string(),
            done,
            total,
            detail: detail.map(|s| s.to_string()),
        },
    )
    .ok();
}

fn emit_live_duplicate(
    app: &tauri::AppHandle,
    seen_hashes: &Mutex<HashMap<String, Vec<ImageInfo>>>,
    img: &ImageInfo,
) {
    let group = {
        let mut seen = match seen_hashes.lock() {
            Ok(s) => s,
            Err(poisoned) => poisoned.into_inner(),
        };
        let entry = seen.entry(img.hash.clone()).or_default();
        entry.push(img.clone());
        if entry.len() < 2 {
            return;
        }
        let mut images = entry.clone();
        crate::matcher::sort_by_best(&mut images);
        DupGroup {
            confidence: "exacto".to_string(),
            total_size: images.iter().map(|i| i.size_bytes).sum(),
            images,
        }
    };
    app.emit("dup-found", &group).ok();
}

#[tauri::command]
pub async fn scan_folder(
    dir: String,
    app: tauri::AppHandle,
    buscar_similares: Option<bool>,
    umbral: Option<u32>,
) -> Result<ScanResult, String> {
    CANCEL_TOKEN.reset();

    let buscar = buscar_similares.unwrap_or(false);
    let umbral_dhash = umbral.unwrap_or(crate::hasher::DHASH_THRESHOLD);

    let paths = scanner::scan_images(&dir);
    let total = paths.len();

    if total == 0 {
        return Ok(ScanResult {
            groups: Vec::new(),
            skipped: 0,
        });
    }

    // ── FASE 1: Comparar tamaños ──────────────────────────────
    emit_progress(&app, "Comparando tamaños", 0, total, None);

    let mut size_groups: HashMap<u64, Vec<usize>> = HashMap::new();
    for (i, path) in paths.iter().enumerate() {
        if let Ok(meta) = fs::metadata(path) {
            size_groups.entry(meta.len()).or_default().push(i);
        }
    }

    let size_candidates: Vec<Vec<usize>> = if buscar {
        vec![(0..total).collect()]
    } else {
        size_groups
            .into_iter()
            .filter(|(_, idxs)| idxs.len() > 1)
            .map(|(_, idxs)| idxs)
            .collect()
    };

    let size_candidate_count: usize = size_candidates.iter().map(|g| g.len()).sum();
    let size_discarded = total - size_candidate_count;

    emit_progress(
        &app,
        "Comparando tamaños",
        total,
        total,
        Some(&format!(
            "{} descartados, {} candidatos",
            size_discarded, size_candidate_count
        )),
    );

    if cancel_check(&app).await {
        return Err("Escaneo cancelado".to_string());
    }

    // ── FASE 2: Leer metadatos ────────────────────────────────
    let cancel = CANCEL_TOKEN.clone();
    let processed = AtomicUsize::new(0);
    let skipped = AtomicUsize::new(0);

    emit_progress(&app, "Leyendo metadatos", 0, size_candidate_count, None);

    type MetaItem = (usize, String, String, u64, Option<String>, metadata::Meta);

    let meta_results: Vec<MetaItem> = size_candidates
        .iter()
        .flatten()
        .par_bridge()
        .filter_map(|&idx| {
            if cancel.is_cancelled() {
                return None;
            }
            let path = &paths[idx];
            let item = (|| -> Option<MetaItem> {
                let data = reader::FileData::open(path)?;
                let bytes = data.bytes();
                let exif = metadata::extract_from_bytes(bytes);
                let (file_name, dir_str, size_bytes, modified) = build_fs_info(path)?;
                Some((idx, file_name, dir_str, size_bytes, modified, exif))
            })();
            match item {
                Some(item) => {
                    let done = processed.fetch_add(1, Ordering::Relaxed) + 1;
                    if done % PROGRESS_BATCH == 0 || done == size_candidate_count {
                        emit_progress(&app, "Leyendo metadatos", done, size_candidate_count, None);
                    }
                    Some(item)
                }
                None => {
                    skipped.fetch_add(1, Ordering::Relaxed);
                    None
                }
            }
        })
        .collect();

    if cancel_check(&app).await {
        return Err("Escaneo cancelado".to_string());
    }

    // Agrupar por firma de metadatos
    let mut meta_signature_groups: HashMap<String, Vec<usize>> = HashMap::new();
    for (idx, _, _, _, modified, exif) in &meta_results {
        if let Some(key) = metadata::signature(
            exif.date_taken.as_deref(),
            modified.as_deref(),
            exif.width,
            exif.height,
            exif.camera.as_deref(),
        ) {
            meta_signature_groups.entry(key).or_default().push(*idx);
        }
    }

    let meta_candidates: Vec<Vec<usize>> = meta_signature_groups
        .into_iter()
        .filter(|(_, idxs)| idxs.len() > 1)
        .map(|(_, idxs)| idxs)
        .collect();

    let meta_candidate_count: usize = meta_candidates.iter().map(|g| g.len()).sum();
    let meta_discarded = meta_results.len() - meta_candidate_count;

    emit_progress(
        &app,
        "Leyendo metadatos",
        size_candidate_count,
        size_candidate_count,
        Some(&format!(
            "{} descartados, {} candidatos",
            meta_discarded, meta_candidate_count
        )),
    );

    if cancel_check(&app).await {
        return Err("Escaneo cancelado".to_string());
    }

    // ── FASE 3: Calcular hashes ───────────────────────────────
    let hash_indices: Vec<usize> = if buscar {
        meta_results.iter().map(|m| m.0).collect()
    } else {
        meta_candidates.iter().flatten().copied().collect()
    };
    let hash_total = hash_indices.len();

    emit_progress(&app, "Calculando hashes", 0, hash_total, None);

    let processed2 = AtomicUsize::new(0);
    let meta_map: HashMap<usize, MetaItem> =
        meta_results.into_iter().map(|m| (m.0, m)).collect();

    // Hashes ya vistos, para emitir duplicados en vivo durante el escaneo
    let seen_hashes: Mutex<HashMap<String, Vec<ImageInfo>>> = Mutex::new(HashMap::new());

    let images: Vec<ImageInfo> = hash_indices
        .par_iter()
        .filter_map(|&idx| {
            if cancel.is_cancelled() {
                return None;
            }
            let path = &paths[idx];
            let hash = (|| -> Option<(String, Option<u64>)> {
                let data = reader::FileData::open(path)?;
                let bytes = data.bytes();
                Some((
                    crate::hasher::sha256_bytes(bytes),
                    crate::hasher::dhash_bytes(bytes),
                ))
            })();
            let Some((hash, dh)) = hash else {
                skipped.fetch_add(1, Ordering::Relaxed);
                return None;
            };
            let (_, ref file_name, ref dir_str, size_bytes, ref modified, ref exif) =
                meta_map[&idx];

            let done = processed2.fetch_add(1, Ordering::Relaxed) + 1;
            if done % PROGRESS_BATCH == 0 || done == hash_total {
                emit_progress(&app, "Calculando hashes", done, hash_total, None);
            }

            let img = ImageInfo {
                path: path.to_string_lossy().to_string(),
                file_name: file_name.clone(),
                dir: dir_str.clone(),
                size_bytes,
                modified: modified.clone(),
                date_taken: exif.date_taken.clone(),
                camera: exif.camera.clone(),
                width: exif.width,
                height: exif.height,
                hash,
                thumbnail: None,
                dhash: dh,
            };

            emit_live_duplicate(&app, &seen_hashes, &img);

            Some(img)
        })
        .collect();

    if cancel_check(&app).await {
        return Err("Escaneo cancelado".to_string());
    }

    // ── FASE 4: Agrupar ───────────────────────────────────────
    emit_progress(&app, "Agrupando duplicados", 0, 1, None);

    let groups = crate::matcher::group(
        images,
        crate::matcher::GroupOptions {
            dhash_umbral: umbral_dhash,
            buscar_similares: buscar,
        },
    );
    let skipped = skipped.into_inner();

    emit_progress(
        &app,
        "Listo",
        1,
        1,
        Some(&format!(
            "{} grupo(s) encontrados{}",
            groups.len(),
            if skipped > 0 {
                format!(", {skipped} archivo(s) omitidos")
            } else {
                String::new()
            }
        )),
    );

    Ok(ScanResult { groups, skipped })
}

async fn cancel_check(app: &tauri::AppHandle) -> bool {
    if CANCEL_TOKEN.is_cancelled() {
        emit_progress(app, "Cancelado", 0, 0, None);
        return true;
    }
    false
}

#[tauri::command]
pub fn cancel_scan() -> Result<(), String> {
    CANCEL_TOKEN.cancel();
    Ok(())
}

#[tauri::command]
pub fn preview(path: String) -> Result<String, String> {
    let p = Path::new(&path);
    let thumb = thumbnails::generate_thumbnail(p)
        .map(|p| p.to_string_lossy().to_string())
        .ok_or_else(|| format!("No se pudo generar la vista previa: {path}"))?;
    Ok(thumb)
}

fn delete(paths: &[String], to_trash: bool) -> Result<(), String> {
    for p in paths {
        let path = Path::new(p);
        if !path.exists() {
            continue;
        }
        if to_trash {
            trash::delete(path)
                .map_err(|e| format!("No se pudo enviar a la papelera {p}: {e}"))?;
        } else {
            fs::remove_file(path)
                .map_err(|e| format!("No se pudo borrar {p}: {e}"))?;
        }
    }
    Ok(())
}

#[tauri::command]
pub fn delete_to_trash(req: DeleteRequest) -> Result<(), String> {
    delete(&req.paths, true)
}

#[tauri::command]
pub fn delete_permanent(req: DeleteRequest) -> Result<(), String> {
    delete(&req.paths, false)
}
