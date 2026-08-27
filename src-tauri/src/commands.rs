use crate::metadata;
use crate::reader;
use crate::scanner;
use crate::thumbnails;
use crate::{DeleteRequest, DupGroup, ImageInfo, ScanProgress, ScanResult, CANCEL_TOKEN};
use image::imageops::FilterType;
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::UNIX_EPOCH;
use tauri::{Emitter, Manager};

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
    excluidas: Option<Vec<String>>,
) -> Result<ScanResult, String> {
    CANCEL_TOKEN.reset();

    let buscar = buscar_similares.unwrap_or(false);
    let umbral_dhash = umbral.unwrap_or(crate::hasher::DHASH_THRESHOLD);

    let paths = scanner::scan_images(&dir, &excluidas.unwrap_or_default());
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

fn history_file(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("No se pudo resolver el directorio de datos: {e}"))?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("historial_borrados.csv"))
}

fn csv_escape(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}

fn registrar_historial(modo: &str, entradas: &[String], app: &tauri::AppHandle) {
    if entradas.is_empty() {
        return;
    }
    let Ok(file) = history_file(app) else {
        return;
    };
    let nueva = !file.exists();
    let Ok(f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&file)
    else {
        return;
    };
    use std::io::Write;
    let mut w = std::io::BufWriter::new(f);
    if nueva {
        let _ = writeln!(w, "fecha,modo,ruta");
    }
    let fecha = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    for ruta in entradas {
        let _ = writeln!(w, "{},{},{}", fecha, modo, csv_escape(ruta));
    }
}

#[tauri::command]
pub fn mover_imagen(
    origen: String,
    dir_destino: String,
    app: tauri::AppHandle,
) -> Result<String, String> {
    let src = Path::new(&origen);
    if !src.is_file() {
        return Err(format!("El archivo no existe: {origen}"));
    }
    let dst_dir = Path::new(&dir_destino);
    if !dst_dir.is_dir() {
        return Err(format!("La carpeta destino no existe: {dir_destino}"));
    }

    let nombre = src
        .file_name()
        .ok_or_else(|| format!("Ruta inválida: {origen}"))?;
    let mut destino = dst_dir.join(nombre);

    if destino.exists() {
        let stem = src
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "imagen".to_string());
        let ext = src
            .extension()
            .map(|e| e.to_string_lossy().to_string())
            .unwrap_or_default();
        let mut i = 2;
        loop {
            destino = if ext.is_empty() {
                dst_dir.join(format!("{stem} ({i})"))
            } else {
                dst_dir.join(format!("{stem} ({i}).{ext}"))
            };
            if !destino.exists() {
                break;
            }
            i += 1;
        }
    }

    if fs::rename(src, &destino).is_err() {
        fs::copy(src, &destino)
            .map_err(|e| format!("No se pudo copiar a {}: {e}", destino.display()))?;
        fs::remove_file(src).map_err(|e| format!("No se pudo quitar el original: {e}"))?;
    }

    registrar_historial(
        "movida",
        &[format!(
            "{} -> {}",
            origen,
            destino.to_string_lossy()
        )],
        &app,
    );

    Ok(destino.to_string_lossy().to_string())
}

#[tauri::command]
pub fn abrir_historial(app: tauri::AppHandle) -> Result<(), String> {
    let file = history_file(&app)?;
    if !file.exists() {
        std::fs::write(&file, "fecha,modo,ruta\n")
            .map_err(|e| format!("No se pudo crear el historial: {e}"))?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&file)
            .spawn()
            .map_err(|e| format!("No se pudo abrir el historial: {e}"))?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&file)
            .spawn()
            .map_err(|e| format!("No se pudo abrir el historial: {e}"))?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&file)
            .spawn()
            .map_err(|e| format!("No se pudo abrir el historial: {e}"))?;
    }
    Ok(())
}

#[tauri::command]
pub fn abrir_en_explorador(ruta: String) -> Result<(), String> {
    let p = Path::new(&ruta);
    if !p.exists() {
        return Err(format!("La carpeta o archivo no existe: {ruta}"));
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&ruta)
            .spawn()
            .map_err(|e| format!("No se pudo abrir el gestor de archivos: {e}"))?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&ruta)
            .spawn()
            .map_err(|e| format!("No se pudo abrir el explorador: {e}"))?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&ruta)
            .spawn()
            .map_err(|e| format!("No se pudo abrir Finder: {e}"))?;
    }
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

const DIFF_THRESHOLD: i32 = 20;
const DIFF_MAX_WIDTH: u32 = 1600;

#[tauri::command]
pub fn image_diff(path_a: String, path_b: String) -> Result<String, String> {
    let decode =
        |p: &str| -> Result<image::DynamicImage, String> {
            image::ImageReader::open(p)
                .map_err(|e| format!("No se pudo abrir {p}: {e}"))?
                .decode()
                .map_err(|e| format!("No se pudo decodificar {p}: {e}"))
        };

    let a = decode(&path_a)?;
    let b = decode(&path_b)?;

    if a.width() != b.width() || a.height() != b.height() {
        return Err(format!(
            "Las imágenes tienen dimensiones distintas ({}x{} vs {}x{}); la comparación de diferencias requiere el mismo tamaño",
            a.width(),
            a.height(),
            b.width(),
            b.height()
        ));
    }

    let cw = a.width().min(DIFF_MAX_WIDTH);
    let ch = ((a.height() as u64 * cw as u64) / a.width().max(1) as u64).max(1) as u32;

    let ra = a.resize_exact(cw, ch, FilterType::Triangle).into_rgb8();
    let rb = b.resize_exact(cw, ch, FilterType::Triangle).into_rgb8();

    let mut out = image::RgbImage::new(cw, ch);
    let mut hay_diferencias = false;
    for (x, y, pa) in ra.enumerate_pixels() {
        let pb = rb.get_pixel(x, y);
        let d = (pa[0].abs_diff(pb[0]) as i32
            + pa[1].abs_diff(pb[1]) as i32
            + pa[2].abs_diff(pb[2]) as i32)
            / 3;
        let p = out.get_pixel_mut(x, y);
        if d > DIFF_THRESHOLD {
            hay_diferencias = true;
            let k = (d as f32 / 80.0).min(1.0);
            *p = image::Rgb([(220.0 * k + 20.0) as u8, 15, 25]);
        } else {
            let lum = (0.299 * pa[0] as f32 + 0.587 * pa[1] as f32 + 0.114 * pa[2] as f32) as u8;
            let g = 40 + (lum as u16 * 3 / 10) as u8;
            *p = image::Rgb([g, g, g]);
        }
    }

    if !hay_diferencias {
        return Ok(String::new());
    }

    let key = crate::hasher::sha256_bytes(format!("{path_a}|{path_b}|v2").as_bytes());
    let out_path = thumbnails::cache_dir().join(format!("diff-{key}.png"));
    out.save(&out_path)
        .map_err(|e| format!("No se pudo guardar el mapa de diferencias: {e}"))?;
    Ok(out_path.to_string_lossy().to_string())
}

#[cfg(target_os = "android")]
fn eliminar_archivo(
    path: &Path,
    to_trash: bool,
    app: &tauri::AppHandle,
) -> Result<(), String> {
    if !to_trash {
        return fs::remove_file(path).map_err(|e| format!("No se pudo borrar {}: {e}", path.display()));
    }
    let papelera = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("papelera");
    fs::create_dir_all(&papelera).map_err(|e| e.to_string())?;
    let nombre = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "imagen".to_string());
    let mut destino = papelera.join(&nombre);
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "imagen".to_string());
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_string())
        .unwrap_or_default();
    let mut i = 2;
    while destino.exists() {
        destino = if ext.is_empty() {
            papelera.join(format!("{stem} ({i})"))
        } else {
            papelera.join(format!("{stem} ({i}).{ext}"))
        };
        i += 1;
    }
    if fs::rename(path, &destino).is_err() {
        fs::copy(path, &destino)
            .map_err(|e| format!("No se pudo copiar a la papelera interna: {e}"))?;
        fs::remove_file(path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(not(target_os = "android"))]
fn eliminar_archivo(
    path: &Path,
    to_trash: bool,
    _app: &tauri::AppHandle,
) -> Result<(), String> {
    if to_trash {
        trash::delete(path)
            .map_err(|e| format!("No se pudo enviar a la papelera {}: {e}", path.display()))
    } else {
        fs::remove_file(path).map_err(|e| format!("No se pudo borrar {}: {e}", path.display()))
    }
}

fn delete(paths: &[String], to_trash: bool, app: &tauri::AppHandle) -> Result<(), String> {
    let mut eliminados: Vec<String> = Vec::new();
    for p in paths {
        let path = Path::new(p);
        if !path.exists() {
            continue;
        }
        eliminar_archivo(path, to_trash, app)?;
        eliminados.push(p.clone());
    }
    registrar_historial(
        if to_trash { "papelera" } else { "permanente" },
        &eliminados,
        app,
    );
    Ok(())
}

#[tauri::command]
pub fn delete_to_trash(req: DeleteRequest, app: tauri::AppHandle) -> Result<(), String> {
    delete(&req.paths, true, &app)
}

#[tauri::command]
pub fn delete_permanent(req: DeleteRequest, app: tauri::AppHandle) -> Result<(), String> {
    delete(&req.paths, false, &app)
}
