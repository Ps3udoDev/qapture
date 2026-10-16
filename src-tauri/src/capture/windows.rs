use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder,
};

use super::{screen::MonitorBounds, session::CaptureHost};
use crate::{
    dto::CapturedEventDto,
    error::{AppError, AppResult},
    system::tray,
};

pub const OVERLAY_PREFIX: &str = "overlay-";
pub const CAPTURED_EVENT: &str = "evidence:captured";

pub fn overlay_label(index: usize) -> String {
    format!("{OVERLAY_PREFIX}{index}")
}

/// Índice del monitor de una ventana overlay; `None` para cualquier otra ventana.
pub fn overlay_index(label: &str) -> Option<usize> {
    let digits = label.strip_prefix(OVERLAY_PREFIX)?;
    let index: usize = digits.parse().ok()?;
    (overlay_label(index) == label).then_some(index)
}

fn window_error(error: tauri::Error) -> AppError {
    AppError::Capture(error.to_string())
}

pub struct TauriHost<'a> {
    pub app: &'a AppHandle,
}

impl CaptureHost for TauriHost<'_> {
    fn hide_main(&self) -> bool {
        let Some(main) = self.app.get_webview_window("main") else {
            return false;
        };
        let visible = main.is_visible().unwrap_or(false) && !main.is_minimized().unwrap_or(false);
        let _ = main.hide();
        visible
    }

    fn show_main(&self) {
        tray::show_main(self.app);
    }

    fn open_overlays(&self, monitors: &[MonitorBounds]) -> AppResult<()> {
        for (index, m) in monitors.iter().enumerate() {
            let window = WebviewWindowBuilder::new(
                self.app,
                overlay_label(index),
                WebviewUrl::App("overlay.html".into()),
            )
            .title("Qapture · Captura")
            .decorations(false)
            .resizable(false)
            .shadow(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .focused(false)
            // Oculta hasta que pinta la imagen congelada (`overlay_ready`): sin parpadeo blanco.
            .visible(false)
            .build()
            .map_err(window_error)?;
            // Primero se mueve al monitor (la ventana adopta su DPI) y después se fija el tamaño físico.
            let position = PhysicalPosition::new(m.x, m.y);
            window.set_position(position).map_err(window_error)?;
            window
                .set_size(PhysicalSize::new(m.width, m.height))
                .map_err(window_error)?;
            // WM_DPICHANGED (Windows 10 antiguos) puede desplazarla al cambiar de DPI: se reafirma.
            window.set_position(position).map_err(window_error)?;
        }
        Ok(())
    }

    fn close_overlays(&self) {
        for (label, window) in self.app.webview_windows() {
            if overlay_index(&label).is_some() {
                let _ = window.destroy();
            }
        }
    }

    fn show_overlay(&self, index: usize, focus: bool) {
        if let Some(window) = self.app.get_webview_window(&overlay_label(index)) {
            let _ = window.show();
            if focus {
                let _ = window.set_focus();
            }
        }
    }

    fn emit_captured(&self, tmp_path: &str) {
        let payload = CapturedEventDto {
            tmp_path: tmp_path.into(),
        };
        let _ = self.app.emit_to("main", CAPTURED_EVENT, payload);
    }

    fn cursor(&self) -> Option<(f64, f64)> {
        self.app.cursor_position().ok().map(|p| (p.x, p.y))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlay_labels_round_trip_and_reject_other_windows() {
        assert_eq!(overlay_label(0), "overlay-0");
        assert_eq!(overlay_index("overlay-0"), Some(0));
        assert_eq!(overlay_index(&overlay_label(12)), Some(12));
        for label in [
            "main",
            "overlay-",
            "overlay-x",
            "overlay-01",
            "overlay--1",
            "xoverlay-1",
            "overlay-1 ",
        ] {
            assert_eq!(overlay_index(label), None, "{label}");
        }
    }
}
