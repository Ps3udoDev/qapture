//! Perfil versionado y validación del mapeo del guion.
use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::read::column_index;
use crate::domain::{fields::FieldType, status::STATUSES, text::fold};

pub const CONDITIONED: &str = "Aprobado+condicionado";
pub const CONDITIONED_KEY: &str = "condicionado";
pub const CONDITIONED_LABEL: &str = "Condicionado";
pub const NO_CODE: &str = "Falta asignar la columna del código del caso.";

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfileDefinition {
    pub version: u32,
    pub sheet: String,
    pub header_row: u32,
    pub columns: Vec<ColumnMap>,
    pub status_map: StatusMap,
    pub cover: CoverCells,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ColumnMap {
    pub column: String,
    pub header: String,
    pub target: Target,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum Target {
    Code,
    Title,
    Status,
    Field {
        key: String,
        label: String,
        #[serde(rename = "type")]
        field_type: FieldType,
        #[serde(default)]
        options: Vec<String>,
        #[serde(default)]
        role: Option<FieldRole>,
    },
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[serde(rename_all = "camelCase")]
pub enum FieldRole {
    ExecutionDate,
    Tester,
    Conditions,
}

impl FieldRole {
    fn label(self) -> &'static str {
        match self {
            Self::ExecutionDate => "Fecha de ejecución",
            Self::Tester => "Tester",
            Self::Conditions => "Condiciones",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StatusMap {
    pub import: BTreeMap<String, String>,
    pub export: BTreeMap<String, String>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CoverCells {
    #[serde(default)]
    pub execution_date: Option<String>,
    #[serde(default)]
    pub responsible: Option<String>,
    #[serde(default)]
    pub observations: Option<String>,
}

pub fn default_status_map() -> StatusMap {
    StatusMap {
        import: [
            ("aceptado", "Aprobado"),
            ("aprobado", "Aprobado"),
            ("rechazado", "Fallido"),
            ("condicionado", CONDITIONED),
            ("", "No ejecutado"),
        ]
        .into_iter()
        .map(|(key, value)| (key.into(), value.into()))
        .collect(),
        export: [
            ("Aprobado", "Aceptado"),
            (CONDITIONED, "Condicionado"),
            ("Fallido", "Rechazado"),
        ]
        .into_iter()
        .map(|(key, value)| (key.into(), value.into()))
        .collect(),
    }
}

pub fn map_status(def: &ProfileDefinition, raw: &str) -> Option<(String, bool)> {
    let wanted = fold(raw);
    let target = def
        .status_map
        .import
        .iter()
        .find(|(key, _)| fold(key) == wanted)?
        .1;
    Some(if target == CONDITIONED {
        ("Aprobado".into(), true)
    } else {
        (target.clone(), false)
    })
}

pub fn uses_conditioned(def: &ProfileDefinition) -> bool {
    def.status_map
        .import
        .values()
        .any(|target| target == CONDITIONED)
}

pub fn match_option(options: &[String], raw: &str) -> Option<String> {
    let wanted = fold(raw);
    options.iter().find(|value| fold(value) == wanted).cloned()
}

pub fn problems(def: &ProfileDefinition) -> Vec<String> {
    let mut errors = Vec::new();
    if def.version != 1 {
        errors.push("Versión de perfil no soportada.".into());
    }
    if def.header_row == 0 {
        errors.push("La fila de encabezados debe ser 1 o mayor.".into());
    }
    let mut columns = BTreeSet::new();
    let mut keys = BTreeSet::new();
    let mut roles = BTreeSet::new();
    let mut codes = 0;
    let mut titles = 0;
    let mut statuses = 0;
    for col in &def.columns {
        if column_index(&col.column).is_none() {
            errors.push(format!("«{}» no es una columna válida.", col.column));
        }
        if !columns.insert(&col.column) {
            errors.push(format!(
                "La columna «{}» está asignada dos veces.",
                col.column
            ));
        }
        match &col.target {
            Target::Code => codes += 1,
            Target::Title => titles += 1,
            Target::Status => statuses += 1,
            Target::Field {
                key,
                label,
                field_type,
                options,
                role,
            } => {
                if !keys.insert(key) {
                    errors.push(format!("La clave «{key}» se repite."));
                }
                if label.trim().is_empty() {
                    errors.push(format!("«{label}» necesita un nombre."));
                }
                if uses_conditioned(def) && key == CONDITIONED_KEY {
                    errors
                        .push("«condicionado» está reservado para la casilla Condicionado.".into());
                }
                if !matches!(
                    field_type,
                    FieldType::Text | FieldType::Longtext | FieldType::List | FieldType::Date
                ) {
                    errors.push(format!(
                        "El tipo «{}» no se puede importar.",
                        field_type.as_str()
                    ));
                }
                if *field_type == FieldType::List && options.is_empty() {
                    errors.push(format!("«{label}» es una lista sin opciones."));
                }
                if let Some(role) = role {
                    if !roles.insert(*role) {
                        errors.push(format!("El rol «{}» está en dos columnas.", role.label()));
                    }
                    if *role == FieldRole::ExecutionDate && *field_type != FieldType::Date {
                        errors.push(
                            "El rol «Fecha de ejecución» solo va en una columna de fecha.".into(),
                        );
                    }
                }
            }
        }
    }
    if codes == 0 {
        errors.push(NO_CODE.into());
    }
    for (count, name) in [(codes, "código"), (titles, "título"), (statuses, "estado")] {
        if count > 1 {
            errors.push(format!("Solo una columna puede ser el {name}."));
        }
    }
    for target in def
        .status_map
        .import
        .values()
        .chain(def.status_map.export.keys())
    {
        if target != CONDITIONED && !STATUSES.contains(&target.as_str()) {
            let error = format!("«{target}» no es un estado de Qapture.");
            if !errors.contains(&error) {
                errors.push(error);
            }
        }
    }
    errors
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::{domain::text::slugify, excel::guion_fixture};

    pub(crate) fn reference() -> ProfileDefinition {
        ProfileDefinition {
            version: 1,
            sheet: guion_fixture::SHEET.into(),
            header_row: 15,
            columns: guion_fixture::HEADERS
                .iter()
                .enumerate()
                .map(|(i, header)| {
                    let target = match i {
                        2 => Target::Code,
                        5 => Target::Title,
                        10 => Target::Status,
                        _ => Target::Field {
                            key: match i {
                                8 => "fecha_prueba".into(),
                                11 => "tester".into(),
                                _ => slugify(header),
                            },
                            label: (*header).into(),
                            field_type: match i {
                                7 => FieldType::List,
                                8 => FieldType::Date,
                                12 => FieldType::Longtext,
                                _ => FieldType::Text,
                            },
                            options: if i == 7 {
                                vec!["Crítico".into(), "Importante".into()]
                            } else {
                                vec![]
                            },
                            role: match i {
                                8 => Some(FieldRole::ExecutionDate),
                                11 => Some(FieldRole::Tester),
                                12 => Some(FieldRole::Conditions),
                                _ => None,
                            },
                        },
                    };
                    ColumnMap {
                        column: crate::transfer::guion::read::column_letter(i as u32),
                        header: (*header).into(),
                        target,
                    }
                })
                .collect(),
            status_map: default_status_map(),
            cover: CoverCells {
                execution_date: Some("D5".into()),
                responsible: Some("D6".into()),
                observations: Some("D8".into()),
            },
        }
    }

    impl ColumnMap {
        fn with_column(mut self, column: &str) -> Self {
            self.column = column.into();
            self
        }
    }

    struct FieldMut<'a> {
        options: &'a mut Vec<String>,
        role: &'a mut Option<FieldRole>,
    }
    fn field_mut<'a>(def: &'a mut ProfileDefinition, column: &str) -> FieldMut<'a> {
        let target = &mut def
            .columns
            .iter_mut()
            .find(|c| c.column == column)
            .unwrap()
            .target;
        match target {
            Target::Field { options, role, .. } => FieldMut { options, role },
            _ => panic!("columna de campo"),
        }
    }

    #[test]
    fn the_spec_json_round_trips() {
        let json = serde_json::to_value(reference()).unwrap();
        assert_eq!(json["headerRow"], 15);
        assert_eq!(json["columns"][0]["target"]["kind"], "field");
        assert_eq!(json["statusMap"]["import"]["condicionado"], CONDITIONED);
        let back: ProfileDefinition = serde_json::from_value(json.clone()).unwrap();
        assert_eq!(back, reference());
        let mut extra = json;
        extra["otro"] = serde_json::json!(1);
        assert!(serde_json::from_value::<ProfileDefinition>(extra).is_err());
    }

    #[test]
    fn status_matching_ignores_case_accents_and_spaces() {
        let def = reference();
        assert_eq!(
            map_status(&def, " ACEPTADO "),
            Some(("Aprobado".into(), false))
        );
        assert_eq!(
            map_status(&def, "Condicionádo"),
            Some(("Aprobado".into(), true))
        );
        assert_eq!(
            map_status(&def, "rechazado"),
            Some(("Fallido".into(), false))
        );
        assert_eq!(
            map_status(&def, "   "),
            Some(("No ejecutado".into(), false))
        );
        assert_eq!(map_status(&def, "Pendiente"), None);
        assert!(uses_conditioned(&def));
    }

    #[test]
    fn list_values_match_without_accents() {
        let options = vec!["Crítico".to_string(), "Importante".to_string()];
        assert_eq!(match_option(&options, "Crìtico"), Some("Crítico".into()));
        assert_eq!(
            match_option(&options, "importante "),
            Some("Importante".into())
        );
        assert_eq!(match_option(&options, "Bajo"), None);
        assert_eq!(fold("  Crìtico   Alto "), "critico alto");
    }

    #[test]
    fn structural_problems_are_listed() {
        assert!(problems(&reference()).is_empty());
        let mut def = reference();
        def.columns.retain(|c| c.target != Target::Code);
        assert_eq!(problems(&def), [NO_CODE]);
        type Change = fn(&mut ProfileDefinition);
        let cases: [(Change, &str); 7] = [
            (|d| d.version = 2, "Versión de perfil no soportada."),
            (
                |d| d.columns[1].column = "a1".into(),
                "«a1» no es una columna válida.",
            ),
            (
                |d| d.columns[1].column = d.columns[0].column.clone(),
                "La columna «A» está asignada dos veces.",
            ),
            (
                |d| {
                    d.columns.push(
                        d.columns
                            .iter()
                            .find(|c| c.target == Target::Status)
                            .unwrap()
                            .clone()
                            .with_column("Z"),
                    )
                },
                "Solo una columna puede ser el estado.",
            ),
            (
                |d| field_mut(d, "H").options.clear(),
                "«Criticidad» es una lista sin opciones.",
            ),
            (
                |d| *field_mut(d, "L").role = Some(FieldRole::ExecutionDate),
                "El rol «Fecha de ejecución» solo va en una columna de fecha.",
            ),
            (
                |d| {
                    d.status_map
                        .import
                        .insert("pendiente".into(), "Cerrado".into());
                },
                "«Cerrado» no es un estado de Qapture.",
            ),
        ];
        for (change, expected) in cases {
            let mut def = reference();
            change(&mut def);
            assert!(
                problems(&def).iter().any(|p| p == expected),
                "falta «{expected}» en {:?}",
                problems(&def)
            );
        }
    }
    #[test]
    fn all_structural_guards_are_checked() {
        let checks: [(&str, serde_json::Value, &str); 8] = [
            (
                "/headerRow",
                serde_json::json!(0),
                "La fila de encabezados debe ser 1 o mayor.",
            ),
            (
                "/columns/1/target/key",
                serde_json::json!("modulo"),
                "La clave «modulo» se repite.",
            ),
            (
                "/columns/0/target/label",
                serde_json::json!(""),
                "«» necesita un nombre.",
            ),
            (
                "/columns/0/target/key",
                serde_json::json!("condicionado"),
                "«condicionado» está reservado para la casilla Condicionado.",
            ),
            (
                "/columns/0/target/type",
                serde_json::json!("boolean"),
                "El tipo «boolean» no se puede importar.",
            ),
            (
                "/columns/0/target/type",
                serde_json::json!("number"),
                "El tipo «number» no se puede importar.",
            ),
            (
                "/columns/11/target/role",
                serde_json::json!("conditions"),
                "El rol «Condiciones» está en dos columnas.",
            ),
            (
                "/columns/1/target",
                serde_json::json!({"kind": "code"}),
                "Solo una columna puede ser el código.",
            ),
        ];
        for (path, value, expected) in checks {
            let mut json = serde_json::to_value(reference()).unwrap();
            *json.pointer_mut(path).unwrap() = value;
            let def = serde_json::from_value(json).unwrap();
            assert!(
                problems(&def).iter().any(|e| e == expected),
                "{path}: {expected}"
            );
        }
        let mut def = reference();
        def.columns.push(ColumnMap {
            column: "Z".into(),
            header: "Título extra".into(),
            target: Target::Title,
        });
        assert!(problems(&def).contains(&"Solo una columna puede ser el título.".into()));
        def.status_map
            .import
            .retain(|_, target| target != CONDITIONED);
        assert!(!uses_conditioned(&def));
    }

    #[test]
    fn nested_objects_reject_unknown_fields_and_optional_field_data_defaults() {
        for path in ["/columns/0", "/columns/0/target", "/statusMap", "/cover"] {
            let mut json = serde_json::to_value(reference()).unwrap();
            json.pointer_mut(path).unwrap()["otro"] = serde_json::json!(1);
            assert!(
                serde_json::from_value::<ProfileDefinition>(json).is_err(),
                "{path}"
            );
        }
        let target: Target = serde_json::from_value(serde_json::json!({
            "kind": "field", "key": "texto", "label": "Texto", "type": "text"
        }))
        .unwrap();
        assert!(matches!(target, Target::Field { options, role: None, .. } if options.is_empty()));
        assert_eq!(
            serde_json::from_str::<CoverCells>("{}").unwrap(),
            CoverCells::default()
        );
    }
}
