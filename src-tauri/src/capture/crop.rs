use std::io::Cursor;

use image::{DynamicImage, ImageFormat, RgbaImage, imageops};
use serde::Deserialize;

use crate::error::{AppError, AppResult, validation};

const TOO_SMALL: &str = "La selección es demasiado pequeña.";
const INVALID: &str = "La selección no es válida.";

/// Rectángulo que envía el overlay en píxeles CSS de su ventana, junto con el tamaño de esa
/// ventana. El cociente imagen/ventana absorbe el escalado DPI del monitor (100 %, 125 %, 150 %…),
/// sin depender del factor de escala que reporte el sistema.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Selection {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub viewport_width: f64,
    pub viewport_height: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

pub fn full(width: u32, height: u32) -> PixelRect {
    PixelRect {
        x: 0,
        y: 0,
        width,
        height,
    }
}

/// Evita que un 375.00000000001 se convierta en 376 al redondear hacia arriba.
fn snap(v: f64) -> f64 {
    (v * 1e6).round() / 1e6
}

/// Convierte la selección a píxeles de la imagen del monitor. Los bordes izquierdo y superior
/// se redondean hacia abajo y los bordes derecho e inferior hacia arriba, para no perder la
/// fila o columna que el usuario ve parcialmente incluida. Luego se ajusta al monitor.
pub fn to_pixels(
    selection: &Selection,
    image_width: u32,
    image_height: u32,
) -> AppResult<PixelRect> {
    let s = selection;
    let values = [
        s.x,
        s.y,
        s.width,
        s.height,
        s.viewport_width,
        s.viewport_height,
    ];
    if values.iter().any(|v| !v.is_finite())
        || s.viewport_width <= 0.0
        || s.viewport_height <= 0.0
        || s.width < 0.0
        || s.height < 0.0
    {
        return Err(validation(INVALID));
    }
    let sx = f64::from(image_width) / s.viewport_width;
    let sy = f64::from(image_height) / s.viewport_height;
    let clamp = |v: f64, max: u32| v.clamp(0.0, f64::from(max));
    let left = clamp(snap(s.x * sx).floor(), image_width);
    let top = clamp(snap(s.y * sy).floor(), image_height);
    let right = clamp(snap((s.x + s.width) * sx).ceil(), image_width);
    let bottom = clamp(snap((s.y + s.height) * sy).ceil(), image_height);
    if right - left < 1.0 || bottom - top < 1.0 {
        return Err(validation(TOO_SMALL));
    }
    Ok(PixelRect {
        x: left as u32,
        y: top as u32,
        width: (right - left) as u32,
        height: (bottom - top) as u32,
    })
}

/// Recorta y codifica en PNG RGB: las capturas de pantalla no tienen transparencia.
pub fn crop_png(image: &RgbaImage, rect: PixelRect) -> AppResult<Vec<u8>> {
    let region = imageops::crop_imm(image, rect.x, rect.y, rect.width, rect.height).to_image();
    let rgb = DynamicImage::ImageRgb8(DynamicImage::ImageRgba8(region).to_rgb8());
    let mut png = Vec::new();
    rgb.write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
        .map_err(|e| AppError::Capture(e.to_string()))?;
    Ok(png)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sel(x: f64, y: f64, width: f64, height: f64, vw: f64, vh: f64) -> Selection {
        Selection {
            x,
            y,
            width,
            height,
            viewport_width: vw,
            viewport_height: vh,
        }
    }

    #[test]
    fn scales_css_pixels_to_physical_pixels_at_100_125_and_150_percent() {
        // La ventana overlay mide (imagen / escala) px CSS: así lo reporta WebView2 en cada monitor.
        for (scale, (iw, ih)) in [
            (1.0, (1920, 1080)),
            (1.25, (1920, 1080)),
            (1.5, (2880, 1620)),
        ] {
            let (vw, vh) = (f64::from(iw) / scale, f64::from(ih) / scale);
            let rect = to_pixels(&sel(100.0, 40.0, 200.0, 120.0, vw, vh), iw, ih).unwrap();
            let px = |v: f64| (v * scale).round() as u32;
            assert_eq!(
                rect,
                PixelRect {
                    x: px(100.0),
                    y: px(40.0),
                    width: px(200.0),
                    height: px(120.0)
                },
                "escala {scale}"
            );
        }
    }

    #[test]
    fn partially_covered_pixels_are_included() {
        // 125 %: 10.4 → 13.0, 10.6 → 13.25 (piso 13); 30.6 → 38.25 (techo 39); 15.6 → 19.5 (techo 20).
        let rect = to_pixels(&sel(10.4, 10.6, 20.2, 5.0, 1536.0, 864.0), 1920, 1080).unwrap();
        assert_eq!(
            rect,
            PixelRect {
                x: 13,
                y: 13,
                width: 26,
                height: 7
            }
        );
    }

    #[test]
    fn float_noise_does_not_add_an_extra_pixel() {
        // 0.1 + 0.2 = 0.30000000000000004: sin ajuste, el techo daría un píxel de más.
        let rect = to_pixels(&sel(0.1, 0.0, 0.2, 10.0, 100.0, 100.0), 1000, 1000).unwrap();
        assert_eq!(
            rect,
            PixelRect {
                x: 1,
                y: 0,
                width: 2,
                height: 100
            }
        );
    }

    #[test]
    fn selections_beyond_the_monitor_are_clamped() {
        let rect = to_pixels(
            &sel(-50.0, -50.0, 5000.0, 5000.0, 1536.0, 864.0),
            1920,
            1080,
        )
        .unwrap();
        assert_eq!(rect, full(1920, 1080));
    }

    #[test]
    fn rejects_empty_offscreen_or_malformed_selections() {
        let too_small = "La selección es demasiado pequeña.";
        let invalid = "La selección no es válida.";
        for (s, message) in [
            (sel(10.0, 10.0, 0.0, 50.0, 100.0, 100.0), too_small),
            (sel(2000.0, 10.0, 50.0, 50.0, 100.0, 100.0), too_small),
            (sel(f64::NAN, 10.0, 50.0, 50.0, 100.0, 100.0), invalid),
            (sel(10.0, 10.0, f64::INFINITY, 50.0, 100.0, 100.0), invalid),
            (sel(10.0, 10.0, 50.0, 50.0, 0.0, 100.0), invalid),
            (sel(10.0, 10.0, -5.0, 50.0, 100.0, 100.0), invalid),
        ] {
            assert_eq!(
                to_pixels(&s, 1920, 1080).unwrap_err().to_string(),
                message,
                "{s:?}"
            );
        }
    }

    #[test]
    fn selection_deserializes_from_camel_case_and_rejects_unknown_fields() {
        let ok = r#"{"x":1,"y":2,"width":3,"height":4,"viewportWidth":5,"viewportHeight":6}"#;
        assert_eq!(
            serde_json::from_str::<Selection>(ok).unwrap(),
            sel(1.0, 2.0, 3.0, 4.0, 5.0, 6.0)
        );
        let extra = r#"{"x":1,"y":2,"width":3,"height":4,"viewportWidth":5,"viewportHeight":6,"monitor":0}"#;
        assert!(serde_json::from_str::<Selection>(extra).is_err());
    }

    #[test]
    fn crop_png_keeps_only_the_selected_pixels_without_alpha() {
        let mut image = RgbaImage::from_pixel(4, 2, image::Rgba([200, 30, 30, 255]));
        for y in 0..2 {
            for x in 2..4 {
                image.put_pixel(x, y, image::Rgba([43, 98, 158, 255]));
            }
        }
        let png = crop_png(
            &image,
            PixelRect {
                x: 2,
                y: 0,
                width: 2,
                height: 2,
            },
        )
        .unwrap();
        let decoded = image::load_from_memory(&png).unwrap();
        assert_eq!(decoded.color(), image::ColorType::Rgb8);
        assert_eq!((decoded.width(), decoded.height()), (2, 2));
        assert!(decoded.to_rgb8().pixels().all(|p| p.0 == [43, 98, 158]));
    }
}
