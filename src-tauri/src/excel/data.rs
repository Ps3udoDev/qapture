use std::{collections::HashMap, ops::RangeInclusive, path::PathBuf};

use chrono::NaiveDate;
use rusqlite::Connection;

use crate::{
    domain::{fields::is_iso_date, status::STATUSES, text::required_text},
    dto::{CaseDto, DocVersionDto, ExportOptionsInput, FieldDto, ProjectDto},
    error::{AppError, AppResult, validation},
    media::storage::EvidenceFiles,
    repo::{cases, doc_versions, integrations, projects},
};

/// Ancho máximo de las capturas en la hoja Evidencias. El valor por defecto (940 px, el de la
/// Bitácora v1) lo pone el front; F6 lo hará configurable.
pub const IMAGE_WIDTH_RANGE: RangeInclusive<u32> = 400..=1600;

#[derive(Debug, Clone, PartialEq)]
pub struct ExportEvidence {
    pub description: String,
    pub render_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExportCase {
    pub case: CaseDto,
    pub evidence: Vec<ExportEvidence>,
    /// `(repo#n, url)` de los issues vinculados, por repo y número.
    pub issues: Vec<(String, String)>,
}

/// Todo lo que necesita el libro, leído en una sola pasada para soltar la base de datos
/// antes de generar el archivo. `project.fields` solo trae las columnas visibles, en orden.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportData {
    pub project: ProjectDto,
    pub cases: Vec<ExportCase>,
    pub versions: Vec<DocVersionDto>,
}

impl ExportData {
    pub fn load(conn: &Connection, files: &EvidenceFiles, project_id: &str) -> AppResult<Self> {
        let mut project = projects::get(conn, project_id)?;
        project.fields.retain(|f| !f.archived);
        project.fields.sort_by_key(|f| f.position);
        let mut stmt = conn.prepare(
            "SELECT e.case_id, e.description, e.rendered_path FROM evidence e
             JOIN test_case c ON c.id = e.case_id
             WHERE c.project_id = ?1 ORDER BY e.position, e.captured_at",
        )?;
        let rows = stmt.query_map([project_id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?;
        let mut by_case: HashMap<String, Vec<ExportEvidence>> = HashMap::new();
        for row in rows {
            let (case_id, description, rel) = row?;
            by_case.entry(case_id).or_default().push(ExportEvidence {
                description,
                render_path: files.absolute(&rel),
            });
        }
        let mut links = integrations::links_for_project(conn, project_id)?;
        links.sort_by(|a, b| (&a.repo, a.issue_number).cmp(&(&b.repo, b.issue_number)));
        let mut issues_by_case: HashMap<String, Vec<(String, String)>> = HashMap::new();
        for l in links {
            issues_by_case
                .entry(l.case_id)
                .or_default()
                .push((format!("{}#{}", l.repo, l.issue_number), l.html_url));
        }
        let cases = cases::list(conn, project_id)?
            .into_iter()
            .map(|case| ExportCase {
                evidence: by_case.remove(&case.id).unwrap_or_default(),
                issues: issues_by_case.remove(&case.id).unwrap_or_default(),
                case,
            })
            .collect();
        Ok(Self {
            project,
            cases,
            versions: doc_versions::list(conn, project_id)?,
        })
    }

    pub fn field(&self, key: &str) -> Option<&FieldDto> {
        self.project.fields.iter().find(|f| f.key == key)
    }

    pub fn value<'a>(case: &'a CaseDto, field: &FieldDto) -> &'a str {
        case.values.get(&field.id).map_or("", String::as_str)
    }

    pub fn evidence_total(&self) -> usize {
        self.cases.iter().map(|c| c.evidence.len()).sum()
    }

    /// Valores distintos y no vacíos de una columna, en orden de aparición.
    pub fn distinct_values(&self, key: &str) -> Vec<String> {
        let Some(field) = self.field(key) else {
            return Vec::new();
        };
        let mut seen = Vec::<String>::new();
        for c in &self.cases {
            let value = Self::value(&c.case, field).trim();
            if !value.is_empty() && !seen.iter().any(|s| s == value) {
                seen.push(value.to_string());
            }
        }
        seen
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Kpis {
    pub total: usize,
    /// En el orden de `STATUSES`.
    pub by_status: [usize; 6],
    /// Spec D15: Aprobado / (Aprobado + Fallido + Bloqueado); `None` si no hay veredictos.
    pub success_rate: Option<f64>,
    pub evidence: usize,
}

impl Kpis {
    pub fn of(data: &ExportData) -> Self {
        let mut by_status = [0; 6];
        for c in &data.cases {
            if let Some(i) = STATUSES.iter().position(|s| *s == c.case.status) {
                by_status[i] += 1;
            }
        }
        let mut kpis = Self {
            total: data.cases.len(),
            by_status,
            success_rate: None,
            evidence: data.evidence_total(),
        };
        let finished = kpis.count("Aprobado") + kpis.count("Fallido") + kpis.count("Bloqueado");
        if finished > 0 {
            kpis.success_rate = Some(kpis.count("Aprobado") as f64 / finished as f64);
        }
        kpis
    }

    pub fn count(&self, status: &str) -> usize {
        STATUSES
            .iter()
            .position(|s| *s == status)
            .map_or(0, |i| self.by_status[i])
    }
}

/// Opciones ya validadas y resueltas contra los datos del proyecto.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportSettings {
    pub tester: String,
    pub version: Option<DocVersionDto>,
    pub include_evidence: bool,
    pub include_summary: bool,
    pub image_max_width: u32,
    pub today: NaiveDate,
    pub template: bool,
}

pub fn resolve_options(
    data: &ExportData,
    input: &ExportOptionsInput,
    template: bool,
    today: NaiveDate,
) -> AppResult<ExportSettings> {
    let tester = required_text("Tester", &input.tester, 100)?;
    if !IMAGE_WIDTH_RANGE.contains(&input.image_max_width) {
        return Err(validation(format!(
            "El ancho máximo de imagen debe estar entre {} y {} px.",
            IMAGE_WIDTH_RANGE.start(),
            IMAGE_WIDTH_RANGE.end()
        )));
    }
    let version = match input.doc_version_id.as_deref() {
        Some(id) => Some(
            data.versions
                .iter()
                .find(|v| v.id == id)
                .cloned()
                .ok_or_else(|| AppError::NotFound("La versión del documento no existe.".into()))?,
        ),
        None if template => None,
        None => return Err(validation("Elige la versión del documento.")),
    };
    Ok(ExportSettings {
        tester,
        version,
        include_evidence: input.include_evidence && !template,
        include_summary: input.include_summary && !template,
        image_max_width: input.image_max_width,
        today,
        template,
    })
}

/// Un tramo seguro para el nombre de archivo en Windows: sin `<>:"/\|?*` ni controles,
/// espacios convertidos en `-` y sin puntos ni guiones en los extremos.
pub fn file_segment(raw: &str, max: usize) -> String {
    let mut out = String::new();
    for ch in raw.trim().chars() {
        if ch.is_whitespace() {
            if !out.ends_with('-') {
                out.push('-');
            }
        } else if !ch.is_control() && !"<>:\"/\\|?*".contains(ch) {
            out.push(ch);
        }
    }
    let limited: String = out.chars().take(max).collect();
    limited.trim_matches(['-', '.', ' ']).to_string()
}

/// `T4S_<Cliente>_<Proyecto>_Evidencias_<AAAAMMDD>_v<versión>.xlsx` (spec §6.5);
/// la plantilla vacía usa `Plantilla` y no lleva versión.
pub fn suggested_file_name(project: &ProjectDto, settings: &ExportSettings) -> String {
    let mut parts = vec!["T4S".to_string()];
    let client = file_segment(&project.client, 40);
    if !client.is_empty() {
        parts.push(client);
    }
    let name = file_segment(&project.name, 60);
    parts.push(if name.is_empty() {
        "Proyecto".into()
    } else {
        name
    });
    parts.push(
        if settings.template {
            "Plantilla"
        } else {
            "Evidencias"
        }
        .into(),
    );
    parts.push(settings.today.format("%Y%m%d").to_string());
    if let (false, Some(v)) = (settings.template, &settings.version) {
        let version = file_segment(v.version.trim_start_matches(['v', 'V']), 20);
        if !version.is_empty() {
            parts.push(format!("v{version}"));
        }
    }
    format!("{}.xlsx", parts.join("_"))
}

pub fn parse_iso(value: &str) -> Option<NaiveDate> {
    if is_iso_date(value) {
        NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()
    } else {
        None
    }
}

pub fn format_dmy(date: NaiveDate) -> String {
    date.format("%d/%m/%Y").to_string()
}

pub fn period(start: &str, end: &str) -> String {
    let show = |v: &str| parse_iso(v).map(format_dmy);
    match (show(start), show(end)) {
        (Some(s), Some(e)) => format!("{s} – {e}"),
        (Some(s), None) => format!("Desde {s}"),
        (None, Some(e)) => format!("Hasta {e}"),
        (None, None) => "—".into(),
    }
}

#[cfg(test)]
mod tests {
    use rusqlite::params;

    use super::*;
    use crate::{
        excel::testing::{case, fixture, version},
        media::storage::TempDir,
        repo::{
            doc_versions::tests::input,
            test_support::{case as db_case, conn, field_id, project},
        },
    };

    fn options() -> ExportOptionsInput {
        ExportOptionsInput {
            tester: " Ana Pérez ".into(),
            doc_version_id: Some("ver-1".into()),
            include_evidence: true,
            include_summary: true,
            image_max_width: 940,
        }
    }

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 16).unwrap()
    }

    #[test]
    fn load_reads_visible_fields_cases_evidence_and_versions() {
        let conn = conn();
        let tmp = TempDir::new();
        let files = tmp.files();
        let p = project(&conn);
        conn.execute(
            "UPDATE project_field SET archived = 1 WHERE id = ?1",
            [field_id(&p, "menu")],
        )
        .unwrap();
        let first = db_case(&conn, &p);
        let second = db_case(&conn, &p);
        for (id, position, description) in [("e2", 1, "Detalle"), ("e1", 0, "Listado")] {
            conn.execute(
                "INSERT INTO evidence (id, case_id, position, description, original_path, rendered_path,
                                       annotations, width, height, captured_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, '', ?5, '{}', 10, 10, '', '')",
                params![id, first.id, position, description, format!("evidence/{}/{id}.render.png", p.id)],
            )
            .unwrap();
        }
        doc_versions::create(&conn, &p.id, &input("1.0", "2026-10-01")).unwrap();

        let data = ExportData::load(&conn, &files, &p.id).unwrap();
        assert_eq!(data.project.fields.len(), 10);
        assert!(data.field("menu").is_none());
        assert_eq!(data.cases.len(), 2);
        assert_eq!(data.cases[1].case.id, second.id);
        let descriptions: Vec<_> = data.cases[0]
            .evidence
            .iter()
            .map(|e| e.description.as_str())
            .collect();
        assert_eq!(descriptions, ["Listado", "Detalle"]);
        assert_eq!(
            data.cases[0].evidence[0].render_path,
            files.absolute(&format!("evidence/{}/e1.render.png", p.id))
        );
        assert!(data.cases[1].evidence.is_empty());
        assert_eq!(data.versions.len(), 1);
        assert_eq!(data.evidence_total(), 2);
        assert_eq!(
            ExportData::load(&conn, &files, "nope").unwrap_err().code(),
            "not_found"
        );
    }

    #[test]
    fn load_reads_issue_links_ordered_by_repo() {
        let conn = conn();
        let tmp = TempDir::new();
        let p = project(&conn);
        let c = db_case(&conn, &p);
        db_case(&conn, &p);
        for (repo, n) in [("web", 2u64), ("lib", 10), ("app", 9)] {
            integrations::insert_link(
                &conn,
                &integrations::CaseIssueLink {
                    case_id: c.id.clone(),
                    owner: "o".into(),
                    repo: repo.into(),
                    issue_number: n,
                    node_id: format!("I_{repo}{n}"),
                    html_url: format!("https://github.com/o/{repo}/issues/{n}"),
                    direction: "exported".into(),
                    issue_state: "open".into(),
                },
            )
            .unwrap();
        }
        let data = ExportData::load(&conn, &tmp.files(), &p.id).unwrap();
        let texts: Vec<_> = data.cases[0].issues.iter().map(|i| i.0.as_str()).collect();
        assert_eq!(texts, ["app#9", "lib#10", "web#2"]);
        assert_eq!(
            data.cases[0].issues[0].1,
            "https://github.com/o/app/issues/9"
        );
        assert!(data.cases[1].issues.is_empty());
    }

    #[test]
    fn kpis_follow_the_success_rate_rule() {
        let mut data = fixture();
        let k = Kpis::of(&data);
        assert_eq!(k.total, 4);
        assert_eq!(k.count("Aprobado"), 2);
        assert_eq!(k.count("Fallido"), 1);
        assert_eq!(k.success_rate, Some(2.0 / 3.0));
        assert_eq!(k.evidence, 3);
        data.cases.retain(|c| c.case.status == "No ejecutado");
        assert_eq!(Kpis::of(&data).success_rate, None);
        data.cases.clear();
        assert_eq!(Kpis::of(&data).total, 0);
    }

    #[test]
    fn distinct_values_keep_first_appearance_and_skip_blanks() {
        let mut data = fixture();
        assert_eq!(data.distinct_values("entorno"), ["QA", "UAT"]);
        data.cases.push(case(
            &data.project,
            "EJ-005",
            "Aprobado",
            &[("entorno", "  ")],
        ));
        assert_eq!(data.distinct_values("entorno"), ["QA", "UAT"]);
        assert!(data.distinct_values("no_existe").is_empty());
    }

    #[test]
    fn resolve_options_validates_and_trims() {
        let mut data = fixture();
        data.versions.push(version("ver-1", "1.0", "2026-10-01"));
        let s = resolve_options(&data, &options(), false, today()).unwrap();
        assert_eq!(s.tester, "Ana Pérez");
        assert_eq!(s.version.as_ref().unwrap().version, "1.0");
        assert!(s.include_evidence && s.include_summary);

        let mut bad = options();
        bad.tester = " ".into();
        assert_eq!(
            resolve_options(&data, &bad, false, today())
                .unwrap_err()
                .to_string(),
            "«Tester» es obligatorio."
        );
        let mut bad = options();
        bad.image_max_width = 399;
        assert_eq!(
            resolve_options(&data, &bad, false, today())
                .unwrap_err()
                .to_string(),
            "El ancho máximo de imagen debe estar entre 400 y 1600 px."
        );
        let mut bad = options();
        bad.doc_version_id = None;
        assert_eq!(
            resolve_options(&data, &bad, false, today())
                .unwrap_err()
                .to_string(),
            "Elige la versión del documento."
        );
        bad.doc_version_id = Some("otra".into());
        assert_eq!(
            resolve_options(&data, &bad, false, today())
                .unwrap_err()
                .code(),
            "not_found"
        );
    }

    #[test]
    fn templates_need_no_version_and_skip_evidence_and_summary() {
        let data = fixture();
        let mut o = options();
        o.doc_version_id = None;
        let s = resolve_options(&data, &o, true, today()).unwrap();
        assert!(s.version.is_none());
        assert!(!s.include_evidence && !s.include_summary);
    }

    #[test]
    fn file_segments_are_safe_for_windows() {
        assert_eq!(file_segment("  Pólizas  de   Auto ", 60), "Pólizas-de-Auto");
        assert_eq!(file_segment("a<b>c:d\"e/f\\g|h?i*j", 60), "abcdefghij");
        assert_eq!(file_segment("...Recibos. ", 60), "Recibos");
        assert_eq!(file_segment("línea\nnueva\ttab", 60), "línea-nueva-tab");
        assert_eq!(file_segment(&"x".repeat(80), 40).len(), 40);
        assert_eq!(file_segment("???", 40), "");
    }

    #[test]
    fn suggested_names_follow_the_t4s_convention() {
        let mut data = fixture();
        data.versions.push(version("ver-1", "v1.0", "2026-10-01"));
        let s = resolve_options(&data, &options(), false, today()).unwrap();
        assert_eq!(
            suggested_file_name(&data.project, &s),
            "T4S_ACME_Recibos_Evidencias_20261016_v1.0.xlsx"
        );
        data.project.client = String::new();
        data.project.name = "Pólizas / Auto".into();
        assert_eq!(
            suggested_file_name(&data.project, &s),
            "T4S_Pólizas-Auto_Evidencias_20261016_v1.0.xlsx"
        );
        let mut o = options();
        o.doc_version_id = None;
        let t = resolve_options(&data, &o, true, today()).unwrap();
        assert_eq!(
            suggested_file_name(&data.project, &t),
            "T4S_Pólizas-Auto_Plantilla_20261016.xlsx"
        );
        data.project.name = "***".into();
        assert_eq!(
            suggested_file_name(&data.project, &t),
            "T4S_Proyecto_Plantilla_20261016.xlsx"
        );
    }

    #[test]
    fn periods_and_dates() {
        assert_eq!(
            period("2026-10-01", "2026-10-31"),
            "01/10/2026 – 31/10/2026"
        );
        assert_eq!(period("2026-10-01", ""), "Desde 01/10/2026");
        assert_eq!(period("", "2026-10-31"), "Hasta 31/10/2026");
        assert_eq!(period("", "basura"), "—");
        assert_eq!(parse_iso("2026-02-30"), None);
        assert_eq!(format_dmy(today()), "16/10/2026");
    }

    #[test]
    fn values_use_field_ids() {
        let conn = conn();
        let p = project(&conn);
        let c = db_case(&conn, &p);
        let entorno = p.fields.iter().find(|f| f.key == "entorno").unwrap();
        let modulo = p.fields.iter().find(|f| f.key == "modulo").unwrap();
        assert_eq!(ExportData::value(&c, entorno), "QA");
        assert_eq!(ExportData::value(&c, modulo), "");
    }
}
