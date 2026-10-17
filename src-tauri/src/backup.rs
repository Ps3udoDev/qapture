//! UC-20: respaldo diario de la base (se conservan 7), respaldo manual y restauración.
//! Las copias se hacen con la API de backup de SQLite, que da una copia consistente aunque
//! la base esté en uso, y se escriben en un temporal que luego se renombra.

use std::{
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

use chrono::{DateTime, Local, NaiveDate, NaiveDateTime};
use rusqlite::{Connection, MAIN_DB, OpenFlags, backup::Progress};

use crate::{
    db::{self, new_id},
    dto::{BackupDto, BackupKind, RestoreResultDto},
    error::{AppError, AppResult, validation},
    media::storage::EvidenceFiles,
    repo::evidence,
};

pub const BACKUP_DIR: &str = "backups";
pub const KEEP_DAILY: usize = 7;
const DAILY: &str = "qapture-diario-";
const MANUAL: &str = "qapture-manual";
const BEFORE_RESTORE: &str = "qapture-antes-de-restaurar";

fn not_found() -> AppError {
    AppError::NotFound("El respaldo no existe.".into())
}

fn damaged() -> AppError {
    validation("El respaldo está dañado y no se puede restaurar.")
}

/// Tipo de respaldo según su nombre; `None` para cualquier otro archivo de la carpeta.
fn kind_of(name: &str) -> Option<BackupKind> {
    let stem = name.strip_suffix(".db")?;
    if stem.starts_with(DAILY) {
        Some(BackupKind::Daily)
    } else if stem.starts_with(&format!("{MANUAL}-")) {
        Some(BackupKind::Manual)
    } else if stem.starts_with(&format!("{BEFORE_RESTORE}-")) {
        Some(BackupKind::BeforeRestore)
    } else {
        // `qapture-v<N>-…`: los deja db::open antes de migrar (F1).
        let (version, _) = stem.strip_prefix("qapture-v")?.split_once('-')?;
        (!version.is_empty() && version.bytes().all(|b| b.is_ascii_digit()))
            .then_some(BackupKind::BeforeMigration)
    }
}

/// La copia hereda de la base viva la marca de WAL en la cabecera. Pasarla a `DELETE` la deja
/// en un solo archivo (sin `-wal`/`-shm`): se abre en solo lectura y se copia tal cual al recuperar.
fn standalone(path: &Path) -> AppResult<()> {
    let conn = Connection::open(path)?;
    conn.query_row("PRAGMA journal_mode = DELETE", [], |_| Ok(()))?;
    Ok(())
}

pub struct Backups {
    dir: PathBuf,
}

impl Backups {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            dir: data_dir.join(BACKUP_DIR),
        }
    }

    #[cfg(test)]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn write(&self, conn: &Connection, name: &str) -> AppResult<PathBuf> {
        fs::create_dir_all(&self.dir)?;
        let dest = self.dir.join(name);
        let tmp = self.dir.join(format!("{name}.{}.tmp", new_id()));
        let result = conn
            .backup(MAIN_DB, &tmp, None)
            .map_err(AppError::from)
            .and_then(|()| standalone(&tmp))
            .and_then(|()| Ok(fs::rename(&tmp, &dest)?));
        if let Err(error) = result {
            let _ = fs::remove_file(&tmp);
            return Err(error);
        }
        Ok(dest)
    }

    /// `<prefijo>-AAAAMMDD-HHMMSS.db`, con `-2`, `-3`… si ya existe uno en el mismo segundo.
    fn stamped(&self, prefix: &str, now: NaiveDateTime) -> String {
        let base = format!("{prefix}-{}", now.format("%Y%m%d-%H%M%S"));
        let mut name = format!("{base}.db");
        let mut n = 2;
        while self.dir.join(&name).exists() {
            name = format!("{base}-{n}.db");
            n += 1;
        }
        name
    }

    fn describe(&self, path: &Path) -> AppResult<BackupDto> {
        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let kind = kind_of(&file_name).ok_or_else(not_found)?;
        let meta = fs::metadata(path)?;
        let modified: DateTime<Local> = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH).into();
        Ok(BackupDto {
            file_name,
            kind,
            created_at: modified.format("%Y-%m-%dT%H:%M:%S").to_string(),
            size_bytes: meta.len() as i64,
        })
    }

    fn names(&self) -> AppResult<Vec<String>> {
        if !self.dir.exists() {
            return Ok(Vec::new());
        }
        let mut names = Vec::new();
        for entry in fs::read_dir(&self.dir)? {
            let name = entry?.file_name().to_string_lossy().into_owned();
            if kind_of(&name).is_some() {
                names.push(name);
            }
        }
        Ok(names)
    }

    /// Una copia por día (fecha local). Solo rotan los diarios: quedan los 7 más recientes.
    pub fn ensure_daily(
        &self,
        conn: &Connection,
        today: NaiveDate,
    ) -> AppResult<Option<BackupDto>> {
        let name = format!("{DAILY}{}.db", today.format("%Y%m%d"));
        if self.dir.join(&name).exists() {
            return Ok(None);
        }
        let path = self.write(conn, &name)?;
        let mut daily: Vec<String> = self
            .names()?
            .into_iter()
            .filter(|n| kind_of(n) == Some(BackupKind::Daily))
            .collect();
        daily.sort();
        let excess = daily.len().saturating_sub(KEEP_DAILY);
        for old in &daily[..excess] {
            fs::remove_file(self.dir.join(old))?;
        }
        Ok(Some(self.describe(&path)?))
    }

    pub fn create_manual(&self, conn: &Connection, now: NaiveDateTime) -> AppResult<BackupDto> {
        let path = self.write(conn, &self.stamped(MANUAL, now))?;
        self.describe(&path)
    }

    /// Del más reciente al más antiguo.
    pub fn list(&self) -> AppResult<Vec<BackupDto>> {
        let mut list = self
            .names()?
            .into_iter()
            .map(|name| self.describe(&self.dir.join(name)))
            .collect::<AppResult<Vec<_>>>()?;
        list.sort_by(|a, b| {
            b.created_at
                .cmp(&a.created_at)
                .then_with(|| b.file_name.cmp(&a.file_name))
        });
        Ok(list)
    }

    /// Solo nombres simples de la carpeta `backups/` con forma de respaldo: el nombre llega del webview.
    pub fn resolve(&self, file_name: &str) -> AppResult<PathBuf> {
        let plain = !file_name.contains(['/', '\\']) && !file_name.contains("..");
        if !plain || kind_of(file_name).is_none() {
            return Err(not_found());
        }
        let path = self.dir.join(file_name);
        if path.is_file() {
            Ok(path)
        } else {
            Err(not_found())
        }
    }

    /// Un respaldo restaurable: base SQLite íntegra de Qapture y no más nueva que esta app.
    pub fn check(path: &Path) -> AppResult<()> {
        let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|_| damaged())?;
        let integrity: String = conn
            .query_row("PRAGMA quick_check", [], |r| r.get(0))
            .map_err(|_| damaged())?;
        let has_projects: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'project')",
                [],
                |r| r.get(0),
            )
            .map_err(|_| damaged())?;
        let version = db::schema_version(&conn).map_err(|_| damaged())?;
        if integrity != "ok" || !has_projects || version < 1 {
            return Err(damaged());
        }
        if version > db::latest_version() {
            return Err(validation(
                "El respaldo es de una versión más nueva de Qapture. Actualiza la app para restaurarlo.",
            ));
        }
        Ok(())
    }

    /// Vuelve al estado del respaldo sobre la conexión viva. Antes guarda una copia del estado
    /// actual y, al final, aparta las capturas que el respaldo no conoce.
    pub fn restore(
        &self,
        conn: &mut Connection,
        files: &EvidenceFiles,
        file_name: &str,
        now: NaiveDateTime,
    ) -> AppResult<RestoreResultDto> {
        let source = self.resolve(file_name)?;
        Self::check(&source)?;
        let restored = self.describe(&source)?;
        let safety = self.write(conn, &self.stamped(BEFORE_RESTORE, now))?;
        conn.restore(MAIN_DB, &source, None::<fn(Progress)>)?;
        db::prepare(conn)?;
        let (referenced, projects) = evidence::references(conn)?;
        let stamp = now.format("%Y%m%d-%H%M%S").to_string();
        let set_aside_files = files.set_aside(&referenced, &projects, &stamp)? as i64;
        Ok(RestoreResultDto {
            restored,
            safety_copy: self.describe(&safety)?.file_name,
            missing_images: evidence::missing_images(conn, files)?,
            set_aside_files,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        dto::NewEvidenceMeta,
        media::{
            raster::sample_png,
            storage::{TempDir, UNUSED_DIR},
        },
        repo::{evidence, projects, test_support},
        transfer::bundle::EMPTY_ANNOTATIONS,
    };

    fn day(d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, d).unwrap()
    }

    fn at(h: u32, m: u32, s: u32) -> NaiveDateTime {
        day(16).and_hms_opt(h, m, s).unwrap()
    }

    fn open(tmp: &TempDir) -> Connection {
        db::open(&tmp.path().join(db::DB_FILE)).unwrap()
    }

    fn project_names(path: &Path) -> Vec<String> {
        let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let mut stmt = conn
            .prepare("SELECT name FROM project ORDER BY name")
            .unwrap();
        stmt.query_map([], |r| r.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect()
    }

    fn sorted_names(backups: &Backups) -> Vec<String> {
        let mut names: Vec<String> = backups
            .list()
            .unwrap()
            .into_iter()
            .map(|b| b.file_name)
            .collect();
        names.sort();
        names
    }

    #[test]
    fn daily_backup_runs_once_per_day_and_keeps_the_last_seven() {
        let tmp = TempDir::new();
        let conn = open(&tmp);
        let backups = Backups::new(tmp.path());
        let first = backups.ensure_daily(&conn, day(1)).unwrap().unwrap();
        assert_eq!(
            (first.file_name.as_str(), first.kind),
            ("qapture-diario-20261001.db", BackupKind::Daily)
        );
        assert!(
            backups.ensure_daily(&conn, day(1)).unwrap().is_none(),
            "una vez por día"
        );
        for d in 2..=9 {
            backups.ensure_daily(&conn, day(d)).unwrap();
        }
        let expected: Vec<String> = (3..=9)
            .map(|d| format!("qapture-diario-202610{d:02}.db"))
            .collect();
        assert_eq!(sorted_names(&backups), expected);
    }

    #[test]
    fn manual_backups_are_readable_copies_and_do_not_rotate() {
        let tmp = TempDir::new();
        let conn = open(&tmp);
        test_support::project(&conn);
        let backups = Backups::new(tmp.path());
        let a = backups.create_manual(&conn, at(9, 30, 0)).unwrap();
        let b = backups.create_manual(&conn, at(9, 30, 0)).unwrap();
        assert_eq!(a.file_name, "qapture-manual-20261016-093000.db");
        assert_eq!(
            b.file_name, "qapture-manual-20261016-093000-2.db",
            "no pisa una copia del mismo segundo"
        );
        assert_eq!(a.kind, BackupKind::Manual);
        assert!(a.size_bytes > 0);
        assert_eq!(a.created_at.len(), 19);
        assert_eq!(
            project_names(&backups.dir().join(&a.file_name)),
            ["Recibos"]
        );
        for d in 1..=9 {
            backups.ensure_daily(&conn, day(d)).unwrap();
        }
        assert_eq!(
            backups
                .list()
                .unwrap()
                .iter()
                .filter(|b| b.kind == BackupKind::Manual)
                .count(),
            2
        );
        let leftovers: Vec<String> = fs::read_dir(backups.dir())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(".tmp") || n.ends_with("-wal") || n.ends_with("-shm"))
            .collect();
        assert!(leftovers.is_empty(), "{leftovers:?}");
        let journal: String = Connection::open_with_flags(
            backups.dir().join(&a.file_name),
            OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap()
        .query_row("PRAGMA journal_mode", [], |r| r.get(0))
        .unwrap();
        assert_eq!(journal, "delete", "un solo archivo, sin WAL");
    }

    #[test]
    fn list_shows_every_kind_and_ignores_other_files() {
        let tmp = TempDir::new();
        let backups = Backups::new(tmp.path());
        fs::create_dir_all(backups.dir()).unwrap();
        for name in [
            "qapture-diario-20261016.db",
            "qapture-manual-20261016-093000.db",
            "qapture-antes-de-restaurar-20261016-100000.db",
            "qapture-v2-20261015-120000.db",
            "notas.txt",
            "qapture-diario-20261016.db.abc.tmp",
            "otra-base.db",
        ] {
            fs::write(backups.dir().join(name), b"x").unwrap();
        }
        let mut kinds: Vec<BackupKind> = backups
            .list()
            .unwrap()
            .into_iter()
            .map(|b| b.kind)
            .collect();
        kinds.sort_by_key(|k| format!("{k:?}"));
        assert_eq!(
            kinds,
            [
                BackupKind::BeforeMigration,
                BackupKind::BeforeRestore,
                BackupKind::Daily,
                BackupKind::Manual
            ]
        );
        assert!(
            Backups::new(&tmp.path().join("nada"))
                .list()
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn restore_returns_to_the_backup_and_keeps_a_safety_copy() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let mut conn = open(&tmp);
        test_support::project(&conn); // «Recibos»
        let backups = Backups::new(tmp.path());
        let saved = backups.create_manual(&conn, at(9, 0, 0)).unwrap();

        let polizas = test_support::other_project(&conn);
        let case = test_support::case(&conn, &polizas);
        let staged = evidence::stage(&files, &sample_png(80, 60)).unwrap();
        let meta = NewEvidenceMeta {
            case_id: case.id.clone(),
            staged_path: staged.path,
            description: "Póliza emitida".into(),
            annotations: EMPTY_ANNOTATIONS.into(),
        };
        evidence::add(&conn, &files, &meta, &sample_png(80, 60)).unwrap();

        let result = backups
            .restore(&mut conn, &files, &saved.file_name, at(10, 0, 0))
            .unwrap();
        assert_eq!(result.restored.file_name, saved.file_name);
        assert_eq!(
            result.safety_copy,
            "qapture-antes-de-restaurar-20261016-100000.db"
        );
        assert_eq!((result.missing_images, result.set_aside_files), (0, 2));

        let names: Vec<String> = projects::list(&conn)
            .unwrap()
            .into_iter()
            .map(|p| p.name)
            .collect();
        assert_eq!(names, ["Recibos"]);
        assert_eq!(
            project_names(&backups.dir().join(&result.safety_copy)),
            ["Pólizas", "Recibos"]
        );
        let aside = tmp
            .path()
            .join(UNUSED_DIR)
            .join("20261016-100000")
            .join(&polizas.id);
        assert_eq!(
            fs::read_dir(aside).unwrap().count(),
            2,
            "orig y render de Pólizas, apartados"
        );
        assert!(!files.project_dir(&polizas.id).exists());
        assert_eq!(db::schema_version(&conn).unwrap(), db::latest_version());
        let fk: i64 = conn
            .query_row("PRAGMA foreign_keys", [], |r| r.get(0))
            .unwrap();
        assert_eq!(fk, 1);
        test_support::case(
            &conn,
            &projects::get(&conn, &projects::list(&conn).unwrap()[0].id).unwrap(),
        );
    }

    #[test]
    fn restore_reports_images_that_no_longer_exist() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let mut conn = open(&tmp);
        let p = test_support::project(&conn);
        let case = test_support::case(&conn, &p);
        let staged = evidence::stage(&files, &sample_png(80, 60)).unwrap();
        let meta = NewEvidenceMeta {
            case_id: case.id.clone(),
            staged_path: staged.path,
            description: "Recibo".into(),
            annotations: EMPTY_ANNOTATIONS.into(),
        };
        let ev = evidence::add(&conn, &files, &meta, &sample_png(80, 60)).unwrap();
        let backups = Backups::new(tmp.path());
        let saved = backups.create_manual(&conn, at(9, 0, 0)).unwrap();
        evidence::delete(&conn, &files, &ev.id).unwrap();
        let result = backups
            .restore(&mut conn, &files, &saved.file_name, at(10, 0, 0))
            .unwrap();
        assert_eq!(result.missing_images, 1);
    }

    #[test]
    fn restore_rejects_unknown_names_and_damaged_files_without_touching_anything() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let mut conn = open(&tmp);
        test_support::project(&conn);
        let backups = Backups::new(tmp.path());
        fs::create_dir_all(backups.dir()).unwrap();
        fs::write(
            backups.dir().join("qapture-manual-20261016-080000.db"),
            b"no soy una base",
        )
        .unwrap();
        for (name, message) in [
            ("..\\qapture.db", "El respaldo no existe."),
            ("../qapture.db", "El respaldo no existe."),
            ("qapture.db", "El respaldo no existe."),
            (
                "qapture-manual-20991231-000000.db",
                "El respaldo no existe.",
            ),
            (
                "qapture-manual-20261016-080000.db",
                "El respaldo está dañado y no se puede restaurar.",
            ),
        ] {
            let err = backups
                .restore(&mut conn, &files, name, at(10, 0, 0))
                .unwrap_err();
            assert_eq!(err.to_string(), message, "{name}");
        }
        assert_eq!(projects::list(&conn).unwrap().len(), 1);
        assert_eq!(backups.list().unwrap().len(), 1, "no se creó copia previa");
    }

    #[test]
    fn a_pre_migration_backup_is_listed_and_restorable() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let path = tmp.path().join(db::DB_FILE);
        db::write_v1_for_tests(&path);
        let mut conn = db::open(&path).unwrap();
        let backups = Backups::new(tmp.path());
        let list = backups.list().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].kind, BackupKind::BeforeMigration);
        test_support::project(&conn); // «Recibos», solo en la base viva
        let result = backups
            .restore(&mut conn, &files, &list[0].file_name, at(10, 0, 0))
            .unwrap();
        assert_eq!(result.restored.kind, BackupKind::BeforeMigration);
        let names: Vec<String> = projects::list(&conn)
            .unwrap()
            .into_iter()
            .map(|p| p.name)
            .collect();
        assert_eq!(names, ["Antiguo"]);
        assert_eq!(db::schema_version(&conn).unwrap(), db::latest_version());
        let leftovers: Vec<String> = fs::read_dir(backups.dir())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(".tmp") || n.ends_with("-wal") || n.ends_with("-shm"))
            .collect();
        assert!(leftovers.is_empty(), "{leftovers:?}");
    }
    #[test]
    fn a_backup_from_a_newer_app_is_refused() {
        let tmp = TempDir::new();
        let conn = open(&tmp);
        let backups = Backups::new(tmp.path());
        let saved = backups.create_manual(&conn, at(9, 0, 0)).unwrap();
        let path = backups.dir().join(&saved.file_name);
        Connection::open(&path)
            .unwrap()
            .pragma_update(None, "user_version", db::latest_version() + 1)
            .unwrap();
        assert_eq!(
            Backups::check(&path).unwrap_err().to_string(),
            "El respaldo es de una versión más nueva de Qapture. Actualiza la app para restaurarlo."
        );
    }
}
