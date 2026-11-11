//! Vista previa y alta transaccional de un guion con su copia de origen.
use std::collections::{BTreeMap, HashMap};

use rusqlite::Connection;
use sha2::{Digest, Sha256};

use super::{
    profile::{self, FieldRole, ProfileDefinition, Target},
    read::{self, GuionBook},
};
use crate::{
    db::{new_id, now},
    domain::{fields::FieldType, text::required_text},
    dto::{
        GuionPreviewCaseDto, GuionPreviewDto, GuionPreviewFieldDto, ImportSummaryDto,
        ProjectMetaInput,
    },
    error::AppResult,
    media::storage::EvidenceFiles,
    repo::guion,
    transfer::{
        bundle::{self, BundleCase, BundleField, BundleSource, ImportFiles, ProjectBundle},
        github_json::GithubPayload,
    },
};

pub const GUION_FILE: &str = "guion.xlsx";

pub struct ImportRequest<'a> {
    pub book: &'a GuionBook,
    pub bytes: &'a [u8],
    pub file_name: &'a str,
    pub def: &'a ProfileDefinition,
    pub project_name: &'a str,
    pub save_profile_as: Option<&'a str>,
}

pub fn import(
    conn: &Connection,
    files: &EvidenceFiles,
    request: ImportRequest<'_>,
) -> AppResult<ImportSummaryDto> {
    let planned = plan(request.book, request.def);
    let mut problems = bundle::Problems::default();
    for error in &planned.preview.errors {
        problems.add("", error);
    }
    problems.into_result()?;
    let name = required_text("Nombre del proyecto", request.project_name, 150)?;
    let id = new_id();
    let stamp = now();
    let mut out = ImportFiles::new(files);
    let stored_path = out.write_file(&id, GUION_FILE, request.bytes)?;
    let tx = conn.unchecked_transaction()?;
    let profile_id = request
        .save_profile_as
        .map(|name| guion::create_profile(&tx, name, request.def).map(|profile| profile.id))
        .transpose()?;
    let bundle = ProjectBundle {
        source: Some(BundleSource {
            file_name: request.file_name.into(),
            stored_path,
            sha256: format!("{:x}", Sha256::digest(request.bytes)),
            sheet: request.def.sheet.clone(),
            header_row: request.def.header_row,
            profile_id,
            definition: serde_json::to_string(request.def)?,
        }),
        id,
        meta: ProjectMetaInput {
            name,
            client: String::new(),
            test_type: String::new(),
            release_version: String::new(),
            start_date: String::new(),
            end_date: String::new(),
        },
        next_sequence: 1,
        source_template_id: None,
        created_at: stamp.clone(),
        updated_at: stamp.clone(),
        fields: planned
            .preview
            .fields
            .into_iter()
            .map(|f| BundleField {
                key: f.key,
                label: f.label,
                field_type: f.field_type,
                options: f.options,
                required: false,
                clear_on_duplicate: false,
                width: match f.field_type {
                    FieldType::Text => 24,
                    FieldType::Longtext => 44,
                    FieldType::List => 14,
                    FieldType::Date => 15,
                    FieldType::Boolean => 12,
                    FieldType::Number => 24,
                },
                archived: false,
            })
            .collect(),
        cases: planned
            .cases
            .into_iter()
            .map(|c| BundleCase {
                source_row: Some(c.row),
                code: c.code,
                title: c.title,
                status: c.status,
                created_at: stamp.clone(),
                updated_at: stamp.clone(),
                values: c.values,
                evidence: Vec::new(),
            })
            .collect(),
        versions: Vec::new(),
        github: GithubPayload::Absent,
    };
    let mut summary = bundle::insert_in(&tx, &[bundle])?;
    summary.warnings = planned.preview.warnings;
    tx.commit()?;
    out.commit();
    Ok(summary)
}

pub struct Plan {
    pub preview: GuionPreviewDto,
    pub cases: Vec<PlannedCase>,
}

pub struct PlannedCase {
    pub row: u32,
    pub code: String,
    pub title: String,
    pub status: String,
    pub values: BTreeMap<String, String>,
}

pub fn plan(book: &GuionBook, def: &ProfileDefinition) -> Plan {
    let mut result = Plan {
        preview: GuionPreviewDto {
            cases: Vec::new(),
            fields: Vec::new(),
            warnings: Vec::new(),
            errors: profile::problems(def),
        },
        cases: Vec::new(),
    };
    let sheet = book.sheet(&def.sheet);
    if sheet.is_none() {
        result
            .preview
            .errors
            .push(format!("El guion no tiene la hoja «{}».", def.sheet));
    }
    if !result.preview.errors.is_empty() {
        return result;
    }
    let sheet = sheet.expect("hoja comprobada");
    let columns: Vec<_> = def
        .columns
        .iter()
        .map(|column| {
            (
                read::column_index(&column.column).expect("columna validada"),
                &column.target,
            )
        })
        .collect();
    let code_col = columns
        .iter()
        .find(|(_, target)| **target == Target::Code)
        .expect("código validado")
        .0;
    let title_col = columns
        .iter()
        .find(|(_, target)| **target == Target::Title)
        .map(|(col, _)| *col);
    let status_col = columns
        .iter()
        .find(|(_, target)| **target == Target::Status)
        .map(|(col, _)| *col);
    let conditions_key = columns.iter().find_map(|(_, target)| match target {
        Target::Field {
            key,
            role: Some(FieldRole::Conditions),
            ..
        } => Some(key.as_str()),
        _ => None,
    });
    for (_, target) in &columns {
        if let Target::Field {
            key,
            label,
            field_type,
            options,
            ..
        } = target
        {
            result.preview.fields.push(GuionPreviewFieldDto {
                key: key.clone(),
                label: label.clone(),
                field_type: *field_type,
                options: options.clone(),
            });
        }
    }
    if profile::uses_conditioned(def) {
        result.preview.fields.push(GuionPreviewFieldDto {
            key: profile::CONDITIONED_KEY.into(),
            label: profile::CONDITIONED_LABEL.into(),
            field_type: FieldType::Boolean,
            options: Vec::new(),
        });
    }
    let rows: Vec<usize> = sheet
        .rows
        .iter()
        .enumerate()
        .skip(def.header_row as usize)
        .filter(|(row, _)| {
            columns
                .iter()
                .any(|(col, _)| !sheet.cell(*row as u32, *col).text().trim().is_empty())
        })
        .map(|(row, _)| row)
        .collect();
    if rows.len() > 10_000 {
        result
            .preview
            .errors
            .push("El guion supera 10000 casos.".into());
        return result;
    }
    if rows.is_empty() {
        result
            .preview
            .errors
            .push("El guion no tiene casos debajo de la fila de encabezados.".into());
        return result;
    }
    let omitted = rows.last().unwrap() - rows[0] + 1 - rows.len();
    if omitted > 0 {
        result.preview.warnings.push(if omitted == 1 {
            "Se omitió 1 fila vacía.".into()
        } else {
            format!("Se omitieron {omitted} filas vacías.")
        });
    }
    let mut missing = Vec::new();
    let mut seen = HashMap::new();
    let mut order = Vec::new();
    // Primero los errores de código, en el orden que define el contrato.
    for &row in &rows {
        let code = sheet.cell(row as u32, code_col).text().trim().to_string();
        if code.is_empty() {
            missing.push((row + 1).to_string());
        } else {
            let count = seen.entry(code.clone()).or_insert(0_usize);
            if *count == 0 {
                order.push(code);
            }
            *count += 1;
        }
    }
    if !missing.is_empty() {
        result
            .preview
            .errors
            .push(format!("Hay filas sin código: {}.", missing.join(", ")));
    }
    let repeated: Vec<_> = order.into_iter().filter(|code| seen[code] > 1).collect();
    if !repeated.is_empty() {
        result
            .preview
            .errors
            .push(format!("Hay códigos repetidos: {}.", repeated.join(", ")));
    }
    let mut appended_conditions = false;
    for row in rows {
        let code = sheet.cell(row as u32, code_col).text().trim().to_string();
        if code.chars().count() > 40 {
            result
                .preview
                .errors
                .push(format!("{}: el código supera 40 caracteres.", row + 1));
        }
        let mut title = title_col
            .map(|col| sheet.cell(row as u32, col).text())
            .unwrap_or_default();
        if title.trim().is_empty() {
            title.clone_from(&code);
            result
                .preview
                .warnings
                .push(format!("{code}: sin título; se usa el código."));
        } else if title.chars().count() > 300 {
            title = title.chars().take(300).collect();
            result
                .preview
                .warnings
                .push(format!("{code}: el título se recortó a 300 caracteres."));
        }
        let raw_status = status_col
            .map(|col| sheet.cell(row as u32, col).text())
            .unwrap_or_default();
        let (status, conditioned, original) = if status_col.is_none() {
            ("No ejecutado".into(), false, None)
        } else if let Some((status, conditioned)) = profile::map_status(def, &raw_status) {
            (status, conditioned, None)
        } else {
            result.preview.warnings.push(format!("{code}: «{raw_status}» no es un estado del perfil; el caso entra como «No ejecutado»."));
            ("No ejecutado".into(), false, Some(raw_status))
        };
        let mut values = BTreeMap::new();
        let mut field_index = 0;
        for (col, target) in &columns {
            let Target::Field { key, .. } = target else {
                continue;
            };
            let field = &mut result.preview.fields[field_index];
            field_index += 1;
            let cell = sheet.cell(row as u32, *col);
            let raw = cell.text();
            if raw.trim().is_empty() {
                continue;
            }
            let value = match field.field_type {
                FieldType::List => match profile::match_option(&field.options, &raw) {
                    Some(value) => value,
                    None => {
                        let value = raw.trim().to_string();
                        field.options.push(value.clone());
                        result.preview.warnings.push(format!(
                            "«{value}» no estaba en la lista de «{}»; se agregó como opción.",
                            field.label
                        ));
                        value
                    }
                },
                FieldType::Date => match read::parse_date(cell) {
                    Some(value) => value,
                    None => {
                        result.preview.warnings.push(format!(
                            "{code}: «{raw}» no es una fecha en «{}»; se deja vacía.",
                            field.label
                        ));
                        String::new()
                    }
                },
                _ => raw.clone(),
            };
            if raw.chars().count() > 16000 {
                result.preview.errors.push(format!(
                    "{code}: «{}» supera 16000 caracteres.",
                    field.label
                ));
            }
            if !value.is_empty() {
                values.insert(key.clone(), value);
            }
        }
        if let (Some(original), Some(key)) = (original, conditions_key) {
            appended_conditions = true;
            let value = values.entry(key.into()).or_insert_with(String::new);
            if !value.is_empty() {
                value.push('\n');
            }
            value.push_str(&format!("Estado original: {original}"));
        }
        for field in &mut result.preview.fields {
            if let Some(value) = values.get(&field.key) {
                let len = value.chars().count();
                if field.field_type == FieldType::Text && len > 500 {
                    field.field_type = FieldType::Longtext;
                }
                if len > 16000 {
                    let error = format!("{code}: «{}» supera 16000 caracteres.", field.label);
                    if !result.preview.errors.contains(&error) {
                        result.preview.errors.push(error);
                    }
                }
            }
        }
        if conditioned {
            values.insert(profile::CONDITIONED_KEY.into(), "true".into());
        }
        result.preview.cases.push(GuionPreviewCaseDto {
            row: row as u32 + 1,
            code: code.clone(),
            title: title.clone(),
            status: status.clone(),
            conditioned,
        });
        result.cases.push(PlannedCase {
            row: row as u32 + 1,
            code,
            title,
            status,
            values,
        });
    }
    // El perfil admite Conditions en listas y fechas. El texto de auditoría requiere
    // un campo textual; se promueve después de normalizar todas las filas para que
    // las fechas y opciones posteriores sigan interpretándose con el tipo original.
    if appended_conditions {
        for field in &mut result.preview.fields {
            if Some(field.key.as_str()) == conditions_key
                && matches!(field.field_type, FieldType::List | FieldType::Date)
            {
                field.field_type = FieldType::Longtext;
                field.options.clear();
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::super::{profile, read};
    use super::*;
    use crate::{domain::fields::FieldType, excel::guion_fixture};
    use guion_fixture::Put;

    #[test]
    fn the_reference_guion_gives_36_cases_and_the_13_columns() {
        let book = read::read(&guion_fixture::build()).unwrap();
        let plan = plan(&book, &profile::tests::reference());
        assert!(plan.preview.errors.is_empty(), "{:?}", plan.preview.errors);
        assert_eq!(plan.cases.len(), 36);
        assert_eq!(
            (plan.cases[0].row, plan.cases[0].code.as_str()),
            (16, "CP01")
        );
        assert_eq!(plan.cases[35].code, "CP36");
        assert_eq!(plan.cases[0].title, "Escenario 1");
        assert_eq!(plan.cases[0].status, "No ejecutado");
        let keys: Vec<&str> = plan.preview.fields.iter().map(|f| f.key.as_str()).collect();
        assert_eq!(
            keys.len(),
            11,
            "10 columnas como campos + Condicionado: {keys:?}"
        );
        assert_eq!(keys.last(), Some(&"condicionado"));
        assert_eq!(plan.cases[2].values["criticidad"], "Crítico");
    }

    #[test]
    fn statuses_and_unknown_values() {
        let r = guion_fixture::FIRST_CASE_ROW;
        let bytes = guion_fixture::build_edited(&[
            (r, 10, Put::Text("Aceptado")),
            (r + 1, 10, Put::Text("Condicionado")),
            (r + 2, 10, Put::Text("Pendiente")),
        ]);
        let plan = plan(&read::read(&bytes).unwrap(), &profile::tests::reference());
        assert_eq!(plan.cases[0].status, "Aprobado");
        assert_eq!(plan.cases[1].values["condicionado"], "true");
        assert_eq!(plan.cases[2].status, "No ejecutado");
        assert!(plan.cases[2].values["condiciones"].ends_with("Estado original: Pendiente"));
        assert!(
            plan.preview.warnings.contains(
                &"CP03: «Pendiente» no es un estado del perfil; el caso entra como «No ejecutado»."
                    .to_string()
            )
        );
    }

    #[test]
    fn empty_rows_are_skipped_and_rows_without_code_block() {
        let r = guion_fixture::FIRST_CASE_ROW;
        let blank: Vec<_> = (0..13).map(|col| (r + 4, col, Put::Blank)).collect();
        let mut book = read::read(&guion_fixture::build_edited(&blank)).unwrap();
        // Filas usadas solo por formato al final no son casos ni avisos.
        book.sheets[0]
            .rows
            .extend(vec![vec![read::Cell::Empty; 13]; 3]);
        let result = plan(&book, &profile::tests::reference());
        assert!(result.preview.errors.is_empty());
        assert_eq!(result.cases.len(), 35);
        assert_eq!(result.preview.warnings, ["Se omitió 1 fila vacía."]);
        book.sheets[0].rows[(r + 6) as usize][2] = read::Cell::Empty;
        assert_eq!(
            plan(&book, &profile::tests::reference()).preview.errors,
            ["Hay filas sin código: 22."]
        );
    }

    #[test]
    fn repeated_codes_and_missing_code_column_block() {
        let bytes = guion_fixture::build_edited(&[(
            guion_fixture::FIRST_CASE_ROW + 1,
            2,
            Put::Text("CP01"),
        )]);
        let book = read::read(&bytes).unwrap();
        let mut def = profile::tests::reference();
        assert_eq!(
            plan(&book, &def).preview.errors,
            ["Hay códigos repetidos: CP01."]
        );
        def.columns.retain(|c| c.target != profile::Target::Code);
        let result = plan(&book, &def);
        assert_eq!(result.preview.errors, [profile::NO_CODE]);
        assert!(result.cases.is_empty());
    }

    #[test]
    fn a_bad_date_is_left_empty_with_a_warning() {
        let r = guion_fixture::FIRST_CASE_ROW;
        let bytes = guion_fixture::build_edited(&[
            (r, 8, Put::Text("mañana")),
            (r + 1, 8, Put::Date(2026, 11, 3)),
        ]);
        let result = plan(&read::read(&bytes).unwrap(), &profile::tests::reference());
        assert!(!result.cases[0].values.contains_key("fecha_prueba"));
        assert_eq!(result.cases[1].values["fecha_prueba"], "2026-11-03");
        assert_eq!(
            result.preview.warnings,
            ["CP01: «mañana» no es una fecha en «Fecha de Prueba»; se deja vacía."]
        );
    }

    #[test]
    fn unknown_list_values_become_options() {
        let r = guion_fixture::FIRST_CASE_ROW;
        let bytes = guion_fixture::build_edited(&[
            (r, 7, Put::Text("Bajo")),
            (r + 1, 7, Put::Text(" bajo ")),
        ]);
        let result = plan(&read::read(&bytes).unwrap(), &profile::tests::reference());
        let field = result
            .preview
            .fields
            .iter()
            .find(|f| f.key == "criticidad")
            .unwrap();
        assert_eq!(field.options, ["Crítico", "Importante", "Bajo"]);
        assert_eq!(result.cases[1].values["criticidad"], "Bajo");
        assert_eq!(
            result.preview.warnings,
            ["«Bajo» no estaba en la lista de «Criticidad»; se agregó como opción."]
        );
    }

    #[test]
    fn detects_the_cover_cells() {
        let book = read::read(&guion_fixture::build()).unwrap();
        let cover = read::detect_cover(&book.sheets[0], 15);
        assert_eq!(cover.execution_date.as_deref(), Some("D5"));
        assert_eq!(cover.responsible.as_deref(), Some("D6"));
        assert_eq!(cover.observations.as_deref(), Some("D8"));
    }

    #[test]
    fn title_and_field_lengths_are_handled_without_losing_unicode() {
        let mut book = read::read(&guion_fixture::build()).unwrap();
        book.sheets[0].rows[15][5] = read::Cell::Empty;
        book.sheets[0].rows[16][5] = read::Cell::Text("á".repeat(301));
        book.sheets[0].rows[15][0] = read::Cell::Text("ñ".repeat(501));
        book.sheets[0].rows[16][12] = read::Cell::Text("x".repeat(16001));
        let result = plan(&book, &profile::tests::reference());
        assert_eq!(result.cases[0].title, "CP01");
        assert_eq!(result.cases[1].title, "á".repeat(300));
        assert_eq!(result.preview.fields[0].field_type, FieldType::Longtext);
        assert!(
            result
                .preview
                .warnings
                .contains(&"CP01: sin título; se usa el código.".into())
        );
        assert!(
            result
                .preview
                .warnings
                .contains(&"CP02: el título se recortó a 300 caracteres.".into())
        );
        assert_eq!(
            result.preview.errors,
            ["CP02: «Condiciones» supera 16000 caracteres."]
        );
    }

    #[test]
    fn absent_sheets_empty_guions_and_case_limit_are_errors() {
        let mut book = read::read(&guion_fixture::build()).unwrap();
        let mut def = profile::tests::reference();
        def.sheet = "Otra".into();
        assert_eq!(
            plan(&book, &def).preview.errors,
            ["El guion no tiene la hoja «Otra»."]
        );
        def.sheet = guion_fixture::SHEET.into();
        def.header_row = u32::MAX;
        assert_eq!(
            plan(&book, &def).preview.errors,
            ["El guion no tiene casos debajo de la fila de encabezados."]
        );
        def.header_row = 1;
        book.sheets[0].rows = vec![vec![]];
        for i in 0..10001 {
            let mut row = vec![read::Cell::Empty; 13];
            row[2] = read::Cell::Text(format!("CP{i}"));
            row[5] = read::Cell::Text("Caso".into());
            book.sheets[0].rows.push(row);
        }
        assert_eq!(
            plan(&book, &def).preview.errors,
            ["El guion supera 10000 casos."]
        );
    }

    #[test]
    fn appended_original_status_obeys_the_field_limit_and_preserves_lines() {
        let mut book = read::read(&guion_fixture::build()).unwrap();
        book.sheets[0].rows[15][12] = read::Cell::Text("x".repeat(15990));
        book.sheets[0].rows[15][10] = read::Cell::Text("Pendiente".into());
        let result = plan(&book, &profile::tests::reference());
        assert_eq!(
            result.preview.errors,
            ["CP01: «Condiciones» supera 16000 caracteres."]
        );
        assert!(result.cases[0].values["condiciones"].ends_with("\nEstado original: Pendiente"));
    }

    #[test]
    fn conditions_promote_lists_and_dates_after_normalizing_all_rows() {
        for field_type in [FieldType::List, FieldType::Date] {
            let mut book = read::read(&guion_fixture::build()).unwrap();
            book.sheets[0].rows.truncate(17);
            book.sheets[0].rows[15][10] = read::Cell::Text("Pendiente".into());
            let (first, second, normalized_first, normalized_second) =
                if field_type == FieldType::Date {
                    ("03/11/2026", "04/11/2026", "2026-11-03", "2026-11-04")
                } else {
                    ("Valor", " valor ", "Valor", "Valor")
                };
            book.sheets[0].rows[15][12] = read::Cell::Text(first.into());
            book.sheets[0].rows[16][12] = read::Cell::Text(second.into());
            let mut def = profile::tests::reference();
            let column = def.columns.iter_mut().find(|c| c.column == "M").unwrap();
            if let profile::Target::Field {
                field_type: kind,
                options,
                ..
            } = &mut column.target
            {
                *kind = field_type;
                *options = if field_type == FieldType::List {
                    vec!["Valor".into()]
                } else {
                    vec![]
                };
            }
            let result = plan(&book, &def);
            assert!(result.preview.errors.is_empty());
            let field = result
                .preview
                .fields
                .iter()
                .find(|f| f.key == "condiciones")
                .unwrap();
            assert_eq!(field.field_type, FieldType::Longtext);
            assert!(field.options.is_empty());
            assert_eq!(
                result.cases[0].values["condiciones"],
                format!("{normalized_first}\nEstado original: Pendiente")
            );
            assert_eq!(result.cases[1].values["condiciones"], normalized_second);
            assert!(
                matches!(&def.columns.iter().find(|c| c.column == "M").unwrap().target,
                profile::Target::Field { field_type: kind, .. } if *kind == field_type)
            );
        }
    }

    #[test]
    fn repeated_codes_follow_first_appearance_and_empty_rows_have_plural() {
        let mut book = read::read(&guion_fixture::build()).unwrap();
        book.sheets[0].rows[17][2] = read::Cell::Text("CP02".into());
        book.sheets[0].rows[18][2] = read::Cell::Text("CP01".into());
        book.sheets[0].rows[19] = vec![read::Cell::Empty; 13];
        book.sheets[0].rows[20] = vec![read::Cell::Empty; 13];
        let result = plan(&book, &profile::tests::reference());
        assert_eq!(
            result.preview.errors,
            ["Hay códigos repetidos: CP01, CP02."]
        );
        assert_eq!(result.preview.warnings, ["Se omitieron 2 filas vacías."]);
    }

    #[test]
    fn cover_uses_nonempty_value_then_merge_then_neighbor_and_stops_at_headers() {
        let sheet = read::GuionSheet {
            name: "Hoja".into(),
            rows: vec![
                vec![
                    read::Cell::Text(" FECHA DE LA PRUEBA: ".into()),
                    read::Cell::Empty,
                    read::Cell::Text("Valor".into()),
                ],
                vec![read::Cell::Text("Responsable:".into())],
                vec![read::Cell::Text("Observaciones:".into())],
                vec![
                    read::Cell::Text("Fecha de la prueba:".into()),
                    read::Cell::Text("Fuera".into()),
                ],
            ],
            merged: vec![(1, 3, 1, 5)],
        };
        let cover = read::detect_cover(&sheet, 4);
        assert_eq!(cover.execution_date.as_deref(), Some("C1"));
        assert_eq!(cover.responsible.as_deref(), Some("D2"));
        assert_eq!(cover.observations.as_deref(), Some("B3"));
        let mut vertical = sheet.clone();
        vertical.merged = vec![(0, 3, 1, 5)];
        assert_eq!(
            read::detect_cover(&vertical, 4).responsible.as_deref(),
            Some("D1")
        );
        assert_eq!(
            read::detect_cover(&sheet, 0),
            profile::CoverCells::default()
        );
    }
}

#[cfg(test)]
mod persistence_tests {
    use std::{collections::BTreeMap, fs};

    use super::*;
    use crate::{
        dto::CaseInput,
        excel::guion_fixture,
        media::storage::TempDir,
        repo::{cases, guion, projects, test_support},
        transfer::bundle::{IMPORT_FAILED, tests::project_dirs},
    };

    fn request<'a>(
        book: &'a GuionBook,
        bytes: &'a [u8],
        def: &'a ProfileDefinition,
        save_profile_as: Option<&'a str>,
    ) -> ImportRequest<'a> {
        ImportRequest {
            book,
            bytes,
            file_name: "guion_cliente.xlsx",
            def,
            project_name: "guion_cliente",
            save_profile_as,
        }
    }

    fn new_case() -> CaseInput {
        CaseInput {
            title: "Un caso nuevo".into(),
            status: "No ejecutado".into(),
            values: BTreeMap::new(),
        }
    }

    #[test]
    fn imports_the_reference_guion_with_its_source_and_copy() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let conn = test_support::conn();
        let bytes = guion_fixture::build();
        let book = read::read(&bytes).unwrap();
        let def = profile::tests::reference();
        let summary = import(
            &conn,
            &files,
            request(&book, &bytes, &def, Some("Cliente A · v3")),
        )
        .unwrap();
        assert_eq!(summary.cases, 36);
        let p = projects::get(&conn, &summary.projects[0].id).unwrap();
        assert_eq!(p.name, "guion_cliente");
        assert_eq!(p.fields.len(), 11);
        let list = cases::list(&conn, &p.id).unwrap();
        assert_eq!(list[0].code, "CP01");
        let rows = guion::case_rows(&conn, &p.id).unwrap();
        assert_eq!(rows[0], (list[0].id.clone(), 16));
        assert_eq!(rows[35].1, 51);
        let source = guion::source(&conn, &p.id).unwrap().unwrap();
        assert_eq!(
            (source.sheet.as_str(), source.header_row),
            (guion_fixture::SHEET, 15)
        );
        assert_eq!(source.definition, def);
        assert_eq!(
            fs::read(files.absolute(&source.stored_path)).unwrap(),
            bytes
        );
        assert_eq!(source.sha256.len(), 64);
        assert!(
            source
                .sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        );
        assert_eq!(
            guion::list_profiles(&conn).unwrap()[0].id,
            source.profile_id.unwrap()
        );
        let next = cases::create(&conn, &p.id, &new_case()).unwrap();
        assert_eq!(
            next.code, "EJ-001",
            "los casos nuevos siguen la secuencia de Qapture"
        );
    }

    #[test]
    fn a_failed_import_leaves_no_rows_no_profile_and_no_copy() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let conn = test_support::conn();
        let bytes = guion_fixture::build();
        let book = read::read(&bytes).unwrap();
        let def = profile::tests::reference();
        guion::create_profile(&conn, "Cliente A · v3", &def).unwrap();
        let error = import(
            &conn,
            &files,
            request(&book, &bytes, &def, Some("Cliente A · v3")),
        )
        .unwrap_err();
        assert_eq!(
            error.to_string(),
            "Ya existe un perfil llamado «Cliente A · v3»."
        );
        assert!(projects::list(&conn).unwrap().is_empty());
        assert_eq!(guion::list_profiles(&conn).unwrap().len(), 1);
        assert_eq!(project_dirs(&files), 0);
    }

    #[test]
    fn errors_in_the_preview_stop_the_import() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let conn = test_support::conn();
        let bytes = guion_fixture::build_edited(&[(
            guion_fixture::FIRST_CASE_ROW + 1,
            2,
            guion_fixture::Put::Text("CP01"),
        )]);
        let book = read::read(&bytes).unwrap();
        let def = profile::tests::reference();
        let error = import(
            &conn,
            &files,
            request(&book, &bytes, &def, Some("Nuevo perfil")),
        )
        .unwrap_err()
        .to_string();
        assert!(error.starts_with(IMPORT_FAILED));
        assert!(error.contains("• Hay códigos repetidos: CP01."));
        assert!(projects::list(&conn).unwrap().is_empty());
        assert!(guion::list_profiles(&conn).unwrap().is_empty());
        assert_eq!(project_dirs(&files), 0);
    }

    #[test]
    fn a_failure_after_creating_the_profile_rolls_back_rows_and_copy() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let conn = test_support::conn();
        let bytes = guion_fixture::build();
        let book = read::read(&bytes).unwrap();
        let def = profile::tests::reference();
        conn.execute_batch("CREATE TRIGGER fail_import_case BEFORE INSERT ON test_case BEGIN SELECT RAISE(FAIL, 'fallo simulado'); END;").unwrap();
        assert!(
            import(
                &conn,
                &files,
                request(&book, &bytes, &def, Some("Perfil nuevo"))
            )
            .is_err()
        );
        assert!(projects::list(&conn).unwrap().is_empty());
        assert!(guion::list_profiles(&conn).unwrap().is_empty());
        let sources: i64 = conn
            .query_row("SELECT COUNT(*) FROM project_source", [], |r| r.get(0))
            .unwrap();
        let fields: i64 = conn
            .query_row("SELECT COUNT(*) FROM project_field", [], |r| r.get(0))
            .unwrap();
        assert_eq!((sources, fields), (0, 0));
        assert_eq!(project_dirs(&files), 0);
    }

    #[test]
    fn importing_without_saving_a_profile_keeps_definition_and_warnings() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let conn = test_support::conn();
        let bytes = guion_fixture::build_edited(&[(
            guion_fixture::FIRST_CASE_ROW,
            10,
            guion_fixture::Put::Text("Pendiente"),
        )]);
        let book = read::read(&bytes).unwrap();
        let def = profile::tests::reference();
        let first = import(&conn, &files, request(&book, &bytes, &def, None)).unwrap();
        let second = import(&conn, &files, request(&book, &bytes, &def, None)).unwrap();
        assert_eq!(second.projects[0].name, "guion_cliente (importado)");
        assert_eq!(
            first.warnings,
            ["CP01: «Pendiente» no es un estado del perfil; el caso entra como «No ejecutado»."]
        );
        let source = guion::source(&conn, &first.projects[0].id)
            .unwrap()
            .unwrap();
        assert!(source.profile_id.is_none());
        assert_eq!(source.definition, def);
        assert!(guion::list_profiles(&conn).unwrap().is_empty());
        let project = projects::get(&conn, &first.projects[0].id).unwrap();
        assert_eq!(project.source_template_id, None);
        assert!(project.fields.iter().all(|f| f.width
            == match f.field_type {
                FieldType::Text => 24,
                FieldType::Longtext => 44,
                FieldType::List => 14,
                FieldType::Date => 15,
                FieldType::Boolean => 12,
                FieldType::Number => 24,
            }));
    }
}
