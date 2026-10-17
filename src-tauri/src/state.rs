use std::{
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard},
};

use rusqlite::Connection;

use crate::{
    backup::Backups,
    capture::session::{CaptureController, HIDE_DELAY, READY_TIMEOUT},
    db::DB_FILE,
    error::{AppResult, validation},
    excel::save::RecentExports,
    media::storage::EvidenceFiles,
};

pub struct AppState {
    db: Mutex<Connection>,
    files: EvidenceFiles,
    capture: CaptureController,
    exports: RecentExports,
    backups: Backups,
    db_path: PathBuf,
    /// `Some(detalle)` mientras la base está dañada y la app usa una base vacía en memoria.
    recovery: Mutex<Option<String>>,
}

#[allow(dead_code)] // Los usan los comandos de la Task 8.
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
