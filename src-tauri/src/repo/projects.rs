use rusqlite::{Connection, OptionalExtension, Row, params};

use super::{fields, templates};
use crate::{
    db::{new_id, now},
    domain::{
        fields::is_iso_date,
        text::{optional_text, required_text},
    },
    dto::{NewProjectInput, ProjectDto, ProjectMetaInput, ProjectSummaryDto},
    error::{AppError, AppResult, validation},
};

fn not_found() -> AppError {
    AppError::NotFound("El proyecto no existe.".into())
}

pub(crate) fn validate_meta(m: &ProjectMetaInput) -> AppResult<ProjectMetaInput> {
    let name = required_text("Nombre del proyecto", &m.name, 150)?;
    let client = optional_text("Cliente", &m.client, 150)?;
    let test_type = optional_text("Tipo de prueba", &m.test_type, 100)?;
    let release_version = optional_text("Versión / release", &m.release_version, 100)?;
    let start_date = optional_text("Fecha de inicio", &m.start_date, 10)?;
    let end_date = optional_text("Fecha de fin", &m.end_date, 10)?;
    for (label, date) in [
        ("Fecha de inicio", &start_date),
        ("Fecha de fin", &end_date),
    ] {
        if !date.is_empty() && !is_iso_date(date) {
            return Err(validation(format!(
                "«{label}» debe ser una fecha válida (AAAA-MM-DD)."
            )));
        }
    }
    if !start_date.is_empty() && !end_date.is_empty() && start_date > end_date {
        return Err(validation(
            "La fecha de fin no puede ser anterior a la de inicio.",
        ));
    }
    Ok(ProjectMetaInput {
        name,
        client,
        test_type,
        release_version,
        start_date,
        end_date,
    })
}

fn project_from_row(r: &Row) -> rusqlite::Result<ProjectDto> {
    Ok(ProjectDto {
        id: r.get("id")?,
        name: r.get("name")?,
        client: r.get("client")?,
        test_type: r.get("test_type")?,
        release_version: r.get("release_version")?,
        start_date: r.get("start_date")?,
        end_date: r.get("end_date")?,
        next_sequence: r.get("next_sequence")?,
        source_template_id: r.get("source_template_id")?,
        created_at: r.get("created_at")?,
        updated_at: r.get("updated_at")?,
        fields: Vec::new(),
    })
}

pub fn ensure_exists(conn: &Connection, id: &str) -> AppResult<()> {
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM project WHERE id = ?1)",
        [id],
        |r| r.get(0),
    )?;
    if exists { Ok(()) } else { Err(not_found()) }
}

pub fn get(conn: &Connection, id: &str) -> AppResult<ProjectDto> {
    let project = conn
        .query_row(
            "SELECT * FROM project WHERE id = ?1",
            [id],
            project_from_row,
        )
        .optional()?
        .ok_or_else(not_found)?;
    Ok(ProjectDto {
        fields: fields::list_for_project(conn, id)?,
        ..project
    })
}

pub fn list(conn: &Connection) -> AppResult<Vec<ProjectSummaryDto>> {
    let mut stmt = conn.prepare(
        "SELECT p.id, p.name, p.client, p.updated_at,
                (SELECT COUNT(*) FROM test_case c WHERE c.project_id = p.id)
         FROM project p ORDER BY p.updated_at DESC, p.name COLLATE NOCASE",
    )?;
    let rows = stmt
        .query_map([], |r| {
            Ok(ProjectSummaryDto {
                id: r.get(0)?,
                name: r.get(1)?,
                client: r.get(2)?,
                updated_at: r.get(3)?,
                case_count: r.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn create(conn: &Connection, input: NewProjectInput) -> AppResult<ProjectDto> {
    let meta = validate_meta(&input.meta)?;
    let template = templates::get(conn, &input.template_id)?;
    let id = new_id();
    let ts = now();
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "INSERT INTO project (id, name, client, test_type, release_version, start_date, end_date,
                              next_sequence, source_template_id, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, ?8, ?9, ?9)",
        params![
            id,
            meta.name,
            meta.client,
            meta.test_type,
            meta.release_version,
            meta.start_date,
            meta.end_date,
            template.id,
            ts
        ],
    )?;
    for (position, f) in template.fields.iter().enumerate() {
        fields::insert_project_field(&tx, &id, f, position as i64)?;
    }
    tx.commit()?;
    get(conn, &id)
}

pub fn update_meta(conn: &Connection, id: &str, input: &ProjectMetaInput) -> AppResult<ProjectDto> {
    ensure_exists(conn, id)?;
    let m = validate_meta(input)?;
    conn.execute(
        "UPDATE project SET name = ?1, client = ?2, test_type = ?3, release_version = ?4,
                            start_date = ?5, end_date = ?6, updated_at = ?7
         WHERE id = ?8",
        params![
            m.name,
            m.client,
            m.test_type,
            m.release_version,
            m.start_date,
            m.end_date,
            now(),
            id
        ],
    )?;
    get(conn, id)
}

pub fn delete(conn: &Connection, id: &str) -> AppResult<()> {
    let removed = conn.execute("DELETE FROM project WHERE id = ?1", [id])?;
    if removed == 0 {
        Err(not_found())
    } else {
        Ok(())
    }
}

pub(crate) fn touch(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE project SET updated_at = ?1 WHERE id = ?2",
        params![now(), id],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        dto::NewProjectInput,
        repo::test_support::{conn, meta, project},
    };

    #[test]
    fn create_copies_template_fields_with_new_ids() {
        let conn = conn();
        let p = project(&conn);
        assert_eq!(p.fields.len(), 11);
        assert_eq!(p.fields[0].key, "entorno");
        assert!(p.fields[0].required);
        assert_ne!(p.fields[0].id, "tpl-funcional-entorno");
        assert_eq!(p.next_sequence, 1);
        assert_eq!(p.source_template_id.as_deref(), Some("tpl-funcional"));
    }

    #[test]
    fn create_validates_meta_and_template() {
        let conn = conn();
        let mut bad = meta("  ");
        let err = create(
            &conn,
            NewProjectInput {
                meta: bad.clone(),
                template_id: "tpl-funcional".into(),
            },
        )
        .unwrap_err();
        assert_eq!(err.to_string(), "«Nombre del proyecto» es obligatorio.");
        bad.name = "Ok".into();
        bad.start_date = "2026-11-01".into();
        let err = create(
            &conn,
            NewProjectInput {
                meta: bad.clone(),
                template_id: "tpl-funcional".into(),
            },
        )
        .unwrap_err();
        assert_eq!(
            err.to_string(),
            "La fecha de fin no puede ser anterior a la de inicio."
        );
        let err = create(
            &conn,
            NewProjectInput {
                meta: meta("Ok"),
                template_id: "nope".into(),
            },
        )
        .unwrap_err();
        assert_eq!(err.code(), "not_found");
    }

    #[test]
    fn update_meta_trims_and_touches() {
        let conn = conn();
        let p = project(&conn);
        std::thread::sleep(std::time::Duration::from_millis(5));
        let mut m = meta("  Recibos v2 ");
        m.client = " GMX ".into();
        let updated = update_meta(&conn, &p.id, &m).unwrap();
        assert_eq!(updated.name, "Recibos v2");
        assert_eq!(updated.client, "GMX");
        assert!(updated.updated_at > p.updated_at);
    }

    #[test]
    fn list_and_delete() {
        let conn = conn();
        let p = project(&conn);
        let summaries = list(&conn).unwrap();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].case_count, 0);
        delete(&conn, &p.id).unwrap();
        assert!(list(&conn).unwrap().is_empty());
        let fields: i64 = conn
            .query_row("SELECT COUNT(*) FROM project_field", [], |r| r.get(0))
            .unwrap();
        assert_eq!(fields, 0);
        assert_eq!(delete(&conn, &p.id).unwrap_err().code(), "not_found");
    }
}
