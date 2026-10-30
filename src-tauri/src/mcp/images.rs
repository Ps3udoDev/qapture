use std::{
    fs::File,
    io::{Cursor, Read},
    path::Path,
};

use base64::{Engine, engine::general_purpose::STANDARD};
use image::{ImageFormat, imageops::FilterType};
use serde_json::{Map, Value};

use crate::error::{AppResult, validation};

pub const AGENT_MAX_IMAGE_BYTES: usize = 25 * 1024 * 1024;
const UNSUPPORTED: &str = "El archivo no es una imagen PNG, JPEG o WebP.";
const TOO_LARGE: &str = "La imagen supera 25 MB.";

pub fn sniff(bytes: &[u8]) -> bool {
    bytes.starts_with(b"\x89PNG")
        || bytes.starts_with(b"\xff\xd8\xff")
        || (bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP"))
}

pub fn load(args: &Map<String, Value>) -> Result<Vec<u8>, String> {
    let path = args.get("path");
    let base64 = args.get("base64");
    let bytes = match (path, base64) {
        (Some(_), Some(_)) => return Err("Envía «path» o «base64», no los dos.".into()),
        (Some(path), None) => {
            let path = path.as_str().ok_or("«path» debe ser texto.")?;
            let path = Path::new(path);
            if !path.is_absolute() {
                return Err("La ruta debe ser absoluta.".into());
            }
            let file = File::open(path).map_err(|_| "No se pudo leer la imagen.".to_string())?;
            let metadata = file
                .metadata()
                .map_err(|_| "No se pudo leer la imagen.".to_string())?;
            if !metadata.is_file() {
                return Err(UNSUPPORTED.into());
            }
            if metadata.len() > AGENT_MAX_IMAGE_BYTES as u64 {
                return Err(TOO_LARGE.into());
            }
            let mut bytes = Vec::new();
            file.take(AGENT_MAX_IMAGE_BYTES as u64 + 1)
                .read_to_end(&mut bytes)
                .map_err(|_| "No se pudo leer la imagen.".to_string())?;
            bytes
        }
        (None, Some(value)) => {
            let encoded = value.as_str().ok_or("«base64» debe ser texto.")?;
            if encoded.len() > AGENT_MAX_IMAGE_BYTES.div_ceil(3) * 4 {
                return Err(TOO_LARGE.into());
            }
            STANDARD
                .decode(encoded)
                .map_err(|_| UNSUPPORTED.to_string())?
        }
        (None, None) => return Err("Envía «path» o «base64».".into()),
    };
    if bytes.len() > AGENT_MAX_IMAGE_BYTES {
        return Err(TOO_LARGE.into());
    }
    if !sniff(&bytes) {
        return Err(UNSUPPORTED.into());
    }
    Ok(bytes)
}

pub fn thumbnail_png(bytes: &[u8], max_side: u32) -> AppResult<Vec<u8>> {
    let normalized = crate::media::raster::normalize(bytes)?;
    let mut image =
        image::load_from_memory(&normalized.png).map_err(|_| validation(UNSUPPORTED))?;
    if image.width().max(image.height()) > max_side {
        image = image.resize(max_side, max_side, FilterType::Lanczos3);
    }
    let mut png = Vec::new();
    image
        .write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
        .map_err(|_| validation(UNSUPPORTED))?;
    Ok(png)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::{raster::sample_png, storage::TempDir};

    #[test]
    fn a_non_image_path_is_rejected_without_echoing_it() {
        let tmp = TempDir::new();
        let path = tmp.path().join("secreto.png");
        std::fs::write(&path, b"-----BEGIN OPENSSH PRIVATE KEY-----").unwrap();
        assert_eq!(
            load(serde_json::json!({"path":path}).as_object().unwrap()).unwrap_err(),
            UNSUPPORTED
        );
    }

    #[test]
    fn an_image_over_25_mb_is_rejected_before_reading() {
        let tmp = TempDir::new();
        let path = tmp.path().join("huge.png");
        File::create(&path)
            .unwrap()
            .set_len(AGENT_MAX_IMAGE_BYTES as u64 + 1)
            .unwrap();
        assert_eq!(
            load(serde_json::json!({"path":path}).as_object().unwrap()).unwrap_err(),
            TOO_LARGE
        );
    }

    #[test]
    fn path_and_base64_together_is_an_error_and_relative_paths_are_rejected() {
        assert_eq!(
            load(
                serde_json::json!({"path":"x", "base64":"x"})
                    .as_object()
                    .unwrap()
            )
            .unwrap_err(),
            "Envía «path» o «base64», no los dos."
        );
        assert_eq!(
            load(serde_json::json!({"path":"x"}).as_object().unwrap()).unwrap_err(),
            "La ruta debe ser absoluta."
        );
        assert_eq!(
            load(
                serde_json::json!({"base64": STANDARD.encode(sample_png(4,4))})
                    .as_object()
                    .unwrap()
            )
            .unwrap(),
            sample_png(4, 4)
        );
    }

    #[test]
    fn thumbnail_reduces_to_1568() {
        let png = thumbnail_png(&sample_png(2400, 1200), 1568).unwrap();
        let image = image::load_from_memory(&png).unwrap();
        assert_eq!((image.width(), image.height()), (1568, 784));
    }
}
