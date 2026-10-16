//! Ayudas de prueba del libro: datos ficticios en memoria y lectura del xlsx con `calamine`
//! (celdas) y `zip` (vínculos, imágenes y gráficos, que calamine no expone).

use std::{
    collections::BTreeMap,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use calamine::{Data, Reader, Xlsx, open_workbook};
use rust_xlsxwriter::Workbook;

use super::data::{ExportCase, ExportData, ExportEvidence};
use crate::{
    domain::fields::FieldType,
    dto::{CaseDto, DocVersionDto, FieldDto, ProjectDto},
    media::storage::TempDir,
};

fn field(key: &str, label: &str, field_type: FieldType, options: &[&str], width: i64) -> FieldDto {
    FieldDto {
        id: format!("fld-{key}"),
        key: key.into(),
        label: label.into(),
        field_type,
        options: options.iter().map(|o| o.to_string()).collect(),
        required: false,
        clear_on_duplicate: false,
        width,
        position: 0,
        archived: false,
    }
}

pub fn project() -> ProjectDto {
    let mut fields = vec![
        field("entorno", "Entorno", FieldType::Text, &[], 21),
        field("modulo", "Módulo", FieldType::Text, &[], 22),
        field(
            "resultado_esperado",
            "Resultado esperado",
            FieldType::Longtext,
            &[],
            44,
        ),
        field(
            "resultado_obtenido",
            "Resultado obtenido",
            FieldType::Longtext,
            &[],
            44,
        ),
        field(
            "fecha_ejecucion",
            "Fecha de ejecución",
            FieldType::Date,
            &[],
            15,
        ),
        field(
            "severidad",
            "Severidad",
            FieldType::List,
            &["Alta", "Media", "Baja"],
            12,
        ),
        field("aceptado", "Aceptado", FieldType::Boolean, &[], 12),
        field("horas", "Horas", FieldType::Number, &[], 10),
    ];
    for (position, f) in fields.iter_mut().enumerate() {
        f.position = position as i64;
    }
    ProjectDto {
        id: "prj-1".into(),
        name: "Recibos".into(),
        client: "ACME".into(),
        test_type: "Funcional".into(),
        release_version: "2.4.0".into(),
        start_date: "2026-10-01".into(),
        end_date: "2026-10-31".into(),
        next_sequence: 5,
        source_template_id: Some("tpl-funcional".into()),
        created_at: String::new(),
        updated_at: String::new(),
        fields,
    }
}

/// Un caso con valores por `key` de columna.
pub fn case(project: &ProjectDto, code: &str, status: &str, values: &[(&str, &str)]) -> ExportCase {
    let values: BTreeMap<String, String> = values
        .iter()
        .map(|(key, value)| (format!("fld-{key}"), value.to_string()))
        .collect();
    ExportCase {
        case: CaseDto {
            id: format!("case-{code}"),
            project_id: project.id.clone(),
            code: code.into(),
            title: format!("Caso {code}"),
            status: status.into(),
            values,
            evidence_count: 0,
            created_at: String::new(),
            updated_at: String::new(),
        },
        evidence: Vec::new(),
    }
}

pub fn evidence(description: &str, path: &Path) -> ExportEvidence {
    ExportEvidence {
        description: description.into(),
        render_path: path.to_path_buf(),
    }
}

pub fn version(id: &str, version: &str, date: &str) -> DocVersionDto {
    DocVersionDto {
        id: id.into(),
        project_id: "prj-1".into(),
        version: version.into(),
        date: date.into(),
        author: "Ana Pérez".into(),
        reviewer: "Luis Gómez".into(),
        approver: String::new(),
        changes: "Versión inicial".into(),
        created_at: String::new(),
    }
}

/// 4 casos (2 aprobados, 1 fallido, 1 sin ejecutar) y 3 evidencias con rutas inexistentes:
/// sirve para todo lo que no lee imágenes.
pub fn fixture() -> ExportData {
    let project = project();
    let mut first = case(
        &project,
        "EJ-001",
        "Aprobado",
        &[
            ("entorno", "QA"),
            ("modulo", "Cobranzas"),
            ("resultado_esperado", "Se aplica el recibo"),
            ("resultado_obtenido", "Se aplicó el recibo"),
            ("fecha_ejecucion", "2026-10-16"),
            ("severidad", "Alta"),
            ("aceptado", "true"),
            ("horas", "1.5"),
        ],
    );
    first.evidence = vec![
        evidence("Listado de recibos", Path::new("no-existe-1.png")),
        evidence("Detalle del recibo", Path::new("no-existe-2.png")),
    ];
    let mut second = case(
        &project,
        "EJ-002",
        "Aprobado",
        &[("entorno", "UAT"), ("modulo", "Cobranzas")],
    );
    second.evidence = vec![evidence("Recibo en UAT", Path::new("no-existe-3.png"))];
    let third = case(
        &project,
        "EJ-003",
        "Fallido",
        &[("entorno", "QA"), ("modulo", "Pólizas")],
    );
    let fourth = case(&project, "EJ-004", "No ejecutado", &[("entorno", "QA")]);
    ExportData {
        project,
        cases: vec![first, second, third, fourth],
        versions: Vec::new(),
    }
}

/// PNG sintético con un patrón que cambia según `seed` (imágenes distintas no se deduplican en el xlsx).
pub fn write_png(dir: &Path, name: &str, width: u32, height: u32, seed: u8) -> PathBuf {
    let img = image::RgbImage::from_fn(width, height, |x, y| {
        image::Rgb([(x % 256) as u8, (y % 256) as u8, seed])
    });
    let path = dir.join(name);
    img.save(&path).expect("png de prueba");
    path
}

/// Un xlsx guardado en disco, listo para inspeccionar.
pub struct Book {
    path: PathBuf,
    _dir: Option<TempDir>,
}

impl Book {
    pub fn save(workbook: &mut Workbook) -> Self {
        let dir = TempDir::new();
        let path = dir.path().join("libro.xlsx");
        workbook.save(&path).expect("guardar xlsx");
        Self {
            path,
            _dir: Some(dir),
        }
    }

    pub fn at(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
            _dir: None,
        }
    }

    fn reader(&self) -> Xlsx<std::io::BufReader<File>> {
        open_workbook(&self.path).expect("calamine abre el libro")
    }

    pub fn sheet_names(&self) -> Vec<String> {
        self.reader().sheet_names()
    }

    pub fn cell(&self, sheet: &str, row: u32, col: u32) -> Data {
        self.reader()
            .worksheet_range(sheet)
            .expect("la hoja existe")
            .get_value((row, col))
            .cloned()
            .unwrap_or(Data::Empty)
    }

    pub fn text(&self, sheet: &str, row: u32, col: u32) -> String {
        self.cell(sheet, row, col).to_string()
    }

    pub fn row_texts(&self, sheet: &str, row: u32, cols: u32) -> Vec<String> {
        (0..cols).map(|col| self.text(sheet, row, col)).collect()
    }

    /// Filas con datos según calamine (la última fila usada + 1).
    pub fn height(&self, sheet: &str) -> u32 {
        self.reader()
            .worksheet_range(sheet)
            .expect("la hoja existe")
            .end()
            .map_or(0, |(row, _)| row + 1)
    }

    /// Busca la primera celda con ese texto exacto en la columna A.
    pub fn find_row(&self, sheet: &str, text: &str) -> Option<u32> {
        let range = self
            .reader()
            .worksheet_range(sheet)
            .expect("la hoja existe");
        let (end, _) = range.end()?;
        (0..=end).find(|row| {
            range
                .get_value((*row, 0))
                .is_some_and(|v| matches!(v, Data::String(s) if s == text))
        })
    }

    pub fn has_part(&self, name: &str) -> bool {
        let mut zip = zip::ZipArchive::new(File::open(&self.path).unwrap()).unwrap();
        zip.by_name(name).is_ok()
    }

    pub fn part(&self, name: &str) -> String {
        let mut zip = zip::ZipArchive::new(File::open(&self.path).unwrap()).unwrap();
        let mut out = String::new();
        zip.by_name(name)
            .unwrap_or_else(|_| panic!("falta {name}"))
            .read_to_string(&mut out)
            .unwrap();
        out
    }

    fn sheet_number(&self, sheet: &str) -> usize {
        self.sheet_names()
            .iter()
            .position(|s| s == sheet)
            .unwrap_or_else(|| panic!("no existe la hoja {sheet}"))
            + 1
    }

    pub fn sheet_xml(&self, sheet: &str) -> String {
        self.part(&format!(
            "xl/worksheets/sheet{}.xml",
            self.sheet_number(sheet)
        ))
    }

    /// `(celda, destino)` de cada vínculo interno de la hoja.
    pub fn hyperlinks(&self, sheet: &str) -> Vec<(String, String)> {
        self.sheet_xml(sheet)
            .split("<hyperlink ")
            .skip(1)
            .filter_map(|tag| Some((attr(tag, "ref")?, attr(tag, "location")?)))
            .collect()
    }

    fn drawing(&self, sheet: &str) -> Option<String> {
        let rels = format!(
            "xl/worksheets/_rels/sheet{}.xml.rels",
            self.sheet_number(sheet)
        );
        if !self.has_part(&rels) {
            return None;
        }
        let rels = self.part(&rels);
        let target = rels
            .split("Target=\"../drawings/")
            .nth(1)?
            .split('"')
            .next()?
            .to_string();
        Some(self.part(&format!("xl/drawings/{target}")))
    }

    pub fn pictures(&self, sheet: &str) -> usize {
        self.drawing(sheet)
            .map_or(0, |xml| xml.matches("<xdr:pic>").count())
    }

    pub fn charts(&self, sheet: &str) -> usize {
        self.drawing(sheet)
            .map_or(0, |xml| xml.matches("<xdr:graphicFrame").count())
    }
}

fn attr(tag: &str, name: &str) -> Option<String> {
    let key = format!("{name}=\"");
    let start = tag.find(&key)? + key.len();
    let end = tag[start..].find('"')? + start;
    Some(tag[start..end].to_string())
}
