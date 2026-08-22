use chrono::NaiveDateTime;
use exif::{Exif, In, Reader, Tag};
use std::io::BufReader;

pub struct Meta {
    pub date_taken: Option<String>,
    pub camera: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

fn ascii_str(value: &exif::Value) -> Option<String> {
    use exif::Value;
    match value {
        Value::Ascii(arr) => {
            let bytes = arr.first()?;
            Some(String::from_utf8_lossy(bytes).trim_matches('\0').trim().to_string())
        }
        _ => None,
    }
}

fn normalize_date(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    let formats = [
        "%Y:%m:%d %H:%M:%S",
        "%Y-%m-%d %H:%M:%S",
        "%Y/%m/%d %H:%M:%S",
        "%Y:%m:%d",
        "%Y-%m-%d",
    ];
    for fmt in formats {
        if let Ok(dt) = NaiveDateTime::parse_from_str(trimmed, fmt) {
            return Some(dt.format("%Y-%m-%d %H:%M:%S").to_string());
        }
        if fmt.contains('H') {
            if let Ok(d) = chrono::NaiveDate::parse_from_str(trimmed, fmt) {
                return Some(d.format("%Y-%m-%d 00:00:00").to_string());
            }
        }
    }
    Some(trimmed.to_string())
}

fn field_str(exif: &Exif, tag: Tag) -> Option<String> {
    exif.get_field(tag, In::PRIMARY)
        .and_then(|f| ascii_str(&f.value))
}

fn field_u32(exif: &Exif, tag: Tag) -> Option<u32> {
    exif.get_field(tag, In::PRIMARY).and_then(|f| f.value.get_uint(0))
}

fn parse_exif(exif: &Exif) -> Meta {
    let date = field_str(exif, Tag::DateTimeOriginal)
        .or_else(|| field_str(exif, Tag::DateTimeDigitized))
        .or_else(|| field_str(exif, Tag::DateTime))
        .and_then(|d| normalize_date(&d));

    let make = field_str(exif, Tag::Make).unwrap_or_default();
    let model = field_str(exif, Tag::Model).unwrap_or_default();
    let camera = if make.is_empty() {
        if model.is_empty() {
            None
        } else {
            Some(model)
        }
    } else if model.is_empty() {
        Some(make)
    } else {
        Some(format!("{} {}", make, model))
    };

    let width = field_u32(exif, Tag::PixelXDimension).or_else(|| field_u32(exif, Tag::ImageWidth));
    let height = field_u32(exif, Tag::PixelYDimension).or_else(|| field_u32(exif, Tag::ImageLength));

    Meta {
        date_taken: date,
        camera,
        width,
        height,
    }
}

pub fn signature(
    date_taken: Option<&str>,
    modified: Option<&str>,
    width: Option<u32>,
    height: Option<u32>,
    camera: Option<&str>,
) -> Option<String> {
    let date = date_taken.or(modified)?;
    let w = width?;
    let h = height?;
    let cam = camera.unwrap_or("");
    Some(format!("{}|{}x{}|{}", date, w, h, cam))
}

pub fn extract_from_bytes(bytes: &[u8]) -> Meta {
    let cursor = std::io::Cursor::new(bytes);
    let mut reader = BufReader::new(cursor);
    if let Ok(exif) = Reader::new().read_from_container(&mut reader) {
        let mut meta = parse_exif(&exif);
        if meta.width.is_none() || meta.height.is_none() {
            let reader = image::ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format();
            if let Ok(img) = reader {
                if let Ok(dims) = img.into_dimensions() {
                    meta.width = Some(dims.0);
                    meta.height = Some(dims.1);
                }
            }
        }
        return meta;
    }
    let mut meta = Meta {
        date_taken: None,
        camera: None,
        width: None,
        height: None,
    };
    let reader = image::ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format();
    if let Ok(img) = reader {
        if let Ok(dims) = img.into_dimensions() {
            meta.width = Some(dims.0);
            meta.height = Some(dims.1);
        }
    }
    meta
}
