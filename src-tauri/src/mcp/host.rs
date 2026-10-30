use crate::error::AppResult;
use image::RgbaImage;
use std::path::PathBuf;
#[derive(serde::Serialize, Debug, Clone)]
pub struct WindowInfo {
    pub id: u32,
    pub title: String,
    pub app: String,
}
pub trait AgentScreen: Send + Sync {
    fn monitors(&self) -> AppResult<Vec<RgbaImage>>;
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
