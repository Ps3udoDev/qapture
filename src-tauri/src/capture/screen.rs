use image::{
    ExtendedColorType, ImageEncoder, RgbaImage,
    codecs::png::{CompressionType, FilterType, PngEncoder},
};

use crate::error::{AppError, AppResult};

/// Posición y tamaño de un monitor en píxeles físicos del escritorio virtual: el principal
/// empieza en 0,0 y los que están a la izquierda o arriba tienen offset negativo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MonitorBounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl MonitorBounds {
    pub fn contains(&self, x: f64, y: f64) -> bool {
        let (left, top) = (f64::from(self.x), f64::from(self.y));
        x >= left
            && y >= top
            && x < left + f64::from(self.width)
            && y < top + f64::from(self.height)
    }
}

pub struct CapturedMonitor {
    pub bounds: MonitorBounds,
    pub image: RgbaImage,
}

/// La pantalla real (`XcapScreen`) o un doble en los tests.
pub trait Screen {
    fn capture_all(&self) -> AppResult<Vec<CapturedMonitor>>;
}

fn capture_error(error: impl std::fmt::Display) -> AppError {
    AppError::Capture(error.to_string())
}

pub struct XcapScreen;

impl Screen for XcapScreen {
    fn capture_all(&self) -> AppResult<Vec<CapturedMonitor>> {
        let monitors = xcap::Monitor::all().map_err(capture_error)?;
        let mut captured = Vec::with_capacity(monitors.len());
        for monitor in monitors {
            // En Windows, xcap lee posición y tamaño de DEVMODE: siempre en píxeles físicos.
            let bounds = MonitorBounds {
                x: monitor.x().map_err(capture_error)?,
                y: monitor.y().map_err(capture_error)?,
                width: monitor.width().map_err(capture_error)?,
                height: monitor.height().map_err(capture_error)?,
            };
            let image = monitor.capture_image().map_err(capture_error)?;
            captured.push(CapturedMonitor { bounds, image });
        }
        if captured.is_empty() {
            return Err(capture_error("no se encontró ningún monitor"));
        }
        Ok(captured)
    }
}

/// Índice del monitor que contiene el punto (coordenadas físicas del escritorio virtual).
pub fn monitor_at(monitors: &[MonitorBounds], x: f64, y: f64) -> Option<usize> {
    monitors.iter().position(|m| m.contains(x, y))
}

/// PNG con compresión rápida para mostrar la imagen congelada en el overlay; el recorte
/// final se hace sobre la imagen en memoria, no sobre este PNG.
pub fn preview_png(image: &RgbaImage) -> AppResult<Vec<u8>> {
    let mut png = Vec::new();
    PngEncoder::new_with_quality(&mut png, CompressionType::Fast, FilterType::Sub)
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            ExtendedColorType::Rgba8,
        )
        .map_err(capture_error)?;
    Ok(png)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bounds(x: i32, y: i32, width: u32, height: u32) -> MonitorBounds {
        MonitorBounds {
            x,
            y,
            width,
            height,
        }
    }

    #[test]
    fn finds_the_monitor_under_a_point_with_negative_offsets() {
        // Izquierda 1920×1080 en x negativo; principal 2880×1620 (150 %); arriba 2560×1440 en y negativo.
        let monitors = [
            bounds(-1920, 0, 1920, 1080),
            bounds(0, 0, 2880, 1620),
            bounds(0, -1440, 2560, 1440),
        ];
        assert_eq!(monitor_at(&monitors, -1.0, 500.0), Some(0));
        assert_eq!(monitor_at(&monitors, -1920.0, 0.0), Some(0));
        assert_eq!(monitor_at(&monitors, 0.0, 0.0), Some(1));
        assert_eq!(monitor_at(&monitors, 2879.5, 1619.5), Some(1));
        assert_eq!(monitor_at(&monitors, 100.0, -1.0), Some(2));
        assert_eq!(monitor_at(&monitors, 2880.0, 0.0), None);
        assert_eq!(monitor_at(&monitors, -1921.0, 0.0), None);
        assert_eq!(monitor_at(&[], 0.0, 0.0), None);
    }

    #[test]
    fn preview_is_a_png_with_the_monitor_size() {
        let image = RgbaImage::from_pixel(64, 36, image::Rgba([43, 98, 158, 255]));
        let png = preview_png(&image).unwrap();
        assert_eq!(image::guess_format(&png).unwrap(), image::ImageFormat::Png);
        assert_eq!(crate::media::raster::dimensions(&png).unwrap(), (64, 36));
    }

    /// Manual, con un escritorio real: `cargo test real_screen -- --ignored --nocapture`.
    #[test]
    #[ignore = "necesita un escritorio real"]
    fn captures_the_real_screen() {
        let monitors = XcapScreen.capture_all().unwrap();
        assert!(!monitors.is_empty());
        for m in &monitors {
            println!(
                "{:?} → imagen {}×{}",
                m.bounds,
                m.image.width(),
                m.image.height()
            );
            assert!(m.image.width() > 0 && m.image.height() > 0);
        }
    }
}
