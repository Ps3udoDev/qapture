use std::collections::HashSet;

use rusqlite::{params, types::Type, Connection, Row};

use super::projects;
use crate::{
    db::new_id,
    domain::{
        fields::FieldType,
        text::{required_text, slugify},
    },
    dto::{FieldDto, FieldInput},
    error::{validation, AppResult},
};

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

const MAX_FIELDS: usize = 60;

fn has_values(conn: &Connection, field_id: &str) -> AppResult<bool> {
    Ok(conn.query_row("SELECT EXISTS(SELECT 1 FROM field_value WHERE field_id = ?1)", [field_id], |r| r.get(0))?)
}

fn values_in_use(conn: &Connection, field_id: &str) -> AppResult<Vec<String>> {
    let mut stmt = conn.prepare("SELECT DISTINCT value FROM field_value WHERE field_id = ?1 ORDER BY value")?;
    let values = stmt.query_map([field_id], |r| r.get(0))?.collect::<Result<Vec<String>, _>>()?;
    Ok(values)
}

fn normalize_options(label: &str, options: &[String]) -> AppResult<Vec<String>> {
    let mut out: Vec<String> = Vec::new();
    for option in options {
        let o = option.trim();
        if o.is_empty() || out.iter().any(|x| x == o) {
            continue;
        }
        if o.chars().count() > 100 {
            return Err(validation(format!("Una opción de «{label}» supera 100 caracteres.")));
        }
        out.push(o.to_string());
    }
    if out.is_empty() {
        return Err(validation(format!("La lista «{label}» necesita al menos una opción.")));
    }
    if out.len() > 50 {
        return Err(validation(format!("La lista «{label}» admite como máximo 50 opciones.")));
    }
    Ok(out)
}

fn unique_key(base: String, used: &mut HashSet<String>) -> String {
    let mut key = base.clone();
    let mut n = 2;
    while used.contains(&key) {
        key = format!("{base}_{n}");
        n += 1;
    }
    used.insert(key.clone());
    key
}

struct Normalized<'a> {
    input: &'a FieldInput,
    label: String,
    options: Vec<String>,
}

pub fn save(conn: &Connection, project_id: &str, inputs: Vec<FieldInput>) -> AppResult<Vec<FieldDto>> {
    projects::ensure_exists(conn, project_id)?;
    if inputs.len() > MAX_FIELDS {
        return Err(validation(format!("Un proyecto admite como máximo {MAX_FIELDS} columnas.")));
    }
    let existing = list_for_project(conn, project_id)?;

    let mut labels = HashSet::new();
    let mut ids = HashSet::new();
    let mut normalized = Vec::with_capacity(inputs.len());
    for input in &inputs {
        let label = required_text("Nombre de la columna", &input.label, 100)?;
        if !input.archived && !labels.insert(label.to_lowercase()) {
            return Err(validation(format!("La columna «{label}» está repetida.")));
        }
        if !(8..=100).contains(&input.width) {
            return Err(validation(format!("El ancho de «{label}» debe estar entre 8 y 100.")));
        }
        if let Some(id) = &input.id {
            if !ids.insert(id.clone()) {
                return Err(validation(format!("La columna «{label}» está repetida.")));
            }
        }
        let options =
            if input.field_type == FieldType::List { normalize_options(&label, &input.options)? } else { Vec::new() };
        normalized.push(Normalized { input, label, options });
    }

    let tx = conn.unchecked_transaction()?;
    let mut used_keys: HashSet<String> = existing.iter().map(|f| f.key.clone()).collect();
    for (position, n) in normalized.iter().enumerate() {
        let input = n.input;
        let label = &n.label;
        let options_json = serde_json::to_string(&n.options)?;
        match &input.id {
            Some(id) => {
                let current = existing
                    .iter()
                    .find(|f| &f.id == id)
                    .ok_or_else(|| validation("La columna ya no existe. Recarga el proyecto."))?;
                let with_data = has_values(&tx, id)?;
                if with_data && current.field_type != input.field_type && !input.field_type.is_textual() {
                    return Err(validation(format!(
                        "No se puede cambiar el tipo de «{label}» porque ya tiene datos. Solo se permite cambiar a Texto o Texto largo."
                    )));
                }
                if with_data && input.field_type == FieldType::List {
                    if let Some(missing) = values_in_use(&tx, id)?.into_iter().find(|v| !n.options.contains(v)) {
                        return Err(validation(format!(
                            "La opción «{missing}» de «{label}» está en uso; no se puede quitar."
                        )));
                    }
                }
                tx.execute(
                    "UPDATE project_field SET label = ?1, type = ?2, options = ?3, required = ?4,
                            clear_on_duplicate = ?5, width = ?6, position = ?7, archived = ?8
                     WHERE id = ?9",
                    params![
                        label,
                        input.field_type.as_str(),
                        options_json,
                        input.required,
                        input.clear_on_duplicate,
                        input.width,
                        position as i64,
                        input.archived,
                        id
                    ],
                )?;
            }
            None => {
                let key = unique_key(slugify(label), &mut used_keys);
                tx.execute(
                    "INSERT INTO project_field (id, project_id, key, label, type, options, required,
                                                clear_on_duplicate, width, position, archived)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                    params![
                        new_id(),
                        project_id,
                        key,
                        label,
                        input.field_type.as_str(),
                        options_json,
                        input.required,
                        input.clear_on_duplicate,
                        input.width,
                        position as i64,
                        input.archived
                    ],
                )?;
            }
        }
    }

    let kept: HashSet<&str> = inputs.iter().filter_map(|i| i.id.as_deref()).collect();
    for removed in existing.iter().filter(|f| !kept.contains(f.id.as_str())) {
        if has_values(&tx, &removed.id)? {
            tx.execute(
                "UPDATE project_field SET archived = 1, position = ?1 WHERE id = ?2",
                params![(normalized.len() + removed.position as usize) as i64, removed.id],
            )?;
        } else {
            tx.execute("DELETE FROM project_field WHERE id = ?1", [&removed.id])?;
        }
    }
    projects::touch(&tx, project_id)?;
    tx.commit()?;
    list_for_project(conn, project_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        dto::FieldInput,
        repo::test_support::{conn, field_id, insert_value, project},
    };

    fn to_input(f: &FieldDto) -> FieldInput {
        FieldInput {
            id: Some(f.id.clone()),
            label: f.label.clone(),
            field_type: f.field_type,
            options: f.options.clone(),
            required: f.required,
            clear_on_duplicate: f.clear_on_duplicate,
            width: f.width,
            archived: f.archived,
        }
    }

    fn new_input(label: &str, field_type: FieldType, options: &[&str]) -> FieldInput {
        FieldInput {
            id: None,
            label: label.into(),
            field_type,
            options: options.iter().map(|o| o.to_string()).collect(),
            required: false,
            clear_on_duplicate: false,
            width: 14,
            archived: false,
        }
    }

    #[test]
    fn adds_and_reorders() {
        let conn = conn();
        let p = project(&conn);
        let mut inputs: Vec<FieldInput> = p.fields.iter().rev().map(to_input).collect();
        inputs.push(new_input("Severidad", FieldType::List, &[" Alta ", "Media", "Media", ""]));
        let saved = save(&conn, &p.id, inputs).unwrap();
        assert_eq!(saved[0].key, "observaciones");
        let sev = saved.iter().find(|f| f.label == "Severidad").unwrap();
        assert_eq!(sev.key, "severidad");
        assert_eq!(sev.options, vec!["Alta", "Media"]);
        assert_eq!(sev.position, 11);
    }

    #[test]
    fn new_keys_are_unique() {
        let conn = conn();
        let p = project(&conn);
        let mut inputs: Vec<FieldInput> = p.fields.iter().map(to_input).collect();
        inputs.push(new_input("Entorno!", FieldType::Text, &[]));
        let saved = save(&conn, &p.id, inputs).unwrap();
        assert!(saved.iter().any(|f| f.key == "entorno_2"));
    }

    #[test]
    fn rejects_invalid_inputs() {
        let conn = conn();
        let p = project(&conn);
        let base: Vec<FieldInput> = p.fields.iter().map(to_input).collect();

        let mut dup = base.clone();
        dup.push(new_input("ENTORNO", FieldType::Text, &[]));
        assert_eq!(save(&conn, &p.id, dup).unwrap_err().to_string(), "La columna «ENTORNO» está repetida.");

        let mut empty_list = base.clone();
        empty_list.push(new_input("Severidad", FieldType::List, &["  "]));
        assert_eq!(
            save(&conn, &p.id, empty_list).unwrap_err().to_string(),
            "La lista «Severidad» necesita al menos una opción."
        );

        let mut narrow = base.clone();
        narrow[0].width = 4;
        assert_eq!(save(&conn, &p.id, narrow).unwrap_err().to_string(), "El ancho de «Entorno» debe estar entre 8 y 100.");

        let mut unknown = base;
        unknown[0].id = Some("no-existe".into());
        assert_eq!(save(&conn, &p.id, unknown).unwrap_err().to_string(), "La columna ya no existe. Recarga el proyecto.");
    }

    #[test]
    fn removed_fields_are_deleted_or_archived() {
        let conn = conn();
        let p = project(&conn);
        insert_value(&conn, &p.id, &field_id(&p, "modulo"), "Cobranzas");
        let inputs: Vec<FieldInput> =
            p.fields.iter().filter(|f| f.key != "menu" && f.key != "modulo").map(to_input).collect();
        let saved = save(&conn, &p.id, inputs).unwrap();
        assert!(saved.iter().all(|f| f.key != "menu"));
        let modulo = saved.iter().find(|f| f.key == "modulo").unwrap();
        assert!(modulo.archived);
        assert_eq!(saved.last().unwrap().key, "modulo");

        let mut restore: Vec<FieldInput> = saved.iter().map(to_input).collect();
        restore.last_mut().unwrap().archived = false;
        let restored = save(&conn, &p.id, restore).unwrap();
        assert!(!restored.iter().find(|f| f.key == "modulo").unwrap().archived);
    }

    #[test]
    fn type_change_with_data_only_to_text() {
        let conn = conn();
        let p = project(&conn);
        insert_value(&conn, &p.id, &field_id(&p, "fecha_ejecucion"), "2026-10-15");
        let idx = p.fields.iter().position(|f| f.key == "fecha_ejecucion").unwrap();

        let mut to_number: Vec<FieldInput> = p.fields.iter().map(to_input).collect();
        to_number[idx].field_type = FieldType::Number;
        assert_eq!(
            save(&conn, &p.id, to_number).unwrap_err().to_string(),
            "No se puede cambiar el tipo de «Fecha de ejecución» porque ya tiene datos. Solo se permite cambiar a Texto o Texto largo."
        );

        let mut to_text: Vec<FieldInput> = p.fields.iter().map(to_input).collect();
        to_text[idx].field_type = FieldType::Longtext;
        assert!(save(&conn, &p.id, to_text).is_ok());
    }

    #[test]
    fn list_options_in_use_cannot_be_removed() {
        let conn = conn();
        let p = project(&conn);
        let mut inputs: Vec<FieldInput> = p.fields.iter().map(to_input).collect();
        inputs.push(new_input("Severidad", FieldType::List, &["Alta", "Media"]));
        let saved = save(&conn, &p.id, inputs).unwrap();
        let sev = saved.iter().find(|f| f.key == "severidad").unwrap().clone();
        insert_value(&conn, &p.id, &sev.id, "Media");

        let mut shrink: Vec<FieldInput> = saved.iter().map(to_input).collect();
        shrink.iter_mut().find(|f| f.id.as_deref() == Some(sev.id.as_str())).unwrap().options = vec!["Alta".into()];
        assert_eq!(
            save(&conn, &p.id, shrink).unwrap_err().to_string(),
            "La opción «Media» de «Severidad» está en uso; no se puede quitar."
        );
    }
}
