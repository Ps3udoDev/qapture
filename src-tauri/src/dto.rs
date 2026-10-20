use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{domain::fields::FieldType, settings::Settings, system::shortcuts::ShortcutAction};

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
    /// Avisos no bloqueantes (p. ej. datos de GitHub del paquete que no se pudieron leer).
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BackupKind {
    Daily,
    Manual,
    BeforeRestore,
    BeforeMigration,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupDto {
    pub file_name: String,
    pub kind: BackupKind,
    /// Hora local de la copia (`AAAA-MM-DDTHH:MM:SS`).
    pub created_at: String,
    pub size_bytes: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreResultDto {
    pub restored: BackupDto,
    /// Copia del estado anterior, por si hay que deshacer.
    pub safety_copy: String,
    /// Capturas del respaldo cuyo archivo ya no existe.
    pub missing_images: i64,
    /// Capturas que el respaldo no conoce, movidas a `evidence-sin-uso/`.
    pub set_aside_files: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageExportResultDto {
    pub path: String,
    pub file_name: String,
    pub cases: i64,
    pub evidence: i64,
    pub missing_images: i64,
}

/// Lo primero que pide el front: si la base está dañada, muestra la pantalla de recuperación.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupStatusDto {
    pub recovery: bool,
    pub detail: String,
}

/// Un atajo que el SO no dejó registrar (otra aplicación ya lo usa).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutFailureDto {
    pub action: ShortcutAction,
    pub accelerator: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsDto {
    pub settings: Settings,
    pub shortcut_failures: Vec<ShortcutFailureDto>,
    pub app_version: String,
    pub data_dir: String,
    pub portable: bool,
}

/// Una versión nueva publicada (payload de `update:available` y resultado de `check_update`).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfoDto {
    pub version: String,
    pub current_version: String,
    pub notes: String,
    pub date: Option<String>,
    pub portable: bool,
}

/// Payload de `update:progress`: bytes descargados y total (si el servidor lo informa).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProgressDto {
    pub downloaded: u64,
    pub total: Option<u64>,
}

/// Cuenta de GitHub conectada (payload de `gh:connected` y resultado de `gh_account`).
/// El token nunca viaja al front.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GhAccountDto {
    pub login: String,
    pub name: Option<String>,
    pub avatar_data_url: Option<String>,
    pub connected_at: String,
    pub needs_reconnect: bool,
}

/// Código del Device Flow para mostrar. Sin `device_code`: ese solo lo usa el sondeo en Rust.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceCodeDto {
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GhRepoDto {
    pub owner: String,
    pub name: String,
    pub full_name: String,
    pub private: bool,
    pub can_write_issues: bool,
    pub can_write_contents: bool,
}

/// Repo vinculado a un proyecto. Se usa tanto de salida como dentro de `ProjectGithubInput`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LinkedRepoDto {
    pub full_name: String,
    pub is_default: bool,
    pub default_labels: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectGithubDto {
    pub repos: Vec<LinkedRepoDto>,
    pub routing_field_id: Option<String>,
    pub routing_map: BTreeMap<String, String>,
    pub evidence_repo: Option<String>,
    /// La columna de ruteo se archivó, cambió de tipo o perdió alguna opción del mapa.
    pub routing_stale: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectGithubInput {
    pub repos: Vec<LinkedRepoDto>,
    pub routing_field_id: Option<String>,
    pub routing_map: BTreeMap<String, String>,
    pub evidence_repo: Option<String>,
}

/// Payload de `gh:connect-failed`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GhConnectFailedDto {
    pub message: String,
}

/// Vínculo caso ↔ issue.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseIssueLinkDto {
    pub case_id: String,
    pub owner: String,
    pub repo: String,
    pub full_name: String,
    pub issue_number: u64,
    pub html_url: String,
    pub direction: String,
    pub issue_state: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MilestoneDto {
    pub number: u64,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoMetaDto {
    pub labels: Vec<String>,
    pub milestones: Vec<MilestoneDto>,
    pub assignees: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReportInputDto {
    pub case_id: String,
    /// `dueño/repositorio`.
    pub repo: String,
    pub title: String,
    pub body: String,
    pub labels: Vec<String>,
    pub assignees: Vec<String>,
    pub milestone: Option<u64>,
    pub evidence_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportResultDto {
    pub link: CaseIssueLinkDto,
    /// `repo` o `assisted`.
    pub upload_mode: String,
    /// Texto fijo de la UI cuando `upload_mode` es `assisted`.
    pub assisted_reason: Option<String>,
    pub pending_evidence: Vec<String>,
    pub comment_url: Option<String>,
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
            "saveSettings": serde_json::to_value(Settings::default()).unwrap_or_default(),
            "exportOptions": { "tester": "Ana Pérez", "docVersionId": "ver-1", "includeEvidence": true,
                               "includeSummary": true, "imageMaxWidth": 940 },
            "projectGithubInput": project_github_input_json(),
            "reportInput": report_input_json()
        })
    }

    /// F8b: lo que manda el diálogo de reporte.
    fn report_input_json() -> Value {
        json!({
            "caseId": "case-1",
            "repo": "TEAMS4SOFT/GMX-frontend",
            "title": "[EJ-001] Aplicación de recibos",
            "body": "Pasos\n\n### Evidencias\n\n**1.** Login\n{{img:1}}",
            "labels": ["bug"],
            "assignees": ["Ps3udoDev"],
            "milestone": 3,
            "evidenceIds": ["ev-1"]
        })
    }

    /// F8a: el spec §5.2 los llama `gh_project_repos_get/set`; aquí son `gh_project_github_*`
    /// porque guardan repos, ruteo y repo de evidencias.
    fn project_github_input_json() -> Value {
        json!({
            "repos": [
                { "fullName": "TEAMS4SOFT/GMX-frontend", "isDefault": true, "defaultLabels": ["bug", "qapture"] },
                { "fullName": "TEAMS4SOFT/GMX-backend", "isDefault": false, "defaultLabels": [] }
            ],
            "routingFieldId": "fld-1",
            "routingMap": { "QA": "TEAMS4SOFT/GMX-frontend" },
            "evidenceRepo": "TEAMS4SOFT/qa-evidencias"
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

    /// F5: en un objeto aparte para no superar el límite de recursión de `json!`.
    fn data_samples() -> Value {
        let backup = BackupDto {
            file_name: "qapture-diario-20261016.db".into(),
            kind: BackupKind::Daily,
            created_at: "2026-10-16T09:00:00".into(),
            size_bytes: 98_304,
        };
        json!({
            "importSummary": ImportSummaryDto {
                projects: vec![ImportedProjectDto { id: "prj-2".into(), name: "Aplicación de recibos".into() }],
                cases: 3, evidence: 2, warnings: vec![],
            },
            "packageExportResult": PackageExportResultDto {
                path: "C:\\Users\\ana\\Documents\\T4S_ACME_Recibos_20261016.qapture".into(),
                file_name: "T4S_ACME_Recibos_20261016.qapture".into(),
                cases: 10, evidence: 20, missing_images: 0,
            },
            "backup": backup.clone(),
            "restoreResult": RestoreResultDto {
                restored: backup,
                safety_copy: "qapture-antes-de-restaurar-20261016-100000.db".into(),
                missing_images: 0, set_aside_files: 2,
            },
            "startupStatus": StartupStatusDto { recovery: true, detail: "base dañada: database disk image is malformed".into() },
            "corruptError": AppError::Corrupt("database disk image is malformed".into()),
            "settings": SettingsDto {
                settings: Settings::default(),
                shortcut_failures: vec![ShortcutFailureDto {
                    action: ShortcutAction::Capture,
                    accelerator: "Ctrl+Alt+S".into(),
                }],
                app_version: "0.5.0".into(),
                data_dir: "C:\\Users\\ana\\AppData\\Roaming\\com.teams4soft.qapture".into(),
                portable: false,
            },
            "updateInfo": UpdateInfoDto {
                version: "1.0.1".into(),
                current_version: "1.0.0".into(),
                notes: "Correcciones de la exportación a Excel.".into(),
                date: Some("2026-10-17T12:00:00Z".into()),
                portable: false,
            },
            "updateProgress": UpdateProgressDto { downloaded: 1_048_576, total: Some(8_388_608) },
        })
    }

    /// F8a: cuenta, Device Flow, repos y configuración GitHub del proyecto.
    fn github_samples() -> Value {
        let linked = LinkedRepoDto {
            full_name: "TEAMS4SOFT/GMX-frontend".into(),
            is_default: true,
            default_labels: vec!["bug".into(), "qapture".into()],
        };
        let link = CaseIssueLinkDto {
            case_id: "case-1".into(),
            owner: "TEAMS4SOFT".into(),
            repo: "GMX-frontend".into(),
            full_name: "TEAMS4SOFT/GMX-frontend".into(),
            issue_number: 45,
            html_url: "https://github.com/TEAMS4SOFT/GMX-frontend/issues/45".into(),
            direction: "exported".into(),
            issue_state: "open".into(),
        };
        json!({
            "ghAccount": GhAccountDto {
                login: "Ps3udoDev".into(),
                name: Some("Pseudo Dev".into()),
                avatar_data_url: Some("data:image/png;base64,UE5HREFUQQ==".into()),
                connected_at: "2026-10-19T12:00:00+00:00".into(),
                needs_reconnect: false,
            },
            "deviceCode": DeviceCodeDto {
                user_code: "ABCD-1234".into(),
                verification_uri: "https://github.com/login/device".into(),
                expires_in: 900,
                interval: 5,
            },
            "ghRepo": GhRepoDto {
                owner: "TEAMS4SOFT".into(),
                name: "GMX-frontend".into(),
                full_name: "TEAMS4SOFT/GMX-frontend".into(),
                private: true,
                can_write_issues: true,
                can_write_contents: false,
            },
            "linkedRepo": linked.clone(),
            "projectGithub": ProjectGithubDto {
                repos: vec![linked],
                routing_field_id: Some("fld-1".into()),
                routing_map: BTreeMap::from([("QA".to_string(), "TEAMS4SOFT/GMX-frontend".to_string())]),
                evidence_repo: Some("TEAMS4SOFT/qa-evidencias".into()),
                routing_stale: false,
            },
            "ghConnectFailed": GhConnectFailedDto { message: "Se canceló la autorización en GitHub.".into() },
            "caseIssueLink": link.clone(),
            "repoMeta": RepoMetaDto {
                labels: vec!["bug".into(), "qapture".into()],
                milestones: vec![MilestoneDto { number: 3, title: "Sprint 3".into() }],
                assignees: vec!["Ps3udoDev".into()],
            },
            "reportResult": ReportResultDto {
                link: link.clone(),
                upload_mode: "assisted".into(),
                assisted_reason: Some("No hay repo de evidencias configurado.".into()),
                pending_evidence: vec!["ev-1".into()],
                comment_url: None,
            },
            "commentResult": ReportResultDto {
                link,
                upload_mode: "repo".into(),
                assisted_reason: None,
                pending_evidence: vec![],
                comment_url: Some("https://github.com/TEAMS4SOFT/GMX-frontend/issues/45#issuecomment-1".into()),
            },
            "ghAuthError": crate::error::gh(
                crate::error::GhErrorKind::Auth,
                "Tu cuenta de GitHub no está conectada o la sesión venció. Conéctala en Ajustes › GitHub.",
            ),
        })
    }

    fn sample() -> Value {
        let mut sample = base_sample();
        if let Some(target) = sample.as_object_mut() {
            for extra in [export_samples(), data_samples(), github_samples()] {
                if let Value::Object(extra) = extra {
                    target.extend(extra);
                }
            }
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
        serde_json::from_value::<Settings>(i["saveSettings"].clone()).unwrap();
        serde_json::from_value::<ProjectGithubInput>(i["projectGithubInput"].clone()).unwrap();
        serde_json::from_value::<ReportInputDto>(i["reportInput"].clone()).unwrap();
    }

    #[test]
    fn inputs_reject_unknown_fields() {
        let mut case = inputs_json()["caseInput"].clone();
        case["extra"] = json!(1);
        assert!(serde_json::from_value::<CaseInput>(case).is_err());
        let mut github = inputs_json()["projectGithubInput"].clone();
        github["routingStale"] = json!(false);
        assert!(serde_json::from_value::<ProjectGithubInput>(github).is_err());
        let mut linked = inputs_json()["projectGithubInput"].clone();
        linked["repos"][0]["extra"] = json!(1);
        assert!(serde_json::from_value::<ProjectGithubInput>(linked).is_err());
        let mut report = inputs_json()["reportInput"].clone();
        report["extra"] = json!(1);
        assert!(serde_json::from_value::<ReportInputDto>(report).is_err());
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
