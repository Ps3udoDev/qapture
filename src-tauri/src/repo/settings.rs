use rusqlite::{Connection, OptionalExtension, params};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

use crate::{db::now, error::AppResult, settings::Settings};

const PROFILE: &str = "profile";
const SHORTCUTS: &str = "shortcuts";
const SYSTEM: &str = "system";
const EXPORT: &str = "export";

/// Lee cada sección por separado: una fila ausente, un JSON inválido o un campo de tipo
/// erróneo dejan esa sección (o ese campo) con su valor por defecto.
pub fn load(conn: &Connection) -> AppResult<Settings> {
    Ok(Settings {
        profile: load_section(conn, PROFILE)?,
        shortcuts: load_section(conn, SHORTCUTS)?,
        system: load_section(conn, SYSTEM)?,
        export: load_section(conn, EXPORT)?,
    })
}

pub fn save(conn: &Connection, settings: &Settings) -> AppResult<()> {
    let tx = conn.unchecked_transaction()?;
    save_section(&tx, PROFILE, &settings.profile)?;
    save_section(&tx, SHORTCUTS, &settings.shortcuts)?;
    save_section(&tx, SYSTEM, &settings.system)?;
    save_section(&tx, EXPORT, &settings.export)?;
    tx.commit()?;
    Ok(())
}

fn save_section<T: Serialize>(conn: &Connection, key: &str, section: &T) -> AppResult<()> {
    conn.execute(
        "INSERT INTO setting (key, value, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        params![key, serde_json::to_string(section)?, now()],
    )?;
    Ok(())
}

fn load_section<T>(conn: &Connection, key: &str) -> AppResult<T>
where
    T: Default + Serialize + DeserializeOwned,
{
    let raw: Option<String> = conn
        .query_row("SELECT value FROM setting WHERE key = ?1", [key], |r| {
            r.get(0)
        })
        .optional()?;
    Ok(raw
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .map_or_else(T::default, merge_over_default))
}

/// Superpone sobre el valor por defecto cada campo conocido que, tomado solo, siga
/// deserializando; los desconocidos o de tipo erróneo se descartan.
fn merge_over_default<T>(stored: Value) -> T
where
    T: Default + Serialize + DeserializeOwned,
{
    let Value::Object(stored) = stored else {
        return T::default();
    };
    let Ok(Value::Object(mut merged)) = serde_json::to_value(T::default()) else {
        return T::default();
    };
    for (key, value) in stored {
        let Some(previous) = merged.get(&key).cloned() else {
            continue;
        };
        merged.insert(key.clone(), value);
        if serde_json::from_value::<T>(Value::Object(merged.clone())).is_err() {
            merged.insert(key, previous);
        }
    }
    serde_json::from_value(Value::Object(merged)).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::*;
    use crate::{
        db,
        media::storage::TempDir,
        repo::test_support::conn,
        settings::{Settings, SystemPrefs},
    };

    #[test]
    fn a_new_db_returns_the_defaults() {
        let loaded = load(&conn()).unwrap();
        assert_eq!(loaded, Settings::default());
        assert_eq!(loaded.shortcuts.toggle, "Ctrl+Alt+Q");
        assert_eq!(loaded.shortcuts.capture, "Ctrl+Alt+S");
        assert_eq!(loaded.shortcuts.paste, "Ctrl+Alt+V");
        assert_eq!(loaded.shortcuts.new_case, "Ctrl+Alt+N");
        assert_eq!(loaded.export.image_max_width, 940);
        assert!(loaded.system.close_to_tray);
    }

    #[test]
    fn saved_settings_survive_reopening_the_file_db() {
        let tmp = TempDir::new();
        let path = tmp.path().join("qapture.db");
        let mut saved = Settings::default();
        saved.profile.tester = "Ana Pérez".into();
        saved.shortcuts.capture = "Ctrl+Shift+F12".into();
        saved.system.autostart = true;
        {
            let conn = db::open(&path).unwrap();
            save(&conn, &saved).unwrap();
        }
        let conn = db::open(&path).unwrap();
        assert_eq!(load(&conn).unwrap(), saved);
    }

    #[test]
    fn damaged_or_partial_rows_fall_back_per_field() {
        let conn = conn();
        conn.execute_batch(
            r#"INSERT INTO setting (key, value, updated_at) VALUES
               ('profile', '{"tester":"Ana"}', ''),
               ('system', 'no es json', ''),
               ('shortcuts', '{"toggle":7,"capture":"Ctrl+Alt+X"}', '')"#,
        )
        .unwrap();
        let loaded = load(&conn).unwrap();
        assert_eq!(loaded.profile.tester, "Ana");
        assert_eq!(loaded.profile.role, "");
        assert_eq!(loaded.system, SystemPrefs::default());
        assert_eq!(loaded.shortcuts.capture, "Ctrl+Alt+X");
        assert_eq!(loaded.shortcuts.toggle, "Ctrl+Alt+Q");
    }

    #[test]
    fn settings_survive_the_0004_migration() {
        let tmp = TempDir::new();
        let path = tmp.path().join("qapture.db");
        {
            let old = Connection::open(&path).unwrap();
            for sql in &db::MIGRATIONS[..3] {
                old.execute_batch(sql).unwrap();
            }
            old.pragma_update(None, "user_version", 3).unwrap();
            old.execute(
                "INSERT INTO project (id, name, created_at, updated_at) VALUES ('p-old', 'Antiguo', '', '')",
                [],
            )
            .unwrap();
        }
        let conn = db::open(&path).unwrap();
        assert_eq!(db::schema_version(&conn).unwrap(), 4);
        let kept: i64 = conn
            .query_row("SELECT COUNT(*) FROM project WHERE id = 'p-old'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(kept, 1);
        assert_eq!(load(&conn).unwrap(), Settings::default());
    }
}
