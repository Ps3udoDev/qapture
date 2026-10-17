use std::io::Cursor;

use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader, Limits, imageops::FilterType};

use crate::error::{AppError, AppResult, validation};

pub const MAX_INPUT_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_INPUT_SIDE: u32 = 12_000;
pub const MAX_STORED_SIDE: u32 = 2_400;

const UNSUPPORTED: &str = "El archivo no es una imagen PNG, JPEG o WebP.";

pub struct NormalizedImage {
    pub png: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

fn reader(bytes: &[u8]) -> AppResult<ImageReader<Cursor<&[u8]>>> {
    if bytes.len() > MAX_INPUT_BYTES {
        return Err(validation("La imagen supera 64 MB."));
    }
    let format = image::guess_format(bytes).map_err(|_| validation(UNSUPPORTED))?;
    if !matches!(
        format,
        ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::WebP
    ) {
        return Err(validation(UNSUPPORTED));
    }
    let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
    let mut limits = Limits::default();
    limits.max_alloc = Some(1024 * 1024 * 1024);
    reader.limits(limits);
    Ok(reader)
}

pub fn dimensions(bytes: &[u8]) -> AppResult<(u32, u32)> {
    reader(bytes)?
        .into_dimensions()
        .map_err(|_| validation(UNSUPPORTED))
}

fn check_side(width: u32, height: u32) -> AppResult<()> {
    if width == 0 || height == 0 {
        return Err(validation(UNSUPPORTED));
    }
    if width > MAX_INPUT_SIDE || height > MAX_INPUT_SIDE {
        return Err(validation(format!(
            "La imagen mide {width}×{height} px; el máximo es {MAX_INPUT_SIDE} px por lado."
        )));
    }
    Ok(())
}

fn encode_png(img: &DynamicImage) -> AppResult<Vec<u8>> {
    let mut png = Vec::new();
    img.write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
        .map_err(|e| AppError::Io(std::io::Error::other(e.to_string())))?;
    Ok(png)
}

pub fn normalize(bytes: &[u8]) -> AppResult<NormalizedImage> {
    let (width, height) = dimensions(bytes)?;
    check_side(width, height)?;
    let mut decoder = reader(bytes)?
        .into_decoder()
        .map_err(|_| validation(UNSUPPORTED))?;
    let orientation = decoder
        .orientation()
        .unwrap_or(image::metadata::Orientation::NoTransforms);
    let mut img = DynamicImage::from_decoder(decoder)
        .map_err(|_| validation("La imagen está dañada o incompleta."))?;
    // Las fotos de celular traen la rotación en EXIF: se aplica para guardarlas derechas.
    img.apply_orientation(orientation);
    if img.width().max(img.height()) > MAX_STORED_SIDE {
        img = img.resize(MAX_STORED_SIDE, MAX_STORED_SIDE, FilterType::Lanczos3);
    }
    let img = if img.color().has_alpha() {
        DynamicImage::ImageRgba8(img.to_rgba8())
    } else {
        DynamicImage::ImageRgb8(img.to_rgb8())
    };
    Ok(NormalizedImage {
        png: encode_png(&img)?,
        width: img.width(),
        height: img.height(),
    })
}

/// Importación de la v1: decodifica la imagen completa (un PNG truncado falla aquí) y la
/// devuelve como PNG del mismo tamaño. A diferencia de `normalize`, no reduce a 2400 px.
pub fn to_png(bytes: &[u8]) -> AppResult<NormalizedImage> {
    let (width, height) = dimensions(bytes)?;
    check_side(width, height)?;
    let img = reader(bytes)?
        .decode()
        .map_err(|_| validation("La imagen está dañada o incompleta."))?;
    Ok(NormalizedImage {
        png: encode_png(&img)?,
        width,
        height,
    })
}

/// Paquetes `.qapture`: el archivo debe ser un PNG completo de hasta 12000 px por lado.
pub fn verify_png(bytes: &[u8]) -> AppResult<(u32, u32)> {
    if image::guess_format(bytes).ok() != Some(ImageFormat::Png) {
        return Err(validation(UNSUPPORTED));
    }
    let (width, height) = dimensions(bytes)?;
    check_side(width, height)?;
    reader(bytes)?
        .decode()
        .map_err(|_| validation("La imagen está dañada o incompleta."))?;
    Ok((width, height))
}

pub fn validate_render(bytes: &[u8]) -> AppResult<(u32, u32)> {
    let invalid =
        || validation("El render de la evidencia debe ser un PNG de hasta 2400 px por lado.");
    if image::guess_format(bytes).ok() != Some(ImageFormat::Png) {
        return Err(invalid());
    }
    let (w, h) = dimensions(bytes).map_err(|_| invalid())?;
    if w == 0 || h == 0 || w > MAX_STORED_SIDE || h > MAX_STORED_SIDE {
        return Err(invalid());
    }
    Ok((w, h))
}

#[cfg(test)]
pub(crate) fn sample_png(width: u32, height: u32) -> Vec<u8> {
    let img = DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
        width,
        height,
        image::Rgb([40, 98, 158]),
    ));
    encode_png(&img).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode(img: DynamicImage, format: ImageFormat) -> Vec<u8> {
        let mut out = Vec::new();
        img.write_to(&mut Cursor::new(&mut out), format).unwrap();
        out
    }

    fn rgb(w: u32, h: u32) -> DynamicImage {
        DynamicImage::ImageRgb8(image::RgbImage::from_pixel(w, h, image::Rgb([200, 30, 30])))
    }

    #[test]
    fn to_png_keeps_the_size_and_detects_truncated_files() {
        let out = to_png(&encode(rgb(3000, 20), ImageFormat::Jpeg)).unwrap();
        assert_eq!((out.width, out.height), (3000, 20), "no se reduce a 2400");
        assert_eq!(image::guess_format(&out.png).unwrap(), ImageFormat::Png);
        let png = sample_png(64, 48);
        assert!(to_png(&png[..png.len() / 2]).is_err());
        assert!(to_png(b"hola").is_err());
    }

    #[test]
    fn verify_png_requires_a_complete_png() {
        assert_eq!(verify_png(&sample_png(64, 48)).unwrap(), (64, 48));
        let png = sample_png(64, 48);
        assert!(verify_png(&png[..png.len() - 20]).is_err(), "truncado");
        assert!(
            verify_png(&encode(rgb(10, 10), ImageFormat::Jpeg)).is_err(),
            "no es PNG"
        );
        assert!(verify_png(&sample_png(12_001, 1)).is_err());
    }

    #[test]
    fn keeps_small_images_and_outputs_png() {
        let out = normalize(&sample_png(800, 600)).unwrap();
        assert_eq!((out.width, out.height), (800, 600));
        assert_eq!(image::guess_format(&out.png).unwrap(), ImageFormat::Png);
    }

    #[test]
    fn downscales_the_longest_side_to_2400() {
        let wide = normalize(&sample_png(3000, 1500)).unwrap();
        assert_eq!((wide.width, wide.height), (2400, 1200));
        let tall = normalize(&sample_png(1000, 3000)).unwrap();
        assert_eq!((tall.width, tall.height), (800, 2400));
    }

    #[test]
    fn accepts_jpeg_and_webp() {
        for format in [ImageFormat::Jpeg, ImageFormat::WebP] {
            let out = normalize(&encode(rgb(320, 200), format)).unwrap();
            assert_eq!((out.width, out.height), (320, 200), "{format:?}");
        }
    }

    #[test]
    fn keeps_transparency() {
        let rgba = DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            10,
            10,
            image::Rgba([0, 0, 0, 0]),
        ));
        let out = normalize(&encode(rgba, ImageFormat::Png)).unwrap();
        assert!(
            image::load_from_memory(&out.png)
                .unwrap()
                .color()
                .has_alpha()
        );
    }

    #[test]
    fn rejects_non_images_and_other_formats() {
        let gif = encode(rgb(4, 4), ImageFormat::Gif);
        for bytes in [
            b"".to_vec(),
            b"hola mundo".to_vec(),
            b"%PDF-1.7".to_vec(),
            gif,
        ] {
            assert_eq!(
                normalize(&bytes).err().unwrap().to_string(),
                "El archivo no es una imagen PNG, JPEG o WebP."
            );
        }
    }

    #[test]
    fn rejects_more_than_12000_px_per_side() {
        let err = normalize(&sample_png(12_001, 1)).err().unwrap().to_string();
        assert_eq!(
            err,
            "La imagen mide 12001×1 px; el máximo es 12000 px por lado."
        );
    }

    #[test]
    fn validate_render_requires_png_up_to_2400() {
        assert_eq!(validate_render(&sample_png(2400, 10)).unwrap(), (2400, 10));
        assert!(validate_render(&sample_png(2401, 10)).is_err());
        assert!(validate_render(&encode(rgb(10, 10), ImageFormat::Jpeg)).is_err());
        assert!(validate_render(b"").is_err());
    }
}
