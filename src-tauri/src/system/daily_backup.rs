use std::{thread, time::Duration};

use tauri::{AppHandle, Manager};

use crate::state::AppState;

const CHECK_EVERY: Duration = Duration::from_secs(60 * 60);

/// Spec §5: copia diaria de la base. Se revisa al arrancar y luego cada hora, porque la app
/// puede quedar días abierta en la bandeja.
pub fn spawn(app: AppHandle) {
    thread::spawn(move || {
        loop {
            run_once(&app.state::<AppState>());
            thread::sleep(CHECK_EVERY);
        }
    });
}

pub(crate) fn run_once(state: &AppState) {
    // Nunca respaldar la base vacía del modo recuperación: rotaría las copias buenas.
    if state.recovery().is_some() {
        return;
    }
    let conn = state.db();
    if let Err(error) = state
        .backups()
        .ensure_daily(&conn, chrono::Local::now().date_naive())
    {
        eprintln!("No se pudo crear el respaldo diario: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{db, media::storage::TempDir};

    #[test]
    fn skips_the_daily_backup_in_recovery_mode() {
        let tmp = TempDir::new();
        let state = AppState::new(
            db::open_in_memory().unwrap(),
            tmp.files(),
            tmp.path(),
            Some("base dañada".into()),
        );
        run_once(&state);
        assert!(state.backups().list().unwrap().is_empty());
        state.finish_recovery(db::open(&tmp.path().join(db::DB_FILE)).unwrap());
        run_once(&state);
        run_once(&state);
        assert_eq!(state.backups().list().unwrap().len(), 1, "uno por día");
    }
}
