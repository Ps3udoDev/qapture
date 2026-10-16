use rusqlite::{params, types::Type, Connection, Row};

use crate::{db::new_id, domain::fields::FieldType, dto::FieldDto, error::AppResult};

const FIELD_COLUMNS: &str = "id, key, label, type, options, required, clear_on_duplicate, width, position";

pub(crate) fn field_from_row(r: &Row) -> rusqlite::Result<FieldDto> {
    let ty: String = r.get("type")?;
    let options: String = r.get("options")?;
    Ok(FieldDto {
        id: r.get("id")?,
        key: r.get("key")?,
        label: r.get("label")?,
        field_type: FieldType::parse(&ty)
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(3, Type::Text, Box::new(e)))?,
        options: serde_json::from_str(&options)
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(4, Type::Text, Box::new(e)))?,
        required: r.get("required")?,
        clear_on_duplicate: r.get("clear_on_duplicate")?,
        width: r.get("width")?,
        position: r.get("position")?,
        archived: r.get("archived")?,
    })
}

pub fn list_for_template(conn: &Connection, template_id: &str) -> AppResult<Vec<FieldDto>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {FIELD_COLUMNS}, 0 AS archived FROM template_field WHERE template_id = ?1 ORDER BY position"
    ))?;
    let rows = stmt.query_map([template_id], field_from_row)?.collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Columnas del proyecto: primero las activas y luego las archivadas, cada grupo por posición.
pub fn list_for_project(conn: &Connection, project_id: &str) -> AppResult<Vec<FieldDto>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {FIELD_COLUMNS}, archived FROM project_field WHERE project_id = ?1 ORDER BY archived, position"
    ))?;
    let rows = stmt.query_map([project_id], field_from_row)?.collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub(crate) fn insert_template_field(conn: &Connection, template_id: &str, f: &FieldDto, position: i64) -> AppResult<()> {
    conn.execute(
        "INSERT INTO template_field (id, template_id, key, label, type, options, required, clear_on_duplicate, width, position)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            new_id(),
            template_id,
            f.key,
            f.label,
            f.field_type.as_str(),
            serde_json::to_string(&f.options)?,
            f.required,
            f.clear_on_duplicate,
            f.width,
            position
        ],
    )?;
    Ok(())
}

pub(crate) fn insert_project_field(conn: &Connection, project_id: &str, f: &FieldDto, position: i64) -> AppResult<String> {
    let id = new_id();
    conn.execute(
        "INSERT INTO project_field (id, project_id, key, label, type, options, required, clear_on_duplicate, width, position, archived)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            id,
            project_id,
            f.key,
            f.label,
            f.field_type.as_str(),
            serde_json::to_string(&f.options)?,
            f.required,
            f.clear_on_duplicate,
            f.width,
            position,
            f.archived
        ],
    )?;
    Ok(id)
}
