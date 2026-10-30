use crate::error::{AppResult, validation};
use image::RgbaImage;
use std::path::PathBuf;
use tauri::{Emitter, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};
use tauri_plugin_notification::NotificationExt;
#[derive(serde::Serialize, Debug, Clone)]
pub struct WindowInfo {
    pub id: u32,
    pub title: String,
    pub app: String,
}
pub trait AgentScreen: Send + Sync {
    fn monitors(&self) -> AppResult<Vec<RgbaImage>>;
    fn monitor_count(&self) -> AppResult<usize> {
        Ok(self.monitors()?.len())
    }
    fn capture_monitor(&self, index: usize) -> AppResult<RgbaImage> {
        let mut monitors = self.monitors()?;
        if index >= monitors.len() {
            return Err(validation(format!(
                "No existe el monitor {index}. Hay {}.",
                monitors.len()
            )));
        }
        Ok(monitors.swap_remove(index))
    }
    fn windows(&self) -> AppResult<Vec<WindowInfo>>;
    fn capture_window(&self, id: u32) -> AppResult<RgbaImage>;
}
pub trait AgentHost: Send + Sync {
    fn data_changed(&self, project_id: Option<&str>, case_id: Option<&str>, summary: &str);
    fn notify(&self, message: &str);
    fn confirm(&self, title: &str, message: &str) -> bool;
    fn screen(&self) -> &dyn AgentScreen;
    fn default_export_dir(&self) -> PathBuf;
}
pub struct XcapAgentScreen;
impl AgentScreen for XcapAgentScreen {
    fn monitor_count(&self) -> AppResult<usize> {
        xcap::Monitor::all()
            .map(|monitors| monitors.len())
            .map_err(|_| validation("No se pudieron consultar los monitores."))
    }
    fn capture_monitor(&self, index: usize) -> AppResult<RgbaImage> {
        let monitors = xcap::Monitor::all()
            .map_err(|_| validation("No se pudieron consultar los monitores."))?;
        let monitor = monitors.get(index).ok_or_else(|| {
            validation(format!(
                "No existe el monitor {index}. Hay {}.",
                monitors.len()
            ))
        })?;
        monitor
            .capture_image()
            .map_err(|_| validation("No se pudo capturar el monitor."))
    }
    fn monitors(&self) -> AppResult<Vec<RgbaImage>> {
        xcap::Monitor::all()
            .map_err(|_| validation("No se pudieron consultar los monitores."))?
            .iter()
            .map(|m| {
                m.capture_image()
                    .map_err(|_| validation("No se pudo capturar el monitor."))
            })
            .collect()
    }
    fn windows(&self) -> AppResult<Vec<WindowInfo>> {
        Ok(xcap::Window::all()
            .map_err(|_| validation("No se pudieron consultar las ventanas."))?
            .into_iter()
            .filter_map(|w| {
                let title = w.title().ok()?;
                if title.starts_with("Qapture") || w.is_minimized().unwrap_or(true) {
                    return None;
                }
                Some(WindowInfo {
                    id: w.id().ok()?,
                    title,
                    app: w.app_name().unwrap_or_default(),
                })
            })
            .collect())
    }
    fn capture_window(&self, id: u32) -> AppResult<RgbaImage> {
        let window = xcap::Window::all()
            .map_err(|_| validation("No se pudieron consultar las ventanas."))?
            .into_iter()
            .find(|w| w.id().ok() == Some(id))
            .ok_or_else(|| validation("La ventana ya no está disponible."))?;
        window
            .capture_image()
            .map_err(|_| validation("No se pudo capturar la ventana."))
    }
}
pub struct TauriAgentHost {
    pub app: tauri::AppHandle,
}
impl AgentHost for TauriAgentHost {
    fn data_changed(&self, project_id: Option<&str>, case_id: Option<&str>, summary: &str) {
        let _ = self.app.emit_to(
            "main",
            "agent:data-changed",
            serde_json::json!({"projectId": project_id, "caseId": case_id, "summary": summary}),
        );
    }
    fn notify(&self, message: &str) {
        let _ = self
            .app
            .notification()
            .builder()
            .title("Qapture")
            .body(message)
            .show();
    }
    fn confirm(&self, title: &str, message: &str) -> bool {
        self.app
            .dialog()
            .message(message)
            .title(title)
            .buttons(MessageDialogButtons::OkCancelCustom(
                "Permitir".into(),
                "Rechazar".into(),
            ))
            .blocking_show()
    }
    fn screen(&self) -> &dyn AgentScreen {
        &XcapAgentScreen
    }
    fn default_export_dir(&self) -> PathBuf {
        self.app
            .path()
            .document_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
    }
}
#[cfg(test)]
pub struct RecordingHost {
    pub calls: std::sync::Mutex<Vec<String>>,
    pub allow: bool,
    pub dir: PathBuf,
}
#[cfg(test)]
impl Default for RecordingHost {
    fn default() -> Self {
        Self {
            calls: Default::default(),
            allow: true,
            dir: std::env::temp_dir(),
        }
    }
}
#[cfg(test)]
impl AgentScreen for RecordingHost {
    fn monitors(&self) -> AppResult<Vec<RgbaImage>> {
        Ok(vec![RgbaImage::new(4, 4), RgbaImage::new(4, 4)])
    }
    fn windows(&self) -> AppResult<Vec<WindowInfo>> {
        Ok(vec![WindowInfo {
            id: 1,
            title: "Editor de pruebas".into(),
            app: "Editor".into(),
        }])
    }
    fn capture_window(&self, _: u32) -> AppResult<RgbaImage> {
        Ok(RgbaImage::new(4, 4))
    }
}
#[cfg(test)]
impl AgentHost for RecordingHost {
    fn data_changed(&self, p: Option<&str>, c: Option<&str>, s: &str) {
        self.calls
            .lock()
            .unwrap()
            .push(format!("changed:{p:?}:{c:?}:{s}"));
    }
    fn notify(&self, s: &str) {
        self.calls.lock().unwrap().push(s.into());
    }
    fn confirm(&self, _: &str, s: &str) -> bool {
        self.calls.lock().unwrap().push(s.into());
        self.allow
    }
    fn screen(&self) -> &dyn AgentScreen {
        self
    }
    fn default_export_dir(&self) -> PathBuf {
        self.dir.clone()
    }
}
