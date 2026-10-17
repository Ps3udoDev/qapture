//! UC-18: paquete `.qapture` = ZIP con `project.json` (`formatVersion: 1`) y las capturas en
//! `evidence/<código>-<n>.orig.png` / `.render.png`. `project.json` no lleva ids ni la plantilla de
//! origen: exportar → importar → exportar da el mismo archivo. Importar crea siempre una copia.

use std::{
    collections::{BTreeMap, HashMap},
    fs::{self, File},
    io::{self, BufWriter, Write},
    path::{Path, PathBuf},
};

use chrono::NaiveDate;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

use crate::{
    db::new_id,
    domain::fields::FieldType,
    error::{AppError, AppResult},
    excel::{data::file_segment, save::ensure_writable},
    media::storage::EvidenceFiles,
    repo::{cases, doc_versions, projects},
};

pub const FORMAT_VERSION: i64 = 1;
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
    /// Capturas omitidas porque falta su original o su render.
    pub missing_images: i64,
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
        let values = case
            .values
            .iter()
            .filter_map(|(field_id, value)| {
                keys.get(field_id.as_str())
                    .map(|key| (key.to_string(), value.clone()))
            })
            .collect();
        package_cases.push(PackageCase {
            code: case.code,
            title: case.title,
            status: case.status,
            created_at: case.created_at,
            updated_at: case.updated_at,
            values,
            evidence,
        });
    }
    let doc = PackageDoc {
        format_version: FORMAT_VERSION,
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
        missing_images,
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
    for (name, source) in &export.images {
        zip.start_file(name.as_str(), stored).map_err(zip_error)?;
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
}
