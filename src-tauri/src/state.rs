use std::sync::{Mutex, MutexGuard};

use rusqlite::Connection;

pub struct AppState {
    db: Mutex<Connection>,
}

impl AppState {
    pub fn new(conn: Connection) -> Self {
        Self {
            db: Mutex::new(conn),
        }
    }

    /// Una sola conexión serializada: SQLite + WAL es suficiente para una app de escritorio de un usuario.
    pub fn db(&self) -> MutexGuard<'_, Connection> {
        self.db
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}
