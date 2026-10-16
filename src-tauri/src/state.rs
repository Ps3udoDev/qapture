use std::sync::{Mutex, MutexGuard};

use rusqlite::Connection;

use crate::capture::session::{CaptureController, HIDE_DELAY, READY_TIMEOUT};
use crate::media::storage::EvidenceFiles;

pub struct AppState {
    db: Mutex<Connection>,
    files: EvidenceFiles,
    capture: CaptureController,
}

impl AppState {
    pub fn new(conn: Connection, files: EvidenceFiles) -> Self {
        Self {
            db: Mutex::new(conn),
            files,
            capture: CaptureController::new(HIDE_DELAY, Some(READY_TIMEOUT)),
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
}
