//! UC-19: importa `matrices.json` o un respaldo JSON de la Bitácora v1 (schema 1).
//! Valida con las reglas y mensajes de `app-local/store.mjs` (`validateState`), pero junta
//! hasta 10 problemas con su ubicación en lugar de cortar en el primero (spec §8).

use std::collections::{HashMap, HashSet};

use base64::{
    Engine, alphabet,
    engine::{DecodePaddingMode, GeneralPurpose, GeneralPurposeConfig},
};
use serde_json::Value;

use rusqlite::Connection;

#[cfg(test)]
use super::bundle;
use super::bundle::{
    BundleCase, BundleEvidence, BundleField, EMPTY_ANNOTATIONS, IMPORT_FAILED, ImportFiles,
    Problems, ProjectBundle, code_number, is_case_code,
};
#[cfg(test)]
use crate::dto::ImportSummaryDto;
use crate::{
    db::{new_id, now},
    domain::{
        fields::{FieldType, is_iso_date, normalize_value},
        status::is_status,
    },
    dto::ProjectMetaInput,
    error::{AppResult, validation},
    media::{raster, storage::EvidenceFiles},
    repo::templates,
};

pub const NOT_LEGACY: &str = "El archivo no es un respaldo válido de la Bitácora v1.";
pub const NO_PROJECTS: &str = "El respaldo no tiene proyectos para importar.";
pub const BAD_IMAGE: &str = "Contenido de imagen inválido.";

const BAD_TEXT: &str = "Texto inválido o demasiado largo.";
const BAD_PROJECT: &str = "Nombre o ID de proyecto inválido.";
const TOO_MANY_CASES: &str = "Demasiados casos.";
const BAD_SEQUENCE: &str = "Secuencia inválida.";
const MISSING_TITLE: &str = "Completa caso y entorno.";
const LONG_TITLE: &str = "«Caso de prueba» supera 300 caracteres.";
const BAD_CODE: &str = "ID de ejecución repetido o inválido.";
const LOW_SEQUENCE: &str = "La secuencia debe superar los IDs usados.";
const BAD_STATUS: &str = "Estado inválido.";
const BAD_DATE: &str = "Fecha inválida.";
const TOO_MANY_PICTURES: &str = "Demasiadas capturas por caso.";
const DUP_PICTURE: &str = "Captura repetida.";
const NO_DESCRIPTION: &str = "La captura necesita una descripción.";
const BAD_SIZE: &str = "Dimensiones de imagen inválidas.";
const BAD_IMAGE_FORMAT: &str = "Formato de imagen inválido.";

/// Campos de texto de cada caso v1 (`fields` en `store.mjs`).
const CASE_TEXTS: [&str; 14] = [
    "id",
    "code",
    "title",
    "environment",
    "module",
    "menu",
    "submenu",
    "functionality",
    "input",
    "expected",
    "actual",
    "status",
    "date",
    "owner",
];

/// Campo v1 → clave de la plantilla Funcional. `code`, `title` y `status` van al caso; `id` se descarta.
pub const FIELD_MAP: [(&str, &str); 10] = [
    ("environment", "entorno"),
    ("module", "modulo"),
    ("menu", "menu"),
    ("submenu", "submenu"),
    ("functionality", "funcionalidad"),
    ("input", "datos_entrada"),
    ("expected", "resultado_esperado"),
    ("actual", "resultado_obtenido"),
    ("date", "fecha_ejecucion"),
    ("owner", "responsable"),
];

const MAX_PROJECTS: usize = 100;
const MAX_CASES: usize = 10_000;
const MAX_TEXT: usize = 16_000;
const MAX_TITLE: usize = 300;
const MAX_PICTURES: usize = 200;
const MAX_DESCRIPTION: usize = 4_000;
const MAX_DATA_URL: usize = 16 * 1024 * 1024;
const MAX_SIDE: i64 = 12_000;
const MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991;
const PNG_MAGIC: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];

/// Como `Buffer.from(…, 'base64')` de Node: el relleno `=` es opcional.
const LENIENT: GeneralPurpose = GeneralPurpose::new(
    &alphabet::STANDARD,
    GeneralPurposeConfig::new()
        .with_decode_padding_mode(DecodePaddingMode::Indifferent)
        .with_decode_allow_trailing_bits(true),
);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageKind {
    Png,
    Jpeg,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LegacyEvidence {
    pub description: String,
    pub kind: ImageKind,
    /// El base64 del `dataUrl` ya decodificado.
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LegacyCase {
    pub code: String,
    pub title: String,
    pub status: String,
    /// Clave Funcional → valor tal como venía (con sus saltos de línea).
    pub values: Vec<(&'static str, String)>,
    pub evidence: Vec<LegacyEvidence>,
}

impl LegacyCase {
    pub fn value(&self, key: &str) -> &str {
        self.values
            .iter()
            .find(|(k, _)| *k == key)
            .map_or("", |(_, v)| v.as_str())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LegacyProject {
    pub name: String,
    pub next_sequence: i64,
    pub cases: Vec<LegacyCase>,
}

/// `typeof v === 'string' && v.length <= max`, con la longitud en UTF-16 como JavaScript.
fn text(value: Option<&Value>, max: usize) -> Option<&str> {
    value?.as_str().filter(|s| s.encode_utf16().count() <= max)
}

/// `Number.isSafeInteger`: `6` y `6.0` valen; `6.5`, `"6"` o enteros enormes no.
fn integer(value: Option<&Value>) -> Option<i64> {
    let Value::Number(n) = value? else {
        return None;
    };
    let i = n.as_i64().or_else(|| {
        n.as_f64()
            .filter(|f| f.fract() == 0.0 && f.abs() <= MAX_SAFE_INTEGER as f64)
            .map(|f| f as i64)
    })?;
    (i.abs() <= MAX_SAFE_INTEGER).then_some(i)
}

/// `/^data:image\/(png|jpeg);base64,[A-Za-z0-9+/]+={0,2}$/`
fn split_data_url(url: &str) -> Option<(ImageKind, &str)> {
    let (kind, payload) = match url.strip_prefix("data:image/png;base64,") {
        Some(rest) => (ImageKind::Png, rest),
        None => (
            ImageKind::Jpeg,
            url.strip_prefix("data:image/jpeg;base64,")?,
        ),
    };
    let body = payload.trim_end_matches('=');
    let valid = payload.len() - body.len() <= 2
        && !body.is_empty()
        && body
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'/');
    valid.then_some((kind, payload))
}

fn content_matches(kind: ImageKind, bytes: &[u8]) -> bool {
    match kind {
        ImageKind::Png => bytes.starts_with(&PNG_MAGIC),
        ImageKind::Jpeg => bytes.starts_with(&[0xFF, 0xD8]),
    }
}

struct ProjectRules<'a> {
    at: &'a str,
    next_sequence: i64,
    ids: HashSet<String>,
    codes: HashSet<String>,
}

pub fn parse(bytes: &[u8]) -> AppResult<Vec<LegacyProject>> {
    let root: Value = serde_json::from_slice(bytes).map_err(|_| validation(NOT_LEGACY))?;
    let schema_ok = integer(root.get("schema")) == Some(1);
    let projects = match root.get("projects").and_then(Value::as_array) {
        Some(list) if schema_ok && list.len() <= MAX_PROJECTS => list,
        _ => return Err(validation(NOT_LEGACY)),
    };
    if projects.is_empty() {
        return Err(validation(NO_PROJECTS));
    }
    let mut problems = Problems::default();
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for (index, p) in projects.iter().enumerate() {
        if let Some(project) = parse_project(p, index, &mut seen, &mut problems) {
            out.push(project);
        }
    }
    problems.into_result()?;
    Ok(out)
}

fn parse_project(
    p: &Value,
    index: usize,
    seen: &mut HashSet<String>,
    problems: &mut Problems,
) -> Option<LegacyProject> {
    let name = text(p.get("name"), 150);
    let at = match name.map(str::trim) {
        Some(n) if !n.is_empty() => format!("Proyecto «{n}»"),
        _ => format!("Proyecto {}", index + 1),
    };
    let (Some(id), Some(name)) = (text(p.get("id"), 100), name) else {
        problems.add(&at, BAD_TEXT);
        return None;
    };
    if name.trim().is_empty() || !seen.insert(id.to_string()) {
        problems.add(&at, BAD_PROJECT);
        return None;
    }
    let Some(cases) = p
        .get("cases")
        .and_then(Value::as_array)
        .filter(|c| c.len() <= MAX_CASES)
    else {
        problems.add(&at, TOO_MANY_CASES);
        return None;
    };
    let Some(next_sequence) = integer(p.get("nextSequence")).filter(|n| *n > 0) else {
        problems.add(&at, BAD_SEQUENCE);
        return None;
    };
    let mut rules = ProjectRules {
        at: &at,
        next_sequence,
        ids: HashSet::new(),
        codes: HashSet::new(),
    };
    let before = problems.total();
    let cases: Vec<LegacyCase> = cases
        .iter()
        .enumerate()
        .filter_map(|(i, c)| parse_case(c, i, &mut rules, problems))
        .collect();
    (problems.total() == before).then(|| LegacyProject {
        name: name.trim().to_string(),
        next_sequence,
        cases,
    })
}

fn parse_case(
    c: &Value,
    index: usize,
    rules: &mut ProjectRules,
    problems: &mut Problems,
) -> Option<LegacyCase> {
    let label = text(c.get("code"), 20)
        .filter(|s| !s.trim().is_empty())
        .map_or_else(|| format!("caso {}", index + 1), str::to_string);
    let at = format!("{}, {label}", rules.at);
    let mut texts: HashMap<&str, &str> = HashMap::new();
    for key in CASE_TEXTS {
        let Some(value) = text(c.get(key), MAX_TEXT) else {
            problems.add(&at, BAD_TEXT);
            return None;
        };
        texts.insert(key, value);
    }
    let before = problems.total();
    let title = texts["title"].trim();
    if title.is_empty() || texts["environment"].trim().is_empty() {
        problems.add(&at, MISSING_TITLE);
    } else if title.chars().count() > MAX_TITLE {
        problems.add(&at, LONG_TITLE);
    }
    let code = texts["code"];
    if !is_case_code(code)
        || !rules.ids.insert(texts["id"].to_string())
        || !rules.codes.insert(code.to_string())
    {
        problems.add(&at, BAD_CODE);
    } else if code_number(code).is_none_or(|n| n >= rules.next_sequence) {
        problems.add(&at, LOW_SEQUENCE);
    }
    if !is_status(texts["status"]) {
        problems.add(&at, BAD_STATUS);
    }
    let date = texts["date"];
    if !date.is_empty() && !is_iso_date(date) {
        problems.add(&at, BAD_DATE);
    }
    let evidence = match c
        .get("evidence")
        .and_then(Value::as_array)
        .filter(|e| e.len() <= MAX_PICTURES)
    {
        Some(list) => parse_evidence(list, &at, problems),
        None => {
            problems.add(&at, TOO_MANY_PICTURES);
            Vec::new()
        }
    };
    (problems.total() == before).then(|| LegacyCase {
        code: code.to_string(),
        title: title.to_string(),
        status: texts["status"].to_string(),
        values: FIELD_MAP
            .iter()
            .map(|&(v1, key)| (key, texts[v1].to_string()))
            .collect(),
        evidence,
    })
}

fn parse_evidence(list: &[Value], case_at: &str, problems: &mut Problems) -> Vec<LegacyEvidence> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for (index, e) in list.iter().enumerate() {
        let at = format!("{case_at}, captura {}", index + 1);
        let Some(id) = text(e.get("id"), 100) else {
            problems.add(&at, BAD_TEXT);
            continue;
        };
        if !seen.insert(id.to_string()) {
            problems.add(&at, DUP_PICTURE);
            continue;
        }
        let Some(description) = text(e.get("description"), MAX_DESCRIPTION) else {
            problems.add(&at, BAD_TEXT);
            continue;
        };
        if description.trim().is_empty() {
            problems.add(&at, NO_DESCRIPTION);
            continue;
        }
        let side = |key: &str| integer(e.get(key)).filter(|n| (1..=MAX_SIDE).contains(n));
        if side("width").is_none() || side("height").is_none() {
            problems.add(&at, BAD_SIZE);
            continue;
        }
        let Some(url) = text(e.get("dataUrl"), MAX_DATA_URL) else {
            problems.add(&at, BAD_TEXT);
            continue;
        };
        let Some((kind, payload)) = split_data_url(url) else {
            problems.add(&at, BAD_IMAGE_FORMAT);
            continue;
        };
        let bytes = LENIENT.decode(payload).unwrap_or_default();
        if !content_matches(kind, &bytes) {
            problems.add(&at, BAD_IMAGE);
            continue;
        }
        out.push(LegacyEvidence {
            description: description.trim().to_string(),
            kind,
            bytes,
        });
    }
    out
}

pub const FUNCIONAL: &str = "tpl-funcional";
pub const MAX_FILE_BYTES: u64 = 200 * 1024 * 1024;

/// Columnas de la plantilla Funcional tal como están en esta base (spec §5).
pub fn template_fields(conn: &Connection) -> AppResult<Vec<BundleField>> {
    Ok(templates::get(conn, FUNCIONAL)?
        .fields
        .into_iter()
        .map(|f| BundleField {
            key: f.key,
            label: f.label,
            field_type: f.field_type,
            options: f.options,
            required: f.required,
            clear_on_duplicate: f.clear_on_duplicate,
            width: f.width,
            archived: false,
        })
        .collect())
}

/// La plantilla, con «Texto largo» en las columnas de texto corto cuyo valor v1 no cabe.
fn columns_for(template: &[BundleField], cases: &[LegacyCase]) -> Vec<BundleField> {
    template
        .iter()
        .cloned()
        .map(|mut f| {
            let too_long = cases
                .iter()
                .any(|c| c.value(&f.key).chars().count() > FieldType::Text.max_len());
            if f.field_type == FieldType::Text && too_long {
                f.field_type = FieldType::Longtext;
            }
            f
        })
        .collect()
}

/// PNG: los mismos bytes (mismo hash). JPEG: convertido a PNG sin reducir.
fn as_png(at: &str, kind: ImageKind, bytes: Vec<u8>) -> AppResult<(Vec<u8>, u32, u32)> {
    let bad = || validation(format!("{IMPORT_FAILED}\n• {at}: {BAD_IMAGE}"));
    let decoded = raster::to_png(&bytes).map_err(|_| bad())?;
    let (width, height) = (decoded.width, decoded.height);
    let png = match kind {
        ImageKind::Png => bytes,
        ImageKind::Jpeg => decoded.png,
    };
    Ok((png, width, height))
}

fn to_bundles(
    projects: Vec<LegacyProject>,
    template: &[BundleField],
    out: &mut ImportFiles,
) -> AppResult<Vec<ProjectBundle>> {
    let ts = now();
    let mut bundles = Vec::with_capacity(projects.len());
    for p in projects {
        let id = new_id();
        let at = format!("Proyecto «{}»", p.name);
        let fields = columns_for(template, &p.cases);
        let mut cases = Vec::with_capacity(p.cases.len());
        for c in p.cases {
            let mut evidence = Vec::with_capacity(c.evidence.len());
            for (index, e) in c.evidence.into_iter().enumerate() {
                let place = format!("{at}, {}, captura {}", c.code, index + 1);
                let (png, width, height) = as_png(&place, e.kind, e.bytes)?;
                let evidence_id = new_id();
                out.write(&id, &evidence_id, &png, &png)?;
                evidence.push(BundleEvidence {
                    id: evidence_id,
                    description: e.description,
                    annotations: EMPTY_ANNOTATIONS.into(),
                    width: width.into(),
                    height: height.into(),
                    captured_at: ts.clone(),
                    updated_at: ts.clone(),
                });
            }
            let values = c
                .values
                .iter()
                .map(|(key, raw)| {
                    let field_type = fields
                        .iter()
                        .find(|f| f.key == *key)
                        .map_or(FieldType::Text, |f| f.field_type);
                    (key.to_string(), normalize_value(field_type, raw))
                })
                .filter(|(_, value)| !value.is_empty())
                .collect();
            cases.push(BundleCase {
                code: c.code,
                title: c.title,
                status: c.status,
                created_at: ts.clone(),
                updated_at: ts.clone(),
                values,
                evidence,
            });
        }
        bundles.push(ProjectBundle {
            id,
            meta: ProjectMetaInput {
                name: p.name,
                client: String::new(),
                test_type: "Funcional".into(),
                release_version: String::new(),
                start_date: String::new(),
                end_date: String::new(),
            },
            next_sequence: p.next_sequence,
            source_template_id: Some(FUNCIONAL.into()),
            created_at: ts.clone(),
            updated_at: ts.clone(),
            fields,
            cases,
            versions: Vec::new(),
        });
    }
    Ok(bundles)
}

/// Parte de UC-19 que no necesita la base: valida, decodifica y escribe las capturas.
pub fn prepare<'a>(
    template: &[BundleField],
    files: &'a EvidenceFiles,
    bytes: &[u8],
) -> AppResult<(Vec<ProjectBundle>, ImportFiles<'a>)> {
    let projects = parse(bytes)?;
    let mut out = ImportFiles::new(files);
    let bundles = to_bundles(projects, template, &mut out)?;
    Ok((bundles, out))
}

/// UC-19 completo: `prepare` y crea los proyectos en una transacción.
#[cfg(test)]
pub fn import(
    conn: &Connection,
    files: &EvidenceFiles,
    bytes: &[u8],
) -> AppResult<ImportSummaryDto> {
    let template = template_fields(conn)?;
    let (bundles, out) = prepare(&template, files, bytes)?;
    bundle::insert(conn, bundles, out)
}

#[cfg(test)]
pub(crate) mod tests {
    use std::{io::Cursor, path::PathBuf};

    use base64::engine::general_purpose::STANDARD;
    use serde_json::json;

    use super::*;
    use crate::{error::AppError, media::raster::sample_png};

    use std::{collections::BTreeMap, fs};

    use crate::{
        dto::CaseInput,
        media::storage::TempDir,
        repo::{cases, evidence, projects, test_support},
        transfer::bundle::tests::project_dirs,
    };

    fn decoded(value: &Value) -> Vec<u8> {
        let url = value["dataUrl"].as_str().unwrap();
        STANDARD.decode(url.split_once(',').unwrap().1).unwrap()
    }

    #[test]
    fn imports_the_fixture_keeping_codes_sequence_text_and_images() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let conn = test_support::conn();
        let summary = import(&conn, &files, &fixture_bytes()).unwrap();

        let names: Vec<&str> = summary.projects.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["Aplicación de recibos", "Pólizas"]);
        assert_eq!((summary.cases, summary.evidence), (3, 3));

        let recibos = projects::get(&conn, &summary.projects[0].id).unwrap();
        assert_eq!(recibos.next_sequence, 6);
        assert_eq!(recibos.test_type, "Funcional");
        assert_eq!(recibos.source_template_id.as_deref(), Some(FUNCIONAL));
        assert_eq!(recibos.fields.len(), 11, "plantilla Funcional completa");
        let list = cases::list(&conn, &recibos.id).unwrap();
        let codes: Vec<&str> = list.iter().map(|c| c.code.as_str()).collect();
        assert_eq!(codes, ["EJ-001", "EJ-002", "EJ-004"]);
        let statuses: Vec<&str> = list.iter().map(|c| c.status.as_str()).collect();
        assert_eq!(statuses, ["Por documentar", "Aprobado", "Fallido"]);
        let value = |case: usize, key: &str| {
            list[case]
                .values
                .get(&test_support::field_id(&recibos, key))
                .cloned()
                .unwrap_or_default()
        };
        assert_eq!(
            value(0, "datos_entrada"),
            "Recibo: 1000001\nSucursal: 1\nImporte: $1,234.56 (Moneda Nacional)"
        );
        assert_eq!(
            value(2, "resultado_obtenido"),
            "Error 500 al cancelar.\nSe reintentó dos veces."
        );
        assert_eq!(value(1, "fecha_ejecucion"), "2026-10-14");
        assert_eq!(value(0, "entorno"), "Máquina Virtual");
        assert_eq!(
            value(0, "resultado_obtenido"),
            "",
            "los vacíos no se guardan"
        );

        let fixture = fixture_value();
        let pictures = &fixture["projects"][0]["cases"][0]["evidence"];
        let ev = evidence::list(&conn, &files, &list[0].id).unwrap();
        assert_eq!(ev[0].description, "Pantalla de captura del recibo.");
        let original = fs::read(&ev[0].original_path).unwrap();
        assert_eq!(
            original,
            decoded(&pictures[0]),
            "PNG byte a byte (mismo hash)"
        );
        assert_eq!(
            fs::read(&ev[0].rendered_path).unwrap(),
            original,
            "render = original"
        );
        assert_eq!((ev[0].width, ev[0].height), (64, 48));
        assert_eq!(
            ev[0].annotations,
            crate::transfer::bundle::EMPTY_ANNOTATIONS
        );
        let from_jpeg = fs::read(&ev[1].original_path).unwrap();
        assert_eq!(
            image::guess_format(&from_jpeg).unwrap(),
            image::ImageFormat::Png
        );
        assert_eq!((ev[1].width, ev[1].height), (32, 24));
        let second = evidence::list(&conn, &files, &list[1].id).unwrap();
        let second_picture = &fixture["projects"][0]["cases"][1]["evidence"][0];
        assert_eq!(
            fs::read(&second[0].original_path).unwrap(),
            decoded(second_picture)
        );

        let next = cases::create(
            &conn,
            &recibos.id,
            &CaseInput {
                title: "Nuevo caso".into(),
                status: "No ejecutado".into(),
                values: BTreeMap::from([(
                    test_support::field_id(&recibos, "entorno"),
                    "QA".into(),
                )]),
            },
        )
        .unwrap();
        assert_eq!(
            next.code, "EJ-006",
            "la secuencia continúa donde la dejó la v1"
        );
    }

    #[test]
    fn a_rejected_file_writes_nothing() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let conn = test_support::conn();
        let bytes = mutated(|v| v["projects"][0]["cases"][2]["status"] = json!("Cerrado"));
        assert!(import(&conn, &files, &bytes).is_err());
        assert!(projects::list(&conn).unwrap().is_empty());
        assert_eq!(project_dirs(&files), 0);
    }

    #[test]
    fn a_truncated_png_in_the_second_project_rolls_back_everything() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let conn = test_support::conn();
        // Cabecera PNG válida (pasa la regla de store.mjs) pero archivo incompleto.
        let truncated = &sample_png(64, 48)[..40];
        let bytes = mutated(|v| {
            let mut broken = case("p-1", "EJ-001", "Emitir póliza", "Aprobado", vec![]);
            broken["evidence"] = json!([picture(
                "x",
                "Póliza emitida.",
                data_url("png", truncated),
                64,
                48
            )]);
            v["projects"][1]["nextSequence"] = json!(2);
            v["projects"][1]["cases"] = json!([broken]);
        });
        let err = import(&conn, &files, &bytes).unwrap_err().to_string();
        assert_eq!(
            err,
            format!(
                "{}\n• Proyecto «Pólizas», EJ-001, captura 1: Contenido de imagen inválido.",
                crate::transfer::bundle::IMPORT_FAILED
            )
        );
        assert!(projects::list(&conn).unwrap().is_empty());
        assert_eq!(
            project_dirs(&files),
            0,
            "las capturas del primer proyecto también se borran"
        );
    }

    #[test]
    fn long_values_turn_the_column_into_long_text() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let conn = test_support::conn();
        let long = "m".repeat(600);
        let bytes = mutated(|v| v["projects"][0]["cases"][0]["menu"] = json!(long));
        let summary = import(&conn, &files, &bytes).unwrap();
        let p = projects::get(&conn, &summary.projects[0].id).unwrap();
        let menu = p.fields.iter().find(|f| f.key == "menu").unwrap();
        assert_eq!(menu.field_type, FieldType::Longtext);
        let modulo = p.fields.iter().find(|f| f.key == "modulo").unwrap();
        assert_eq!(modulo.field_type, FieldType::Text);
        let list = cases::list(&conn, &p.id).unwrap();
        assert_eq!(list[0].values[&menu.id].chars().count(), 600);
        let polizas = projects::get(&conn, &summary.projects[1].id).unwrap();
        let other_menu = polizas.fields.iter().find(|f| f.key == "menu").unwrap();
        assert_eq!(
            other_menu.field_type,
            FieldType::Text,
            "solo cambia en el proyecto afectado"
        );
    }

    #[test]
    fn importing_twice_adds_the_importado_suffix() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let conn = test_support::conn();
        import(&conn, &files, &fixture_bytes()).unwrap();
        let again = import(&conn, &files, &fixture_bytes()).unwrap();
        let names: Vec<&str> = again.projects.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(
            names,
            ["Aplicación de recibos (importado)", "Pólizas (importado)"]
        );
        assert_eq!(projects::list(&conn).unwrap().len(), 4);
    }

    const FIXTURE: &str = "fixtures/bitacora-v1.json";

    fn data_url(mime: &str, bytes: &[u8]) -> String {
        format!("data:image/{mime};base64,{}", STANDARD.encode(bytes))
    }

    pub(crate) fn jpeg(width: u32, height: u32) -> Vec<u8> {
        let img = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
            width,
            height,
            image::Rgb([186, 200, 45]),
        ));
        let mut out = Vec::new();
        img.write_to(&mut Cursor::new(&mut out), image::ImageFormat::Jpeg)
            .unwrap();
        out
    }

    fn picture(id: &str, description: &str, url: String, width: u32, height: u32) -> Value {
        json!({ "id": id, "description": description, "dataUrl": url, "width": width, "height": height })
    }

    fn case(id: &str, code: &str, title: &str, status: &str, evidence: Vec<Value>) -> Value {
        json!({
            "id": id, "code": code, "title": title, "environment": "QA", "module": "Cobranza",
            "menu": "Aplicación", "submenu": "Aplicación de recibos", "functionality": "Primas por cobrar",
            "input": "", "expected": "El recibo queda aplicado.", "actual": "", "status": status,
            "date": "", "owner": "Ana Pérez", "evidence": evidence
        })
    }

    /// Bitácora v1 ficticia con la misma forma que `matrices.json` (spec §9: sin datos de clientes).
    pub(crate) fn fixture_value() -> Value {
        let mut first = case(
            "caso-1",
            "EJ-001",
            "Aplicación de recibos",
            "Por documentar",
            vec![
                picture(
                    "cap-1",
                    "Pantalla de captura del recibo.",
                    data_url("png", &sample_png(64, 48)),
                    64,
                    48,
                ),
                picture(
                    "cap-2",
                    "Falta un SP.",
                    data_url("jpeg", &jpeg(32, 24)),
                    32,
                    24,
                ),
            ],
        );
        first["environment"] = json!("Máquina Virtual");
        first["input"] =
            json!("Recibo: 1000001\nSucursal: 1\nImporte: $1,234.56 (Moneda Nacional)");
        let mut second = case(
            "caso-2",
            "EJ-002",
            "Aplicación de recibos",
            "Aprobado",
            vec![picture(
                "cap-3",
                "Recibo aplicado.",
                data_url("png", &sample_png(120, 80)),
                120,
                80,
            )],
        );
        second["date"] = json!("2026-10-14");
        let mut fourth = case("caso-4", "EJ-004", "Cancelar recibo", "Fallido", vec![]);
        fourth["actual"] = json!("Error 500 al cancelar.\nSe reintentó dos veces.");
        fourth["date"] = json!("2026-10-15");
        json!({
            "schema": 1,
            "revision": 3,
            "projects": [
                { "id": "recibos-demo", "name": "Aplicación de recibos", "nextSequence": 6, "cases": [first, second, fourth] },
                { "id": "polizas-demo", "name": "Pólizas", "nextSequence": 1, "cases": [] }
            ]
        })
    }

    fn fixture_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(FIXTURE)
    }

    pub(crate) fn fixture_bytes() -> Vec<u8> {
        std::fs::read(fixture_path())
            .expect("falta el fixture; ejecuta UPDATE_LEGACY_FIXTURE=1 cargo test legacy")
    }

    pub(crate) fn mutated(change: impl FnOnce(&mut Value)) -> Vec<u8> {
        let mut value = fixture_value();
        change(&mut value);
        serde_json::to_vec(&value).unwrap()
    }

    fn problems_of(bytes: &[u8]) -> String {
        match parse(bytes) {
            Err(AppError::Validation(message)) => message,
            other => panic!("se esperaba un error de validación: {other:?}"),
        }
    }

    /// Regenerar: `UPDATE_LEGACY_FIXTURE=1 cargo test legacy`
    #[test]
    fn fixture_is_generated_and_anonymous() {
        let actual = serde_json::to_string_pretty(&fixture_value()).unwrap() + "\n";
        if std::env::var("UPDATE_LEGACY_FIXTURE").is_ok() {
            std::fs::create_dir_all(fixture_path().parent().unwrap()).unwrap();
            std::fs::write(fixture_path(), &actual).unwrap();
        }
        let expected = String::from_utf8(fixture_bytes())
            .unwrap()
            .replace("\r\n", "\n");
        assert_eq!(
            actual, expected,
            "El fixture cambió: regenéralo con UPDATE_LEGACY_FIXTURE=1"
        );
        // Textos de la Bitácora real de GMX que nunca deben llegar al repo público.
        for banned in ["GMX", "4644564", "7043816", "25,834.23", "captura-original"] {
            assert!(!expected.contains(banned), "el fixture contiene «{banned}»");
        }
    }

    #[test]
    fn parses_the_fixture() {
        let projects = parse(&fixture_bytes()).unwrap();
        assert_eq!(projects.len(), 2);
        let recibos = &projects[0];
        assert_eq!(
            (recibos.name.as_str(), recibos.next_sequence),
            ("Aplicación de recibos", 6)
        );
        let codes: Vec<&str> = recibos.cases.iter().map(|c| c.code.as_str()).collect();
        assert_eq!(codes, ["EJ-001", "EJ-002", "EJ-004"]);
        let first = &recibos.cases[0];
        assert_eq!(first.value("entorno"), "Máquina Virtual");
        assert_eq!(
            first.value("datos_entrada"),
            "Recibo: 1000001\nSucursal: 1\nImporte: $1,234.56 (Moneda Nacional)"
        );
        assert_eq!(first.value("responsable"), "Ana Pérez");
        assert_eq!(first.evidence[0].kind, ImageKind::Png);
        assert_eq!(first.evidence[0].bytes, sample_png(64, 48));
        assert_eq!(first.evidence[1].kind, ImageKind::Jpeg);
        assert_eq!(first.evidence[1].description, "Falta un SP.");
        assert_eq!(recibos.cases[1].value("fecha_ejecucion"), "2026-10-14");
        assert!(projects[1].cases.is_empty());
    }

    #[test]
    fn every_v1_rule_reports_where_it_failed() {
        const R: &str = "Proyecto «Aplicación de recibos»";
        type Rule = (fn(&mut Value), String);
        let rules: [Rule; 15] = [
            (
                |v| v["projects"][0]["cases"][0]["environment"] = json!("  "),
                format!("{R}, EJ-001: Completa caso y entorno."),
            ),
            (
                |v| v["projects"][0]["cases"][0]["title"] = json!("x".repeat(301)),
                format!("{R}, EJ-001: «Caso de prueba» supera 300 caracteres."),
            ),
            (
                |v| v["projects"][0]["cases"][1]["code"] = json!("EJ-001"),
                format!("{R}, EJ-001: ID de ejecución repetido o inválido."),
            ),
            (
                |v| v["projects"][0]["cases"][1]["code"] = json!("EJ-2"),
                format!("{R}, EJ-2: ID de ejecución repetido o inválido."),
            ),
            (
                |v| v["projects"][0]["nextSequence"] = json!(4),
                format!("{R}, EJ-004: La secuencia debe superar los IDs usados."),
            ),
            (
                |v| v["projects"][0]["cases"][2]["status"] = json!("Cerrado"),
                format!("{R}, EJ-004: Estado inválido."),
            ),
            (
                |v| v["projects"][0]["cases"][1]["date"] = json!("2026-02-30"),
                format!("{R}, EJ-002: Fecha inválida."),
            ),
            (
                |v| v["projects"][0]["cases"][0]["menu"] = json!(7),
                format!("{R}, EJ-001: Texto inválido o demasiado largo."),
            ),
            (
                |v| v["projects"][0]["cases"][0]["evidence"][1]["description"] = json!(" "),
                format!("{R}, EJ-001, captura 2: La captura necesita una descripción."),
            ),
            (
                |v| v["projects"][0]["cases"][0]["evidence"][1]["id"] = json!("cap-1"),
                format!("{R}, EJ-001, captura 2: Captura repetida."),
            ),
            (
                |v| v["projects"][0]["cases"][0]["evidence"][0]["width"] = json!(0),
                format!("{R}, EJ-001, captura 1: Dimensiones de imagen inválidas."),
            ),
            (
                |v| {
                    v["projects"][0]["cases"][0]["evidence"][0]["dataUrl"] =
                        json!("data:image/gif;base64,R0lGOD")
                },
                format!("{R}, EJ-001, captura 1: Formato de imagen inválido."),
            ),
            (
                |v| {
                    v["projects"][0]["cases"][0]["evidence"][0]["dataUrl"] =
                        json!("data:image/png;base64,/9j/4AAQ")
                },
                format!("{R}, EJ-001, captura 1: Contenido de imagen inválido."),
            ),
            (
                |v| v["projects"][1]["id"] = json!("recibos-demo"),
                "Proyecto «Pólizas»: Nombre o ID de proyecto inválido.".to_string(),
            ),
            (
                |v| v["projects"][1]["nextSequence"] = json!(0),
                "Proyecto «Pólizas»: Secuencia inválida.".to_string(),
            ),
        ];
        for (change, expected) in rules {
            let message = problems_of(&mutated(change));
            assert!(
                message.starts_with(super::super::bundle::IMPORT_FAILED),
                "{message}"
            );
            assert!(
                message.contains(&format!("\n• {expected}")),
                "falta «{expected}» en:\n{message}"
            );
        }
    }

    #[test]
    fn a_project_without_a_valid_name_is_located_by_position() {
        let message = problems_of(&mutated(|v| v["projects"][1]["name"] = json!("   ")));
        assert!(
            message.contains("• Proyecto 2: Nombre o ID de proyecto inválido."),
            "{message}"
        );
        let message = problems_of(&mutated(|v| {
            v["projects"][0]["cases"][0]["code"] = json!(null)
        }));
        assert!(
            message.contains(
                "• Proyecto «Aplicación de recibos», caso 1: Texto inválido o demasiado largo."
            ),
            "{message}"
        );
    }

    #[test]
    fn lists_at_most_ten_problems() {
        let bad: Vec<Value> = (1..=12)
            .map(|n| {
                let mut c = case(&format!("c{n}"), &format!("EJ-{n:03}"), "", "", vec![]);
                c["environment"] = json!("");
                c
            })
            .collect();
        // Cada caso: sin título/entorno, código ≥ secuencia (1) y estado vacío → 36 problemas.
        let message = problems_of(&mutated(|v| v["projects"][1]["cases"] = json!(bad)));
        let lines: Vec<&str> = message.lines().collect();
        assert_eq!(lines.len(), 12);
        assert_eq!(
            lines[1],
            "• Proyecto «Pólizas», EJ-001: Completa caso y entorno."
        );
        assert_eq!(lines[11], "• … y 26 problemas más.");
    }

    #[test]
    fn rejects_files_that_are_not_v1() {
        let many: Vec<Value> = (0..101)
            .map(|n| json!({ "id": format!("p{n}"), "name": format!("P{n}"), "nextSequence": 1, "cases": [] }))
            .collect();
        for bytes in [
            b"hola".to_vec(),
            b"[]".to_vec(),
            br#"{"schema":2,"projects":[]}"#.to_vec(),
            br#"{"schema":1,"projects":{}}"#.to_vec(),
            serde_json::to_vec(&json!({ "schema": 1, "projects": many })).unwrap(),
        ] {
            assert_eq!(parse(&bytes).unwrap_err().to_string(), NOT_LEGACY);
        }
        assert_eq!(
            parse(br#"{"schema":1,"projects":[]}"#)
                .unwrap_err()
                .to_string(),
            NO_PROJECTS
        );
    }

    #[test]
    fn accepts_what_node_accepts() {
        let bytes = mutated(|v| {
            v["schema"] = json!(1.0);
            v["projects"][0]["nextSequence"] = json!(6.0);
            let url = v["projects"][0]["cases"][1]["evidence"][0]["dataUrl"]
                .as_str()
                .unwrap()
                .trim_end_matches('=')
                .to_string();
            v["projects"][0]["cases"][1]["evidence"][0]["dataUrl"] = json!(url);
        });
        let projects = parse(&bytes).unwrap();
        assert_eq!(projects[0].next_sequence, 6);
        assert_eq!(projects[0].cases[1].evidence[0].bytes, sample_png(120, 80));
    }
}
