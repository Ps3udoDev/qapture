use rusqlite::{params, Connection, OptionalExtension};

use super::{fields, projects};
use crate::{
    db::{new_id, now},
    domain::text::{optional_text, required_text},
    dto::TemplateDto,
    error::{validation, AppError, AppResult},
};

fn header(r: &rusqlite::Row) -> rusqlite::Result<TemplateDto> {
    Ok(TemplateDto { id: r.get(0)?, name: r.get(1)?, test_type: r.get(2)?, builtin: r.get(3)?, fields: Vec::new() })
}

pub fn list(conn: &Connection) -> AppResult<Vec<TemplateDto>> {
    let mut stmt =
        conn.prepare("SELECT id, name, test_type, builtin FROM template ORDER BY builtin DESC, name COLLATE NOCASE")?;
    let headers = stmt.query_map([], header)?.collect::<Result<Vec<_>, _>>()?;
    headers
        .into_iter()
        .map(|t| Ok(TemplateDto { fields: fields::list_for_template(conn, &t.id)?, ..t }))
        .collect()
}

pub fn get(conn: &Connection, id: &str) -> AppResult<TemplateDto> {
    let t = conn
        .query_row("SELECT id, name, test_type, builtin FROM template WHERE id = ?1", [id], header)
        .optional()?
        .ok_or_else(|| AppError::NotFound("La plantilla no existe.".into()))?;
    Ok(TemplateDto { fields: fields::list_for_template(conn, id)?, ..t })
}

pub fn save_from_project(conn: &Connection, project_id: &str, name: &str, test_type: &str) -> AppResult<TemplateDto> {
    let name = required_text("Nombre de la plantilla", name, 100)?;
    let test_type = optional_text("Tipo de prueba", test_type, 100)?;
    projects::ensure_exists(conn, project_id)?;
    let taken: bool =
        conn.query_row("SELECT EXISTS(SELECT 1 FROM template WHERE name = ?1 COLLATE NOCASE)", [&name], |r| r.get(0))?;
    if taken {
        return Err(validation("Ya existe una plantilla con ese nombre."));
    }
    let source = fields::list_for_project(conn, project_id)?;
    let id = new_id();
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "INSERT INTO template (id, name, test_type, builtin, created_at) VALUES (?1, ?2, ?3, 0, ?4)",
        params![id, name, test_type, now()],
    )?;
    for (position, f) in source.iter().filter(|f| !f.archived).enumerate() {
        fields::insert_template_field(&tx, &id, f, position as i64)?;
    }
    tx.commit()?;
    get(conn, &id)
}

pub fn delete(conn: &Connection, id: &str) -> AppResult<()> {
    if get(conn, id)?.builtin {
        return Err(validation("Las plantillas incluidas no se pueden eliminar."));
    }
    conn.execute("DELETE FROM template WHERE id = ?1", [id])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repo::test_support::{conn, project};

    #[test]
    fn lists_builtins_first() {
        let conn = conn();
        let all = list(&conn).unwrap();
        assert_eq!(all.len(), 4);
        assert!(all.iter().all(|t| t.builtin));
        let api = all.iter().find(|t| t.id == "tpl-api").unwrap();
        let metodo = api.fields.iter().find(|f| f.key == "metodo").unwrap();
        assert_eq!(metodo.options, vec!["GET", "POST", "PUT", "PATCH", "DELETE"]);
    }

    #[test]
    fn save_from_project_copies_active_fields() {
        let conn = conn();
        let p = project(&conn);
        conn.execute("UPDATE project_field SET archived = 1 WHERE project_id = ?1 AND key = 'menu'", [&p.id]).unwrap();
        let t = save_from_project(&conn, &p.id, " GMX Cobranzas ", "Funcional").unwrap();
        assert_eq!(t.name, "GMX Cobranzas");
        assert!(!t.builtin);
        assert_eq!(t.fields.len(), 10);
        assert!(t.fields.iter().all(|f| f.key != "menu"));
        assert_eq!(t.fields.iter().map(|f| f.position).collect::<Vec<_>>(), (0..10).collect::<Vec<_>>());
    }

    #[test]
    fn template_names_are_unique_case_insensitive() {
        let conn = conn();
        let p = project(&conn);
        let err = save_from_project(&conn, &p.id, "funcional", "").unwrap_err();
        assert_eq!(err.to_string(), "Ya existe una plantilla con ese nombre.");
    }

    #[test]
    fn builtin_templates_cannot_be_deleted() {
        let conn = conn();
        let p = project(&conn);
        assert_eq!(delete(&conn, "tpl-funcional").unwrap_err().to_string(), "Las plantillas incluidas no se pueden eliminar.");
        let t = save_from_project(&conn, &p.id, "Propia", "").unwrap();
        delete(&conn, &t.id).unwrap();
        assert_eq!(get(&conn, &t.id).unwrap_err().code(), "not_found");
    }
}
