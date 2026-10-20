//! Modelo intermedio común a la importación de la v1 y de paquetes `.qapture`.
//! Las imágenes se escriben antes de la transacción (`ImportFiles`) y las filas se insertan
//! en una sola transacción (`insert`): o entra todo o no queda nada.

use std::collections::{BTreeMap, HashMap};

use rusqlite::{Connection, params};

use super::github_json::{self, GithubPayload};
use crate::{
    db::new_id,
    domain::fields::FieldType,
    dto::{DocVersionInput, FieldDto, ImportSummaryDto, ImportedProjectDto, ProjectMetaInput},
    error::{AppResult, validation},
    media::storage::{EvidenceFiles, write_atomic},
    repo::fields,
};

pub const IMPORT_FAILED: &str = "No se importó nada. Revisa estos problemas:";
pub const EMPTY_ANNOTATIONS: &str = r#"{"version":1,"crop":null,"shapes":[]}"#;
const MAX_LISTED: usize = 10;
const MAX_NAME: usize = 150;

#[derive(Debug, Clone, PartialEq)]
pub struct BundleField {
    pub key: String,
    pub label: String,
    pub field_type: FieldType,
    pub options: Vec<String>,
    pub required: bool,
    pub clear_on_duplicate: bool,
    pub width: i64,
    pub archived: bool,
}

/// Sus archivos ya están en `evidence/<proyecto>/<id>.orig.png` y `.render.png`.
#[derive(Debug, Clone, PartialEq)]
pub struct BundleEvidence {
    pub id: String,
    pub description: String,
    pub annotations: String,
    pub width: i64,
    pub height: i64,
    pub captured_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BundleCase {
    pub code: String,
    pub title: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    /// Clave de columna → valor. Los vacíos no se guardan.
    pub values: BTreeMap<String, String>,
    pub evidence: Vec<BundleEvidence>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BundleVersion {
    pub input: DocVersionInput,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectBundle {
    pub id: String,
    pub meta: ProjectMetaInput,
    pub next_sequence: i64,
    pub source_template_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub fields: Vec<BundleField>,
    pub cases: Vec<BundleCase>,
    pub versions: Vec<BundleVersion>,
    pub github: GithubPayload,
}

/// Problemas de validación con su ubicación; se informan los primeros 10 (spec §8).
#[derive(Debug, Default)]
pub struct Problems {
    listed: Vec<String>,
    total: usize,
}

impl Problems {
    pub fn add(&mut self, at: &str, message: &str) {
        self.total += 1;
        if self.listed.len() < MAX_LISTED {
            self.listed.push(if at.is_empty() {
                message.to_string()
            } else {
                format!("{at}: {message}")
            });
        }
    }

    pub fn total(&self) -> usize {
        self.total
    }

    pub fn into_result(self) -> AppResult<()> {
        if self.total == 0 {
            return Ok(());
        }
        let mut text = String::from(IMPORT_FAILED);
        for item in &self.listed {
            text.push_str("\n• ");
            text.push_str(item);
        }
        let rest = self.total - self.listed.len();
        if rest > 0 {
            let noun = if rest == 1 { "problema" } else { "problemas" };
            text.push_str(&format!("\n• … y {rest} {noun} más."));
        }
        Err(validation(text))
    }
}

/// `EJ-` seguido de al menos 3 dígitos (regla de la v1).
pub fn is_case_code(code: &str) -> bool {
    code.strip_prefix("EJ-")
        .is_some_and(|digits| digits.len() >= 3 && digits.bytes().all(|b| b.is_ascii_digit()))
}

pub fn code_number(code: &str) -> Option<i64> {
    code.strip_prefix("EJ-")?.parse().ok()
}

/// Imágenes escritas antes de la transacción. Si la importación no llega a confirmarse,
/// al soltarse borra las carpetas de los proyectos nuevos (nunca toca proyectos existentes:
/// sus ids se acaban de generar).
pub struct ImportFiles<'a> {
    files: &'a EvidenceFiles,
    projects: Vec<String>,
    committed: bool,
}

impl<'a> ImportFiles<'a> {
    pub fn new(files: &'a EvidenceFiles) -> Self {
        Self {
            files,
            projects: Vec::new(),
            committed: false,
        }
    }

    pub fn write(
        &mut self,
        project_id: &str,
        evidence_id: &str,
        original: &[u8],
        rendered: &[u8],
    ) -> AppResult<()> {
        if !self.projects.iter().any(|p| p == project_id) {
            self.projects.push(project_id.to_string());
            std::fs::create_dir_all(self.files.project_dir(project_id))?;
        }
        let original_path = self
            .files
            .absolute(&EvidenceFiles::original_rel(project_id, evidence_id));
        write_atomic(&original_path, original)?;
        let render_path = self
            .files
            .absolute(&EvidenceFiles::render_rel(project_id, evidence_id));
        write_atomic(&render_path, rendered)
    }

    fn commit(mut self) {
        self.committed = true;
    }
}

impl Drop for ImportFiles<'_> {
    fn drop(&mut self) {
        if !self.committed {
            for project_id in &self.projects {
                self.files.remove_project(project_id);
            }
        }
    }
}

fn name_taken(conn: &Connection, name: &str) -> AppResult<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM project WHERE name = ?1 COLLATE NOCASE)",
        [name],
        |r| r.get(0),
    )?)
}

/// Importar nunca sobrescribe: si el nombre existe se agrega «(importado)», «(importado 2)»…
pub fn unique_name(conn: &Connection, wanted: &str) -> AppResult<String> {
    if !name_taken(conn, wanted)? {
        return Ok(wanted.to_string());
    }
    let mut n = 1;
    loop {
        let suffix = if n == 1 {
            " (importado)".to_string()
        } else {
            format!(" (importado {n})")
        };
        let room = MAX_NAME - suffix.chars().count();
        let base: String = wanted.chars().take(room).collect();
        let candidate = format!("{}{suffix}", base.trim_end());
        if !name_taken(conn, &candidate)? {
            return Ok(candidate);
        }
        n += 1;
    }
}

fn insert_project(conn: &Connection, b: &ProjectBundle, name: &str) -> AppResult<()> {
    conn.execute(
        "INSERT INTO project (id, name, client, test_type, release_version, start_date, end_date,
                              next_sequence, source_template_id, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            b.id,
            name,
            b.meta.client,
            b.meta.test_type,
            b.meta.release_version,
            b.meta.start_date,
            b.meta.end_date,
            b.next_sequence,
            b.source_template_id,
            b.created_at,
            b.updated_at
        ],
    )?;
    Ok(())
}

fn insert_fields(conn: &Connection, b: &ProjectBundle) -> AppResult<HashMap<String, String>> {
    let mut ids = HashMap::new();
    for (position, f) in b.fields.iter().enumerate() {
        let dto = FieldDto {
            id: String::new(),
            key: f.key.clone(),
            label: f.label.clone(),
            field_type: f.field_type,
            options: f.options.clone(),
            required: f.required,
            clear_on_duplicate: f.clear_on_duplicate,
            width: f.width,
            position: position as i64,
            archived: f.archived,
        };
        let id = fields::insert_project_field(conn, &b.id, &dto, position as i64)?;
        ids.insert(f.key.clone(), id);
    }
    Ok(ids)
}

fn insert_case(
    conn: &Connection,
    project_id: &str,
    field_ids: &HashMap<String, String>,
    c: &BundleCase,
) -> AppResult<()> {
    let case_id = new_id();
    conn.execute(
        "INSERT INTO test_case (id, project_id, code, title, status, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            case_id,
            project_id,
            c.code,
            c.title,
            c.status,
            c.created_at,
            c.updated_at
        ],
    )?;
    for (key, value) in &c.values {
        if value.is_empty() {
            continue;
        }
        let field_id = field_ids
            .get(key)
            .ok_or_else(|| validation(format!("La columna «{key}» no existe en el paquete.")))?;
        conn.execute(
            "INSERT INTO field_value (case_id, field_id, value) VALUES (?1, ?2, ?3)",
            params![case_id, field_id, value],
        )?;
    }
    for (position, e) in c.evidence.iter().enumerate() {
        conn.execute(
            "INSERT INTO evidence (id, case_id, position, description, original_path, rendered_path,
                                   annotations, width, height, captured_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                e.id,
                case_id,
                position as i64,
                e.description,
                EvidenceFiles::original_rel(project_id, &e.id),
                EvidenceFiles::render_rel(project_id, &e.id),
                e.annotations,
                e.width,
                e.height,
                e.captured_at,
                e.updated_at
            ],
        )?;
    }
    Ok(())
}

fn insert_versions(conn: &Connection, b: &ProjectBundle) -> AppResult<()> {
    for v in &b.versions {
        conn.execute(
            "INSERT INTO doc_version (id, project_id, version, date, author, reviewer, approver, changes, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                new_id(),
                b.id,
                v.input.version,
                v.input.date,
                v.input.author,
                v.input.reviewer,
                v.input.approver,
                v.input.changes,
                v.created_at
            ],
        )?;
    }
    Ok(())
}

/// Inserta todos los proyectos en una sola transacción. Si algo falla, se revierte y
/// `files` (al soltarse) borra las imágenes ya escritas.
pub fn insert(
    conn: &Connection,
    bundles: Vec<ProjectBundle>,
    files: ImportFiles,
) -> AppResult<ImportSummaryDto> {
    let tx = conn.unchecked_transaction()?;
    let mut summary = ImportSummaryDto {
        projects: Vec::new(),
        cases: 0,
        evidence: 0,
        warnings: Vec::new(),
    };
    for b in &bundles {
        let name = unique_name(&tx, &b.meta.name)?;
        insert_project(&tx, b, &name)?;
        let field_ids = insert_fields(&tx, b)?;
        for c in &b.cases {
            insert_case(&tx, &b.id, &field_ids, c)?;
            summary.cases += 1;
            summary.evidence += c.evidence.len() as i64;
        }
        insert_versions(&tx, b)?;
        summary.projects.push(ImportedProjectDto {
            id: b.id.clone(),
            name,
        });
    }
    tx.commit()?;
    files.commit();
    // Los datos de GitHub se aplican con la importación ya confirmada (sus helpers abren
    // transacciones propias): si fallan, el proyecto queda sin ellos y se avisa.
    let mut github_failed = false;
    let mut evidence_repo_dropped = false;
    for b in &bundles {
        match &b.github {
            GithubPayload::Absent => {}
            GithubPayload::Invalid => github_failed = true,
            GithubPayload::Valid(json) => {
                if github_json::apply_imported(conn, &b.id, json) {
                    evidence_repo_dropped |= json.evidence_repo.is_some();
                } else {
                    github_failed = true;
                }
            }
        }
    }
    if github_failed {
        summary
            .warnings
            .push(github_json::GITHUB_WARNING.to_string());
    }
    if evidence_repo_dropped {
        summary
            .warnings
            .push(github_json::EVIDENCE_REPO_WARNING.to_string());
    }
    Ok(summary)
}

#[cfg(test)]
pub(crate) mod tests {
    use std::{collections::BTreeMap, fs};

    use super::*;
    use crate::{
        dto::{DocVersionInput, ProjectMetaInput},
        media::{raster::sample_png, storage::TempDir},
        repo::{cases, doc_versions, evidence, projects, test_support},
    };

    const TS: &str = "2026-10-01T10:00:00.000Z";

    pub(crate) fn meta(name: &str) -> ProjectMetaInput {
        ProjectMetaInput {
            name: name.into(),
            client: "ACME".into(),
            test_type: "Funcional".into(),
            release_version: String::new(),
            start_date: String::new(),
            end_date: String::new(),
        }
    }

    fn field(key: &str, label: &str, field_type: FieldType, width: i64) -> BundleField {
        BundleField {
            key: key.into(),
            label: label.into(),
            field_type,
            options: Vec::new(),
            required: key == "entorno",
            clear_on_duplicate: false,
            width,
            archived: false,
        }
    }

    /// Un proyecto con 2 columnas, un caso con texto multilínea, una captura y una versión.
    pub(crate) fn sample_bundle(out: &mut ImportFiles, name: &str) -> ProjectBundle {
        let id = new_id();
        let evidence_id = new_id();
        let png = sample_png(40, 30);
        out.write(&id, &evidence_id, &png, &png).unwrap();
        ProjectBundle {
            id,
            meta: meta(name),
            next_sequence: 4,
            source_template_id: None,
            created_at: TS.into(),
            updated_at: "2026-10-02T10:00:00.000Z".into(),
            fields: vec![
                field("entorno", "Entorno", FieldType::Text, 21),
                field("datos_entrada", "Datos de entrada", FieldType::Longtext, 48),
            ],
            cases: vec![BundleCase {
                code: "EJ-003".into(),
                title: "Aplicar recibo".into(),
                status: "Aprobado".into(),
                created_at: TS.into(),
                updated_at: TS.into(),
                values: BTreeMap::from([
                    ("entorno".to_string(), "QA".to_string()),
                    (
                        "datos_entrada".to_string(),
                        "Recibo: 1000001\nSucursal: 2".to_string(),
                    ),
                ]),
                evidence: vec![BundleEvidence {
                    id: evidence_id,
                    description: "Pantalla del recibo".into(),
                    annotations: EMPTY_ANNOTATIONS.into(),
                    width: 40,
                    height: 30,
                    captured_at: TS.into(),
                    updated_at: TS.into(),
                }],
            }],
            github: GithubPayload::Absent,
            versions: vec![BundleVersion {
                input: DocVersionInput {
                    version: "1.0".into(),
                    date: "2026-10-01".into(),
                    author: "Ana Pérez".into(),
                    reviewer: String::new(),
                    approver: String::new(),
                    changes: "Versión inicial".into(),
                },
                created_at: TS.into(),
            }],
        }
    }

    pub(crate) fn project_dirs(files: &EvidenceFiles) -> usize {
        fs::read_dir(files.root()).map_or(0, |dir| dir.count())
    }

    #[test]
    fn inserts_a_bundle_with_values_evidence_and_versions() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let conn = test_support::conn();
        let mut out = ImportFiles::new(&files);
        let bundle = sample_bundle(&mut out, "Recibos");
        let summary = insert(&conn, vec![bundle], out).unwrap();

        assert_eq!(summary.cases, 1);
        assert_eq!(summary.evidence, 1);
        let p = projects::get(&conn, &summary.projects[0].id).unwrap();
        assert_eq!((p.name.as_str(), p.next_sequence), ("Recibos", 4));
        assert_eq!(p.created_at, TS);
        assert_eq!(p.source_template_id, None);
        assert_eq!(p.fields.len(), 2);
        assert_eq!(p.fields[1].field_type, FieldType::Longtext);
        let list = cases::list(&conn, &p.id).unwrap();
        assert_eq!(list[0].code, "EJ-003");
        let datos = test_support::field_id(&p, "datos_entrada");
        assert_eq!(list[0].values[&datos], "Recibo: 1000001\nSucursal: 2");
        let ev = evidence::list(&conn, &files, &list[0].id).unwrap();
        assert_eq!(ev[0].description, "Pantalla del recibo");
        assert_eq!(fs::read(&ev[0].original_path).unwrap(), sample_png(40, 30));
        assert_eq!(doc_versions::list(&conn, &p.id).unwrap()[0].version, "1.0");
    }

    #[test]
    fn repeated_names_get_the_importado_suffix() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let conn = test_support::conn();
        test_support::project(&conn); // «Recibos»
        let mut names = Vec::new();
        for wanted in ["Recibos", "recibos", "Pólizas"] {
            let mut out = ImportFiles::new(&files);
            let bundle = sample_bundle(&mut out, wanted);
            names.push(
                insert(&conn, vec![bundle], out).unwrap().projects[0]
                    .name
                    .clone(),
            );
        }
        assert_eq!(
            names,
            ["Recibos (importado)", "recibos (importado 2)", "Pólizas"]
        );
    }

    #[test]
    fn long_names_keep_the_suffix_within_150_characters() {
        let conn = test_support::conn();
        let long = "Á".repeat(150);
        conn.execute(
            "INSERT INTO project (id, name, created_at, updated_at) VALUES ('p', ?1, '', '')",
            [&long],
        )
        .unwrap();
        let name = unique_name(&conn, &long).unwrap();
        assert_eq!(name.chars().count(), 150);
        assert!(name.ends_with("Á (importado)"));
    }

    #[test]
    fn a_failed_insert_leaves_no_rows_and_no_files() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let conn = test_support::conn();
        let mut out = ImportFiles::new(&files);
        let good = sample_bundle(&mut out, "Recibos");
        let mut bad = sample_bundle(&mut out, "Pólizas");
        bad.cases[0].values.insert("no_existe".into(), "x".into());
        assert_eq!(project_dirs(&files), 2);

        let err = insert(&conn, vec![good, bad], out).unwrap_err();
        assert_eq!(
            err.to_string(),
            "La columna «no_existe» no existe en el paquete."
        );
        assert!(projects::list(&conn).unwrap().is_empty());
        assert_eq!(project_dirs(&files), 0, "las imágenes escritas se borran");
    }

    #[test]
    fn dropping_import_files_without_insert_cleans_up() {
        let tmp = TempDir::new();
        let files = tmp.files();
        {
            let mut out = ImportFiles::new(&files);
            sample_bundle(&mut out, "Recibos");
            assert_eq!(project_dirs(&files), 1);
        }
        assert_eq!(project_dirs(&files), 0);
    }

    #[test]
    fn problems_list_at_most_ten_with_a_remainder() {
        let mut problems = Problems::default();
        assert!(Problems::default().into_result().is_ok());
        for n in 1..=12 {
            problems.add(&format!("EJ-{n:03}"), "Estado inválido.");
        }
        problems.add("", "Sin ubicación.");
        assert_eq!(problems.total(), 13);
        let text = problems.into_result().unwrap_err().to_string();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines[0], IMPORT_FAILED);
        assert_eq!(lines[1], "• EJ-001: Estado inválido.");
        assert_eq!(lines.len(), 12);
        assert_eq!(lines[11], "• … y 3 problemas más.");

        let mut one_more = Problems::default();
        for _ in 0..11 {
            one_more.add("", "x");
        }
        assert!(
            one_more
                .into_result()
                .unwrap_err()
                .to_string()
                .ends_with("• … y 1 problema más.")
        );
    }

    #[test]
    fn case_codes() {
        assert!(is_case_code("EJ-001"));
        assert!(is_case_code("EJ-1234"));
        for bad in ["EJ-01", "ej-001", "EJ-00a", "EJ-", "XX-001"] {
            assert!(!is_case_code(bad), "{bad}");
        }
        assert_eq!(code_number("EJ-012"), Some(12));
        assert_eq!(code_number("EJ-99999999999999999999999"), None);
    }
}
