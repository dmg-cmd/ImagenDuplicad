use crate::{DupGroup, ImageInfo};
use std::collections::{HashMap, HashSet};

pub fn group(images: Vec<ImageInfo>) -> Vec<DupGroup> {
    let n = images.len();
    let mut parent: Vec<usize> = (0..n).collect();

    fn find(parent: &mut [usize], x: usize) -> usize {
        if parent[x] != x {
            let root = find(parent, parent[x]);
            parent[x] = root;
        }
        parent[x]
    }

    fn union(parent: &mut [usize], a: usize, b: usize) {
        let ra = find(parent, a);
        let rb = find(parent, b);
        if ra != rb {
            parent[rb] = ra;
        }
    }

    let mut by_hash: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, im) in images.iter().enumerate() {
        by_hash.entry(im.hash.as_str()).or_default().push(i);
    }
    for (_, idxs) in by_hash {
        if idxs.len() > 1 {
            for &j in &idxs[1..] {
                union(&mut parent, idxs[0], j);
            }
        }
    }

    let mut by_sig: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, im) in images.iter().enumerate() {
        if let Some(key) = signature(im) {
            by_sig.entry(key).or_default().push(i);
        }
    }
    for (_, idxs) in by_sig {
        if idxs.len() > 1 {
            for &j in &idxs[1..] {
                union(&mut parent, idxs[0], j);
            }
        }
    }

    let mut comps: HashMap<usize, Vec<usize>> = HashMap::new();
    for i in 0..n {
        let root = find(&mut parent, i);
        comps.entry(root).or_default().push(i);
    }

    let mut groups = Vec::new();
    for (_, idxs) in comps {
        if idxs.len() < 2 {
            continue;
        }
        let mut images: Vec<ImageInfo> = idxs.iter().map(|&i| images[i].clone()).collect();
        let hashes: HashSet<String> = images.iter().map(|i| i.hash.clone()).collect();
        let confidence = if hashes.len() < images.len() {
            "exacto"
        } else {
            "probable"
        };
        sort_by_best(&mut images);
        let total_size = images.iter().map(|i| i.size_bytes).sum();
        groups.push(DupGroup {
            confidence: confidence.to_string(),
            images,
            total_size,
        });
    }

    groups.sort_by(|a, b| b.total_size.cmp(&a.total_size));
    groups
}

fn signature(im: &ImageInfo) -> Option<String> {
    crate::metadata::signature(
        im.date_taken.as_deref(),
        im.modified.as_deref(),
        im.width,
        im.height,
        im.camera.as_deref(),
    )
}

pub(crate) fn sort_by_best(images: &mut [ImageInfo]) {
    images.sort_by(|a, b| {
        let aa = (a.width.unwrap_or(0) * a.height.unwrap_or(0)) as u64;
        let bb = (b.width.unwrap_or(0) * b.height.unwrap_or(0)) as u64;
        bb.cmp(&aa)
            .then_with(|| a.date_taken.cmp(&b.date_taken))
            .then_with(|| a.path.cmp(&b.path))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn img(path: &str, hash: &str, date: Option<&str>, w: u32, h: u32) -> ImageInfo {
        ImageInfo {
            path: path.to_string(),
            file_name: path.split('/').last().unwrap_or(path).to_string(),
            dir: String::new(),
            size_bytes: 100,
            modified: None,
            date_taken: date.map(|s| s.to_string()),
            camera: Some("Cam".to_string()),
            width: Some(w),
            height: Some(h),
            hash: hash.to_string(),
            thumbnail: None,
        }
    }

    #[test]
    fn groups_exact_duplicates() {
        let imgs = vec![
            img("a/foto.png", "hash1", Some("2020-01-01 10:00:00"), 800, 600),
            img("b/foto-copia.png", "hash1", Some("2020-01-01 10:00:00"), 800, 600),
            img("c/unic.png", "hash2", Some("2021-05-05 09:00:00"), 400, 300),
        ];
        let groups = group(imgs);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].confidence, "exacto");
        assert_eq!(groups[0].images.len(), 2);
    }

    #[test]
    fn groups_probable_by_metadata() {
        let imgs = vec![
            img("a/original.jpg", "hashA", Some("2022-03-03 12:00:00"), 1024, 768),
            img("b/recodificado.jpg", "hashB", Some("2022-03-03 12:00:00"), 1024, 768),
            img("c/otro.jpg", "hashC", Some("2022-03-03 12:00:00"), 1024, 768),
            img("d/distinto.jpg", "hashD", Some("2022-03-04 12:00:00"), 1024, 768),
        ];
        let groups = group(imgs);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].confidence, "probable");
        assert_eq!(groups[0].images.len(), 3);
    }

    #[test]
    fn merges_exact_and_metadata_groups() {
        let imgs = vec![
            img("a/original.png", "hash1", Some("2020-01-01 10:00:00"), 800, 600),
            img("b/copia.png", "hash1", Some("2020-01-01 10:00:00"), 800, 600),
            img("c/recodificado.png", "hash2", Some("2020-01-01 10:00:00"), 800, 600),
        ];
        let groups = group(imgs);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].confidence, "exacto");
        assert_eq!(groups[0].images.len(), 3);
    }

    #[test]
    fn does_not_group_without_metadata() {
        let imgs = vec![
            img("a/1.jpg", "hashA", None, 100, 100),
            img("b/2.jpg", "hashB", None, 100, 100),
        ];
        let groups = group(imgs);
        assert_eq!(groups.len(), 0);
    }

    #[test]
    fn keeps_best_first() {
        let imgs = vec![
            img("a/small.png", "hash1", Some("2020-01-01 10:00:00"), 200, 100),
            img("b/big.png", "hash1", Some("2020-01-01 10:00:00"), 2000, 1000),
        ];
        let groups = group(imgs);
        assert_eq!(groups[0].images[0].path, "b/big.png");
    }
}