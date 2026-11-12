//! Comandos para exportar en formato de origen (G4-T5).

use rusqlite::Connection;
use tauri::{AppHandle, Emitter, State, WebviewWindow};

use crate::{
    commands::export::{EXPORT_PROGRESS_EVENT, ask_destination},
    domain::text::required_text,
    dto::{ExportProgressDto, ExportResultDto, ExportStage, OriginExportInput, OriginInfoDto},
    error::{AppResult, validation},
    excel::{
        data::IMAGE_WIDTH_RANGE,
        origin::export::{
            self, Availability, OriginData, OriginRequest, check_copy, formulas_at_risk,
        },
        save,
    },
    media::storage::EvidenceFiles,
    state::AppState,
};

/// Genera el nombre sugerido para el archivo de salida (D11):
/// `<file_name sin .xlsx>_ejecutado.xlsx`. Si no termina en `.xlsx`, `<file_name>_ejecutado.xlsx`.
pub fn suggested_name(file_name: &str) -> String {
    let lower = file_name.to_ascii_lowercase();
    if lower.ends_with(".xlsx") {
        let stem = &file_name[..file_name.len() - 5];
        format!("{stem}_ejecutado.xlsx")
    } else {
        format!("{file_name}_ejecutado.xlsx")
    }
}

/// Valida los parámetros de entrada para la exportación de origen.
pub fn validate_origin_export_input(input: &OriginExportInput) -> AppResult<String> {
    let tester = required_text("Tester", &input.tester, 100)?;
    if input.observations.chars().count() > 500 {
        return Err(validation("La observación supera 500 caracteres."));
    }
    if !IMAGE_WIDTH_RANGE.contains(&input.image_max_width) {
        return Err(validation(format!(
            "El ancho máximo de imagen debe estar entre {} y {} px.",
            IMAGE_WIDTH_RANGE.start(),
            IMAGE_WIDTH_RANGE.end()
        )));
    }
    Ok(tester)
}

/// Calcula la información necesaria para el diálogo de exportación de origen sin dependencias de Tauri.
pub(crate) fn info(
    conn: &Connection,
    files: &EvidenceFiles,
    project_id: &str,
    tester: &str,
) -> AppResult<Option<OriginInfoDto>> {
    let Some(data) = OriginData::load(conn, files, project_id)? else {
        return Ok(None);
    };

    let suggested = suggested_name(&data.source.file_name);

    match check_copy(files, &data.source) {
        Availability::Unavailable(reason) => Ok(Some(OriginInfoDto {
            file_name: data.source.file_name,
            sheet: data.source.sheet,
            cases_written: 0,
            unmapped: Vec::new(),
            evidence: 0,
            formulas: Vec::new(),
            unavailable: Some(reason.to_string()),
            suggested_name: suggested,
        })),
        Availability::Ready(base_bytes) => {
            let cases_written = data.cases.len() as i64;
            let unmapped = data.unmapped.clone();
            let evidence = data.export.evidence_total() as i64;
            let formulas = formulas_at_risk(&data, &base_bytes, tester)?;
            Ok(Some(OriginInfoDto {
                file_name: data.source.file_name,
                sheet: data.source.sheet,
                cases_written,
                unmapped,
                evidence,
                formulas,
                unavailable: None,
                suggested_name: suggested,
            }))
        }
    }
}

#[tauri::command(async)]
pub fn export_origin_info(
    state: State<'_, AppState>,
    project_id: String,
) -> AppResult<Option<OriginInfoDto>> {
    let tester = state.settings().profile.tester;
    info(&state.db(), state.files(), &project_id, &tester)
}

#[tauri::command(async)]
pub fn export_origin(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, AppState>,
    project_id: String,
    input: OriginExportInput,
) -> AppResult<Option<ExportResultDto>> {
    let tester = validate_origin_export_input(&input)?;

    // La base se lee de una vez y se suelta antes del diálogo y de generar el libro.
    let data = OriginData::load(&state.db(), state.files(), &project_id)?
        .ok_or_else(|| validation("El proyecto no proviene de un guion de pruebas."))?;

    let base_bytes = match check_copy(state.files(), &data.source) {
        Availability::Ready(bytes) => bytes,
        Availability::Unavailable(reason) => return Err(validation(reason)),
    };

    let name = suggested_name(&data.source.file_name);
    let Some(dest) = ask_destination(&window, &name) else {
        return Ok(None);
    };

    save::ensure_writable(&dest)?;

    let emit_progress = |stage: ExportStage, current: i64, total: i64| {
        let _ = app.emit_to(
            "main",
            EXPORT_PROGRESS_EVENT,
            ExportProgressDto {
                stage,
                current,
                total,
            },
        );
    };

    emit_progress(ExportStage::Preparing, 0, 1);
    emit_progress(ExportStage::Writing, 0, 1);

    let built = export::build(
        &data,
        &base_bytes,
        &OriginRequest {
            tester: &tester,
            observations: &input.observations,
            include_evidence: input.include_evidence,
            image_max_width: input.image_max_width,
        },
    )?;

    emit_progress(ExportStage::Saving, 0, 1);
    save::save_bytes_atomic(&built.bytes, &dest)?;
    emit_progress(ExportStage::Saving, 1, 1);

    state.exports().remember(&dest);
    let file_name = dest
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_default();

    Ok(Some(ExportResultDto {
        path: dest.to_string_lossy().to_string(),
        file_name,
        cases: built.cases_written as i64,
        evidence: built.evidence as i64,
        missing_images: built.missing_images as i64,
    }))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use rusqlite::params;

    use super::*;
    use crate::{
        dto::{CaseInput, NewProjectInput, ProjectMetaInput},
        excel::{
            guion_fixture,
            origin::{export::CHANGED, export::NO_COPY, parts::XlsxParts},
            testing::write_png,
        },
        media::{fixtures::sha256_hex, storage::TempDir},
        repo::{cases, projects, test_support},
        transfer::guion::{import, profile, read},
    };

    fn setup_imported() -> (rusqlite::Connection, TempDir, Vec<u8>, String) {
        let conn = test_support::conn();
        let tmp = TempDir::new();
        let files = tmp.files();
        let bytes = guion_fixture::build();
        let book = read::read(&bytes).unwrap();
        let def = profile::tests::reference();

        let summary = import::import(
            &conn,
            &files,
            import::tests::request(&book, &bytes, &def, None),
        )
        .unwrap();
        let project_id = summary.projects[0].id.clone();
        (conn, tmp, bytes, project_id)
    }

    #[test]
    fn suggested_name_adds_ejecutado() {
        assert_eq!(
            suggested_name("guion_cliente.xlsx"),
            "guion_cliente_ejecutado.xlsx"
        );
        assert_eq!(
            suggested_name("guion_cliente.XLSX"),
            "guion_cliente_ejecutado.xlsx"
        );
        assert_eq!(
            suggested_name("guion_cliente"),
            "guion_cliente_ejecutado.xlsx"
        );
    }

    #[test]
    fn validate_input_accepts_valid_data() {
        let input = OriginExportInput {
            tester: "  Ana Pérez  ".into(),
            observations: "Todo en orden".into(),
            include_evidence: true,
            image_max_width: 940,
        };
        assert_eq!(validate_origin_export_input(&input).unwrap(), "Ana Pérez");
    }

    #[test]
    fn validate_input_rejects_empty_tester() {
        let input = OriginExportInput {
            tester: "   ".into(),
            observations: "".into(),
            include_evidence: false,
            image_max_width: 940,
        };
        let err = validate_origin_export_input(&input).unwrap_err();
        assert_eq!(err.to_string(), "«Tester» es obligatorio.");
    }

    #[test]
    fn validate_input_rejects_long_tester() {
        let input = OriginExportInput {
            tester: "a".repeat(101),
            observations: "".into(),
            include_evidence: false,
            image_max_width: 940,
        };
        let err = validate_origin_export_input(&input).unwrap_err();
        assert_eq!(err.to_string(), "«Tester» supera 100 caracteres.");
    }

    #[test]
    fn validate_input_rejects_observations_over_500_chars() {
        let input = OriginExportInput {
            tester: "Ana".into(),
            observations: "a".repeat(501),
            include_evidence: false,
            image_max_width: 940,
        };
        let err = validate_origin_export_input(&input).unwrap_err();
        assert_eq!(err.to_string(), "La observación supera 500 caracteres.");
    }

    #[test]
    fn validate_input_rejects_image_width_out_of_range() {
        let low = OriginExportInput {
            tester: "Ana".into(),
            observations: "".into(),
            include_evidence: false,
            image_max_width: 399,
        };
        let err_low = validate_origin_export_input(&low).unwrap_err();
        assert_eq!(
            err_low.to_string(),
            "El ancho máximo de imagen debe estar entre 400 y 1600 px."
        );

        let high = OriginExportInput {
            tester: "Ana".into(),
            observations: "".into(),
            include_evidence: false,
            image_max_width: 1601,
        };
        let err_high = validate_origin_export_input(&high).unwrap_err();
        assert_eq!(
            err_high.to_string(),
            "El ancho máximo de imagen debe estar entre 400 y 1600 px."
        );
    }

    #[test]
    fn info_is_none_for_projects_not_imported() {
        let conn = test_support::conn();
        let tmp = TempDir::new();
        let files = tmp.files();

        let prj = projects::create(
            &conn,
            NewProjectInput {
                meta: ProjectMetaInput {
                    name: "Manual".into(),
                    client: "Cliente".into(),
                    test_type: "Funcional".into(),
                    release_version: "1.0".into(),
                    start_date: "2026-10-01".into(),
                    end_date: "2026-10-31".into(),
                },
                template_id: "tpl-funcional".into(),
            },
        )
        .unwrap();

        let res = info(&conn, &files, &prj.id, "Ana").unwrap();
        assert!(res.is_none());
    }

    #[test]
    fn info_reports_unavailable_copy() {
        let (conn, tmp, _base, project_id) = setup_imported();
        let files = tmp.files();

        let source: crate::repo::guion::ProjectSource =
            crate::repo::guion::source(&conn, &project_id)
                .unwrap()
                .unwrap();
        let stored_abs = files.absolute(&source.stored_path);

        // Archivo alterado -> CHANGED
        fs::write(&stored_abs, b"bytes corruptos").unwrap();
        let changed = info(&conn, &files, &project_id, "Ana").unwrap().unwrap();
        assert_eq!(changed.unavailable, Some(CHANGED.to_string()));
        assert_eq!(changed.cases_written, 0);
        assert!(changed.unmapped.is_empty());
        assert_eq!(changed.evidence, 0);
        assert!(changed.formulas.is_empty());
        assert_eq!(changed.file_name, source.file_name);
        assert_eq!(changed.sheet, source.sheet);
        assert_eq!(changed.suggested_name, suggested_name(&source.file_name));

        // Archivo eliminado -> NO_COPY
        fs::remove_file(&stored_abs).unwrap();
        let no_copy = info(&conn, &files, &project_id, "Ana").unwrap().unwrap();
        assert_eq!(no_copy.unavailable, Some(NO_COPY.to_string()));
        assert_eq!(no_copy.cases_written, 0);
    }

    #[test]
    fn info_counts_cases_evidence_formulas_and_unmapped() {
        let (conn, tmp, _base, project_id) = setup_imported();
        let files = tmp.files();

        let source = crate::repo::guion::source(&conn, &project_id)
            .unwrap()
            .unwrap();

        // Creamos un caso no mapeado
        cases::create(
            &conn,
            &project_id,
            &CaseInput {
                title: "Caso extra fuera del guion".into(),
                status: "Aprobado".into(),
                values: std::collections::BTreeMap::new(),
            },
        )
        .unwrap();

        // Agregamos evidencia al primer caso del guion
        let case_list = cases::list(&conn, &project_id).unwrap();
        let cp01 = &case_list[0];
        let rel_img = EvidenceFiles::render_rel(&project_id, "e1");
        fs::create_dir_all(files.project_dir(&project_id)).unwrap();
        write_png(
            &files.project_dir(&project_id),
            "e1.render.png",
            100,
            100,
            1,
        );
        conn.execute(
            "INSERT INTO evidence (id, case_id, position, description, original_path, rendered_path, annotations, width, height, captured_at, updated_at) VALUES ('e1', ?1, 0, 'Captura CP01', ?2, ?2, '{}', 100, 100, '2026-11-10T12:00:00Z', '2026-11-10T12:00:00Z')",
            params![cp01.id, rel_img],
        ).unwrap();

        let res = info(&conn, &files, &project_id, "Ana").unwrap().unwrap();
        assert_eq!(res.cases_written, 36);
        assert_eq!(res.unmapped, vec!["EJ-001".to_string()]); // El nuevo caso recibe el código generado EJ-001
        assert_eq!(res.evidence, 1);
        assert!(res.formulas.is_empty());
        assert_eq!(res.unavailable, None);
        assert_eq!(res.file_name, source.file_name);
        assert_eq!(res.sheet, "Guión de Pruebas");
        assert_eq!(res.suggested_name, suggested_name(&source.file_name));
    }

    #[test]
    fn info_detects_formulas_at_risk() {
        let (conn, tmp, base, project_id) = setup_imported();
        let files = tmp.files();

        let source = crate::repo::guion::source(&conn, &project_id)
            .unwrap()
            .unwrap();
        let stored_abs = files.absolute(&source.stored_path);

        // Inyectamos una fórmula en I16
        let mut parts = XlsxParts::read(&base).unwrap();
        let sheet1 = parts.text("xl/worksheets/sheet1.xml").unwrap();
        let sheet1_mod = sheet1.replace(
            r#"<c r="J16""#,
            r#"<c r="I16" s="2"><f>TODAY()</f><v>45962</v></c><c r="J16""#,
        );
        parts.set("xl/worksheets/sheet1.xml", sheet1_mod.into_bytes());
        let modified_bytes = parts.write().unwrap();
        let new_hash = sha256_hex(&modified_bytes);

        // Actualizamos archivo guardado y hash en la base para que check_copy dé Ready
        fs::write(&stored_abs, &modified_bytes).unwrap();
        conn.execute(
            "UPDATE project_source SET sha256 = ?1 WHERE project_id = ?2",
            params![new_hash, project_id],
        )
        .unwrap();

        let res = info(&conn, &files, &project_id, "Ana").unwrap().unwrap();
        assert_eq!(res.formulas, vec!["I16".to_string()]);
    }
}
