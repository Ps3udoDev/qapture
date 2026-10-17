use std::{
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard, RwLock},
};

use rusqlite::Connection;

use crate::{
    backup::Backups,
    capture::session::{CaptureController, HIDE_DELAY, READY_TIMEOUT},
    db::DB_FILE,
    dto::ShortcutFailureDto,
    error::{AppResult, validation},
    excel::save::RecentExports,
    media::storage::EvidenceFiles,
    settings::Settings,
    system::shortcuts::ShortcutRegistrar,
};

/// Se invoca tras guardar los ajustes (el arranque sincroniza el inicio con Windows).
pub type SavedHook = Box<dyn Fn(&Settings) + Send + Sync>;

/// Registrador de atajos que acepta todo, hasta que se instale el real.
struct NullRegistrar;

impl ShortcutRegistrar for NullRegistrar {
    fn register(&mut self, _accel: &str) -> Result<(), String> {
        Ok(())
    }
    fn unregister(&mut self, _accel: &str) {}
}

pub struct AppState {
    db: Mutex<Connection>,
    files: EvidenceFiles,
    capture: CaptureController,
    exports: RecentExports,
    backups: Backups,
    db_path: PathBuf,
    /// `Some(detalle)` mientras la base está dañada y la app usa una base vacía en memoria.
    recovery: Mutex<Option<String>>,
    /// Copia en memoria de los ajustes guardados (la lee el handler de atajos sin tocar la base).
    settings: RwLock<Settings>,
    shortcut_failures: Mutex<Vec<ShortcutFailureDto>>,
    registrar: Mutex<Box<dyn ShortcutRegistrar + Send>>,
    on_saved: Mutex<Option<SavedHook>>,
}

impl AppState {
    pub fn new(
        conn: Connection,
        files: EvidenceFiles,
        data_dir: &Path,
        recovery: Option<String>,
    ) -> Self {
        Self {
            db: Mutex::new(conn),
            files,
            capture: CaptureController::new(HIDE_DELAY, Some(READY_TIMEOUT)),
            exports: RecentExports::default(),
            backups: Backups::new(data_dir),
            db_path: data_dir.join(DB_FILE),
            recovery: Mutex::new(recovery),
            settings: RwLock::new(Settings::default()),
            shortcut_failures: Mutex::new(Vec::new()),
            registrar: Mutex::new(Box::new(NullRegistrar)),
            on_saved: Mutex::new(None),
        }
    }

    /// Una sola conexión serializada: SQLite + WAL es suficiente para una app de escritorio de un usuario.
    pub fn db(&self) -> MutexGuard<'_, Connection> {
        self.db
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn capture(&self) -> &CaptureController {
        &self.capture
    }

    pub fn files(&self) -> &EvidenceFiles {
        &self.files
    }

    pub fn exports(&self) -> &RecentExports {
        &self.exports
    }

    pub fn backups(&self) -> &Backups {
        &self.backups
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    pub fn recovery(&self) -> Option<String> {
        self.recovery
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// Importar, exportar paquetes y respaldar no corren sobre la base vacía del modo recuperación.
    pub fn ensure_ready(&self) -> AppResult<()> {
        match self.recovery() {
            Some(_) => Err(validation(
                "Qapture está en modo recuperación: restaura un respaldo o empieza con una base vacía.",
            )),
            None => Ok(()),
        }
    }

    pub fn settings(&self) -> Settings {
        self.settings
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    pub fn set_settings(&self, settings: Settings) {
        *self
            .settings
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = settings;
    }

    pub fn shortcut_failures(&self) -> Vec<ShortcutFailureDto> {
        self.shortcut_failures
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    pub fn set_shortcut_failures(&self, failures: Vec<ShortcutFailureDto>) {
        *self
            .shortcut_failures
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = failures;
    }

    /// Reemplaza el registrador nulo por el real (lo instala el arranque).
    pub fn install_registrar(&self, registrar: Box<dyn ShortcutRegistrar + Send>) {
        *self
            .registrar
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = registrar;
    }

    pub fn registrar(&self) -> MutexGuard<'_, Box<dyn ShortcutRegistrar + Send>> {
        self.registrar
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn set_on_saved(&self, hook: SavedHook) {
        *self
            .on_saved
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(hook);
    }

    pub fn notify_saved(&self, settings: &Settings) {
        let guard = self
            .on_saved
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(hook) = guard.as_ref() {
            hook(settings);
        }
    }

    pub fn finish_recovery(&self, conn: Connection) {
        *self.db() = conn;
        *self
            .recovery
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{db, media::storage::TempDir};

    #[test]
    fn recovery_blocks_data_commands_until_a_new_db_is_installed() {
        let tmp = TempDir::new();
        let state = AppState::new(
            db::open_in_memory().unwrap(),
            tmp.files(),
            tmp.path(),
            Some("base dañada".into()),
        );
        assert_eq!(state.recovery().as_deref(), Some("base dañada"));
        assert_eq!(
            state.ensure_ready().unwrap_err().to_string(),
            "Qapture está en modo recuperación: restaura un respaldo o empieza con una base vacía."
        );
        assert_eq!(state.db_path(), tmp.path().join(db::DB_FILE));
        let fresh = db::open(&tmp.path().join(db::DB_FILE)).unwrap();
        fresh
            .execute("INSERT INTO project (id, name, created_at, updated_at) VALUES ('p', 'Nueva', '', '')", [])
            .unwrap();
        state.finish_recovery(fresh);
        assert!(state.recovery().is_none());
        assert!(state.ensure_ready().is_ok());
        let count: i64 = state
            .db()
            .query_row("SELECT COUNT(*) FROM project", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1, "los comandos usan la base nueva");
    }
}
