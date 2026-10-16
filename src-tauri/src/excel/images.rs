use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

use image::{
    ImageFormat,
    codecs::png::{CompressionType, FilterType as PngFilter, PngEncoder},
    imageops::FilterType,
};

use crate::{
    error::{AppError, AppResult},
    media::raster,
};

/// Imagen lista para insertar en la hoja Evidencias.
#[derive(Debug, Clone, PartialEq)]
pub struct SheetImage {
    pub png: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// Lee un render y lo reduce a `max_width` px de ancho (nunca lo agranda). Un PNG que ya cabe
/// se usa tal cual: no se decodifica.
pub fn prepare(path: &Path, max_width: u32) -> AppResult<SheetImage> {
    let bytes = fs::read(path)?;
    let (width, height) = raster::dimensions(&bytes)?;
    let is_png = image::guess_format(&bytes).ok() == Some(ImageFormat::Png);
    if width <= max_width && is_png {
        return Ok(SheetImage {
            png: bytes,
            width,
            height,
        });
    }
    let img = image::load_from_memory(&bytes)
        .map_err(|e| AppError::Export(format!("imagen dañada: {e}")))?;
    let (w, h) = if width > max_width {
        let h = (f64::from(height) * f64::from(max_width) / f64::from(width)).round();
        (max_width, (h as u32).max(1))
    } else {
        (width, height)
    };
    let small = if (w, h) == (width, height) {
        img
    } else {
        img.resize_exact(w, h, FilterType::Triangle)
    };
    let mut png = Vec::new();
    small
        .write_with_encoder(PngEncoder::new_with_quality(
            &mut png,
            CompressionType::Fast,
            PngFilter::Adaptive,
        ))
        .map_err(|e| AppError::Export(e.to_string()))?;
    Ok(SheetImage {
        png,
        width: w,
        height: h,
    })
}

/// Prepara todas las imágenes en paralelo (hasta 8 hilos) conservando el orden.
/// Una imagen que falta o está dañada queda en `None`; el libro muestra un aviso en su lugar.
/// `on_done(n)` se llama cada vez que termina una, con el total terminado hasta ese momento.
pub fn prepare_all(
    paths: &[PathBuf],
    max_width: u32,
    on_done: &(dyn Fn(usize) + Sync),
) -> Vec<Option<SheetImage>> {
    let workers = std::thread::available_parallelism()
        .map_or(2, |n| n.get())
        .clamp(1, 8)
        .min(paths.len().max(1));
    let next = AtomicUsize::new(0);
    let done = AtomicUsize::new(0);
    let slots: Vec<Mutex<Option<SheetImage>>> = paths.iter().map(|_| Mutex::new(None)).collect();
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(path) = paths.get(i) else { break };
                    let image = match prepare(path, max_width) {
                        Ok(image) => Some(image),
                        Err(error) => {
                            eprintln!("No se pudo preparar {}: {error}", path.display());
                            None
                        }
                    };
                    *slots[i].lock().unwrap_or_else(|p| p.into_inner()) = image;
                    on_done(done.fetch_add(1, Ordering::Relaxed) + 1);
                }
            });
        }
    });
    slots
        .into_iter()
        .map(|slot| slot.into_inner().unwrap_or_else(|p| p.into_inner()))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::{excel::testing::write_png, media::storage::TempDir};

    #[test]
    fn small_pngs_are_used_as_they_are() {
        let tmp = TempDir::new();
        let path = write_png(tmp.path(), "a.png", 800, 450, 1);
        let image = prepare(&path, 940).unwrap();
        assert_eq!((image.width, image.height), (800, 450));
        assert_eq!(image.png, fs::read(&path).unwrap());
    }

    #[test]
    fn wide_images_shrink_to_the_max_width_keeping_the_ratio() {
        let tmp = TempDir::new();
        let path = write_png(tmp.path(), "a.png", 2400, 1350, 1);
        let image = prepare(&path, 940).unwrap();
        assert_eq!((image.width, image.height), (940, 529));
        assert_eq!(raster::dimensions(&image.png).unwrap(), (940, 529));
        assert_eq!(image::guess_format(&image.png).unwrap(), ImageFormat::Png);
    }

    #[test]
    fn missing_or_broken_files_fail() {
        let tmp = TempDir::new();
        let broken = tmp.path().join("roto.png");
        fs::write(&broken, b"no es png").unwrap();
        assert!(prepare(&tmp.path().join("nada.png"), 940).is_err());
        assert!(prepare(&broken, 940).is_err());
    }

    #[test]
    fn prepare_all_keeps_order_reports_progress_and_marks_failures() {
        let tmp = TempDir::new();
        let paths = vec![
            write_png(tmp.path(), "a.png", 1200, 600, 1),
            tmp.path().join("falta.png"),
            write_png(tmp.path(), "c.png", 300, 200, 3),
        ];
        let seen = Mutex::new(Vec::new());
        let images = prepare_all(&paths, 600, &|n| seen.lock().unwrap().push(n));
        assert_eq!(images.len(), 3);
        assert_eq!(
            images[0].as_ref().map(|i| (i.width, i.height)),
            Some((600, 300))
        );
        assert!(images[1].is_none());
        assert_eq!(images[2].as_ref().map(|i| i.width), Some(300));
        let mut seen = seen.into_inner().unwrap();
        seen.sort_unstable();
        assert_eq!(seen, [1, 2, 3]);
        assert!(prepare_all(&[], 600, &|_| {}).is_empty());
    }
}
