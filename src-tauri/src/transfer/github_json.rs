//! `github.json`: datos de GitHub de un proyecto dentro de un paquete `.qapture` (F8b).
//! Entrada opcional del ZIP: `project.json` y `FORMAT_VERSION` no cambian, y un lector que no
//! la conoce (v1.0) la ignora. Un archivo inválido se descarta entero al importar.

use std::collections::{BTreeMap, HashMap};

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::{
    error::AppResult,
    integrations::github::config,
    repo::{
        cases,
        integrations::{
            CaseIssueLink, LinkedRepo, ProjectGithub, RepoRef, get_project_github, insert_link,
            links_for_project, save_project_github,
        },
        projects,
    },
};

pub const GITHUB_JSON: &str = "github.json";
pub const VERSION: i64 = 1;
/// Aviso cuando el paquete traía un repo de evidencias (no se restaura).
pub const EVIDENCE_REPO_WARNING: &str =
    "El paquete traía un repo de evidencias; confírmalo en Configurar proyecto › GitHub.";
/// Aviso fijo cuando el `github.json` no se pudo leer o aplicar.
pub const GITHUB_WARNING: &str =
    "No se pudieron leer los datos de GitHub del paquete; el proyecto se importó sin ellos.";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubRepoJson {
    pub full_name: String,
    pub is_default: bool,
    pub default_labels: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubLinkJson {
    pub case_code: String,
    pub owner: String,
    pub repo: String,
    pub issue_number: u64,
    pub node_id: String,
    pub html_url: String,
    pub direction: String,
    pub issue_state: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubJson {
    pub version: i64,
    pub repos: Vec<GithubRepoJson>,
    pub routing_field_key: Option<String>,
    pub routing_map: BTreeMap<String, String>,
    pub evidence_repo: Option<String>,
    pub links: Vec<GithubLinkJson>,
}

/// Lo que `prepare` encontró en el ZIP.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum GithubPayload {
    #[default]
    Absent,
    Valid(GithubJson),
    /// Presente pero ilegible o inválido: se ignora y se avisa.
    Invalid,
}

impl GithubJson {
    /// Parsea y valida: cualquier repo, vínculo o versión inválidos invalidan todo el archivo.
    pub fn parse(bytes: &[u8]) -> Option<GithubJson> {
        let json: GithubJson = serde_json::from_slice(bytes).ok()?;
        json.is_valid().then_some(json)
    }

    fn is_valid(&self) -> bool {
        self.version == VERSION
            && self
                .repos
                .iter()
                .all(|r| RepoRef::parse(&r.full_name).is_some())
            && self
                .evidence_repo
                .as_deref()
                .is_none_or(|r| RepoRef::parse(r).is_some())
            && self
                .routing_map
                .values()
                .all(|r| RepoRef::parse(r).is_some())
            && self.links.iter().all(|l| {
                RepoRef::parse(&format!("{}/{}", l.owner, l.repo)).is_some()
                    // Se guarda como INTEGER (i64) de SQLite.
                    && (1..=i64::MAX as u64).contains(&l.issue_number)
                    // Un paquete ajeno no decide qué abre Qapture: solo el issue en GitHub.
                    && config::is_issue_url(&l.html_url, &l.owner, &l.repo, l.issue_number)
                    && matches!(l.direction.as_str(), "exported" | "imported")
            })
    }
}

/// `None` si el proyecto no tiene repos, ajustes ni vínculos.
pub fn collect(conn: &Connection, project_id: &str) -> AppResult<Option<GithubJson>> {
    let config = get_project_github(conn, project_id)?;
    let links = links_for_project(conn, project_id)?;
    if config == ProjectGithub::default() && links.is_empty() {
        return Ok(None);
    }
    let project = projects::get(conn, project_id)?;
    let routing_field_key = config.routing_field_id.as_deref().and_then(|id| {
        project
            .fields
            .iter()
            .find(|f| f.id == id)
            .map(|f| f.key.clone())
    });
    let code_of: HashMap<String, String> = cases::list(conn, project_id)?
        .into_iter()
        .map(|c| (c.id, c.code))
        .collect();
    Ok(Some(GithubJson {
        version: VERSION,
        repos: config
            .repos
            .iter()
            .map(|r| GithubRepoJson {
                full_name: r.repo.full_name(),
                is_default: r.is_default,
                default_labels: r.default_labels.clone(),
            })
            .collect(),
        routing_field_key,
        routing_map: config.routing_map,
        evidence_repo: config.evidence_repo.map(|r| r.full_name()),
        links: links
            .into_iter()
            .filter_map(|l| {
                let case_code = code_of.get(&l.case_id)?.clone();
                Some(GithubLinkJson {
                    case_code,
                    owner: l.owner,
                    repo: l.repo,
                    issue_number: l.issue_number,
                    node_id: l.node_id,
                    html_url: l.html_url,
                    direction: l.direction,
                    issue_state: l.issue_state,
                })
            })
            .collect(),
    }))
}

pub fn to_bytes(json: &GithubJson) -> AppResult<Vec<u8>> {
    let mut out = serde_json::to_vec_pretty(json)?;
    out.push(b'\n');
    Ok(out)
}

/// Aplica los datos al proyecto ya importado (sin repo de evidencias; un vínculo repetido para
/// el mismo caso y repo falla por la UNIQUE y descarta todo en `apply_imported`). Los vínculos se reasignan por código (los de
/// códigos desconocidos se descartan) y la columna de ruteo se busca por `key`.
/// No abre una transacción propia: `save_project_github` e `insert_link` ya usan las suyas.
pub fn apply(
    conn: &Connection,
    project_id: &str,
    json: &GithubJson,
    code_to_case: &HashMap<String, String>,
    key_to_field: &HashMap<String, String>,
) -> AppResult<()> {
    let routing_field_id = json
        .routing_field_key
        .as_ref()
        .and_then(|k| key_to_field.get(k))
        .cloned();
    let has_field = routing_field_id.is_some();
    let config = ProjectGithub {
        repos: json
            .repos
            .iter()
            .filter_map(|r| {
                Some(LinkedRepo {
                    repo: RepoRef::parse(&r.full_name)?,
                    is_default: r.is_default,
                    default_labels: r.default_labels.clone(),
                })
            })
            .collect(),
        routing_field_id,
        // Un mapa sin columna de ruteo no tiene sentido: se descarta con ella.
        routing_map: if has_field {
            json.routing_map.clone()
        } else {
            BTreeMap::new()
        },
        // Un paquete ajeno no decide adónde suben capturas: el repo de evidencias
        // se confirma a mano (ver el aviso en `bundle::insert`).
        evidence_repo: None,
    };
    save_project_github(conn, project_id, &config)?;
    for l in &json.links {
        let Some(case_id) = code_to_case.get(&l.case_code) else {
            continue;
        };
        insert_link(
            conn,
            &CaseIssueLink {
                case_id: case_id.clone(),
                owner: l.owner.clone(),
                repo: l.repo.clone(),
                issue_number: l.issue_number,
                node_id: l.node_id.clone(),
                html_url: l.html_url.clone(),
                direction: l.direction.clone(),
                issue_state: l.issue_state.clone(),
            },
        )?;
    }
    Ok(())
}

/// Importa los datos de un proyecto recién creado. Un fallo al aplicar no aborta la
/// importación: se deshace lo escrito y se devuelve `false` (el llamador suma el aviso).
pub fn apply_imported(conn: &Connection, project_id: &str, json: &GithubJson) -> bool {
    let run = || -> AppResult<()> {
        let code_to_case: HashMap<String, String> = cases::list(conn, project_id)?
            .into_iter()
            .map(|c| (c.code, c.id))
            .collect();
        let key_to_field: HashMap<String, String> = projects::get(conn, project_id)?
            .fields
            .into_iter()
            .map(|f| (f.key, f.id))
            .collect();
        apply(conn, project_id, json, &code_to_case, &key_to_field)
    };
    if run().is_ok() {
        return true;
    }
    let _ = conn.execute(
        "DELETE FROM case_issue_link WHERE case_id IN
           (SELECT id FROM test_case WHERE project_id = ?1)",
        [project_id],
    );
    let _ = conn.execute(
        "DELETE FROM project_repo_link WHERE project_id = ?1",
        [project_id],
    );
    let _ = conn.execute(
        "DELETE FROM project_gh_settings WHERE project_id = ?1",
        [project_id],
    );
    false
}
