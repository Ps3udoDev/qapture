use std::sync::{Mutex, MutexGuard};

use rusqlite::Connection;

use crate::media::storage::EvidenceFiles;

pub struct AppState {
    db: Mutex<Connection>,
    files: EvidenceFiles,
}

impl AppState {
    pub fn new(conn: Connection, files: EvidenceFiles) -> Self {
        Self {
            db: Mutex::new(conn),
            files,
        }
    }

    /// Una sola conexión serializada: SQLite + WAL es suficiente para una app de escritorio de un usuario.
    pub fn db(&self) -> MutexGuard<'_, Connection> {
        self.db
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn files(&self) -> &EvidenceFiles {
        &self.files
    }
}
