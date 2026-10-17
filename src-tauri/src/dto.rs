use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::domain::fields::FieldType;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldDto {
    pub id: String,
    pub key: String,
    pub label: String,
    #[serde(rename = "type")]
    pub field_type: FieldType,
    pub options: Vec<String>,
    pub required: bool,
    pub clear_on_duplicate: bool,
    pub width: i64,
    pub position: i64,
    pub archived: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateDto {
    pub id: String,
    pub name: String,
    pub test_type: String,
    pub builtin: bool,
    pub fields: Vec<FieldDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSummaryDto {
    pub id: String,
    pub name: String,
    pub client: String,
    pub case_count: i64,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDto {
    pub id: String,
    pub name: String,
    pub client: String,
    pub test_type: String,
    pub release_version: String,
    pub start_date: String,
    pub end_date: String,
    pub next_sequence: i64,
    pub source_template_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub fields: Vec<FieldDto>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectMetaInput {
    pub name: String,
    pub client: String,
    pub test_type: String,
    pub release_version: String,
    pub start_date: String,
    pub end_date: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NewProjectInput {
    pub meta: ProjectMetaInput,
    pub template_id: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FieldInput {
    pub id: Option<String>,
    pub label: String,
    #[serde(rename = "type")]
    pub field_type: FieldType,
    pub options: Vec<String>,
    pub required: bool,
    pub clear_on_duplicate: bool,
    pub width: i64,
    pub archived: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseDto {
    pub id: String,
    pub project_id: String,
    pub code: String,
    pub title: String,
    pub status: String,
    pub values: BTreeMap<String, String>,
    pub evidence_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaseInput {
    pub title: String,
    pub status: String,
    pub values: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceDto {
    pub id: String,
    pub case_id: String,
    pub position: i64,
    pub description: String,
    pub original_path: String,
    pub rendered_path: String,
    pub annotations: String,
    pub width: i64,
    pub height: i64,
    pub captured_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StagedImageDto {
    pub path: String,
    pub width: i64,
    pub height: i64,
}

/// Payload del evento `evidence:captured` (Rust → ventana principal).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapturedEventDto {
    pub tmp_path: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NewEvidenceMeta {
    pub case_id: String,
    pub staged_path: String,
    pub description: String,
    pub annotations: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EvidenceUpdateMeta {
    pub evidence_id: String,
    pub description: String,
    pub annotations: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocVersionDto {
    pub id: String,
    pub project_id: String,
    pub version: String,
    pub date: String,
    pub author: String,
    pub reviewer: String,
    pub approver: String,
    pub changes: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DocVersionInput {
    pub version: String,
    pub date: String,
    pub author: String,
    pub reviewer: String,
    pub approver: String,
    pub changes: String,
}

/// Opciones del diálogo de exportación (UC-16 / UC-17).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportOptionsInput {
    pub tester: String,
    pub doc_version_id: Option<String>,
    pub include_evidence: bool,
    pub include_summary: bool,
    pub image_max_width: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResultDto {
    pub path: String,
    pub file_name: String,
    pub cases: i64,
    pub evidence: i64,
    pub missing_images: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportStage {
    Preparing,
    Images,
    Writing,
    Saving,
}

/// Payload del evento `export:progress` (Rust → ventana principal).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportProgressDto {
    pub stage: ExportStage,
    pub current: i64,
    pub total: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedProjectDto {
    pub id: String,
    pub name: String,
}

/// Resumen de una importación (UC-18 / UC-19): los proyectos creados, en orden, y sus totales.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummaryDto {
    pub projects: Vec<ImportedProjectDto>,
    pub cases: i64,
    pub evidence: i64,
}

#[cfg(test)]
mod contract {
    use std::{collections::BTreeMap, path::PathBuf};

    use serde_json::{Value, json};

    use super::*;
    use crate::error::AppError;

    const ANNOTATIONS: &str = r#"{"version":1,"crop":null,"shapes":[]}"#;

    fn field() -> FieldDto {
        FieldDto {
            id: "fld-1".into(),
            key: "entorno".into(),
            label: "Entorno".into(),
            field_type: FieldType::List,
            options: vec!["QA".into(), "Producción".into()],
            required: true,
            clear_on_duplicate: false,
            width: 21,
            position: 0,
            archived: false,
        }
    }

    fn meta_json() -> Value {
        json!({ "name": "Recibos", "client": "ACME", "testType": "Funcional", "releaseVersion": "1.0",
                "startDate": "2026-10-01", "endDate": "2026-10-31" })
    }

    fn inputs_json() -> Value {
        json!({
            "newProjectInput": { "meta": meta_json(), "templateId": "tpl-funcional" },
            "projectMetaInput": meta_json(),
            "fieldInput": { "id": null, "label": "Severidad", "type": "list", "options": ["Alta", "Media", "Baja"],
                            "required": false, "clearOnDuplicate": false, "width": 14, "archived": false },
            "caseInput": { "title": "Aplicación de recibos", "status": "Aprobado", "values": { "fld-1": "QA" } },
            "newEvidenceMeta": { "caseId": "case-1", "stagedPath": "C:\\datos\\evidence\\_staging\\stg-1.png",
                                 "description": "Pantalla de recibos", "annotations": ANNOTATIONS },
            "evidenceUpdateMeta": { "evidenceId": "ev-1", "description": "Pantalla de recibos", "annotations": ANNOTATIONS },
            "selection": { "x": 10.5, "y": 20.0, "width": 300.0, "height": 200.0,
                           "viewportWidth": 1536.0, "viewportHeight": 864.0 },
            "docVersionInput": { "version": "1.0", "date": "2026-10-16", "author": "Ana Pérez",
                                 "reviewer": "Luis Gómez", "approver": "", "changes": "Versión inicial" },
            "exportOptions": { "tester": "Ana Pérez", "docVersionId": "ver-1", "includeEvidence": true,
                               "includeSummary": true, "imageMaxWidth": 940 }
        })
    }

    /// F4: en un objeto aparte para no superar el límite de recursión de `json!`.
    fn export_samples() -> Value {
        json!({
            "docVersion": DocVersionDto { id: "ver-1".into(), project_id: "prj-1".into(), version: "1.0".into(),
                                          date: "2026-10-16".into(), author: "Ana Pérez".into(),
                                          reviewer: "Luis Gómez".into(), approver: String::new(),
                                          changes: "Versión inicial".into(), created_at: "2026-10-16T12:00:00.000Z".into() },
            "exportResult": ExportResultDto { path: "C:\\Users\\ana\\Documents\\T4S_ACME_Recibos_Evidencias_20261016_v1.0.xlsx".into(),
                                              file_name: "T4S_ACME_Recibos_Evidencias_20261016_v1.0.xlsx".into(),
                                              cases: 10, evidence: 20, missing_images: 0 },
            "exportProgress": ExportProgressDto { stage: ExportStage::Images, current: 12, total: 20 },
            "lockedError": AppError::FileLocked("C:\\Users\\ana\\Documents\\matriz.xlsx".into()),
            "exportError": AppError::Export("rust_xlsxwriter: formato inválido".into()),
        })
    }

    fn sample() -> Value {
        let mut sample = base_sample();
        if let (Some(target), Value::Object(extra)) = (sample.as_object_mut(), export_samples()) {
            target.extend(extra);
        }
        sample
    }

    fn base_sample() -> Value {
        let ts = "2026-10-15T12:00:00.000Z".to_string();
        json!({
            "field": field(),
            "template": TemplateDto { id: "tpl-funcional".into(), name: "Funcional".into(), test_type: "Funcional".into(),
                                      builtin: true, fields: vec![field()] },
            "projectSummary": ProjectSummaryDto { id: "prj-1".into(), name: "Recibos".into(), client: "ACME".into(),
                                                  case_count: 3, updated_at: ts.clone() },
            "project": ProjectDto { id: "prj-1".into(), name: "Recibos".into(), client: "ACME".into(),
                                    test_type: "Funcional".into(), release_version: "1.0".into(),
                                    start_date: "2026-10-01".into(), end_date: "2026-10-31".into(), next_sequence: 4,
                                    source_template_id: Some("tpl-funcional".into()), created_at: ts.clone(),
                                    updated_at: ts.clone(), fields: vec![field()] },
            "evidence": EvidenceDto { id: "ev-1".into(), case_id: "case-1".into(), position: 0,
                                      description: "Pantalla de recibos aplicados".into(),
                                      original_path: "C:\\datos\\evidence\\prj-1\\ev-1.orig.png".into(),
                                      rendered_path: "C:\\datos\\evidence\\prj-1\\ev-1.render.png".into(),
                                      annotations: ANNOTATIONS.into(), width: 1920, height: 1080,
                                      captured_at: ts.clone(), updated_at: ts.clone() },
            "stagedImage": StagedImageDto { path: "C:\\datos\\evidence\\_staging\\stg-1.png".into(), width: 1920, height: 1080 },
            "capturedEvent": CapturedEventDto { tmp_path: "C:\\datos\\evidence\\_staging\\stg-2.png".into() },
            "captureError": AppError::Capture("xcap: monitor desconectado".into()),
            "case": CaseDto { id: "case-1".into(), project_id: "prj-1".into(), code: "EJ-001".into(),
                              title: "Aplicación de recibos".into(), status: "Aprobado".into(),
                              values: BTreeMap::from([("fld-1".to_string(), "QA".to_string())]),
                              evidence_count: 2,
                              created_at: ts.clone(), updated_at: ts },
            "error": AppError::Validation("«Entorno» es obligatorio.".into()),
            "inputs": inputs_json(),
        })
    }

    #[test]
    fn inputs_deserialize_from_ts_shape() {
        let i = inputs_json();
        serde_json::from_value::<NewProjectInput>(i["newProjectInput"].clone()).unwrap();
        serde_json::from_value::<ProjectMetaInput>(i["projectMetaInput"].clone()).unwrap();
        serde_json::from_value::<FieldInput>(i["fieldInput"].clone()).unwrap();
        serde_json::from_value::<CaseInput>(i["caseInput"].clone()).unwrap();
        serde_json::from_value::<NewEvidenceMeta>(i["newEvidenceMeta"].clone()).unwrap();
        serde_json::from_value::<EvidenceUpdateMeta>(i["evidenceUpdateMeta"].clone()).unwrap();
        serde_json::from_value::<crate::capture::crop::Selection>(i["selection"].clone()).unwrap();
        serde_json::from_value::<DocVersionInput>(i["docVersionInput"].clone()).unwrap();
        serde_json::from_value::<ExportOptionsInput>(i["exportOptions"].clone()).unwrap();
    }

    #[test]
    fn inputs_reject_unknown_fields() {
        let mut case = inputs_json()["caseInput"].clone();
        case["extra"] = json!(1);
        assert!(serde_json::from_value::<CaseInput>(case).is_err());
    }

    /// Regenerar: `UPDATE_CONTRACTS=1 cargo test contract`
    #[test]
    fn dto_contract_matches_fixture() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../src/infrastructure/tauri/__fixtures__/contracts.json");
        let actual = serde_json::to_string_pretty(&sample()).unwrap() + "\n";
        if std::env::var("UPDATE_CONTRACTS").is_ok() {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, &actual).unwrap();
        }
        let expected = std::fs::read_to_string(&path)
            .expect("falta el fixture; ejecuta UPDATE_CONTRACTS=1 cargo test contract")
            .replace("\r\n", "\n");
        assert_eq!(
            actual, expected,
            "El contrato cambió: regenera el fixture y actualiza src/infrastructure/tauri/dto.ts"
        );
    }
}
