use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use rusqlite::Connection;

use crate::error::AppResult;

pub mod seed;

pub const DB_FILE: &str = "qapture.db";

/// Versión de esquema que deja esta app (`PRAGMA user_version` tras migrar).
pub fn latest_version() -> i64 {
    MIGRATIONS.len() as i64
}

/// Claves foráneas, migraciones y plantillas incluidas. También se usa tras restaurar un respaldo.
pub fn prepare(conn: &Connection) -> AppResult<()> {
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrate(conn)?;
    seed::ensure_builtin_templates(conn)
}

const MIGRATIONS: &[&str] = &[
    include_str!("migrations/0001_init.sql"),
    include_str!("migrations/0002_evidence.sql"),
    include_str!("migrations/0003_doc_version.sql"),
];

pub fn open(path: &Path) -> AppResult<Connection> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let conn = Connection::open(path)?;
    conn.busy_timeout(Duration::from_secs(5))?;
    conn.query_row("PRAGMA journal_mode = WAL", [], |_| Ok(()))?;
    backup_before_migrate(&conn, path)?;
    init(conn)
}

/// Spec §8: antes de migrar una base que ya tiene datos, deja una copia en `backups/`.
fn backup_before_migrate(conn: &Connection, path: &Path) -> AppResult<Option<PathBuf>> {
    let current = schema_version(conn)?;
    if current == 0 || current >= latest_version() {
        return Ok(None);
    }
    let dir = path
        .parent()
        .map_or_else(|| PathBuf::from("backups"), |p| p.join("backups"));
    std::fs::create_dir_all(&dir)?;
    let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
    let target = dir.join(format!("qapture-v{current}-{stamp}.db"));
    conn.execute("VACUUM INTO ?1", [target.to_string_lossy().into_owned()])?;
    Ok(Some(target))
}

#[cfg(test)]
pub fn open_in_memory() -> AppResult<Connection> {
    init(Connection::open_in_memory()?)
}

fn init(conn: Connection) -> AppResult<Connection> {
    prepare(&conn)?;
    Ok(conn)
}

/// Solo para tests: escribe una base en el esquema v1 con un proyecto «Antiguo».
#[cfg(test)]
pub(crate) fn write_v1_for_tests(path: &Path) {
    let old = Connection::open(path).unwrap();
    old.execute_batch(MIGRATIONS[0]).unwrap();
    old.pragma_update(None, "user_version", 1).unwrap();
    old.execute(
        "INSERT INTO project (id, name, created_at, updated_at) VALUES ('p-old', 'Antiguo', '', '')",
        [],
    )
    .unwrap();
}

pub fn schema_version(conn: &Connection) -> AppResult<i64> {
    Ok(conn.query_row("PRAGMA user_version", [], |r| r.get(0))?)
}

fn migrate(conn: &Connection) -> AppResult<()> {
    let current = schema_version(conn)? as usize;
    for (index, sql) in MIGRATIONS.iter().enumerate().skip(current) {
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", (index + 1) as i64)?;
        tx.commit()?;
    }
    Ok(())
}

pub fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_are_idempotent() {
        let conn = open_in_memory().unwrap();
        assert_eq!(schema_version(&conn).unwrap(), MIGRATIONS.len() as i64);
        migrate(&conn).unwrap();
        assert_eq!(schema_version(&conn).unwrap(), MIGRATIONS.len() as i64);
    }

    #[test]
    fn seeds_four_builtin_templates_once() {
        let conn = open_in_memory().unwrap();
        seed::ensure_builtin_templates(&conn).unwrap();
        let templates: i64 = conn
            .query_row("SELECT COUNT(*) FROM template WHERE builtin = 1", [], |r| {
                r.get(0)
            })
            .unwrap();
        let funcional: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM template_field WHERE template_id = 'tpl-funcional'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(templates, 4);
        assert_eq!(funcional, 11);
    }

    #[test]
    fn foreign_keys_are_enabled() {
        let conn = open_in_memory().unwrap();
        let fk: i64 = conn
            .query_row("PRAGMA foreign_keys", [], |r| r.get(0))
            .unwrap();
        assert_eq!(fk, 1);
    }

    #[test]
    fn two_connections_to_the_same_file_can_write() {
        let dir = std::env::temp_dir().join(format!("qapture-test-{}", new_id()));
        let path = dir.join("nested").join("qapture.db");
        let a = open(&path).unwrap();
        let b = open(&path).unwrap();
        let mode: String = a
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))
            .unwrap();
        assert_eq!(mode, "wal");
        for (conn, name) in [(&a, "A"), (&b, "B")] {
            conn.execute(
                "INSERT INTO template (id, name, test_type, builtin, created_at) VALUES (?1, ?2, '', 0, ?3)",
                rusqlite::params![new_id(), name, now()],
            )
            .unwrap();
        }
        let total: i64 = b
            .query_row("SELECT COUNT(*) FROM template", [], |r| r.get(0))
            .unwrap();
        assert_eq!(total, 6);
        drop(a);
        drop(b);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn backs_up_an_existing_db_before_migrating() {
        let dir = std::env::temp_dir().join(format!("qapture-test-{}", new_id()));
        let path = dir.join("qapture.db");
        std::fs::create_dir_all(&dir).unwrap();
        {
            let old = Connection::open(&path).unwrap();
            old.execute_batch(MIGRATIONS[0]).unwrap();
            old.pragma_update(None, "user_version", 1).unwrap();
            old.execute(
                "INSERT INTO template (id, name, test_type, builtin, created_at) VALUES ('t-old', 'Vieja', '', 0, '')",
                [],
            )
            .unwrap();
        }
        let conn = open(&path).unwrap();
        assert_eq!(schema_version(&conn).unwrap(), MIGRATIONS.len() as i64);
        let kept: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM template WHERE id = 't-old'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(kept, 1);
        let backups = || std::fs::read_dir(dir.join("backups")).unwrap().count();
        assert_eq!(backups(), 1);
        drop(conn);
        drop(open(&path).unwrap());
        assert_eq!(
            backups(),
            1,
            "sin migraciones pendientes no se respalda de nuevo"
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_new_db_is_not_backed_up() {
        let dir = std::env::temp_dir().join(format!("qapture-test-{}", new_id()));
        drop(open(&dir.join("qapture.db")).unwrap());
        assert!(!dir.join("backups").exists());
        let _ = std::fs::remove_dir_all(dir);
    }
}
