//! Imagen del portapapeles → `_staging`. Lógica pura (sin tipos de Tauri).

use std::io::Cursor;

use image::{ImageFormat, RgbaImage};

use crate::{
    dto::StagedImageDto,
    error::{AppResult, validation},
    media::storage::EvidenceFiles,
    repo::evidence,
};

pub const NO_IMAGE: &str = "El portapapeles no contiene una imagen.";

pub enum PasteOutcome {
    Staged(StagedImageDto),
    NoImage,
    Unusable(String),
}

pub fn rgba_to_png(width: u32, height: u32, rgba: &[u8]) -> AppResult<Vec<u8>> {
    let expected = (width as usize)
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(4));
    if expected != Some(rgba.len()) {
        return Err(validation(
            "La imagen del portapapeles no se pudo interpretar.",
        ));
    }
    let image = RgbaImage::from_raw(width, height, rgba.to_vec())
        .ok_or_else(|| validation("La imagen del portapapeles no se pudo interpretar."))?;
    let mut out = Cursor::new(Vec::new());
    image
        .write_to(&mut out, ImageFormat::Png)
        .map_err(|_| validation("La imagen del portapapeles no se pudo interpretar."))?;
    Ok(out.into_inner())
}

pub fn stage_from_clipboard(
    files: &EvidenceFiles,
    image: Option<(u32, u32, Vec<u8>)>,
) -> PasteOutcome {
    let Some((width, height, rgba)) = image else {
        return PasteOutcome::NoImage;
    };
    match rgba_to_png(width, height, &rgba).and_then(|png| evidence::stage(files, &png)) {
        Ok(staged) => PasteOutcome::Staged(staged),
        Err(error) => PasteOutcome::Unusable(error.user_message()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::storage::TempDir;

    fn staged_count(files: &EvidenceFiles) -> usize {
        std::fs::read_dir(files.staging_dir())
            .map(|d| d.count())
            .unwrap_or(0)
    }

    #[test]
    fn rgba_from_the_clipboard_becomes_a_staged_png() {
        let dir = TempDir::new();
        let files = dir.files();
        let PasteOutcome::Staged(s) =
            stage_from_clipboard(&files, Some((80, 60, vec![255; 80 * 60 * 4])))
        else {
            panic!("debía quedar en staging");
        };
        assert_eq!(s.width, 80);
        let bytes = std::fs::read(&s.path).unwrap();
        assert_eq!(image::guess_format(&bytes).unwrap(), ImageFormat::Png);
    }

    #[test]
    fn an_empty_clipboard_is_reported_not_staged() {
        let dir = TempDir::new();
        let files = dir.files();
        assert!(matches!(
            stage_from_clipboard(&files, None),
            PasteOutcome::NoImage
        ));
        assert_eq!(staged_count(&files), 0);
    }

    #[test]
    fn an_oversized_or_malformed_image_is_unusable() {
        let dir = TempDir::new();
        let files = dir.files();
        match stage_from_clipboard(&files, Some((12_001, 1, vec![0; 12_001 * 4]))) {
            PasteOutcome::Unusable(m) => assert_eq!(
                m,
                "La imagen mide 12001×1 px; el máximo es 12000 px por lado."
            ),
            _ => panic!("debía ser inutilizable"),
        }
        assert!(matches!(
            stage_from_clipboard(&files, Some((10, 10, vec![0; 7]))),
            PasteOutcome::Unusable(_)
        ));
        assert_eq!(staged_count(&files), 0);
        assert!(rgba_to_png(2, 2, &[0; 15]).is_err());
    }
}
