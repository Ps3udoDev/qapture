//! UC-18: paquete `.qapture` = ZIP con `project.json` (`formatVersion: 1`) y las capturas en
//! `evidence/<código>-<n>.orig.png` / `.render.png`. `project.json` no lleva ids ni la plantilla de
//! origen: exportar → importar → exportar da el mismo archivo. Importar crea siempre una copia.

use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs::{self, File},
    io::{self, BufWriter, Read, Seek, Write},
    path::{Path, PathBuf},
};

use chrono::NaiveDate;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use zip::{CompressionMethod, ZipArchive, ZipWriter, write::SimpleFileOptions};

#[cfg(test)]
use super::bundle;
use super::bundle::{
    BundleCase, BundleEvidence, BundleField, BundleFixture, BundleSource, BundleVersion,
    IMPORT_FAILED, ImportFiles, Problems, ProjectBundle, code_number, is_case_code,
};
use super::github_json::{self, GITHUB_JSON, GithubJson, GithubPayload};
#[cfg(test)]
use crate::dto::ImportSummaryDto;
use crate::{
    db::new_id,
    domain::{
        fields::{FieldRule, FieldType, validate_value},
        status::is_status,
        text::required_text,
    },
    dto::{DocVersionInput, ProjectMetaInput},
    error::{AppError, AppResult, validation},
    excel::{data::file_segment, save::ensure_writable},
    media::{
        fixtures::{self, MAX_FIXTURE_BYTES},
        raster,
        storage::EvidenceFiles,
    },
    repo::{cases, doc_versions, evidence, fixtures as repo_fixtures, guion, projects},
    transfer::guion::{
        import::GUION_FILE,
        profile::{self, ProfileDefinition},
        read::MAX_FILE_BYTES as MAX_GUION_BYTES,
    },
};

pub const FORMAT_VERSION: i64 = 2;
pub const FORMAT_V1: i64 = 1;
pub const PROJECT_JSON: &str = "project.json";
pub const EXTENSION: &str = "qapture";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackageDoc {
    pub format_version: i64,
    pub project: PackageProject,
    pub fields: Vec<PackageField>,
    pub cases: Vec<PackageCase>,
    pub doc_versions: Vec<PackageDocVersion>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackageProject {
    pub name: String,
    pub client: String,
    pub test_type: String,
    pub release_version: String,
    pub start_date: String,
    pub end_date: String,
    pub next_sequence: i64,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<PackageSource>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackageSource {
    pub file_name: String,
    pub sha256: String,
    pub sheet: String,
    pub header_row: u32,
    pub definition: ProfileDefinition,
    pub imported_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackageField {
    pub key: String,
    pub label: String,
    #[serde(rename = "type")]
    pub field_type: FieldType,
    pub options: Vec<String>,
    pub required: bool,
    pub clear_on_duplicate: bool,
    pub width: i64,
    pub archived: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackageCase {
    pub code: String,
    pub title: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    /// Clave de columna → valor.
    pub values: BTreeMap<String, String>,
    pub evidence: Vec<PackageEvidence>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_row: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fixtures: Vec<PackageFixture>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackageFixture {
    pub file_name: String,
    pub note: String,
    pub size: i64,
    pub sha256: String,
    pub added_at: String,
    pub file: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackageEvidence {
    pub description: String,
    pub annotations: String,
    pub width: i64,
    pub height: i64,
    pub captured_at: String,
    pub updated_at: String,
    /// Rutas dentro del ZIP.
    pub original: String,
    pub rendered: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackageDocVersion {
    pub version: String,
    pub date: String,
    pub author: String,
    pub reviewer: String,
    pub approver: String,
    pub changes: String,
    pub created_at: String,
}

/// Lo que se escribe en el ZIP. Se arma con la base bloqueada y se escribe sin ella.
pub struct PackageExport {
    pub doc: PackageDoc,
    /// Ruta dentro del ZIP → archivo en `evidence/`.
    pub images: Vec<(String, PathBuf)>,
    /// Entradas no-imagen en el ZIP (source/guion.xlsx y fixtures/<n>) → archivo en `evidence/`.
    pub files: Vec<(String, PathBuf)>,
    /// Capturas omitidas porque falta su original o su render.
    pub missing_images: i64,
    /// Archivos de prueba o de origen del guion omitidos porque no están en disco.
    pub missing_files: i64,
    /// Entrada opcional `github.json`: solo si el proyecto tiene datos de GitHub.
    pub github: Option<GithubJson>,
}

impl PackageExport {
    pub fn evidence_count(&self) -> i64 {
        self.doc.cases.iter().map(|c| c.evidence.len() as i64).sum()
    }
}

struct StoredEvidence {
    description: String,
    annotations: String,
    width: i64,
    height: i64,
    captured_at: String,
    updated_at: String,
    original_rel: String,
    render_rel: String,
}

fn evidence_by_case(
    conn: &Connection,
    project_id: &str,
) -> AppResult<HashMap<String, Vec<StoredEvidence>>> {
    let mut stmt = conn.prepare(
        "SELECT e.case_id, e.description, e.annotations, e.width, e.height, e.captured_at,
                e.updated_at, e.original_path, e.rendered_path
         FROM evidence e JOIN test_case c ON c.id = e.case_id
         WHERE c.project_id = ?1 ORDER BY e.position, e.captured_at",
    )?;
    let rows = stmt.query_map([project_id], |r| {
        Ok((
            r.get::<_, String>(0)?,
            StoredEvidence {
                description: r.get(1)?,
                annotations: r.get(2)?,
                width: r.get(3)?,
                height: r.get(4)?,
                captured_at: r.get(5)?,
                updated_at: r.get(6)?,
                original_rel: r.get(7)?,
                render_rel: r.get(8)?,
            },
        ))
    })?;
    let mut out: HashMap<String, Vec<StoredEvidence>> = HashMap::new();
    for row in rows {
        let (case_id, stored) = row?;
        out.entry(case_id).or_default().push(stored);
    }
    Ok(out)
}

pub fn collect(
    conn: &Connection,
    files: &EvidenceFiles,
    project_id: &str,
) -> AppResult<PackageExport> {
    let project = projects::get(conn, project_id)?;
    let keys: HashMap<&str, &str> = project
        .fields
        .iter()
        .map(|f| (f.id.as_str(), f.key.as_str()))
        .collect();
    let mut by_case = evidence_by_case(conn, project_id)?;
    let mut images = Vec::new();
    let mut missing_images = 0;
    let mut package_files = Vec::new();
    let mut missing_files = 0;

    let proj_source_opt = if evidence::has_table(conn, "project_source")? {
        guion::source(conn, project_id)?
    } else {
        None
    };

    let package_source = if let Some(proj_source) = &proj_source_opt {
        let abs = files.absolute(&proj_source.stored_path);
        let file_entry = if abs.is_file() {
            let entry_name = format!("source/{GUION_FILE}");
            package_files.push((entry_name.clone(), abs));
            Some(entry_name)
        } else {
            missing_files += 1;
            None
        };
        Some(PackageSource {
            file_name: proj_source.file_name.clone(),
            sha256: proj_source.sha256.clone(),
            sheet: proj_source.sheet.clone(),
            header_row: proj_source.header_row,
            definition: proj_source.definition.clone(),
            imported_at: proj_source.imported_at.clone(),
            file: file_entry,
        })
    } else {
        None
    };

    let fixtures_by_case = if evidence::has_table(conn, "case_fixture")? {
        repo_fixtures::stored_by_case(conn, project_id)?
    } else {
        HashMap::new()
    };

    let case_rows_map: HashMap<String, u32> = if evidence::has_table(conn, "case_source")? {
        guion::case_rows(conn, project_id)?.into_iter().collect()
    } else {
        HashMap::new()
    };

    let mut fixture_counter = 0;
    let mut package_cases = Vec::new();
    for case in cases::list(conn, project_id)? {
        let mut evidence = Vec::new();
        for stored in by_case.remove(&case.id).unwrap_or_default() {
            let original = files.absolute(&stored.original_rel);
            let rendered = files.absolute(&stored.render_rel);
            if !original.is_file() || !rendered.is_file() {
                missing_images += 1;
                continue;
            }
            let n = evidence.len() + 1;
            let original_name = format!("evidence/{}-{n}.orig.png", case.code);
            let rendered_name = format!("evidence/{}-{n}.render.png", case.code);
            images.push((original_name.clone(), original));
            images.push((rendered_name.clone(), rendered));
            evidence.push(PackageEvidence {
                description: stored.description,
                annotations: stored.annotations,
                width: stored.width,
                height: stored.height,
                captured_at: stored.captured_at,
                updated_at: stored.updated_at,
                original: original_name,
                rendered: rendered_name,
            });
        }

        let mut case_fixtures = Vec::new();
        if let Some(case_fixtures_stored) = fixtures_by_case.get(&case.id) {
            for (dto, stored_path) in case_fixtures_stored {
                let abs = files.absolute(stored_path);
                if abs.is_file() {
                    fixture_counter += 1;
                    let zip_entry = format!("fixtures/{fixture_counter}");
                    package_files.push((zip_entry.clone(), abs));
                    case_fixtures.push(PackageFixture {
                        file_name: dto.file_name.clone(),
                        note: dto.note.clone(),
                        size: dto.size,
                        sha256: dto.sha256.clone(),
                        added_at: dto.added_at.clone(),
                        file: zip_entry,
                    });
                } else {
                    missing_files += 1;
                }
            }
        }

        let values = case
            .values
            .iter()
            .filter_map(|(field_id, value)| {
                keys.get(field_id.as_str())
                    .map(|key| (key.to_string(), value.clone()))
            })
            .collect();

        let source_row = if package_source.is_some() {
            case_rows_map.get(&case.id).copied()
        } else {
            None
        };

        package_cases.push(PackageCase {
            code: case.code,
            title: case.title,
            status: case.status,
            created_at: case.created_at,
            updated_at: case.updated_at,
            values,
            evidence,
            source_row,
            fixtures: case_fixtures,
        });
    }

    let has_source = package_source.is_some();
    let has_fixtures = package_cases.iter().any(|c| !c.fixtures.is_empty());
    let format_version = if has_source || has_fixtures {
        FORMAT_VERSION
    } else {
        FORMAT_V1
    };

    let doc = PackageDoc {
        format_version,
        project: PackageProject {
            name: project.name.clone(),
            client: project.client.clone(),
            test_type: project.test_type.clone(),
            release_version: project.release_version.clone(),
            start_date: project.start_date.clone(),
            end_date: project.end_date.clone(),
            next_sequence: project.next_sequence,
            created_at: project.created_at.clone(),
            updated_at: project.updated_at.clone(),
            source: package_source,
        },
        fields: project
            .fields
            .iter()
            .map(|f| PackageField {
                key: f.key.clone(),
                label: f.label.clone(),
                field_type: f.field_type,
                options: f.options.clone(),
                required: f.required,
                clear_on_duplicate: f.clear_on_duplicate,
                width: f.width,
                archived: f.archived,
            })
            .collect(),
        cases: package_cases,
        doc_versions: doc_versions::list(conn, project_id)?
            .into_iter()
            .map(|v| PackageDocVersion {
                version: v.version,
                date: v.date,
                author: v.author,
                reviewer: v.reviewer,
                approver: v.approver,
                changes: v.changes,
                created_at: v.created_at,
            })
            .collect(),
    };
    Ok(PackageExport {
        doc,
        images,
        files: package_files,
        missing_images,
        missing_files,
        github: github_json::collect(conn, project_id)?,
    })
}

pub fn project_json(doc: &PackageDoc) -> AppResult<Vec<u8>> {
    let mut out = serde_json::to_vec_pretty(doc)?;
    out.push(b'\n');
    Ok(out)
}

fn zip_error(error: zip::result::ZipError) -> AppError {
    AppError::Io(io::Error::other(error.to_string()))
}

fn write_zip(export: &PackageExport, path: &Path) -> AppResult<()> {
    let mut zip = ZipWriter::new(BufWriter::new(File::create(path)?));
    let json = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    // Los PNG ya vienen comprimidos: guardarlos tal cual es más rápido y no ocupa más.
    let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    zip.start_file(PROJECT_JSON, json).map_err(zip_error)?;
    zip.write_all(&project_json(&export.doc)?)?;
    if let Some(github) = &export.github {
        zip.start_file(GITHUB_JSON, json).map_err(zip_error)?;
        zip.write_all(&github_json::to_bytes(github)?)?;
    }
    for (name, source) in &export.images {
        zip.start_file(name.as_str(), stored).map_err(zip_error)?;
        io::copy(&mut File::open(source)?, &mut zip)?;
    }
    let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    for (name, source) in &export.files {
        zip.start_file(name.as_str(), deflated).map_err(zip_error)?;
        io::copy(&mut File::open(source)?, &mut zip)?;
    }
    zip.finish().map_err(zip_error)?.flush()?;
    Ok(())
}

/// Escribe en un temporal junto al destino y lo renombra: nunca deja un paquete a medias.
pub fn write(export: &PackageExport, dest: &Path) -> AppResult<()> {
    ensure_writable(dest)?;
    let tmp = dest.with_extension(format!("{}.tmp", new_id()));
    let result = write_zip(export, &tmp).and_then(|()| Ok(fs::rename(&tmp, dest)?));
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

/// `T4S_<Cliente>_<Proyecto>_<AAAAMMDD>.qapture` (el cliente se omite si está vacío).
pub fn suggested_file_name(client: &str, name: &str, today: NaiveDate) -> String {
    let mut parts = vec!["T4S".to_string()];
    let client = file_segment(client, 40);
    if !client.is_empty() {
        parts.push(client);
    }
    let name = file_segment(name, 60);
    parts.push(if name.is_empty() {
        "Proyecto".into()
    } else {
        name
    });
    parts.push(today.format("%Y%m%d").to_string());
    format!("{}.{EXTENSION}", parts.join("_"))
}

/// «Guardar como» deja escribir cualquier nombre: el paquete siempre termina en `.qapture`.
pub fn with_package_extension(path: PathBuf) -> PathBuf {
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case(EXTENSION))
    {
        return path;
    }
    let mut raw = path.into_os_string();
    raw.push(".");
    raw.push(EXTENSION);
    PathBuf::from(raw)
}

pub const NOT_PACKAGE: &str = "El archivo no es un paquete de Qapture válido.";
pub const NEWER: &str =
    "El paquete se creó con una versión más nueva de Qapture. Actualiza la app para abrirlo.";
pub const TOO_BIG: &str = "El paquete supera el tamaño permitido.";

const MAX_ENTRIES: usize = 20_001;
const MAX_JSON_BYTES: u64 = 64 * 1024 * 1024;
const MAX_IMAGE_BYTES: u64 = raster::MAX_INPUT_BYTES as u64;
/// Tope de todo lo descomprimido: protege el disco de un ZIP «bomba».
const MAX_TOTAL_BYTES: u64 = 4 * 1024 * 1024 * 1024;
const MAX_FIELDS: usize = 60;
const MAX_CASES: usize = 10_000;
const MAX_EVIDENCE_PER_CASE: usize = 500;
/// La v1 admitía descripciones de 4000 caracteres y se importan completas (Task 3).
const MAX_DESCRIPTION: usize = 4_000;
const MAX_ARCHIVED_VALUE: usize = 16_000;

/// Lee entradas por nombre (nunca extrae rutas del ZIP al disco) con topes por entrada y en total.
struct Reader<R> {
    zip: ZipArchive<R>,
    left: u64,
}

impl<R: Read + Seek> Reader<R> {
    fn has(&mut self, name: &str) -> bool {
        self.zip.by_name(name).is_ok()
    }

    fn read(&mut self, name: &str, max: u64) -> AppResult<Vec<u8>> {
        let entry = self
            .zip
            .by_name(name)
            .map_err(|_| validation(format!("El paquete está dañado: falta «{name}».")))?;
        let limit = max.min(self.left);
        let mut bytes = Vec::new();
        entry.take(limit + 1).read_to_end(&mut bytes).map_err(|_| {
            validation(format!("El paquete está dañado: no se pudo leer «{name}»."))
        })?;
        if bytes.len() as u64 > limit {
            return Err(validation(TOO_BIG));
        }
        self.left -= bytes.len() as u64;
        Ok(bytes)
    }
}

fn read_doc<R: Read + Seek>(reader: &mut Reader<R>) -> AppResult<PackageDoc> {
    if !reader.has(PROJECT_JSON) {
        return Err(validation(NOT_PACKAGE));
    }
    let bytes = reader.read(PROJECT_JSON, MAX_JSON_BYTES)?;
    let value: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|_| validation(NOT_PACKAGE))?;
    match value
        .get("formatVersion")
        .and_then(serde_json::Value::as_i64)
    {
        Some(v) if v > FORMAT_VERSION => return Err(validation(NEWER)),
        Some(v) if v == FORMAT_V1 || v == FORMAT_VERSION => {}
        _ => return Err(validation(NOT_PACKAGE)),
    }
    let doc: PackageDoc = serde_json::from_value(value).map_err(|_| validation(NOT_PACKAGE))?;
    if doc.format_version == FORMAT_V1
        && (doc.project.source.is_some()
            || doc
                .cases
                .iter()
                .any(|c| c.source_row.is_some() || !c.fixtures.is_empty()))
    {
        return Err(validation(NOT_PACKAGE));
    }
    Ok(doc)
}

fn meta_of(p: &PackageProject) -> ProjectMetaInput {
    ProjectMetaInput {
        name: p.name.clone(),
        client: p.client.clone(),
        test_type: p.test_type.clone(),
        release_version: p.release_version.clone(),
        start_date: p.start_date.clone(),
        end_date: p.end_date.clone(),
    }
}

fn version_input(v: &PackageDocVersion) -> DocVersionInput {
    DocVersionInput {
        version: v.version.clone(),
        date: v.date.clone(),
        author: v.author.clone(),
        reviewer: v.reviewer.clone(),
        approver: v.approver.clone(),
        changes: v.changes.clone(),
    }
}

/// `evidence/<archivo>.png`, sin subcarpetas ni `..`.
fn is_image_entry(name: &str) -> bool {
    name.strip_prefix("evidence/").is_some_and(|file| {
        file.len() > 4
            && file.ends_with(".png")
            && !file.starts_with('.')
            && file
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
    })
}

fn is_fixture_entry(file: &str) -> bool {
    file.strip_prefix("fixtures/")
        .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
}

fn is_field_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 60
        && key
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

fn validate_fields(fields: &[PackageField], problems: &mut Problems) {
    if fields.is_empty() || fields.len() > MAX_FIELDS {
        problems.add("Columnas", "El paquete debe tener entre 1 y 60 columnas.");
    }
    let mut keys = HashSet::new();
    for f in fields {
        let label = f.label.trim();
        let at = format!("Columna «{label}»");
        if !is_field_key(&f.key) || !keys.insert(f.key.as_str()) {
            problems.add(&at, "Clave de columna inválida o repetida.");
        }
        if let Err(e) = required_text("Nombre de la columna", &f.label, 100) {
            problems.add(&at, &e.to_string());
        }
        if !(8..=100).contains(&f.width) {
            problems.add(
                &at,
                &format!("El ancho de «{label}» debe estar entre 8 y 100."),
            );
        }
        if f.field_type == FieldType::List && f.options.is_empty() {
            problems.add(
                &at,
                &format!("La lista «{label}» necesita al menos una opción."),
            );
        }
    }
}

fn validate_value_of(doc: &PackageDoc, key: &str, value: &str, at: &str, problems: &mut Problems) {
    let Some(f) = doc.fields.iter().find(|f| f.key == key) else {
        problems.add(at, &format!("La columna «{key}» no existe en el paquete."));
        return;
    };
    if f.archived {
        // Una columna archivada puede guardar valores de un tipo anterior: solo se limita el largo.
        if value.chars().count() > MAX_ARCHIVED_VALUE {
            problems.add(
                at,
                &format!("«{}» supera {MAX_ARCHIVED_VALUE} caracteres.", f.label),
            );
        }
        return;
    }
    let rule = FieldRule {
        label: &f.label,
        field_type: f.field_type,
        options: &f.options,
        required: false,
    };
    if let Err(message) = validate_value(&rule, value) {
        problems.add(at, &message);
    }
}

fn validate_cases(doc: &PackageDoc, problems: &mut Problems) {
    if doc.cases.len() > MAX_CASES {
        problems.add("Casos", "Demasiados casos.");
    }
    let mut codes = HashSet::new();
    let mut images = HashSet::new();
    let mut fixture_files = HashSet::new();
    for (index, c) in doc.cases.iter().enumerate() {
        let valid_code = if doc.format_version == FORMAT_V1 {
            is_case_code(&c.code)
        } else {
            !c.code.is_empty() && c.code == c.code.trim() && c.code.chars().count() <= 40
        };
        let at = if valid_code {
            c.code.clone()
        } else {
            format!("Caso {}", index + 1)
        };
        if !valid_code || !codes.insert(c.code.as_str()) {
            problems.add(&at, "ID de ejecución repetido o inválido.");
        } else if doc.format_version == FORMAT_V1
            && code_number(&c.code).is_none_or(|n| n >= doc.project.next_sequence)
        {
            problems.add(&at, "La secuencia debe superar los IDs usados.");
        }
        if let Err(e) = required_text("Caso de prueba", &c.title, 300) {
            problems.add(&at, &e.to_string());
        }
        if !is_status(&c.status) {
            problems.add(&at, &format!("Estado inválido: «{}».", c.status));
        }
        for (key, value) in &c.values {
            validate_value_of(doc, key, value, &at, problems);
        }
        if c.evidence.len() > MAX_EVIDENCE_PER_CASE {
            problems.add(&at, "Demasiadas capturas por caso.");
        }
        for (i, e) in c.evidence.iter().enumerate() {
            let at = format!("{at}, captura {}", i + 1);
            if let Err(err) = required_text("Descripción", &e.description, MAX_DESCRIPTION) {
                problems.add(&at, &err.to_string());
            }
            if let Err(err) = evidence::validate_annotations(&e.annotations) {
                problems.add(&at, &err.to_string());
            }
            for name in [&e.original, &e.rendered] {
                if !is_image_entry(name) || !images.insert(name.as_str()) {
                    problems.add(&at, "Ruta de imagen inválida o repetida.");
                }
            }
        }
        if c.fixtures.len() > 50 {
            problems.add(&at, "El caso ya tiene 50 archivos de prueba.");
        }
        let mut case_fixture_names = HashSet::new();
        for f in &c.fixtures {
            let clean_res = fixtures::clean_name(&f.file_name);
            match clean_res {
                Err(e) => problems.add(&at, &e.to_string()),
                Ok(clean) => {
                    if !case_fixture_names.insert(clean.to_lowercase()) {
                        problems.add(
                            &at,
                            &format!("El caso ya tiene un archivo llamado «{clean}»."),
                        );
                    }
                }
            }
            if let Err(e) = fixtures::check_size(f.size as u64) {
                problems.add(&at, &e.to_string());
            }
            if f.note.chars().count() > 500 {
                problems.add(&at, "La nota supera 500 caracteres.");
            }
            if f.sha256.len() != 64
                || !f
                    .sha256
                    .chars()
                    .all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase())
            {
                problems.add(&at, "Hash sha256 inválido.");
            }
            if !is_fixture_entry(&f.file) || !fixture_files.insert(f.file.as_str()) {
                problems.add(&at, "Ruta de archivo de prueba inválida o repetida.");
            }
        }
    }
}

fn validate_source(source: &PackageSource, problems: &mut Problems) {
    if let Err(e) = required_text("Nombre del archivo de origen", &source.file_name, 200) {
        problems.add("Proyecto", &e.to_string());
    }
    if source.sheet.trim().is_empty() {
        problems.add("Proyecto", "El nombre de la hoja no puede estar vacío.");
    }
    if source.sha256.len() != 64
        || !source
            .sha256
            .chars()
            .all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase())
    {
        problems.add("Proyecto", "Hash sha256 inválido.");
    }
    if source
        .file
        .as_deref()
        .is_some_and(|f| f != format!("source/{GUION_FILE}"))
    {
        problems.add("Proyecto", "Ruta de archivo de guion inválida.");
    }
    for p in profile::problems(&source.definition) {
        problems.add("Perfil de guion", &p);
    }
}

fn validate_versions(versions: &[PackageDocVersion], problems: &mut Problems) {
    let mut seen = HashSet::new();
    for v in versions {
        let at = format!("Versión «{}»", v.version.trim());
        match doc_versions::validate(&version_input(v)) {
            Err(e) => problems.add(&at, &e.to_string()),
            Ok(clean) if !seen.insert(clean.version.to_lowercase()) => problems.add(
                &at,
                &format!("La versión «{}» ya existe en este proyecto.", clean.version),
            ),
            Ok(_) => {}
        }
    }
}

fn validate(doc: &PackageDoc) -> AppResult<()> {
    let mut problems = Problems::default();
    if let Err(e) = projects::validate_meta(&meta_of(&doc.project)) {
        problems.add("Proyecto", &e.to_string());
    }
    if doc.project.next_sequence < 1 {
        problems.add("Proyecto", "Secuencia inválida.");
    }
    if let Some(source) = &doc.project.source {
        validate_source(source, &mut problems);
    }
    validate_fields(&doc.fields, &mut problems);
    validate_cases(doc, &mut problems);
    validate_versions(&doc.doc_versions, &mut problems);
    problems.into_result()
}

fn bad_image(at: &str) -> AppError {
    validation(format!(
        "{IMPORT_FAILED}\n• {at}: Contenido de imagen inválido."
    ))
}

fn to_bundle<R: Read + Seek>(
    doc: PackageDoc,
    reader: &mut Reader<R>,
    out: &mut ImportFiles,
    github: GithubPayload,
) -> AppResult<ProjectBundle> {
    let id = new_id();
    let meta = projects::validate_meta(&meta_of(&doc.project))?;

    let bundle_source = if let Some(source) = doc.project.source {
        let stored_path = format!("evidence/{id}/{GUION_FILE}");
        if let Some(entry_path) = &source.file {
            let bytes = reader.read(entry_path, MAX_GUION_BYTES)?;
            if fixtures::sha256_hex(&bytes) != source.sha256 {
                return Err(validation(format!(
                    "El paquete está dañado: «{entry_path}» no coincide con su registro."
                )));
            }
            out.write_file(&id, GUION_FILE, &bytes)?;
        }
        Some(BundleSource {
            file_name: source.file_name,
            stored_path,
            sha256: source.sha256,
            sheet: source.sheet,
            header_row: source.header_row,
            profile_id: None,
            definition: serde_json::to_string(&source.definition)?,
            imported_at: Some(source.imported_at),
        })
    } else {
        None
    };

    let mut bundle_cases = Vec::with_capacity(doc.cases.len());
    for c in doc.cases {
        let mut evidence = Vec::with_capacity(c.evidence.len());
        for (index, e) in c.evidence.into_iter().enumerate() {
            let at = format!("{}, captura {}", c.code, index + 1);
            let original = reader.read(&e.original, MAX_IMAGE_BYTES)?;
            let rendered = reader.read(&e.rendered, MAX_IMAGE_BYTES)?;
            let (width, height) = raster::verify_png(&original).map_err(|_| bad_image(&at))?;
            raster::verify_png(&rendered).map_err(|_| bad_image(&at))?;
            let evidence_id = new_id();
            out.write(&id, &evidence_id, &original, &rendered)?;
            evidence.push(BundleEvidence {
                id: evidence_id,
                description: e.description.trim().to_string(),
                annotations: e.annotations,
                width: width.into(),
                height: height.into(),
                captured_at: e.captured_at,
                updated_at: e.updated_at,
            });
        }

        let mut bundle_fixtures = Vec::with_capacity(c.fixtures.len());
        for f in c.fixtures {
            let bytes = reader.read(&f.file, MAX_FIXTURE_BYTES)?;
            if bytes.len() as i64 != f.size || fixtures::sha256_hex(&bytes) != f.sha256 {
                return Err(validation(format!(
                    "El paquete está dañado: «{}» no coincide con su registro.",
                    f.file
                )));
            }
            let fixture_id = new_id();
            let stored_path = out.write_fixture(&id, &fixture_id, &f.file_name, &bytes)?;
            bundle_fixtures.push(BundleFixture {
                id: fixture_id,
                file_name: f.file_name,
                stored_path,
                size: f.size,
                sha256: f.sha256,
                note: f.note,
                added_at: f.added_at,
            });
        }

        bundle_cases.push(BundleCase {
            source_row: c.source_row,
            code: c.code,
            title: c.title.trim().to_string(),
            status: c.status,
            created_at: c.created_at,
            updated_at: c.updated_at,
            values: c.values,
            evidence,
            fixtures: bundle_fixtures,
        });
    }
    Ok(ProjectBundle {
        source: bundle_source,
        id,
        meta,
        next_sequence: doc.project.next_sequence,
        source_template_id: None,
        created_at: doc.project.created_at,
        updated_at: doc.project.updated_at,
        fields: doc
            .fields
            .into_iter()
            .map(|f| BundleField {
                key: f.key,
                label: f.label.trim().to_string(),
                field_type: f.field_type,
                options: f.options,
                required: f.required,
                clear_on_duplicate: f.clear_on_duplicate,
                width: f.width,
                archived: f.archived,
            })
            .collect(),
        cases: bundle_cases,
        versions: doc
            .doc_versions
            .iter()
            .map(|v| BundleVersion {
                input: version_input(v),
                created_at: v.created_at.clone(),
            })
            .collect(),
        github,
    })
}

/// `github.json` es opcional y nunca bloquea la importación: ilegible, demasiado grande o
/// inválido se ignora entero (el llamador avisa).
fn read_github<R: Read + Seek>(reader: &mut Reader<R>) -> GithubPayload {
    if !reader.has(GITHUB_JSON) {
        return GithubPayload::Absent;
    }
    match reader
        .read(GITHUB_JSON, MAX_JSON_BYTES)
        .ok()
        .and_then(|bytes| GithubJson::parse(&bytes))
    {
        Some(json) => GithubPayload::Valid(json),
        None => GithubPayload::Invalid,
    }
}

/// Parte de UC-18 que no necesita la base: lee, valida y escribe las capturas.
pub fn prepare<'a>(
    files: &'a EvidenceFiles,
    path: &Path,
) -> AppResult<(ProjectBundle, ImportFiles<'a>)> {
    let zip = ZipArchive::new(File::open(path)?).map_err(|_| validation(NOT_PACKAGE))?;
    if zip.len() > MAX_ENTRIES {
        return Err(validation(TOO_BIG));
    }
    let mut reader = Reader {
        zip,
        left: MAX_TOTAL_BYTES,
    };
    let doc = read_doc(&mut reader)?;
    validate(&doc)?;
    let mut out = ImportFiles::new(files);
    let github = read_github(&mut reader);
    let bundle = to_bundle(doc, &mut reader, &mut out, github)?;
    Ok((bundle, out))
}

/// UC-18: importa el paquete como un proyecto nuevo (nunca sobrescribe). Todo o nada.
#[cfg(test)]
pub fn import(
    conn: &Connection,
    files: &EvidenceFiles,
    path: &Path,
) -> AppResult<ImportSummaryDto> {
    let (bundle, out) = prepare(files, path)?;
    bundle::insert(conn, vec![bundle], out)
}

#[cfg(test)]
pub(crate) mod tests {
    use std::{collections::BTreeMap, io::Read};

    use zip::ZipArchive;

    use super::*;
    use crate::{
        dto::{CaseInput, FieldInput, NewEvidenceMeta, ProjectDto},
        media::{raster::sample_png, storage::TempDir},
        repo::{evidence, fields, test_support},
        transfer::bundle::EMPTY_ANNOTATIONS,
    };

    /// «Recibos» con una columna de lista, una archivada con datos, 2 casos (uno duplicado),
    /// 2 capturas en EJ-001 y una versión del documento.
    pub(crate) fn seeded(conn: &Connection, files: &EvidenceFiles) -> ProjectDto {
        let p = test_support::project(conn);
        let mut inputs: Vec<FieldInput> = p
            .fields
            .iter()
            .map(|f| FieldInput {
                id: Some(f.id.clone()),
                label: f.label.clone(),
                field_type: f.field_type,
                options: f.options.clone(),
                required: f.required,
                clear_on_duplicate: f.clear_on_duplicate,
                width: f.width,
                archived: false,
            })
            .collect();
        inputs.push(FieldInput {
            id: None,
            label: "Severidad".into(),
            field_type: FieldType::List,
            options: vec!["Alta".into(), "Baja".into()],
            required: false,
            clear_on_duplicate: false,
            width: 14,
            archived: false,
        });
        fields::save(conn, &p.id, inputs).unwrap();
        let p = projects::get(conn, &p.id).unwrap();
        let id = |key: &str| test_support::field_id(&p, key);
        let first = cases::create(
            conn,
            &p.id,
            &CaseInput {
                title: "Aplicar recibo".into(),
                status: "Aprobado".into(),
                values: BTreeMap::from([
                    (id("entorno"), "QA".into()),
                    (id("datos_entrada"), "Recibo: 1000001\nSucursal: 2".into()),
                    (id("severidad"), "Alta".into()),
                    (id("observaciones"), "Revisar con el líder.".into()),
                ]),
            },
        )
        .unwrap();
        cases::duplicate(conn, &first.id).unwrap();
        for description in ["Pantalla inicial", "Recibo aplicado"] {
            let staged = evidence::stage(files, &sample_png(320, 200)).unwrap();
            let meta = NewEvidenceMeta {
                case_id: first.id.clone(),
                staged_path: staged.path,
                description: description.into(),
                annotations: EMPTY_ANNOTATIONS.into(),
            };
            evidence::add(conn, files, &meta, &sample_png(320, 200)).unwrap();
        }
        conn.execute(
            "UPDATE project_field SET archived = 1 WHERE project_id = ?1 AND key = 'observaciones'",
            [&p.id],
        )
        .unwrap();
        doc_versions::create(
            conn,
            &p.id,
            &doc_versions::tests::input("1.0", "2026-10-16"),
        )
        .unwrap();
        projects::get(conn, &p.id).unwrap()
    }

    pub(crate) fn entry_names(path: &Path) -> Vec<String> {
        let zip = ZipArchive::new(File::open(path).unwrap()).unwrap();
        zip.file_names()
            .map(|name| name.unwrap().into_owned())
            .collect()
    }

    #[test]
    fn collect_describes_the_project_without_ids() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let conn = test_support::conn();
        let p = seeded(&conn, &files);
        let export = collect(&conn, &files, &p.id).unwrap();
        let doc = &export.doc;

        assert_eq!(doc.format_version, 1);
        assert_eq!(
            (doc.project.name.as_str(), doc.project.client.as_str()),
            ("Recibos", "ACME")
        );
        assert_eq!(doc.project.next_sequence, 3);
        // 11 de la plantilla Funcional + «Severidad»; «Observaciones» quedó archivada.
        assert_eq!(doc.fields.len(), 12);
        assert_eq!(doc.fields[10].key, "severidad");
        assert_eq!(doc.fields[10].options, ["Alta", "Baja"]);
        assert!(doc.fields[11].archived, "las archivadas van al final");
        assert_eq!(doc.fields[11].key, "observaciones");
        let codes: Vec<&str> = doc.cases.iter().map(|c| c.code.as_str()).collect();
        assert_eq!(codes, ["EJ-001", "EJ-002"]);
        let first = &doc.cases[0];
        assert_eq!(
            first.values["datos_entrada"],
            "Recibo: 1000001\nSucursal: 2"
        );
        assert_eq!(first.values["observaciones"], "Revisar con el líder.");
        assert_eq!(first.evidence.len(), 2);
        assert_eq!(first.evidence[1].description, "Recibo aplicado");
        assert_eq!(first.evidence[0].original, "evidence/EJ-001-1.orig.png");
        assert_eq!(first.evidence[1].rendered, "evidence/EJ-001-2.render.png");
        assert_eq!(
            (first.evidence[0].width, first.evidence[0].height),
            (320, 200)
        );
        assert_eq!(doc.doc_versions[0].version, "1.0");
        assert_eq!(export.images.len(), 4);
        assert_eq!((export.evidence_count(), export.missing_images), (2, 0));

        let json = String::from_utf8(project_json(doc).unwrap()).unwrap();
        assert!(json.ends_with("}\n"));
        assert!(!json.contains(&p.id), "sin id del proyecto");
        for f in &p.fields {
            assert!(!json.contains(&f.id), "sin ids de columnas");
        }
        assert!(!json.contains("sourceTemplateId"));
    }

    #[test]
    fn write_creates_the_zip_with_project_json_and_stored_images() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let conn = test_support::conn();
        let p = seeded(&conn, &files);
        let export = collect(&conn, &files, &p.id).unwrap();
        let dest = tmp.path().join("Recibos.qapture");
        write(&export, &dest).unwrap();

        assert_eq!(
            entry_names(&dest)
                .into_iter()
                .collect::<std::collections::BTreeSet<_>>(),
            [
                "project.json",
                "evidence/EJ-001-1.orig.png",
                "evidence/EJ-001-1.render.png",
                "evidence/EJ-001-2.orig.png",
                "evidence/EJ-001-2.render.png",
            ]
            .into_iter()
            .map(str::to_string)
            .collect()
        );
        let mut zip = ZipArchive::new(File::open(&dest).unwrap()).unwrap();
        let mut json = Vec::new();
        zip.by_name(PROJECT_JSON)
            .unwrap()
            .read_to_end(&mut json)
            .unwrap();
        assert_eq!(json, project_json(&export.doc).unwrap());
        for (name, source) in &export.images {
            let mut entry = zip.by_name(name).unwrap();
            assert_eq!(
                entry.compression(),
                zip::CompressionMethod::Stored,
                "{name}"
            );
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).unwrap();
            assert_eq!(bytes, fs::read(source).unwrap(), "{name}");
        }
        let leftovers = fs::read_dir(tmp.path())
            .unwrap()
            .filter(|e| {
                e.as_ref()
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .ends_with(".tmp")
            })
            .count();
        assert_eq!(leftovers, 0);
        write(&export, &dest).unwrap();
        assert_eq!(
            entry_names(&dest).len(),
            5,
            "sobrescribir deja un zip válido"
        );
    }

    #[test]
    fn evidence_without_an_image_is_skipped_and_counted() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let conn = test_support::conn();
        let p = seeded(&conn, &files);
        let case_id = cases::list(&conn, &p.id).unwrap()[0].id.clone();
        let first = evidence::list(&conn, &files, &case_id).unwrap().remove(0);
        fs::remove_file(&first.rendered_path).unwrap();

        let export = collect(&conn, &files, &p.id).unwrap();
        assert_eq!((export.evidence_count(), export.missing_images), (1, 1));
        let kept = &export.doc.cases[0].evidence[0];
        assert_eq!(kept.description, "Recibo aplicado");
        assert_eq!(kept.original, "evidence/EJ-001-1.orig.png");
    }

    #[test]
    fn suggested_name_and_extension() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 16).unwrap();
        assert_eq!(
            suggested_file_name("ACME", "Recibos / Q4", today),
            "T4S_ACME_Recibos-Q4_20261016.qapture"
        );
        assert_eq!(
            suggested_file_name("", "Pólizas", today),
            "T4S_Pólizas_20261016.qapture"
        );
        assert_eq!(
            suggested_file_name("", "///", today),
            "T4S_Proyecto_20261016.qapture"
        );
        assert_eq!(
            with_package_extension(PathBuf::from("a")),
            PathBuf::from("a.qapture")
        );
        assert_eq!(
            with_package_extension(PathBuf::from("a.QAPTURE")),
            PathBuf::from("a.QAPTURE")
        );
        assert_eq!(
            with_package_extension(PathBuf::from("a.zip")),
            PathBuf::from("a.zip.qapture")
        );
    }

    use serde_json::json;
    use zip::{ZipWriter, write::SimpleFileOptions};

    use crate::transfer::bundle::{IMPORT_FAILED, tests::project_dirs};

    fn exported(conn: &Connection, files: &EvidenceFiles, dir: &Path) -> (ProjectDto, PathBuf) {
        let p = seeded(conn, files);
        let path = dir.join("Recibos.qapture");
        write(&collect(conn, files, &p.id).unwrap(), &path).unwrap();
        (p, path)
    }

    /// Copia el ZIP cambiando entradas: `None` la quita, `Some(bytes)` la reemplaza.
    fn rewrite(src: &Path, dest: &Path, change: impl Fn(&str, Vec<u8>) -> Option<Vec<u8>>) {
        let mut input = ZipArchive::new(File::open(src).unwrap()).unwrap();
        let mut output = ZipWriter::new(File::create(dest).unwrap());
        for index in 0..input.len() {
            let mut entry = input.by_index(index).unwrap();
            let name = entry.name().unwrap().into_owned();
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).unwrap();
            drop(entry);
            if let Some(bytes) = change(&name, bytes) {
                output
                    .start_file(name, SimpleFileOptions::default())
                    .unwrap();
                output.write_all(&bytes).unwrap();
            }
        }
        output.finish().unwrap();
    }

    fn with_json(src: &Path, dest: &Path, edit: impl Fn(&mut serde_json::Value)) {
        rewrite(src, dest, |name, bytes| {
            if name != PROJECT_JSON {
                return Some(bytes);
            }
            let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            edit(&mut value);
            Some(serde_json::to_vec(&value).unwrap())
        });
    }

    #[test]
    fn export_import_export_gives_the_same_project_json() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let conn = test_support::conn();
        let (p, path) = exported(&conn, &files, tmp.path());
        let first = collect(&conn, &files, &p.id).unwrap();

        let other = TempDir::new();
        let other_files = other.files();
        let other_conn = test_support::conn();
        let summary = import(&other_conn, &other_files, &path).unwrap();
        assert_eq!(summary.projects[0].name, "Recibos");
        assert_eq!((summary.cases, summary.evidence), (2, 2));
        let imported = projects::get(&other_conn, &summary.projects[0].id).unwrap();
        assert_ne!(imported.id, p.id, "es una copia con ids nuevos");
        assert_eq!(imported.source_template_id, None);

        let second = collect(&other_conn, &other_files, &imported.id).unwrap();
        assert_eq!(
            project_json(&first.doc).unwrap(),
            project_json(&second.doc).unwrap()
        );
        for ((name_a, file_a), (name_b, file_b)) in first.images.iter().zip(&second.images) {
            assert_eq!(name_a, name_b);
            assert_eq!(
                fs::read(file_a).unwrap(),
                fs::read(file_b).unwrap(),
                "{name_a}"
            );
        }
        let again = other.path().join("again.qapture");
        write(&second, &again).unwrap();
        let mut zip = ZipArchive::new(File::open(&again).unwrap()).unwrap();
        let mut json = Vec::new();
        zip.by_name(PROJECT_JSON)
            .unwrap()
            .read_to_end(&mut json)
            .unwrap();
        assert_eq!(
            json,
            project_json(&first.doc).unwrap(),
            "el project.json releído es idéntico"
        );
    }

    #[test]
    fn importing_the_same_package_twice_creates_copies() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let conn = test_support::conn();
        let (p, path) = exported(&conn, &files, tmp.path());
        let a = import(&conn, &files, &path).unwrap();
        let b = import(&conn, &files, &path).unwrap();
        assert_eq!(a.projects[0].name, "Recibos (importado)");
        assert_eq!(b.projects[0].name, "Recibos (importado 2)");
        assert_eq!(projects::list(&conn).unwrap().len(), 3);
        assert_eq!(
            cases::list(&conn, &p.id).unwrap().len(),
            2,
            "el original no cambia"
        );
    }

    #[test]
    fn foreign_newer_or_damaged_packages_import_nothing() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let conn = test_support::conn();
        let (_, good) = exported(&conn, &files, tmp.path());
        let bad = tmp.path().join("bad.qapture");
        let not_zip = tmp.path().join("hola.qapture");
        fs::write(&not_zip, b"hola").unwrap();

        type Prepare = Box<dyn Fn(&Path, &Path)>;
        let cases: Vec<(Prepare, String)> = vec![
            (
                Box::new(|_: &Path, dest: &Path| {
                    fs::copy(dest.with_file_name("hola.qapture"), dest)
                        .map(|_| ())
                        .unwrap()
                }),
                NOT_PACKAGE.to_string(),
            ),
            (
                Box::new(|src: &Path, dest: &Path| {
                    rewrite(src, dest, |name, b| (name != PROJECT_JSON).then_some(b))
                }),
                NOT_PACKAGE.to_string(),
            ),
            (
                Box::new(|src: &Path, dest: &Path| {
                    with_json(src, dest, |v| v["formatVersion"] = json!(3))
                }),
                NEWER.to_string(),
            ),
            (
                Box::new(|src: &Path, dest: &Path| {
                    with_json(src, dest, |v| v["formatVersion"] = json!("1"))
                }),
                NOT_PACKAGE.to_string(),
            ),
            (
                Box::new(|src: &Path, dest: &Path| {
                    with_json(src, dest, |v| v["extra"] = json!(true))
                }),
                NOT_PACKAGE.to_string(),
            ),
            (
                Box::new(|src: &Path, dest: &Path| {
                    rewrite(src, dest, |name, b| {
                        (name != "evidence/EJ-001-2.orig.png").then_some(b)
                    })
                }),
                "El paquete está dañado: falta «evidence/EJ-001-2.orig.png».".to_string(),
            ),
            (
                Box::new(|src: &Path, dest: &Path| {
                    rewrite(src, dest, |name, b| {
                        Some(if name == "evidence/EJ-001-2.render.png" {
                            b"no es png".to_vec()
                        } else {
                            b
                        })
                    })
                }),
                format!("{IMPORT_FAILED}\n• EJ-001, captura 2: Contenido de imagen inválido."),
            ),
            (
                Box::new(|src: &Path, dest: &Path| {
                    with_json(src, dest, |v| {
                        v["cases"][0]["evidence"][0]["original"] = json!("../../qapture.db")
                    })
                }),
                format!(
                    "{IMPORT_FAILED}\n• EJ-001, captura 1: Ruta de imagen inválida o repetida."
                ),
            ),
            (
                Box::new(|src: &Path, dest: &Path| {
                    with_json(src, dest, |v| v["cases"][1]["status"] = json!("Cerrado"))
                }),
                format!("{IMPORT_FAILED}\n• EJ-002: Estado inválido: «Cerrado»."),
            ),
            (
                Box::new(|src: &Path, dest: &Path| {
                    with_json(src, dest, |v| v["cases"][1]["code"] = json!("EJ-001"))
                }),
                format!("{IMPORT_FAILED}\n• EJ-001: ID de ejecución repetido o inválido."),
            ),
            (
                Box::new(|src: &Path, dest: &Path| {
                    with_json(src, dest, |v| {
                        v["cases"][0]["values"]["severidad"] = json!("Media")
                    })
                }),
                format!(
                    "{IMPORT_FAILED}\n• EJ-001: «Severidad» debe ser una de las opciones de la lista."
                ),
            ),
            (
                Box::new(|src: &Path, dest: &Path| {
                    with_json(src, dest, |v| {
                        v["cases"][0]["values"]["no_existe"] = json!("x")
                    })
                }),
                format!(
                    "{IMPORT_FAILED}\n• EJ-001: La columna «no_existe» no existe en el paquete."
                ),
            ),
        ];
        for (prepare, expected) in cases {
            prepare(&good, &bad);
            let other = TempDir::new();
            let other_files = other.files();
            let other_conn = test_support::conn();
            let err = import(&other_conn, &other_files, &bad)
                .unwrap_err()
                .to_string();
            assert_eq!(err, expected);
            assert!(
                projects::list(&other_conn).unwrap().is_empty(),
                "{expected}"
            );
            assert_eq!(project_dirs(&other_files), 0, "{expected}");
        }
    }

    #[test]
    fn an_entry_that_inflates_beyond_the_limit_is_rejected() {
        let tmp = TempDir::new();
        let path = tmp.path().join("bomba.qapture");
        let mut zip = ZipWriter::new(File::create(&path).unwrap());
        let deflated =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        zip.start_file(PROJECT_JSON, deflated).unwrap();
        zip.write_all(&vec![b' '; 65 * 1024 * 1024]).unwrap();
        zip.finish().unwrap();
        assert!(
            fs::metadata(&path).unwrap().len() < 1024 * 1024,
            "comprimido pesa poco"
        );
        let conn = test_support::conn();
        assert_eq!(
            import(&conn, &tmp.files(), &path).unwrap_err().to_string(),
            TOO_BIG
        );
    }

    // ---- F8b: github.json ----

    use crate::repo::integrations::{
        CaseIssueLink, LinkedRepo, ProjectGithub, RepoRef, get_project_github, insert_link,
        links_for_project, save_project_github,
    };

    fn repo(text: &str) -> RepoRef {
        RepoRef::parse(text).unwrap()
    }

    fn link(case_id: &str, number: u64) -> CaseIssueLink {
        CaseIssueLink {
            case_id: case_id.into(),
            owner: "o".into(),
            repo: "r".into(),
            issue_number: number,
            node_id: "N1".into(),
            html_url: format!("https://github.com/o/r/issues/{number}"),
            direction: "exported".into(),
            issue_state: "open".into(),
        }
    }

    /// Proyecto sembrado con repos, ruteo por «severidad», repo de evidencias y un vínculo.
    fn with_github(conn: &Connection, files: &EvidenceFiles) -> ProjectDto {
        let p = seeded(conn, files);
        let field = test_support::field_id(&p, "severidad");
        save_project_github(
            conn,
            &p.id,
            &ProjectGithub {
                repos: vec![
                    LinkedRepo {
                        repo: repo("o/r"),
                        is_default: true,
                        default_labels: vec!["bug".into()],
                    },
                    LinkedRepo {
                        repo: repo("o/otro"),
                        is_default: false,
                        default_labels: vec![],
                    },
                ],
                routing_field_id: Some(field),
                routing_map: BTreeMap::from([("Alta".to_string(), "o/otro".to_string())]),
                evidence_repo: Some(repo("o/evidencias")),
            },
        )
        .unwrap();
        let first = cases::list(conn, &p.id).unwrap().remove(0);
        insert_link(conn, &link(&first.id, 45)).unwrap();
        p
    }

    fn zip_json(path: &Path, name: &str) -> Option<Vec<u8>> {
        let mut zip = ZipArchive::new(File::open(path).unwrap()).unwrap();
        let mut bytes = Vec::new();
        zip.by_name(name).ok()?.read_to_end(&mut bytes).unwrap();
        Some(bytes)
    }

    fn with_github_json(src: &Path, dest: &Path, edit: impl Fn(&mut serde_json::Value)) {
        rewrite(src, dest, |name, bytes| {
            if name != GITHUB_JSON {
                return Some(bytes);
            }
            let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            edit(&mut value);
            Some(serde_json::to_vec(&value).unwrap())
        });
    }

    #[test]
    fn export_writes_github_json_only_when_there_is_github_data() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let conn = test_support::conn();
        let (_, plain) = exported(&conn, &files, tmp.path());
        assert!(!entry_names(&plain).contains(&GITHUB_JSON.to_string()));

        let p = with_github(&conn, &files);
        let path = tmp.path().join("gh.qapture");
        write(&collect(&conn, &files, &p.id).unwrap(), &path).unwrap();
        assert!(entry_names(&path).contains(&GITHUB_JSON.to_string()));
        let json: serde_json::Value =
            serde_json::from_slice(&zip_json(&path, GITHUB_JSON).unwrap()).unwrap();
        assert_eq!(json["version"], 1);
        assert_eq!(json["routingFieldKey"], "severidad");
        assert_eq!(json["evidenceRepo"], "o/evidencias");
        assert_eq!(json["repos"][0]["fullName"], "o/r");
        assert_eq!(json["links"][0]["caseCode"], "EJ-001");
        assert_eq!(json["links"][0]["issueNumber"], 45);
    }

    #[test]
    fn round_trip_keeps_repos_routing_evidence_repo_and_links() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let conn = test_support::conn();
        let p = with_github(&conn, &files);
        let path = tmp.path().join("gh.qapture");
        write(&collect(&conn, &files, &p.id).unwrap(), &path).unwrap();

        let summary = import(&conn, &files, &path).unwrap();
        assert_eq!(
            summary.warnings,
            vec![github_json::EVIDENCE_REPO_WARNING.to_string()]
        );
        let new = projects::get(&conn, &summary.projects[0].id).unwrap();
        let config = get_project_github(&conn, &new.id).unwrap();
        let original = get_project_github(&conn, &p.id).unwrap();
        assert_eq!(config.repos, original.repos);
        assert_eq!(original.evidence_repo, Some(repo("o/evidencias")));
        assert_eq!(
            config.evidence_repo, None,
            "no se restaura desde el paquete"
        );
        assert_eq!(config.routing_map, original.routing_map);
        assert_eq!(
            config.routing_field_id,
            Some(test_support::field_id(&new, "severidad")),
            "la columna de ruteo se resuelve por key a su id nuevo"
        );
        let links = links_for_project(&conn, &new.id).unwrap();
        assert_eq!(links.len(), 1);
        let case = cases::list(&conn, &new.id)
            .unwrap()
            .into_iter()
            .find(|c| c.code == "EJ-001")
            .unwrap();
        assert_eq!(links[0], link(&case.id, 45));
    }

    #[test]
    fn links_to_unknown_case_codes_are_dropped() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let conn = test_support::conn();
        let p = with_github(&conn, &files);
        let path = tmp.path().join("gh.qapture");
        write(&collect(&conn, &files, &p.id).unwrap(), &path).unwrap();
        let edited = tmp.path().join("edited.qapture");
        with_github_json(&path, &edited, |v| {
            let mut extra = v["links"][0].clone();
            extra["caseCode"] = json!("EJ-999");
            extra["issueNumber"] = json!(99);
            extra["htmlUrl"] = json!("https://github.com/o/r/issues/99");
            v["links"].as_array_mut().unwrap().push(extra);
        });

        let summary = import(&conn, &files, &edited).unwrap();
        assert_eq!(
            summary.warnings,
            vec![github_json::EVIDENCE_REPO_WARNING.to_string()],
            "descartar vínculos no avisa; solo el repo de evidencias"
        );
        let links = links_for_project(&conn, &summary.projects[0].id).unwrap();
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].issue_number, 45);
    }

    #[test]
    fn a_broken_github_json_imports_without_links() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let conn = test_support::conn();
        let p = with_github(&conn, &files);
        let path = tmp.path().join("gh.qapture");
        write(&collect(&conn, &files, &p.id).unwrap(), &path).unwrap();

        let garbage = tmp.path().join("garbage.qapture");
        rewrite(&path, &garbage, |name, b| {
            Some(if name == GITHUB_JSON {
                b"{no es json".to_vec()
            } else {
                b
            })
        });
        let bad_repo = tmp.path().join("bad-repo.qapture");
        with_github_json(&path, &bad_repo, |v| {
            v["repos"][0]["fullName"] = json!("sin-barra")
        });
        let newer = tmp.path().join("newer.qapture");
        with_github_json(&path, &newer, |v| v["version"] = json!(2));

        for broken in [garbage, bad_repo, newer] {
            let summary = import(&conn, &files, &broken).unwrap();
            assert_eq!(
                summary.warnings,
                vec![github_json::GITHUB_WARNING.to_string()]
            );
            assert_eq!(summary.cases, 2, "el proyecto se importó");
            let id = &summary.projects[0].id;
            assert!(links_for_project(&conn, id).unwrap().is_empty());
            assert_eq!(
                get_project_github(&conn, id).unwrap(),
                ProjectGithub::default()
            );
        }
    }

    /// Un paquete ajeno no decide qué abre Qapture: la `htmlUrl` debe ser la del issue en GitHub.
    #[test]
    fn a_package_with_a_foreign_issue_url_imports_without_links() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let conn = test_support::conn();
        let p = with_github(&conn, &files);
        let path = tmp.path().join("gh.qapture");
        write(&collect(&conn, &files, &p.id).unwrap(), &path).unwrap();

        let edits: [(&str, serde_json::Value, serde_json::Value); 5] = [
            ("file", json!("file:///C:/x.exe"), json!(45)),
            (
                "host",
                json!("https://evil.example/o/r/issues/45"),
                json!(45),
            ),
            (
                "repo",
                json!("https://github.com/o/otro/issues/45"),
                json!(45),
            ),
            (
                "number",
                json!("https://github.com/o/r/issues/46"),
                json!(45),
            ),
            (
                "huge",
                json!(format!("https://github.com/o/r/issues/{}", u64::MAX)),
                json!(u64::MAX),
            ),
        ];
        for (name, url, number) in edits {
            let broken = tmp.path().join(format!("{name}.qapture"));
            with_github_json(&path, &broken, |v| {
                v["links"][0]["htmlUrl"] = url.clone();
                v["links"][0]["issueNumber"] = number.clone();
            });
            let summary = import(&conn, &files, &broken).unwrap();
            assert_eq!(
                summary.warnings,
                vec![github_json::GITHUB_WARNING.to_string()],
                "{name}"
            );
            let id = &summary.projects[0].id;
            assert!(links_for_project(&conn, id).unwrap().is_empty(), "{name}");
        }

        // Mayúsculas distintas en dueño/repo siguen siendo el mismo issue.
        let cased = tmp.path().join("cased.qapture");
        with_github_json(&path, &cased, |v| {
            v["links"][0]["htmlUrl"] = json!("https://github.com/O/R/issues/45");
        });
        let summary = import(&conn, &files, &cased).unwrap();
        assert_eq!(
            summary.warnings,
            vec![github_json::EVIDENCE_REPO_WARNING.to_string()]
        );
        assert_eq!(
            links_for_project(&conn, &summary.projects[0].id)
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn a_failure_while_applying_leaves_no_github_rows_and_warns() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let conn = test_support::conn();
        let p = with_github(&conn, &files);
        let path = tmp.path().join("gh.qapture");
        write(&collect(&conn, &files, &p.id).unwrap(), &path).unwrap();
        let edited = tmp.path().join("dup.qapture");
        // Mismo caso y repo dos veces: el primero se guarda y el segundo choca con la UNIQUE.
        with_github_json(&path, &edited, |v| {
            let mut again = v["links"][0].clone();
            again["issueNumber"] = json!(46);
            v["links"].as_array_mut().unwrap().push(again);
        });

        let summary = import(&conn, &files, &edited).unwrap();
        assert_eq!(
            summary.warnings,
            vec![github_json::GITHUB_WARNING.to_string()]
        );
        assert_eq!(summary.cases, 2, "el proyecto se importó");
        let id = &summary.projects[0].id;
        let count = |sql: &str| -> i64 { conn.query_row(sql, [id], |r| r.get(0)).unwrap() };
        assert_eq!(
            count(
                "SELECT COUNT(*) FROM case_issue_link WHERE case_id IN
                 (SELECT id FROM test_case WHERE project_id = ?1)"
            ),
            0
        );
        assert_eq!(
            count("SELECT COUNT(*) FROM project_repo_link WHERE project_id = ?1"),
            0
        );
        assert_eq!(
            count("SELECT COUNT(*) FROM project_gh_settings WHERE project_id = ?1"),
            0
        );
    }

    #[test]
    fn an_unmatched_routing_key_drops_the_routing_map_too() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let conn = test_support::conn();
        let p = with_github(&conn, &files);
        let path = tmp.path().join("gh.qapture");
        write(&collect(&conn, &files, &p.id).unwrap(), &path).unwrap();
        let edited = tmp.path().join("nokey.qapture");
        with_github_json(&path, &edited, |v| {
            v["routingFieldKey"] = json!("no_existe")
        });

        let summary = import(&conn, &files, &edited).unwrap();
        let config = get_project_github(&conn, &summary.projects[0].id).unwrap();
        assert_eq!(config.routing_field_id, None);
        assert!(config.routing_map.is_empty());
        assert_eq!(config.repos.len(), 2, "lo demás se conserva");
    }

    #[test]
    fn a_package_with_an_unknown_entry_still_imports() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let conn = test_support::conn();
        let (_, path) = exported(&conn, &files, tmp.path());
        let extra = tmp.path().join("extra.qapture");
        let mut input = ZipArchive::new(File::open(&path).unwrap()).unwrap();
        let mut output = ZipWriter::new(File::create(&extra).unwrap());
        for index in 0..input.len() {
            let entry = input.by_index_raw(index).unwrap();
            output.raw_copy_file(entry).unwrap();
        }
        output
            .start_file("otro.json", SimpleFileOptions::default())
            .unwrap();
        output.write_all(b"{}").unwrap();
        output.finish().unwrap();

        let summary = import(&conn, &files, &extra).unwrap();
        assert_eq!((summary.cases, summary.evidence), (2, 2));
        assert!(summary.warnings.is_empty());
    }

    #[test]
    fn project_json_is_unchanged_by_github_data() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let conn = test_support::conn();
        let p = seeded(&conn, &files);
        let before = collect(&conn, &files, &p.id).unwrap();
        assert!(before.github.is_none());
        let plain = tmp.path().join("plain.qapture");
        write(&before, &plain).unwrap();

        let field = test_support::field_id(&p, "severidad");
        save_project_github(
            &conn,
            &p.id,
            &ProjectGithub {
                repos: vec![LinkedRepo {
                    repo: repo("o/r"),
                    is_default: true,
                    default_labels: vec![],
                }],
                routing_field_id: Some(field),
                ..ProjectGithub::default()
            },
        )
        .unwrap();
        let first = cases::list(&conn, &p.id).unwrap().remove(0);
        insert_link(&conn, &link(&first.id, 7)).unwrap();
        let with = tmp.path().join("with.qapture");
        write(&collect(&conn, &files, &p.id).unwrap(), &with).unwrap();

        assert_eq!(
            zip_json(&plain, PROJECT_JSON),
            zip_json(&with, PROJECT_JSON)
        );
        assert!(zip_json(&with, PROJECT_JSON).is_some());
    }

    use crate::excel::guion_fixture;
    use crate::transfer::guion::{
        import as guion_import, profile as guion_profile, read as guion_read,
    };

    #[test]
    fn a_plain_project_still_writes_format_1() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let conn = test_support::conn();
        let p = seeded(&conn, &files);
        let export = collect(&conn, &files, &p.id).unwrap();
        assert_eq!(export.doc.format_version, 1);
        let json = String::from_utf8(project_json(&export.doc).unwrap()).unwrap();
        assert!(!json.contains("\"source\""));
        assert!(!json.contains("\"sourceRow\""));
        assert!(!json.contains("\"fixtures\""));
    }

    #[test]
    fn a_guion_project_round_trips_with_its_files() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let conn = test_support::conn();

        let guion_bytes = guion_fixture::build();
        let book = guion_read::read(&guion_bytes).unwrap();
        let def = guion_profile::tests::reference();
        let req = guion_import::ImportRequest {
            book: &book,
            bytes: &guion_bytes,
            file_name: "guion.xlsx",
            def: &def,
            project_name: "Guion Test",
            save_profile_as: None,
        };
        let summary = guion_import::import(&conn, &files, req).unwrap();
        let project_id = &summary.projects[0].id;

        let cp01 = cases::list(&conn, project_id)
            .unwrap()
            .into_iter()
            .find(|c| c.code == "CP01")
            .unwrap();
        repo_fixtures::add(
            &conn,
            &files,
            &cp01.id,
            "datos.csv",
            "nota 1",
            b"col1,col2\n1,2",
        )
        .unwrap();
        repo_fixtures::add(&conn, &files, &cp01.id, "extra.txt", "", b"texto de prueba").unwrap();

        let cp05 = cases::list(&conn, project_id)
            .unwrap()
            .into_iter()
            .find(|c| c.code == "CP05")
            .unwrap();
        repo_fixtures::add(&conn, &files, &cp05.id, "doc.pdf", "nota 2", b"%PDF-1.4").unwrap();

        let export = collect(&conn, &files, project_id).unwrap();
        assert_eq!(export.doc.format_version, 2);
        let path = tmp.path().join("Guion.qapture");
        write(&export, &path).unwrap();

        let entries = entry_names(&path);
        assert!(entries.contains(&"source/guion.xlsx".to_string()));
        assert!(entries.contains(&"fixtures/1".to_string()));
        assert!(entries.contains(&"fixtures/2".to_string()));
        assert!(entries.contains(&"fixtures/3".to_string()));

        let other = TempDir::new();
        let other_files = other.files();
        let other_conn = test_support::conn();
        let summary2 = import(&other_conn, &other_files, &path).unwrap();
        assert_eq!(summary2.cases, 36);
        let new_id = &summary2.projects[0].id;
        let cases_list = cases::list(&other_conn, new_id).unwrap();
        assert_eq!(cases_list.len(), 36);
        assert_eq!(cases_list[0].code, "CP01");
        assert_eq!(cases_list[35].code, "CP36");

        let rows = guion::case_rows(&other_conn, new_id).unwrap();
        assert_eq!(rows.len(), 36);
        assert_eq!(rows[0].1, 16);

        let src = guion::source(&other_conn, new_id).unwrap().unwrap();
        assert_eq!(src.definition, def);
        assert_eq!(src.sha256, fixtures::sha256_hex(&guion_bytes));
        let copied = other_files.absolute(&src.stored_path);
        assert_eq!(fs::read(&copied).unwrap(), guion_bytes);

        let new_cp01 = cases_list.iter().find(|c| c.code == "CP01").unwrap();
        let f1 = repo_fixtures::list(&other_conn, &new_cp01.id).unwrap();
        assert_eq!(f1.len(), 2);
        assert_eq!(f1[0].file_name, "datos.csv");
        assert_eq!(f1[0].note, "nota 1");
        assert_eq!(
            fs::read(repo_fixtures::absolute(&other_conn, &other_files, &f1[0].id).unwrap())
                .unwrap(),
            b"col1,col2\n1,2"
        );
        assert_eq!(f1[1].file_name, "extra.txt");
        assert_eq!(
            fs::read(repo_fixtures::absolute(&other_conn, &other_files, &f1[1].id).unwrap())
                .unwrap(),
            b"texto de prueba"
        );

        let new_cp05 = cases_list.iter().find(|c| c.code == "CP05").unwrap();
        let f5 = repo_fixtures::list(&other_conn, &new_cp05.id).unwrap();
        assert_eq!(f5.len(), 1);
        assert_eq!(f5[0].file_name, "doc.pdf");
        assert_eq!(f5[0].note, "nota 2");
        assert_eq!(
            fs::read(repo_fixtures::absolute(&other_conn, &other_files, &f5[0].id).unwrap())
                .unwrap(),
            b"%PDF-1.4"
        );

        let export2 = collect(&other_conn, &other_files, new_id).unwrap();
        assert_eq!(
            project_json(&export.doc).unwrap(),
            project_json(&export2.doc).unwrap()
        );
    }

    #[test]
    fn guion_codes_and_low_sequences_are_accepted_in_format_2() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let conn = test_support::conn();
        let doc = PackageDoc {
            format_version: 2,
            project: PackageProject {
                name: "Test Format 2".into(),
                client: "Cliente".into(),
                test_type: "Funcional".into(),
                release_version: "1.0".into(),
                start_date: "2026-11-10".into(),
                end_date: "2026-11-11".into(),
                next_sequence: 1,
                created_at: "2026-11-10T12:00:00.000Z".into(),
                updated_at: "2026-11-10T12:00:00.000Z".into(),
                source: None,
            },
            fields: vec![PackageField {
                key: "obs".into(),
                label: "Observaciones".into(),
                field_type: FieldType::Text,
                options: vec![],
                required: false,
                clear_on_duplicate: false,
                width: 20,
                archived: false,
            }],
            cases: vec![
                PackageCase {
                    code: "CP01".into(),
                    title: "Caso CP01".into(),
                    status: "No ejecutado".into(),
                    created_at: "2026-11-10T12:00:00.000Z".into(),
                    updated_at: "2026-11-10T12:00:00.000Z".into(),
                    values: BTreeMap::new(),
                    evidence: vec![],
                    source_row: None,
                    fixtures: vec![],
                },
                PackageCase {
                    code: "EJ-001".into(),
                    title: "Caso EJ-001".into(),
                    status: "Aprobado".into(),
                    created_at: "2026-11-10T12:00:00.000Z".into(),
                    updated_at: "2026-11-10T12:00:00.000Z".into(),
                    values: BTreeMap::new(),
                    evidence: vec![],
                    source_row: None,
                    fixtures: vec![],
                },
            ],
            doc_versions: vec![],
        };
        assert!(validate(&doc).is_ok());
        let path = tmp.path().join("f2.qapture");
        let export = PackageExport {
            doc,
            images: vec![],
            files: vec![],
            missing_images: 0,
            missing_files: 0,
            github: None,
        };
        write(&export, &path).unwrap();
        let summary = import(&conn, &files, &path).unwrap();
        assert_eq!(summary.cases, 2);
        let loaded = cases::list(&conn, &summary.projects[0].id).unwrap();
        let codes: Vec<_> = loaded.iter().map(|c| c.code.as_str()).collect();
        assert_eq!(codes, ["CP01", "EJ-001"]);
    }

    #[test]
    fn format_1_with_format_2_fields_is_not_a_package() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let conn = test_support::conn();
        let (_, good) = exported(&conn, &files, tmp.path());

        for (name, edit) in [
            (
                "source",
                Box::new(|v: &mut serde_json::Value| {
                    v["project"]["source"] = json!({
                        "fileName": "g.xlsx",
                        "sha256": "a".repeat(64),
                        "sheet": "Casos",
                        "headerRow": 1,
                        "definition": {
                            "version": 1,
                            "sheet": "Casos",
                            "headerRow": 1,
                            "columns": [{"column": "A", "header": "ID", "target": {"kind": "code"}}],
                            "statusMap": {"defaultStatus": "No ejecutado", "mappings": []},
                            "cover": {}
                        },
                        "importedAt": "2026-11-10T12:00:00.000Z"
                    });
                }) as Box<dyn Fn(&mut serde_json::Value)>,
            ),
            (
                "sourceRow",
                Box::new(|v: &mut serde_json::Value| {
                    v["cases"][0]["sourceRow"] = json!(16);
                }),
            ),
            (
                "fixtures",
                Box::new(|v: &mut serde_json::Value| {
                    v["cases"][0]["fixtures"] = json!([{
                        "fileName": "test.txt",
                        "note": "",
                        "size": 4,
                        "sha256": "b".repeat(64),
                        "addedAt": "2026-11-10T12:00:00.000Z",
                        "file": "fixtures/1"
                    }]);
                }),
            ),
        ] {
            let bad = tmp.path().join(format!("{name}.qapture"));
            with_json(&good, &bad, edit);
            let err = import(&conn, &files, &bad).unwrap_err().to_string();
            assert_eq!(err, NOT_PACKAGE, "{name}");
        }
    }

    #[test]
    fn tampered_fixture_entries_are_rejected_without_leftovers() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let conn = test_support::conn();

        let guion_bytes = guion_fixture::build();
        let book = guion_read::read(&guion_bytes).unwrap();
        let def = guion_profile::tests::reference();
        let req = guion_import::ImportRequest {
            book: &book,
            bytes: &guion_bytes,
            file_name: "guion.xlsx",
            def: &def,
            project_name: "Guion Tamper",
            save_profile_as: None,
        };
        let summary = guion_import::import(&conn, &files, req).unwrap();
        let p_id = &summary.projects[0].id;
        let cp01 = cases::list(&conn, p_id)
            .unwrap()
            .into_iter()
            .find(|c| c.code == "CP01")
            .unwrap();
        repo_fixtures::add(&conn, &files, &cp01.id, "datos.csv", "nota", b"1,2,3").unwrap();

        let export = collect(&conn, &files, p_id).unwrap();
        let good = tmp.path().join("good.qapture");
        write(&export, &good).unwrap();

        type TamperMutate = Box<dyn Fn(&Path, &Path)>;
        let tests: Vec<(&str, TamperMutate)> = vec![
            (
                "sha256 cambiado",
                Box::new(|src, dest| {
                    with_json(src, dest, |v| {
                        v["cases"][0]["fixtures"][0]["sha256"] = json!("0".repeat(64));
                    });
                }),
            ),
            (
                "size cambiado",
                Box::new(|src, dest| {
                    with_json(src, dest, |v| {
                        v["cases"][0]["fixtures"][0]["size"] = json!(9999);
                    });
                }),
            ),
            (
                "fileName ..\\x.txt",
                Box::new(|src, dest| {
                    with_json(src, dest, |v| {
                        v["cases"][0]["fixtures"][0]["fileName"] = json!("..\\x.txt");
                    });
                }),
            ),
            (
                "file fixtures/../a",
                Box::new(|src, dest| {
                    with_json(src, dest, |v| {
                        v["cases"][0]["fixtures"][0]["file"] = json!("fixtures/../a");
                    });
                }),
            ),
            (
                "extensión .exe",
                Box::new(|src, dest| {
                    with_json(src, dest, |v| {
                        v["cases"][0]["fixtures"][0]["fileName"] = json!("malware.exe");
                    });
                }),
            ),
        ];

        for (name, mutate) in tests {
            let bad = tmp.path().join(format!(
                "bad_{}.qapture",
                name.replace([' ', '.', '\\', '/'], "_")
            ));
            mutate(&good, &bad);

            let other = TempDir::new();
            let other_files = other.files();
            let other_conn = test_support::conn();

            let err = import(&other_conn, &other_files, &bad);
            assert!(err.is_err(), "{name} debe fallar");
            assert!(
                projects::list(&other_conn).unwrap().is_empty(),
                "{name}: sin proyectos"
            );
            assert_eq!(
                project_dirs(&other_files),
                0,
                "{name}: sin carpetas ni restos en evidence"
            );
        }
    }

    #[test]
    fn missing_files_are_skipped_on_export_and_counted() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let conn = test_support::conn();

        let guion_bytes = guion_fixture::build();
        let book = guion_read::read(&guion_bytes).unwrap();
        let def = guion_profile::tests::reference();
        let req = guion_import::ImportRequest {
            book: &book,
            bytes: &guion_bytes,
            file_name: "guion.xlsx",
            def: &def,
            project_name: "Guion Missing",
            save_profile_as: None,
        };
        let summary = guion_import::import(&conn, &files, req).unwrap();
        let p_id = &summary.projects[0].id;
        let cp01 = cases::list(&conn, p_id)
            .unwrap()
            .into_iter()
            .find(|c| c.code == "CP01")
            .unwrap();
        let f_dto =
            repo_fixtures::add(&conn, &files, &cp01.id, "datos.csv", "nota", b"1,2,3").unwrap();

        let fixture_abs = repo_fixtures::absolute(&conn, &files, &f_dto.id).unwrap();
        let src = guion::source(&conn, p_id).unwrap().unwrap();
        let guion_abs = files.absolute(&src.stored_path);

        crate::media::storage::remove_file(&fixture_abs).unwrap();
        crate::media::storage::remove_file(&guion_abs).unwrap();

        let export = collect(&conn, &files, p_id).unwrap();
        assert_eq!(export.missing_files, 2);
        assert_eq!(export.doc.project.source.as_ref().unwrap().file, None);
        let cp01_exported = export.doc.cases.iter().find(|c| c.code == "CP01").unwrap();
        assert!(
            cp01_exported.fixtures.is_empty(),
            "el fixture faltante se omite"
        );
    }
}
