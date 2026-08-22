use image::imageops::FilterType;
use sha2::{Digest, Sha256};

pub const DHASH_THRESHOLD: u32 = 8;

pub fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

pub fn dhash_bytes(bytes: &[u8]) -> Option<u64> {
    let img = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .ok()?
        .decode()
        .ok()?;
    Some(dhash_image(&img))
}

pub fn dhash_image(img: &image::DynamicImage) -> u64 {
    let gray = img
        .grayscale()
        .resize_exact(9, 8, FilterType::Triangle)
        .into_luma8();

    let mut hash: u64 = 0;
    for y in 0..8 {
        for x in 0..8 {
            let left = gray.get_pixel(x, y)[0] as i16;
            let right = gray.get_pixel(x + 1, y)[0] as i16;
            hash = (hash << 1) | (left > right) as u64;
        }
    }
    hash
}

pub fn hamming_distance(a: u64, b: u64) -> u32 {
    (a ^ b).count_ones()
}

pub fn visualmente_similares(a: Option<u64>, b: Option<u64>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => hamming_distance(a, b) <= DHASH_THRESHOLD,
        _ => false,
    }
}
