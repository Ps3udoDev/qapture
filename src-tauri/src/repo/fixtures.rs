// Elementos consumidos en Task 4 (transfer::package) y Task 5 (commands::fixtures).
#![allow(dead_code)]

use std::collections::HashMap;
use std::path::PathBuf;

use rusqlite::{Connection, OptionalExtension, params};

use crate::{
    db,
    dto::FixtureDto,
    error::{AppError, AppResult, validation},
    media::{
        self,
        storage::{self, EvidenceFiles},
    },
    repo,
};

pub fn add(
    conn: &Connection,
    files: &EvidenceFiles,
    case_id: &str,
    file_name: &str,
    note: &str,
    bytes: &[u8],
) -> AppResult<FixtureDto> {
    let file_name = media::fixtures::clean_name(file_name)?;
    media::fixtures::check_size(bytes.len() as u64)?;
    let note = note.trim();
    if note.chars().count() > 500 {
        return Err(validation("La nota supera 500 caracteres."));
    }
    let project_id: String = conn
        .query_row(
            "SELECT project_id FROM test_case WHERE id = ?1",
            [case_id],
            |r| r.get(0),
        )
        .optional()?
        .ok_or_else(|| AppError::NotFound("El caso no existe.".into()))?;
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM case_fixture WHERE case_id = ?1",
        [case_id],
        |r| r.get(0),
    )?;
    if count >= 50 {
        return Err(validation("El caso ya tiene 50 archivos de prueba."));
    }
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM case_fixture WHERE case_id = ?1 AND file_name = ?2 COLLATE NOCASE)",
        params![case_id, file_name],
        |r| r.get(0),
    )?;
    if exists {
        return Err(validation(format!(
            "El caso ya tiene un archivo llamado «{file_name}»."
        )));
    }

    let id = db::new_id();
    let rel = media::fixtures::fixture_rel(&project_id, &id, &file_name);
    media::fixtures::write(files, &rel, bytes)?;

    let abs_path = files.absolute(&rel);
    let insert_result = (|| -> AppResult<FixtureDto> {
        let tx = conn.unchecked_transaction()?;
        let position: i64 = tx.query_row(
            "SELECT COALESCE(MAX(position), -1) + 1 FROM case_fixture WHERE case_id = ?1",
            [case_id],
            |r| r.get(0),
        )?;
        let added_at = db::now();
        let sha256 = media::fixtures::sha256_hex(bytes);
        let size = bytes.len() as i64;
        tx.execute(
            "INSERT INTO case_fixture (id, case_id, file_name, stored_path, size, sha256, note, position, added_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![id, case_id, file_name, rel, size, sha256, note, position, added_at],
        )?;
        repo::projects::touch(&tx, &project_id)?;
        tx.commit()?;
        Ok(FixtureDto {
            id,
            case_id: case_id.into(),
            file_name,
            size,
            sha256,
            note: note.into(),
            position,
            added_at,
        })
    })();

    match insert_result {
        Ok(dto) => Ok(dto),
        Err(err) => {
            let _ = storage::remove_file(&abs_path);
            if let Some(parent) = abs_path.parent() {
                let _ = std::fs::remove_dir(parent);
            }
            Err(err)
        }
    }
}

pub fn list(conn: &Connection, case_id: &str) -> AppResult<Vec<FixtureDto>> {
    let mut stmt = conn.prepare(
        "SELECT id, case_id, file_name, size, sha256, note, position, added_at
         FROM case_fixture
         WHERE case_id = ?1
         ORDER BY position ASC",
    )?;
    let rows = stmt.query_map([case_id], |r| {
        Ok(FixtureDto {
            id: r.get(0)?,
            case_id: r.get(1)?,
            file_name: r.get(2)?,
            size: r.get(3)?,
            sha256: r.get(4)?,
            note: r.get(5)?,
            position: r.get(6)?,
            added_at: r.get(7)?,
        })
    })?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

fn compact(conn: &Connection, case_id: &str) -> AppResult<()> {
    let mut stmt = conn.prepare(
        "SELECT id FROM case_fixture WHERE case_id = ?1 ORDER BY position ASC, added_at ASC",
    )?;
    let ids: Vec<String> = stmt
        .query_map([case_id], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    let mut update = conn.prepare("UPDATE case_fixture SET position = ?1 WHERE id = ?2")?;
    for (pos, id) in ids.iter().enumerate() {
        update.execute(params![pos as i64, id])?;
    }
    Ok(())
}

pub fn remove(conn: &Connection, files: &EvidenceFiles, id: &str) -> AppResult<()> {
    let (case_id, stored_path, project_id): (String, String, String) = conn
        .query_row(
            "SELECT f.case_id, f.stored_path, c.project_id
             FROM case_fixture f
             JOIN test_case c ON f.case_id = c.id
             WHERE f.id = ?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?
        .ok_or_else(|| AppError::NotFound("El archivo de prueba no existe.".into()))?;

    let tx = conn.unchecked_transaction()?;
    tx.execute("DELETE FROM case_fixture WHERE id = ?1", [id])?;
    compact(&tx, &case_id)?;
    repo::projects::touch(&tx, &project_id)?;
    tx.commit()?;

    let abs_path = files.absolute(&stored_path);
    let _ = storage::remove_file(&abs_path);
    if let Some(parent) = abs_path.parent() {
        let _ = std::fs::remove_dir(parent);
    }
    Ok(())
}

pub fn absolute(conn: &Connection, files: &EvidenceFiles, id: &str) -> AppResult<PathBuf> {
    let stored_path: String = conn
        .query_row(
            "SELECT stored_path FROM case_fixture WHERE id = ?1",
            [id],
            |r| r.get(0),
        )
        .optional()?
        .ok_or_else(|| AppError::NotFound("El archivo de prueba no existe.".into()))?;
    let path = files.absolute(&stored_path);
    if !path.is_file() {
        return Err(validation("El archivo de prueba ya no está en disco."));
    }
    Ok(path)
}

pub fn missing_files(conn: &Connection, files: &EvidenceFiles) -> AppResult<i64> {
    let mut missing = 0;
    if repo::evidence::has_table(conn, "case_fixture")? {
        let mut stmt = conn.prepare("SELECT stored_path FROM case_fixture")?;
        for row in stmt.query_map([], |r| r.get::<_, String>(0))? {
            if !files.absolute(&row?).is_file() {
                missing += 1;
            }
        }
    }
    if repo::evidence::has_table(conn, "project_source")? {
        let mut stmt = conn.prepare("SELECT stored_path FROM project_source")?;
        for row in stmt.query_map([], |r| r.get::<_, String>(0))? {
            if !files.absolute(&row?).is_file() {
                missing += 1;
            }
        }
    }
    Ok(missing)
}

pub fn stored_by_case(
    conn: &Connection,
    project_id: &str,
) -> AppResult<HashMap<String, Vec<(FixtureDto, String)>>> {
    let mut stmt = conn.prepare(
        "SELECT f.id, f.case_id, f.file_name, f.size, f.sha256, f.note, f.position, f.added_at, f.stored_path
         FROM case_fixture f
         JOIN test_case c ON f.case_id = c.id
         WHERE c.project_id = ?1
         ORDER BY f.case_id, f.position ASC",
    )?;
    let mut map: HashMap<String, Vec<(FixtureDto, String)>> = HashMap::new();
    let rows = stmt.query_map([project_id], |row| {
        let dto = FixtureDto {
            id: row.get(0)?,
            case_id: row.get(1)?,
            file_name: row.get(2)?,
            size: row.get(3)?,
            sha256: row.get(4)?,
            note: row.get(5)?,
            position: row.get(6)?,
            added_at: row.get(7)?,
        };
        let stored_path: String = row.get(8)?;
        Ok((dto, stored_path))
    })?;
    for item in rows {
        let (dto, path) = item?;
        map.entry(dto.case_id.clone())
            .or_default()
            .push((dto, path));
    }
    Ok(map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        media::{self, storage::TempDir},
        repo::{
            evidence,
            test_support::{case, conn, project},
        },
    };

    #[test]
    fn adds_lists_and_removes_in_order() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let conn = conn();
        let project = project(&conn);
        let case = case(&conn, &project);

        let f1 = add(
            &conn,
            &files,
            &case.id,
            "datos.csv",
            "primera nota",
            b"col1,col2\n1,2",
        )
        .unwrap();
        assert_eq!(f1.file_name, "datos.csv");
        assert_eq!(f1.position, 0);
        assert_eq!(f1.size, 13);
        assert_eq!(f1.note, "primera nota");
        assert_eq!(f1.sha256, media::fixtures::sha256_hex(b"col1,col2\n1,2"));

        let f2 = add(&conn, &files, &case.id, "doc.pdf", "", b"%PDF-1.4\n...").unwrap();
        assert_eq!(f2.file_name, "doc.pdf");
        assert_eq!(f2.position, 1);
        assert_eq!(f2.size, 12);
        assert_eq!(f2.note, "");

        let list1 = list(&conn, &case.id).unwrap();
        assert_eq!(list1.len(), 2);
        assert_eq!(list1[0].id, f1.id);
        assert_eq!(list1[0].position, 0);
        assert_eq!(list1[1].id, f2.id);
        assert_eq!(list1[1].position, 1);

        let path1 = absolute(&conn, &files, &f1.id).unwrap();
        assert!(path1.is_file());
        let folder1 = path1.parent().unwrap();

        remove(&conn, &files, &f1.id).unwrap();

        assert!(!path1.exists());
        assert!(!folder1.exists());

        let list2 = list(&conn, &case.id).unwrap();
        assert_eq!(list2.len(), 1);
        assert_eq!(list2[0].id, f2.id);
        assert_eq!(list2[0].position, 0);

        let map = stored_by_case(&conn, &project.id).unwrap();
        assert_eq!(map.get(&case.id).unwrap().len(), 1);
        assert_eq!(map.get(&case.id).unwrap()[0].0.id, f2.id);
    }

    #[test]
    fn rejects_duplicates_limits_and_unknown_cases() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let conn = conn();
        let project = project(&conn);
        let case = case(&conn, &project);

        // Duplicates case-insensitive: "A.txt" y "a.TXT"
        add(&conn, &files, &case.id, "A.txt", "nota", b"hello").unwrap();
        let dup_err = add(&conn, &files, &case.id, "a.TXT", "otra", b"world").unwrap_err();
        assert_eq!(
            dup_err.to_string(),
            "El caso ya tiene un archivo llamado «a.TXT»."
        );

        // Caso inexistente
        let unk_err = add(&conn, &files, "caso-inexistente", "b.txt", "", b"xyz").unwrap_err();
        assert_eq!(unk_err.to_string(), "El caso no existe.");

        // Nota de 501 caracteres
        let long_note = "a".repeat(501);
        let note_err = add(&conn, &files, &case.id, "c.txt", &long_note, b"xyz").unwrap_err();
        assert_eq!(note_err.to_string(), "La nota supera 500 caracteres.");

        // Límite de 50 archivos por caso (ya tiene 1, agregamos 49 más)
        for i in 1..50 {
            add(&conn, &files, &case.id, &format!("f_{i}.txt"), "", b"x").unwrap();
        }
        assert_eq!(list(&conn, &case.id).unwrap().len(), 50);

        // El 51.º falla
        let limit_err = add(&conn, &files, &case.id, "extra.txt", "", b"x").unwrap_err();
        assert_eq!(
            limit_err.to_string(),
            "El caso ya tiene 50 archivos de prueba."
        );
    }

    #[test]
    fn a_failed_insert_leaves_no_file() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let conn = conn();
        let project = project(&conn);
        let case = case(&conn, &project);

        // Trigger RAISE(FAIL) sobre case_fixture
        conn.execute(
            "CREATE TRIGGER fail_insert BEFORE INSERT ON case_fixture BEGIN SELECT RAISE(FAIL, 'db error simulado'); END;",
            [],
        ).unwrap();

        let res = add(&conn, &files, &case.id, "fail.txt", "nota", b"contenido");
        assert!(res.is_err());

        // fixtures/ debe quedar vacío sin archivos huérfanos
        let fixtures_dir = files.project_dir(&project.id).join("fixtures");
        if fixtures_dir.exists() {
            assert_eq!(
                std::fs::read_dir(&fixtures_dir).unwrap().count(),
                0,
                "fixtures/ debe estar vacío"
            );
        }
    }

    #[test]
    fn deleting_the_case_or_project_removes_its_fixture_files() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let conn = conn();
        let project = project(&conn);
        let case1 = case(&conn, &project);
        let f1 = add(&conn, &files, &case1.id, "c1.txt", "", b"c1").unwrap();
        let path1 = absolute(&conn, &files, &f1.id).unwrap();
        assert!(path1.is_file());

        evidence::delete_case(&conn, &files, &case1.id).unwrap();
        assert!(
            !path1.exists(),
            "delete_case debe borrar el archivo físico del fixture"
        );

        // Para delete_project
        let case2 = case(&conn, &project);
        let f2 = add(&conn, &files, &case2.id, "c2.txt", "", b"c2").unwrap();
        let path2 = absolute(&conn, &files, &f2.id).unwrap();
        assert!(path2.is_file());

        evidence::delete_project(&conn, &files, &project.id).unwrap();
        assert!(
            !files.project_dir(&project.id).exists(),
            "delete_project debe borrar la carpeta del proyecto"
        );
    }

    #[test]
    fn missing_files_counts_fixtures_and_the_guion_copy() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let conn = conn();
        let project = project(&conn);
        let case = case(&conn, &project);

        // Fixture
        let f = add(&conn, &files, &case.id, "f.txt", "", b"fixture").unwrap();
        let f_path = absolute(&conn, &files, &f.id).unwrap();

        // Copia del guión
        let guion_rel = format!("evidence/{}/guion.xlsx", project.id);
        let guion_path = files.absolute(&guion_rel);
        std::fs::create_dir_all(guion_path.parent().unwrap()).unwrap();
        std::fs::write(&guion_path, b"guion").unwrap();
        let def = crate::transfer::guion::profile::tests::reference();
        conn.execute(
            "INSERT INTO project_source (project_id, file_name, stored_path, sha256, sheet, header_row, profile_id, definition, imported_at) VALUES (?1, 'guion.xlsx', ?2, 'hash', 'Guión de Pruebas', 15, NULL, ?3, '')",
            rusqlite::params![project.id, guion_rel, serde_json::to_string(&def).unwrap()],
        ).unwrap();

        assert_eq!(missing_files(&conn, &files).unwrap(), 0);

        // Borrar el fixture en disco
        std::fs::remove_file(&f_path).unwrap();
        assert_eq!(missing_files(&conn, &files).unwrap(), 1);

        // Borrar la copia del guion en disco
        std::fs::remove_file(&guion_path).unwrap();
        assert_eq!(missing_files(&conn, &files).unwrap(), 2);
    }
}
