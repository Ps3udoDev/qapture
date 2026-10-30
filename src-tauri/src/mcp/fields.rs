use std::collections::BTreeMap;

use serde_json::{Map, Value};

use crate::dto::FieldDto;

pub fn normalize_label(s: &str) -> String {
    s.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'á' | 'à' | 'ä' | 'â' | 'ã' => 'a',
            'é' | 'è' | 'ë' | 'ê' => 'e',
            'í' | 'ì' | 'ï' | 'î' => 'i',
            'ó' | 'ò' | 'ö' | 'ô' | 'õ' => 'o',
            'ú' | 'ù' | 'ü' | 'û' => 'u',
            'ñ' => 'n',
            'ç' => 'c',
            c => c,
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn resolve<'a>(fields: &'a [FieldDto], key: &str) -> Result<&'a FieldDto, String> {
    let active = || fields.iter().filter(|f| !f.archived);
    if let Some(field) = active().find(|f| f.id == key) {
        return Ok(field);
    }
    if let Some(field) = active().find(|f| f.key == key) {
        return Ok(field);
    }
    let label = normalize_label(key);
    let matches: Vec<_> = active()
        .filter(|f| normalize_label(&f.label) == label)
        .collect();
    match matches.as_slice() {
        [field] => Ok(field),
        [] => Err(format!(
            "El campo «{key}» no existe en el proyecto. Campos: {}.",
            active()
                .map(|f| f.label.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )),
        _ => Err(format!(
            "«{key}» coincide con varios campos: {}. Usa el id.",
            matches
                .iter()
                .map(|f| f.label.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

pub fn to_input_values(
    fields: &[FieldDto],
    incoming: &Map<String, Value>,
) -> Result<BTreeMap<String, String>, String> {
    incoming
        .iter()
        .map(|(key, value)| {
            let field = resolve(fields, key)?;
            let text = match value {
                Value::String(s) => s.clone(),
                Value::Number(_) | Value::Bool(_) => value.to_string(),
                _ => {
                    return Err(format!(
                        "El valor de «{key}» debe ser texto, número o booleano."
                    ));
                }
            };
            Ok((field.id.clone(), text))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repo::test_support::{conn, project};

    #[test]
    fn labels_match_ignoring_case_and_accents() {
        let db = conn();
        let p = project(&db);
        let field = resolve(&p.fields, "modulo").unwrap();
        for label in ["MÓDULO", " Módulo "] {
            assert_eq!(resolve(&p.fields, label).unwrap().id, field.id);
        }
        let mut fields = vec![field.clone(), field.clone()];
        fields[0].label = "Paso".into();
        fields[1].label = "paso".into();
        assert_eq!(
            resolve(&fields, "paso").unwrap_err(),
            "«paso» coincide con varios campos: Paso, paso. Usa el id."
        );
    }

    #[test]
    fn unknown_field_lists_the_valid_ones_and_archived_are_ignored() {
        let db = conn();
        let mut p = project(&db);
        p.fields[0].archived = true;
        let message = resolve(&p.fields, "Prioridad").unwrap_err();
        assert!(message.starts_with("El campo «Prioridad» no existe en el proyecto. Campos: "));
        assert_eq!(
            resolve(&p.fields, &p.fields[0].id)
                .unwrap_err()
                .split("Campos: ")
                .nth(1)
                .unwrap(),
            message.split("Campos: ").nth(1).unwrap()
        );
    }

    #[test]
    fn scalar_values_are_converted_and_containers_are_rejected() {
        let db = conn();
        let p = project(&db);
        let input = serde_json::json!({"modulo": 42, "observaciones": true});
        let values = to_input_values(&p.fields, input.as_object().unwrap()).unwrap();
        assert_eq!(values[&resolve(&p.fields, "modulo").unwrap().id], "42");
        assert_eq!(
            values[&resolve(&p.fields, "observaciones").unwrap().id],
            "true"
        );
        assert!(
            to_input_values(
                &p.fields,
                serde_json::json!({"modulo": []}).as_object().unwrap()
            )
            .is_err()
        );
    }
}
