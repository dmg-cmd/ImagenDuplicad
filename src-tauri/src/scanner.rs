use std::path::PathBuf;

const EXTS: &[&str] = &["jpg", "jpeg", "png", "webp", "gif", "tiff", "tif", "bmp"];

pub fn scan_images(root: &str, excluidas: &[String]) -> Vec<PathBuf> {
    let excluidas: Vec<PathBuf> = excluidas.iter().map(PathBuf::from).collect();

    let mut out = Vec::new();
    let walker = walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            if entry.depth() == 0 {
                return true;
            }
            let ruta = entry.path();
            !excluidas.iter().any(|x| ruta.starts_with(x))
        });

    for entry in walker {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn dir_unica(nombre: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "imagen-dup-test-{nombre}-{}",
            std::process::id() as u64 + std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64
        ));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn excluye_carpetas_y_subcarpetas() {
        let raiz = dir_unica("raiz");
        let fotos = raiz.join("fotos");
        let backup = raiz.join("backup");
        std::fs::create_dir_all(fotos.join("2024")).unwrap();
        std::fs::create_dir_all(&backup).unwrap();

        std::fs::write(fotos.join("a.jpg"), "x").unwrap();
        std::fs::write(fotos.join("2024").join("b.png"), "x").unwrap();
        std::fs::write(backup.join("c.jpg"), "x").unwrap();
        std::fs::write(raiz.join("d.webp"), "x").unwrap();

        let encontrados = scan_images(
            raiz.to_str().unwrap(),
            &[backup.to_string_lossy().to_string()],
        );
        let nombres: Vec<String> = encontrados
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();

        assert!(nombres.contains(&"a.jpg".to_string()));
        assert!(nombres.contains(&"b.png".to_string()));
        assert!(nombres.contains(&"d.webp".to_string()));
        assert!(!nombres.iter().any(|n| n == "c.jpg"));

        std::fs::remove_dir_all(&raiz).ok();
    }

    #[test]
    fn sin_exclusiones_encuentra_todo() {
        let raiz = dir_unica("todo");
        let sub = raiz.join("sub");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(raiz.join("a.jpg"), "x").unwrap();
        std::fs::write(sub.join("b.jpg"), "x").unwrap();

        let encontrados = scan_images(raiz.to_str().unwrap(), &[]);
        assert_eq!(encontrados.len(), 2);

        std::fs::remove_dir_all(&raiz).ok();
    }
}