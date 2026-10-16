use std::collections::{BTreeMap, HashMap};

use rusqlite::{Connection, OptionalExtension, Row, params};

use super::{fields, projects};
use crate::{
    db::{new_id, now},
    domain::{
        fields::{FieldRule, normalize_value, validate_value},
        status::{DEFAULT_STATUS, is_status},
        text::required_text,
    },
    dto::{CaseDto, CaseInput, FieldDto},
    error::{AppError, AppResult, validation},
};

const CASE_COLUMNS: &str = "id, project_id, code, title, status, created_at, updated_at";

fn case_from_row(r: &Row) -> rusqlite::Result<CaseDto> {
    Ok(CaseDto {
        id: r.get("id")?,
        project_id: r.get("project_id")?,
        code: r.get("code")?,
        title: r.get("title")?,
        status: r.get("status")?,
        values: BTreeMap::new(),
        created_at: r.get("created_at")?,
        updated_at: r.get("updated_at")?,
    })
}

pub fn list(conn: &Connection, project_id: &str) -> AppResult<Vec<CaseDto>> {
    projects::ensure_exists(conn, project_id)?;
    let mut stmt = conn.prepare(&format!(
        "SELECT {CASE_COLUMNS} FROM test_case WHERE project_id = ?1 ORDER BY rowid"
    ))?;
    let mut cases = stmt
        .query_map([project_id], case_from_row)?
        .collect::<Result<Vec<_>, _>>()?;
    let mut stmt = conn.prepare(
        "SELECT v.case_id, v.field_id, v.value FROM field_value v
         JOIN test_case c ON c.id = v.case_id WHERE c.project_id = ?1",
    )?;
    let mut by_case: HashMap<String, BTreeMap<String, String>> = HashMap::new();
    let rows = stmt.query_map([project_id], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
        ))
    })?;
    for row in rows {
        let (case_id, field_id, value) = row?;
        by_case.entry(case_id).or_default().insert(field_id, value);
    }
    for case in &mut cases {
        if let Some(values) = by_case.remove(&case.id) {
            case.values = values;
        }
    }
    Ok(cases)
}

pub fn get(conn: &Connection, id: &str) -> AppResult<CaseDto> {
    let mut case = conn
        .query_row(
            &format!("SELECT {CASE_COLUMNS} FROM test_case WHERE id = ?1"),
            [id],
            case_from_row,
        )
        .optional()?
        .ok_or_else(|| AppError::NotFound("El caso no existe.".into()))?;
    let mut stmt = conn.prepare("SELECT field_id, value FROM field_value WHERE case_id = ?1")?;
    case.values = stmt
        .query_map([id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })?
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    Ok(case)
}

struct Validated {
    title: String,
    status: String,
    values: Vec<(String, String)>,
}

fn validate_input(project_fields: &[FieldDto], input: &CaseInput) -> AppResult<Validated> {
    let title = required_text("Caso de prueba", &input.title, 300)?;
    if !is_status(&input.status) {
        return Err(validation(format!("Estado inválido: «{}».", input.status)));
    }
    for key in input.values.keys() {
        if !project_fields.iter().any(|f| &f.id == key && !f.archived) {
            return Err(validation(
                "La columna enviada no pertenece al proyecto. Recarga el proyecto.",
            ));
        }
    }
    let mut values = Vec::new();
    for f in project_fields.iter().filter(|f| !f.archived) {
        let raw = input.values.get(&f.id).map(String::as_str).unwrap_or("");
        let rule = FieldRule {
            label: &f.label,
            field_type: f.field_type,
            options: &f.options,
            required: f.required,
        };
        validate_value(&rule, raw).map_err(AppError::Validation)?;
        let value = normalize_value(f.field_type, raw);
        if !value.is_empty() {
            values.push((f.id.clone(), value));
        }
    }
    Ok(Validated {
        title,
        status: input.status.clone(),
        values,
    })
}

fn insert_case(
    conn: &Connection,
    project_id: &str,
    title: &str,
    status: &str,
) -> AppResult<String> {
    // Una sola sentencia: la secuencia se asigna de forma atómica entre instancias.
    let seq: i64 = conn.query_row(
        "UPDATE project SET next_sequence = next_sequence + 1 WHERE id = ?1
         RETURNING next_sequence - 1",
        [project_id],
        |r| r.get(0),
    )?;
    let id = new_id();
    conn.execute(
        "INSERT INTO test_case (id, project_id, code, title, status, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
        params![id, project_id, format!("EJ-{seq:03}"), title, status, now()],
    )?;
    Ok(id)
}

fn insert_values(conn: &Connection, case_id: &str, values: &[(String, String)]) -> AppResult<()> {
    let mut stmt =
        conn.prepare("INSERT INTO field_value (case_id, field_id, value) VALUES (?1, ?2, ?3)")?;
    for (field_id, value) in values {
        stmt.execute(params![case_id, field_id, value])?;
    }
    Ok(())
}

pub fn create(conn: &Connection, project_id: &str, input: &CaseInput) -> AppResult<CaseDto> {
    projects::ensure_exists(conn, project_id)?;
    let v = validate_input(&fields::list_for_project(conn, project_id)?, input)?;
    let tx = conn.unchecked_transaction()?;
    let id = insert_case(&tx, project_id, &v.title, &v.status)?;
    insert_values(&tx, &id, &v.values)?;
    projects::touch(&tx, project_id)?;
    tx.commit()?;
    get(conn, &id)
}

pub fn update(conn: &Connection, case_id: &str, input: &CaseInput) -> AppResult<CaseDto> {
    let current = get(conn, case_id)?;
    let v = validate_input(&fields::list_for_project(conn, &current.project_id)?, input)?;
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "UPDATE test_case SET title = ?1, status = ?2, updated_at = ?3 WHERE id = ?4",
        params![v.title, v.status, now(), case_id],
    )?;
    tx.execute(
        "DELETE FROM field_value WHERE case_id = ?1
           AND field_id IN (SELECT id FROM project_field WHERE project_id = ?2 AND archived = 0)",
        params![case_id, current.project_id],
    )?;
    insert_values(&tx, case_id, &v.values)?;
    projects::touch(&tx, &current.project_id)?;
    tx.commit()?;
    get(conn, case_id)
}

pub fn duplicate(conn: &Connection, case_id: &str) -> AppResult<CaseDto> {
    let source = get(conn, case_id)?;
    let project_fields = fields::list_for_project(conn, &source.project_id)?;
    let kept: Vec<(String, String)> = source
        .values
        .iter()
        .filter(|(field_id, _)| {
            project_fields
                .iter()
                .any(|f| &f.id == *field_id && !f.clear_on_duplicate)
        })
        .map(|(f, v)| (f.clone(), v.clone()))
        .collect();
    let tx = conn.unchecked_transaction()?;
    let id = insert_case(&tx, &source.project_id, &source.title, DEFAULT_STATUS)?;
    insert_values(&tx, &id, &kept)?;
    projects::touch(&tx, &source.project_id)?;
    tx.commit()?;
    get(conn, &id)
}

pub fn delete(conn: &Connection, case_id: &str) -> AppResult<()> {
    let case = get(conn, case_id)?;
    let tx = conn.unchecked_transaction()?;
    tx.execute("DELETE FROM test_case WHERE id = ?1", [case_id])?;
    projects::touch(&tx, &case.project_id)?;
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::{
        dto::{CaseInput, FieldInput},
        repo::{
            fields as field_repo, projects as project_repo,
            test_support::{conn, field_id, project},
        },
    };

    fn input(p: &crate::dto::ProjectDto, pairs: &[(&str, &str)]) -> CaseInput {
        let values: BTreeMap<String, String> = pairs
            .iter()
            .map(|(k, v)| (field_id(p, k), v.to_string()))
            .collect();
        CaseInput {
            title: "Aplicación de recibos".into(),
            status: "Aprobado".into(),
            values,
        }
    }

    #[test]
    fn create_assigns_sequential_codes() {
        let conn = conn();
        let p = project(&conn);
        let a = create(&conn, &p.id, &input(&p, &[("entorno", "QA")])).unwrap();
        let b = create(&conn, &p.id, &input(&p, &[("entorno", "QA")])).unwrap();
        assert_eq!((a.code.as_str(), b.code.as_str()), ("EJ-001", "EJ-002"));
        assert_eq!(project_repo::get(&conn, &p.id).unwrap().next_sequence, 3);
        assert_eq!(project_repo::list(&conn).unwrap()[0].case_count, 2);
    }

    #[test]
    fn create_validates() {
        let conn = conn();
        let p = project(&conn);
        assert_eq!(
            create(&conn, &p.id, &input(&p, &[]))
                .unwrap_err()
                .to_string(),
            "«Entorno» es obligatorio."
        );

        let mut bad_status = input(&p, &[("entorno", "QA")]);
        bad_status.status = "Listo".into();
        assert!(create(&conn, &p.id, &bad_status).is_err());

        let mut unknown = input(&p, &[("entorno", "QA")]);
        unknown.values.insert("otro".into(), "x".into());
        assert_eq!(
            create(&conn, &p.id, &unknown).unwrap_err().to_string(),
            "La columna enviada no pertenece al proyecto. Recarga el proyecto."
        );

        let bad_date = input(&p, &[("entorno", "QA"), ("fecha_ejecucion", "2026-13-01")]);
        assert!(create(&conn, &p.id, &bad_date).is_err());

        let mut blank_title = input(&p, &[("entorno", "QA")]);
        blank_title.title = "   ".into();
        assert_eq!(
            create(&conn, &p.id, &blank_title).unwrap_err().to_string(),
            "«Caso de prueba» es obligatorio."
        );
    }

    #[test]
    fn whitespace_values_are_not_stored_and_multiline_is_kept() {
        let conn = conn();
        let p = project(&conn);
        let c = create(
            &conn,
            &p.id,
            &input(
                &p,
                &[
                    ("entorno", " QA "),
                    ("modulo", "   "),
                    ("datos_entrada", "Recibo: 1\nSucursal: 2 ✅"),
                ],
            ),
        )
        .unwrap();
        assert_eq!(c.values.get(&field_id(&p, "entorno")).unwrap(), " QA ");
        assert!(!c.values.contains_key(&field_id(&p, "modulo")));
        assert_eq!(
            c.values.get(&field_id(&p, "datos_entrada")).unwrap(),
            "Recibo: 1\nSucursal: 2 ✅"
        );
    }

    #[test]
    fn update_keeps_archived_values() {
        let conn = conn();
        let p = project(&conn);
        let c = create(
            &conn,
            &p.id,
            &input(&p, &[("entorno", "QA"), ("modulo", "Cobranzas")]),
        )
        .unwrap();
        let without_modulo: Vec<FieldInput> = p
            .fields
            .iter()
            .filter(|f| f.key != "modulo")
            .map(|f| FieldInput {
                id: Some(f.id.clone()),
                label: f.label.clone(),
                field_type: f.field_type,
                options: f.options.clone(),
                required: f.required,
                clear_on_duplicate: f.clear_on_duplicate,
                width: f.width,
                archived: false,
            })
            .collect();
        field_repo::save(&conn, &p.id, without_modulo).unwrap();

        let updated = update(&conn, &c.id, &input(&p, &[("entorno", "Producción")])).unwrap();
        assert_eq!(
            updated.values.get(&field_id(&p, "entorno")).unwrap(),
            "Producción"
        );
        assert_eq!(
            updated.values.get(&field_id(&p, "modulo")).unwrap(),
            "Cobranzas"
        );

        let stale = input(&p, &[("entorno", "QA"), ("modulo", "Otro")]);
        assert!(update(&conn, &c.id, &stale).is_err());
    }

    #[test]
    fn update_requires_new_required_fields() {
        let conn = conn();
        let p = project(&conn);
        let c = create(&conn, &p.id, &input(&p, &[("entorno", "QA")])).unwrap();
        conn.execute(
            "UPDATE project_field SET required = 1 WHERE id = ?1",
            [field_id(&p, "responsable")],
        )
        .unwrap();
        assert_eq!(
            update(&conn, &c.id, &input(&p, &[("entorno", "QA")]))
                .unwrap_err()
                .to_string(),
            "«Responsable» es obligatorio."
        );
    }

    #[test]
    fn duplicate_clears_marked_fields_and_resets_status() {
        let conn = conn();
        let p = project(&conn);
        let src = create(
            &conn,
            &p.id,
            &input(
                &p,
                &[
                    ("entorno", "QA"),
                    ("datos_entrada", "Póliza 1"),
                    ("resultado_obtenido", "OK"),
                    ("fecha_ejecucion", "2026-10-15"),
                    ("observaciones", "nada"),
                ],
            ),
        )
        .unwrap();
        let copy = duplicate(&conn, &src.id).unwrap();
        assert_eq!(copy.code, "EJ-002");
        assert_eq!(copy.status, "No ejecutado");
        assert_eq!(copy.title, src.title);
        assert_eq!(
            copy.values.get(&field_id(&p, "datos_entrada")).unwrap(),
            "Póliza 1"
        );
        for key in ["resultado_obtenido", "fecha_ejecucion", "observaciones"] {
            assert!(
                !copy.values.contains_key(&field_id(&p, key)),
                "{key} debería limpiarse"
            );
        }
    }

    #[test]
    fn deleted_codes_are_not_reused_and_project_delete_cascades() {
        let conn = conn();
        let p = project(&conn);
        create(&conn, &p.id, &input(&p, &[("entorno", "QA")])).unwrap();
        let b = create(&conn, &p.id, &input(&p, &[("entorno", "QA")])).unwrap();
        delete(&conn, &b.id).unwrap();
        let c = create(&conn, &p.id, &input(&p, &[("entorno", "QA")])).unwrap();
        assert_eq!(c.code, "EJ-003");
        assert_eq!(list(&conn, &p.id).unwrap().len(), 2);
        assert_eq!(delete(&conn, &b.id).unwrap_err().code(), "not_found");

        project_repo::delete(&conn, &p.id).unwrap();
        let values: i64 = conn
            .query_row("SELECT COUNT(*) FROM field_value", [], |r| r.get(0))
            .unwrap();
        assert_eq!(values, 0);
    }
}
