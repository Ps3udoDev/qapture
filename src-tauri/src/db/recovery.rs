//! Spec §8: si la base no abre por corrupción, la app arranca en modo recuperación y ofrece
//! restaurar un respaldo o empezar con una base vacía. La base dañada nunca se borra.

use std::{
    fs,
    path::{Path, PathBuf},
};

use chrono::NaiveDateTime;
use rusqlite::{Connection, ErrorCode};

use crate::{
    backup::Backups,
    db,
    error::{AppError, AppResult},
    media::storage::EvidenceFiles,
    repo::evidence,
};

pub enum Startup {
    Ready(Connection),
    /// Detalle técnico del fallo, para el registro y la pantalla de recuperación.
    Recovery(String),
}

pub fn is_corruption(error: &AppError) -> bool {
    match error {
        AppError::Corrupt(_) => true,
        AppError::Db(rusqlite::Error::SqliteFailure(failure, _)) => matches!(
            failure.code,
            ErrorCode::DatabaseCorrupt | ErrorCode::NotADatabase
        ),
        _ => false,
    }
}

/// Otros errores (permisos, disco) siguen impidiendo el arranque, como antes de F5.
pub fn open_or_recover(path: &Path) -> AppResult<Startup> {
    match db::open(path) {
        Ok(conn) => Ok(Startup::Ready(conn)),
        Err(error) if is_corruption(&error) => Ok(Startup::Recovery(error.to_string())),
        Err(error) => Err(error),
    }
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut raw = path.as_os_str().to_owned();
    raw.push(suffix);
    PathBuf::from(raw)
}

/// Renombra la base (y sus `-wal`/`-shm`) a `qapture-danada-<fecha>.db` en la misma carpeta.
pub fn set_aside_corrupt(path: &Path, now: NaiveDateTime) -> AppResult<Option<PathBuf>> {
    if !path.exists() {
        return Ok(None);
    }
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let target = dir.join(format!("qapture-danada-{}.db", now.format("%Y%m%d-%H%M%S")));
    fs::rename(path, &target)?;
    for suffix in ["-wal", "-shm"] {
        let side = with_suffix(path, suffix);
        if side.exists() {
            let _ = fs::rename(&side, with_suffix(&target, suffix));
        }
    }
    Ok(Some(target))
}

/// Sale del modo recuperación con una base nueva en `path`: la copia de `backup` o, sin él,
/// una vacía. El respaldo se valida antes de mover nada. Las capturas que la base nueva no
/// conoce se apartan en `evidence-sin-uso/` (la limpieza del arranque las borraría).
#[allow(dead_code)] // Lo usa recover_database (Task 8).
pub fn recover(
    path: &Path,
    backups: &Backups,
    files: &EvidenceFiles,
    backup: Option<&str>,
    now: NaiveDateTime,
) -> AppResult<Connection> {
    let source = match backup {
        Some(name) => {
            let source = backups.resolve(name)?;
            Backups::check(&source)?;
            Some(source)
        }
        None => None,
    };
    set_aside_corrupt(path, now)?;
    if let Some(source) = &source {
        fs::copy(source, path)?;
    }
    let conn = db::open(path)?;
    let (referenced, projects) = evidence::references(&conn)?;
    files.set_aside(
        &referenced,
        &projects,
        &now.format("%Y%m%d-%H%M%S").to_string(),
    )?;
    Ok(conn)
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;
    use crate::{
        media::storage::{TempDir, UNUSED_DIR},
        repo::{projects, test_support},
    };

    const GARBAGE: &[u8] = b"esto no es una base de datos de Qapture ";

    fn now() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 10, 16)
            .unwrap()
            .and_hms_opt(10, 0, 0)
            .unwrap()
    }

    fn db_path(tmp: &TempDir) -> PathBuf {
        tmp.path().join(db::DB_FILE)
    }

    #[test]
    fn a_healthy_db_opens_normally() {
        let tmp = TempDir::new();
        assert!(matches!(
            open_or_recover(&db_path(&tmp)).unwrap(),
            Startup::Ready(_)
        ));
    }

    #[test]
    fn a_file_that_is_not_a_database_opens_in_recovery_and_is_kept() {
        let tmp = TempDir::new();
        let garbage = GARBAGE.repeat(200);
        fs::write(db_path(&tmp), &garbage).unwrap();
        let Startup::Recovery(detail) = open_or_recover(&db_path(&tmp)).unwrap() else {
            panic!("debía entrar en modo recuperación");
        };
        assert!(!detail.is_empty());
        assert_eq!(
            fs::read(db_path(&tmp)).unwrap(),
            garbage,
            "no se toca la base dañada"
        );
    }

    #[test]
    fn damaged_pages_are_detected_by_quick_check() {
        let tmp = TempDir::new();
        {
            let conn = db::open(&db_path(&tmp)).unwrap();
            conn.execute_batch("CREATE TABLE relleno (x TEXT);")
                .unwrap();
            for _ in 0..300 {
                conn.execute("INSERT INTO relleno (x) VALUES (?1)", ["x".repeat(1000)])
                    .unwrap();
            }
        }
        let mut bytes = fs::read(db_path(&tmp)).unwrap();
        let len = bytes.len();
        bytes[len - 3 * 4096..].fill(0xFF);
        fs::write(db_path(&tmp), bytes).unwrap();
        assert!(matches!(
            open_or_recover(&db_path(&tmp)).unwrap(),
            Startup::Recovery(_)
        ));
    }

    #[test]
    fn is_corruption_only_matches_damaged_databases() {
        assert!(is_corruption(&AppError::Corrupt("x".into())));
        assert!(!is_corruption(&AppError::Validation("x".into())));
        assert!(!is_corruption(&AppError::Io(std::io::Error::other(
            "disco lleno"
        ))));
    }

    #[test]
    fn recover_with_a_backup_sets_the_damaged_file_aside() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let backups = Backups::new(tmp.path());
        let saved = {
            let conn = db::open(&db_path(&tmp)).unwrap();
            test_support::project(&conn);
            backups.create_manual(&conn, now()).unwrap()
        };
        fs::write(db_path(&tmp), GARBAGE.repeat(200)).unwrap();

        let conn = recover(
            &db_path(&tmp),
            &backups,
            &files,
            Some(&saved.file_name),
            now(),
        )
        .unwrap();
        let names: Vec<String> = projects::list(&conn)
            .unwrap()
            .into_iter()
            .map(|p| p.name)
            .collect();
        assert_eq!(names, ["Recibos"]);
        let aside = tmp.path().join("qapture-danada-20261016-100000.db");
        assert_eq!(
            fs::read(aside).unwrap(),
            GARBAGE.repeat(200),
            "la base dañada se conserva"
        );
    }

    #[test]
    fn recover_without_a_backup_starts_empty_and_keeps_the_images_aside() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let image = files.absolute(&EvidenceFiles::original_rel("p1", "e1"));
        fs::create_dir_all(image.parent().unwrap()).unwrap();
        fs::write(&image, b"png").unwrap();
        fs::write(db_path(&tmp), GARBAGE.repeat(200)).unwrap();

        let conn = recover(
            &db_path(&tmp),
            &Backups::new(tmp.path()),
            &files,
            None,
            now(),
        )
        .unwrap();
        assert!(projects::list(&conn).unwrap().is_empty());
        assert_eq!(db::schema_version(&conn).unwrap(), db::latest_version());
        let aside = tmp
            .path()
            .join(UNUSED_DIR)
            .join("20261016-100000")
            .join("p1")
            .join("e1.orig.png");
        assert_eq!(
            fs::read(aside).unwrap(),
            b"png",
            "las capturas no se pierden"
        );
    }

    #[test]
    fn a_damaged_backup_is_refused_before_touching_the_current_file() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let backups = Backups::new(tmp.path());
        fs::create_dir_all(backups.dir()).unwrap();
        fs::write(
            backups.dir().join("qapture-manual-20261016-080000.db"),
            b"roto",
        )
        .unwrap();
        fs::write(db_path(&tmp), GARBAGE.repeat(200)).unwrap();
        let err = recover(
            &db_path(&tmp),
            &backups,
            &files,
            Some("qapture-manual-20261016-080000.db"),
            now(),
        )
        .unwrap_err();
        assert_eq!(
            err.to_string(),
            "El respaldo está dañado y no se puede restaurar."
        );
        assert_eq!(fs::read(db_path(&tmp)).unwrap(), GARBAGE.repeat(200));
        assert!(
            !tmp.path()
                .join("qapture-danada-20261016-100000.db")
                .exists()
        );
    }
}
