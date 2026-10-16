use rusqlite::{Connection, OptionalExtension, Row, params};

use super::projects;
use crate::{
    db::{new_id, now},
    domain::{
        fields::is_iso_date,
        text::{optional_text, required_text},
    },
    dto::{DocVersionDto, DocVersionInput},
    error::{AppError, AppResult, validation},
};

fn not_found() -> AppError {
    AppError::NotFound("La versión del documento no existe.".into())
}

fn from_row(r: &Row) -> rusqlite::Result<DocVersionDto> {
    Ok(DocVersionDto {
        id: r.get("id")?,
        project_id: r.get("project_id")?,
        version: r.get("version")?,
        date: r.get("date")?,
        author: r.get("author")?,
        reviewer: r.get("reviewer")?,
        approver: r.get("approver")?,
        changes: r.get("changes")?,
        created_at: r.get("created_at")?,
    })
}

fn validate(input: &DocVersionInput) -> AppResult<DocVersionInput> {
    let version = required_text("Versión", &input.version, 20)?;
    let date = required_text("Fecha", &input.date, 10)?;
    if !is_iso_date(&date) {
        return Err(validation(
            "«Fecha» debe ser una fecha válida (AAAA-MM-DD).",
        ));
    }
    Ok(DocVersionInput {
        version,
        date,
        author: required_text("Elaborado por", &input.author, 100)?,
        reviewer: optional_text("Revisado por", &input.reviewer, 100)?,
        approver: optional_text("Aprobado por", &input.approver, 100)?,
        changes: optional_text("Cambios", &input.changes, 1000)?,
    })
}

fn ensure_unique(
    conn: &Connection,
    project_id: &str,
    version: &str,
    except: Option<&str>,
) -> AppResult<()> {
    let taken: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM doc_version
                       WHERE project_id = ?1 AND version = ?2 COLLATE NOCASE AND id IS NOT ?3)",
        params![project_id, version, except],
        |r| r.get(0),
    )?;
    if taken {
        Err(validation(format!(
            "La versión «{version}» ya existe en este proyecto."
        )))
    } else {
        Ok(())
    }
}

/// Orden del historial: fecha y, a igual fecha, orden de registro.
pub fn list(conn: &Connection, project_id: &str) -> AppResult<Vec<DocVersionDto>> {
    projects::ensure_exists(conn, project_id)?;
    let mut stmt = conn.prepare(
        "SELECT * FROM doc_version WHERE project_id = ?1 ORDER BY date, created_at, rowid",
    )?;
    let rows = stmt
        .query_map([project_id], from_row)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn get(conn: &Connection, id: &str) -> AppResult<DocVersionDto> {
    conn.query_row("SELECT * FROM doc_version WHERE id = ?1", [id], from_row)
        .optional()?
        .ok_or_else(not_found)
}

pub fn create(
    conn: &Connection,
    project_id: &str,
    input: &DocVersionInput,
) -> AppResult<DocVersionDto> {
    projects::ensure_exists(conn, project_id)?;
    let v = validate(input)?;
    ensure_unique(conn, project_id, &v.version, None)?;
    let id = new_id();
    conn.execute(
        "INSERT INTO doc_version (id, project_id, version, date, author, reviewer, approver, changes, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            id,
            project_id,
            v.version,
            v.date,
            v.author,
            v.reviewer,
            v.approver,
            v.changes,
            now()
        ],
    )?;
    projects::touch(conn, project_id)?;
    get(conn, &id)
}

pub fn update(conn: &Connection, id: &str, input: &DocVersionInput) -> AppResult<DocVersionDto> {
    let current = get(conn, id)?;
    let v = validate(input)?;
    ensure_unique(conn, &current.project_id, &v.version, Some(id))?;
    conn.execute(
        "UPDATE doc_version SET version = ?1, date = ?2, author = ?3, reviewer = ?4, approver = ?5,
                                changes = ?6
         WHERE id = ?7",
        params![
            v.version, v.date, v.author, v.reviewer, v.approver, v.changes, id
        ],
    )?;
    projects::touch(conn, &current.project_id)?;
    get(conn, id)
}

pub fn delete(conn: &Connection, id: &str) -> AppResult<()> {
    let current = get(conn, id)?;
    conn.execute("DELETE FROM doc_version WHERE id = ?1", [id])?;
    projects::touch(conn, &current.project_id)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::repo::test_support::{conn, other_project, project};

    pub fn input(version: &str, date: &str) -> DocVersionInput {
        DocVersionInput {
            version: version.into(),
            date: date.into(),
            author: "Ana Pérez".into(),
            reviewer: String::new(),
            approver: String::new(),
            changes: "Versión inicial".into(),
        }
    }

    #[test]
    fn create_trims_and_lists_by_date() {
        let conn = conn();
        let p = project(&conn);
        let mut late = input(" 1.1 ", "2026-10-20");
        late.reviewer = "  Luis Gómez ".into();
        create(&conn, &p.id, &late).unwrap();
        create(&conn, &p.id, &input("1.0", "2026-10-01")).unwrap();
        let versions = list(&conn, &p.id).unwrap();
        let names: Vec<_> = versions.iter().map(|v| v.version.as_str()).collect();
        assert_eq!(names, ["1.0", "1.1"]);
        assert_eq!(versions[1].reviewer, "Luis Gómez");
        assert_eq!(versions[1].project_id, p.id);
    }

    #[test]
    fn validates_required_fields_and_dates() {
        let conn = conn();
        let p = project(&conn);
        let no_author = DocVersionInput {
            author: " ".into(),
            ..input("1.0", "2026-10-01")
        };
        let cases = [
            (input(" ", "2026-10-01"), "«Versión» es obligatorio."),
            (
                input("1.0", "2026-02-30"),
                "«Fecha» debe ser una fecha válida (AAAA-MM-DD).",
            ),
            (input("1.0", ""), "«Fecha» es obligatorio."),
            (no_author, "«Elaborado por» es obligatorio."),
            (
                input(&"9".repeat(21), "2026-10-01"),
                "«Versión» supera 20 caracteres.",
            ),
        ];
        for (bad, message) in cases {
            assert_eq!(create(&conn, &p.id, &bad).unwrap_err().to_string(), message);
        }
    }

    #[test]
    fn versions_are_unique_per_project_ignoring_case() {
        let conn = conn();
        let p = project(&conn);
        let other = other_project(&conn);
        create(&conn, &p.id, &input("v1.0", "2026-10-01")).unwrap();
        assert_eq!(
            create(&conn, &p.id, &input("V1.0", "2026-10-02"))
                .unwrap_err()
                .to_string(),
            "La versión «V1.0» ya existe en este proyecto."
        );
        create(&conn, &other.id, &input("v1.0", "2026-10-01")).unwrap();
    }

    #[test]
    fn update_keeps_its_own_version_and_rejects_another() {
        let conn = conn();
        let p = project(&conn);
        let first = create(&conn, &p.id, &input("1.0", "2026-10-01")).unwrap();
        create(&conn, &p.id, &input("1.1", "2026-10-02")).unwrap();
        let mut edit = input("1.0", "2026-10-03");
        edit.changes = "Se agregan casos de Pólizas".into();
        let updated = update(&conn, &first.id, &edit).unwrap();
        assert_eq!(updated.date, "2026-10-03");
        assert_eq!(updated.changes, "Se agregan casos de Pólizas");
        assert!(update(&conn, &first.id, &input("1.1", "2026-10-03")).is_err());
    }

    #[test]
    fn delete_and_cascade_with_the_project() {
        let conn = conn();
        let p = project(&conn);
        let v = create(&conn, &p.id, &input("1.0", "2026-10-01")).unwrap();
        delete(&conn, &v.id).unwrap();
        assert_eq!(delete(&conn, &v.id).unwrap_err().code(), "not_found");
        create(&conn, &p.id, &input("1.0", "2026-10-01")).unwrap();
        projects::delete(&conn, &p.id).unwrap();
        let left: i64 = conn
            .query_row("SELECT COUNT(*) FROM doc_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(left, 0);
        assert_eq!(list(&conn, &p.id).unwrap_err().code(), "not_found");
    }

    #[test]
    fn changes_touch_the_project() {
        let conn = conn();
        let p = project(&conn);
        std::thread::sleep(std::time::Duration::from_millis(5));
        create(&conn, &p.id, &input("1.0", "2026-10-01")).unwrap();
        assert!(projects::get(&conn, &p.id).unwrap().updated_at > p.updated_at);
    }
}
