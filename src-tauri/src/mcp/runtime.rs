use super::{channel, endpoint, host::TauriAgentHost, protocol};
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use tauri::{AppHandle, Manager};
use tokio::sync::broadcast;

pub struct McpRuntime {
    changed: broadcast::Sender<()>,
    disconnect: broadcast::Sender<()>,
    listening: Arc<AtomicBool>,
    key: Arc<Mutex<String>>,
    data_dir: PathBuf,
}
impl McpRuntime {
    pub fn start(app: AppHandle, data_dir: PathBuf) -> Self {
        let runtime = Self::new(data_dir);
        let loaded = endpoint::load_or_create_key(&runtime.data_dir);
        match loaded {
            Ok(key) => *runtime.key.lock().unwrap_or_else(|e| e.into_inner()) = key,
            Err(e) => {
                eprintln!("No se pudo crear la clave MCP: {e}");
                return runtime;
            }
        }
        let handler: channel::Handler = Arc::new(move |mut session, line| {
            let state = app.state::<crate::state::AppState>();
            let host = TauriAgentHost { app: app.clone() };
            let replies = protocol::handle_line(&mut session, &state, &host, &line);
            (session, replies)
        });
        let (name, key, changed, disconnect, listening) = (
            endpoint::channel_name(&runtime.data_dir),
            runtime.key.clone(),
            runtime.changed.clone(),
            runtime.disconnect.clone(),
            runtime.listening.clone(),
        );
        tauri::async_runtime::spawn(async move {
            if let Err(e) =
                channel::listen(name, key, handler, changed, disconnect, listening.clone()).await
            {
                eprintln!("No se pudo abrir el canal MCP: {e}");
            }
            listening.store(false, Ordering::Release);
        });
        runtime
    }
    pub fn new(data_dir: PathBuf) -> Self {
        let (changed, _) = broadcast::channel(32);
        let (disconnect, _) = broadcast::channel(32);
        Self {
            changed,
            disconnect,
            listening: Arc::new(AtomicBool::new(false)),
            key: Arc::new(Mutex::new(String::new())),
            data_dir,
        }
    }
    pub fn permissions_changed(&self) {
        let _ = self.changed.send(());
    }
    pub fn disconnect_all(&self) {
        let _ = self.disconnect.send(());
    }
    #[cfg(test)]
    pub(crate) fn disconnect_events(&self) -> broadcast::Receiver<()> {
        self.disconnect.subscribe()
    }
    pub fn listening(&self) -> bool {
        self.listening.load(Ordering::Acquire)
    }
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }
    pub fn regenerate_key(&self) -> std::io::Result<()> {
        let mut key = self.key.lock().unwrap_or_else(|e| e.into_inner());
        *key = endpoint::regenerate_key(&self.data_dir)?;
        self.disconnect_all();
        Ok(())
    }
}
