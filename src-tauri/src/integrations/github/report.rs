//! Reporte de un caso como issue (o comentario) de GitHub: saneamiento del cuerpo, subida de
//! capturas al repo de evidencias (o modo asistido) y vínculo caso ↔ issue.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use chrono::NaiveDateTime;

use super::evidence_upload::{self, Upload};
use super::issues::{self, NewIssue};
use crate::domain::text::required_text;
use crate::error::{AppError, AppResult, GhErrorKind, gh, validation};
use crate::repo::integrations::{
    CaseIssueLink, RepoRef, conflict, find_link, get_account, get_project_github, insert_link,
};
use crate::repo::{cases, evidence, projects};
use crate::state::AppState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssistedReason {
    NoEvidenceRepo,
    CannotWrite(RepoRef),
    NotInstalled(RepoRef),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UploadMode {
    Repo,
    Assisted(AssistedReason),
}

#[derive(Debug, Clone)]
pub struct ReportInput {
    pub case_id: String,
    pub repo: RepoRef,
    pub title: String,
    pub body: String,
    pub labels: Vec<String>,
    pub assignees: Vec<String>,
    pub milestone: Option<u64>,
    pub evidence_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportOutcome {
    pub link: CaseIssueLink,
    pub mode: UploadMode,
    /// Capturas que quedan por pegar a mano (modo asistido).
    pub pending: Vec<String>,
}

// --- cuerpo ------------------------------------------------------------------------------

const EVIDENCE_HEADING: &str = "### Evidencias";
const HIDDEN_PREFIX: &str = "<!-- qapture:case=";

fn hidden_marker(code: &str) -> String {
    format!("{HIDDEN_PREFIX}{code};v=1 -->")
}

fn marker(n: usize) -> String {
    format!("{{{{img:{n}}}}}")
}

/// Marcadores `{{img:n}}` del texto, tolerando espacios y mayúsculas (`{{ IMG : 2 }}`):
/// `(inicio, fin, n)`. Un `n` que no cabe en `usize` se devuelve como 0 (fuera de rango).
fn find_markers(text: &str) -> Vec<(usize, usize, usize)> {
    let bytes = text.as_bytes();
    let skip_ws = |mut i: usize| {
        while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
            i += 1;
        }
        i
    };
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(offset) = text[from..].find("{{") {
        let start = from + offset;
        let parsed = (|| {
            let mut i = skip_ws(start + 2);
            if !text.get(i..i + 3)?.eq_ignore_ascii_case("img") {
                return None;
            }
            i = skip_ws(i + 3);
            (bytes.get(i)? == &b':').then_some(())?;
            i = skip_ws(i + 1);
            let digits = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if i == digits {
                return None;
            }
            let n = text[digits..i].parse::<usize>().unwrap_or(0);
            i = skip_ws(i);
            (text.get(i..i + 2)? == "}}").then_some((i + 2, n))
        })();
        match parsed {
            Some((end, n)) => {
                found.push((start, end, n));
                from = end;
            }
            None => from = start + 2,
        }
    }
    found
}

fn is_blank(line: &str) -> bool {
    line.trim().is_empty()
}

/// Inserta `block` en `at`, separado por una línea en blanco de lo que haya antes y después.
fn insert_block(lines: Vec<String>, at: usize, block: Vec<String>) -> Vec<String> {
    let mut head = lines[..at].to_vec();
    while head.last().is_some_and(|l| is_blank(l)) {
        head.pop();
    }
    let tail: Vec<String> = lines[at..]
        .iter()
        .skip_while(|l| is_blank(l))
        .cloned()
        .collect();
    let mut out = head;
    if !out.is_empty() {
        out.push(String::new());
    }
    out.extend(block);
    if !tail.is_empty() {
        out.push(String::new());
        out.extend(tail);
    }
    out
}

/// Saneamiento previo a enviar (Global Constraints):
/// (a) quita todo `{{img:n}}` fuera de 1..=`chosen` (y los repetidos: queda uno por `n`);
/// (b) agrega al final de `### Evidencias` (que crea si falta) cada `n` elegido ausente;
/// (c) agrega el comentario oculto del caso si falta.
pub fn sanitize_body(body: &str, code: &str, chosen: usize) -> String {
    let mut seen = vec![false; chosen + 1];
    let mut lines: Vec<String> = Vec::new();
    for line in body.split('\n') {
        let markers = find_markers(line);
        if markers.is_empty() {
            lines.push(line.to_string());
            continue;
        }
        let mut rebuilt = String::new();
        let mut last = 0;
        for (start, end, n) in markers {
            rebuilt.push_str(&line[last..start]);
            if (1..=chosen).contains(&n) && !seen[n] {
                seen[n] = true;
                rebuilt.push_str(&marker(n));
            }
            last = end;
        }
        rebuilt.push_str(&line[last..]);
        // Una línea que solo tenía marcadores descartados desaparece.
        if !is_blank(&rebuilt) {
            lines.push(rebuilt.trim_end_matches([' ', '\t']).to_string());
        }
    }

    let missing: Vec<usize> = (1..=chosen).filter(|n| !seen[*n]).collect();
    if !missing.is_empty() {
        let mut block = Vec::new();
        for (i, n) in missing.into_iter().enumerate() {
            if i > 0 {
                block.push(String::new());
            }
            block.push(format!("**{n}.**"));
            block.push(marker(n));
        }
        let heading = lines.iter().position(|l| l.trim() == EVIDENCE_HEADING);
        lines = match heading {
            Some(h) => {
                let end = lines[h + 1..]
                    .iter()
                    .position(|l| l.trim_start().starts_with('#') || l.contains(HIDDEN_PREFIX))
                    .map_or(lines.len(), |p| h + 1 + p);
                insert_block(lines, end, block)
            }
            None => {
                let last = lines.iter().rposition(|l| !is_blank(l));
                let at = match last {
                    Some(i) if lines[i].contains(HIDDEN_PREFIX) => i,
                    _ => lines.len(),
                };
                let mut section = vec![EVIDENCE_HEADING.to_string(), String::new()];
                section.extend(block);
                insert_block(lines, at, section)
            }
        };
    }

    let mut text = lines.join("\n");
    let hidden = hidden_marker(code);
    if !text.contains(&hidden) {
        text = format!("{}\n\n{hidden}", text.trim_end());
    }
    text
}

/// `{{img:n}}` → `![<code> · n](url)` si se subió; en modo asistido (o sin URL para `n`), el
/// texto pendiente.
pub fn fill_images(body: &str, code: &str, urls: Option<&[String]>) -> String {
    let mut out = String::with_capacity(body.len());
    let mut last = 0;
    for (start, end, n) in find_markers(body) {
        out.push_str(&body[last..start]);
        let url = urls.and_then(|u| u.get(n.wrapping_sub(1)));
        match url {
            Some(url) => out.push_str(&format!("![{code} · {n}]({url})")),
            None => out.push_str(&format!("> 📎 Evidencia {n}: pendiente de pegar")),
        }
        last = end;
    }
    out.push_str(&body[last..]);
    out
}

// --- validación --------------------------------------------------------------------------

const MAX_TITLE: usize = 256;
const MAX_BODY: usize = 65_536;
const MAX_PEOPLE_OR_LABELS: usize = 10;
const MAX_COMMIT_TITLE: usize = 50;

fn validate_text(input: &ReportInput, with_title: bool) -> AppResult<()> {
    if with_title {
        required_text("Título", &input.title, MAX_TITLE)?;
    }
    if input.body.chars().count() > MAX_BODY {
        return Err(validation("La descripción supera 65 536 caracteres."));
    }
    if input.labels.len() > MAX_PEOPLE_OR_LABELS {
        return Err(validation("Se permiten hasta 10 etiquetas."));
    }
    if input.assignees.len() > MAX_PEOPLE_OR_LABELS {
        return Err(validation("Se permiten hasta 10 asignados."));
    }
    Ok(())
}

// --- lectura de la base (sin `.await`) ----------------------------------------------------

struct Shot {
    id: String,
    rendered: PathBuf,
}

/// Todo lo que el reporte necesita de la base, leído de una vez y con el guard ya suelto.
struct Snapshot {
    code: String,
    case_title: String,
    project_name: String,
    /// Único repo donde se escribe: el guardado en el proyecto del caso (la barrera).
    evidence_repo: Option<RepoRef>,
    link: Option<CaseIssueLink>,
    shots: Vec<Shot>,
    login: Option<String>,
}

fn snapshot(state: &AppState, input: &ReportInput) -> AppResult<Snapshot> {
    let conn = state.db();
    let case = cases::get(&conn, &input.case_id)?;
    let project = projects::get(&conn, &case.project_id)?;
    let config = get_project_github(&conn, &case.project_id)?;
    let evidence = evidence::list(&conn, state.files(), &case.id)?;
    if input
        .evidence_ids
        .iter()
        .any(|id| !evidence.iter().any(|e| &e.id == id))
    {
        return Err(validation(
            "Una de las capturas elegidas no pertenece al caso.",
        ));
    }
    // En el orden del caso, solo las elegidas: su posición aquí es `n`.
    let shots = evidence
        .into_iter()
        .filter(|e| input.evidence_ids.contains(&e.id))
        .map(|e| Shot {
            id: e.id,
            rendered: PathBuf::from(e.rendered_path),
        })
        .collect();
    Ok(Snapshot {
        link: find_link(&conn, &case.id, &input.repo)?,
        login: get_account(&conn)?.map(|a| a.login),
        code: case.code,
        case_title: case.title,
        project_name: project.name,
        evidence_repo: config.evidence_repo,
        shots,
    })
}

fn tester(state: &AppState, snap: &Snapshot) -> String {
    let configured = state.settings().profile.tester.trim().to_string();
    if configured.is_empty() {
        snap.login.clone().unwrap_or_default()
    } else {
        configured
    }
}

// --- subida -------------------------------------------------------------------------------

/// 403 → la cuenta no puede escribir; 404 → la app no está instalada. Lo demás aborta.
fn assisted_reason(error: &AppError, repo: &RepoRef) -> Option<AssistedReason> {
    match error {
        AppError::GitHub {
            kind: GhErrorKind::Forbidden,
            ..
        } => Some(AssistedReason::CannotWrite(repo.clone())),
        AppError::GitHub {
            kind: GhErrorKind::NotFound,
            ..
        } => Some(AssistedReason::NotInstalled(repo.clone())),
        _ => None,
    }
}

/// Sube las capturas elegidas (si se puede) y devuelve el modo y sus URL (`None` = asistido).
async fn upload(
    state: &AppState,
    token: &str,
    snap: &Snapshot,
    now: NaiveDateTime,
) -> AppResult<(UploadMode, Option<Vec<String>>)> {
    if snap.shots.is_empty() {
        return Ok((UploadMode::Repo, Some(Vec::new())));
    }
    let assisted = |reason| Ok((UploadMode::Assisted(reason), None));
    let Some(repo) = snap.evidence_repo.as_ref() else {
        return assisted(AssistedReason::NoEvidenceRepo);
    };
    let client = &state.github().client;
    match issues::repo_info(client, token, repo).await {
        Ok(info) if !info.can_push => return assisted(AssistedReason::CannotWrite(repo.clone())),
        Ok(_) => {}
        Err(e) => {
            return match assisted_reason(&e, repo) {
                Some(reason) => assisted(reason),
                None => Err(e),
            };
        }
    }

    let tester = tester(state, snap);
    let dir = evidence_upload::evidence_dir(&snap.project_name, &snap.code, now, &tester);
    let mut files = Vec::with_capacity(snap.shots.len());
    for (i, shot) in snap.shots.iter().enumerate() {
        files.push(Upload {
            path: format!("{dir}/{}.png", i + 1),
            png: fs::read(&shot.rendered)?,
        });
    }
    let title: String = snap.case_title.chars().take(MAX_COMMIT_TITLE).collect();
    let message = format!("qapture: {} {} ({tester})", snap.code, title.trim_end());
    match evidence_upload::commit_files(client, token, repo, &files, &message).await {
        Ok(sha) => {
            let web = client.web_base();
            let urls = files
                .iter()
                .map(|f| evidence_upload::image_url(web, repo, &sha, &f.path))
                .collect();
            Ok((UploadMode::Repo, Some(urls)))
        }
        Err(e) => match assisted_reason(&e, repo) {
            Some(reason) => assisted(reason),
            None => Err(e),
        },
    }
}

/// Sanea el cuerpo, sube (o no) las capturas y deja el texto final listo para GitHub.
async fn prepare(
    state: &AppState,
    token: &str,
    snap: &Snapshot,
    body: &str,
    now: NaiveDateTime,
) -> AppResult<(String, UploadMode, Vec<String>)> {
    let (mode, urls) = upload(state, token, snap, now).await?;
    let sanitized = sanitize_body(body, &snap.code, snap.shots.len());
    let text = fill_images(&sanitized, &snap.code, urls.as_deref());
    let pending = match mode {
        UploadMode::Repo => Vec::new(),
        UploadMode::Assisted(_) => snap.shots.iter().map(|s| s.id.clone()).collect(),
    };
    Ok((text, mode, pending))
}

// --- un solo reporte a la vez por (caso, repo) -------------------------------------------

static IN_FLIGHT: Mutex<BTreeSet<String>> = Mutex::new(BTreeSet::new());

/// Reserva (caso, repo) mientras se crea el issue: un doble clic o una segunda ventana no
/// llegan a crear otro antes de que exista el vínculo.
struct InFlight(String);

impl InFlight {
    fn claim(kind: &str, case_id: &str, repo: &RepoRef) -> Option<Self> {
        let key = format!(
            "{kind}|{case_id}|{}/{}",
            repo.owner.to_lowercase(),
            repo.repo.to_lowercase()
        );
        let claimed = IN_FLIGHT
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .insert(key.clone());
        // El candado ya se soltó: si no se reservó, construir y soltar un `InFlight` aquí
        // borraría la reserva ajena (y con el candado tomado, se bloquearía).
        if claimed { Some(Self(key)) } else { None }
    }
}

impl Drop for InFlight {
    fn drop(&mut self) {
        IN_FLIGHT
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .remove(&self.0);
    }
}

// --- operaciones ---------------------------------------------------------------------------

/// Crea el issue del caso en `input.repo`. Nunca retiene el guard de la base durante un `.await`.
pub async fn create_issue(
    state: &AppState,
    input: ReportInput,
    now: NaiveDateTime,
) -> AppResult<ReportOutcome> {
    validate_text(&input, true)?;
    // Se reserva antes de leer la base: así el vínculo que se lee no puede quedar viejo
    // por otro reporte del mismo (caso, repo) que termine mientras tanto.
    let Some(_claim) = InFlight::claim("create", &input.case_id, &input.repo) else {
        return Err(gh(
            GhErrorKind::Conflict,
            format!(
                "Ya se está creando un issue de este caso en {}.",
                input.repo.full_name()
            ),
        ));
    };
    let snap = snapshot(state, &input)?;
    let token = state.github().token()?;
    if let Some(existing) = &snap.link {
        return Err(conflict(existing));
    }

    let (body, mode, pending) = prepare(state, &token, &snap, &input.body, now).await?;
    let issue = NewIssue {
        title: input.title.trim().to_string(),
        body,
        labels: input.labels,
        assignees: input.assignees,
        milestone: input.milestone,
    };
    let created = issues::create(&state.github().client, &token, &input.repo, &issue).await?;
    let link = CaseIssueLink {
        case_id: input.case_id,
        owner: input.repo.owner,
        repo: input.repo.repo,
        issue_number: created.number,
        node_id: created.node_id,
        html_url: created.html_url,
        direction: "exported".to_string(),
        issue_state: created.state,
    };
    insert_link(&state.db(), &link)?;
    Ok(ReportOutcome {
        link,
        mode,
        pending,
    })
}

/// Comenta en el issue ya vinculado al caso en `input.repo` (el título no se usa).
/// Devuelve la URL del comentario, el modo de subida y las capturas pendientes.
pub async fn comment_issue(
    state: &AppState,
    input: ReportInput,
    now: NaiveDateTime,
) -> AppResult<(String, UploadMode, Vec<String>)> {
    validate_text(&input, false)?;
    // Un doble clic en «Comentar» publica un solo comentario.
    let Some(_claim) = InFlight::claim("comment", &input.case_id, &input.repo) else {
        return Err(gh(
            GhErrorKind::Conflict,
            format!(
                "Ya se está comentando este caso en {}.",
                input.repo.full_name()
            ),
        ));
    };
    let snap = snapshot(state, &input)?;
    let token = state.github().token()?;
    let Some(link) = &snap.link else {
        return Err(gh(
            GhErrorKind::NotFound,
            format!("El caso no tiene un issue en {}.", input.repo.full_name()),
        ));
    };
    let (body, mode, pending) = prepare(state, &token, &snap, &input.body, now).await?;
    let url = issues::comment(
        &state.github().client,
        &token,
        &link.repo_ref(),
        link.issue_number,
        &body,
    )
    .await?;
    Ok((url, mode, pending))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;
    use mockito::{Matcher, Mock, ServerGuard};
    use rusqlite::params;
    use serde_json::json;

    use super::*;
    use crate::{
        db,
        media::storage::{EvidenceFiles, TempDir},
        repo::{
            integrations::{
                LinkedRepo, ProjectGithub, find_link, insert_link, save_project_github,
            },
            test_support,
        },
        settings::Settings,
        state::GithubState,
    };

    const HIDDEN: &str = "<!-- qapture:case=EJ-1;v=1 -->";

    fn count(text: &str, needle: &str) -> usize {
        text.matches(needle).count()
    }

    // --- saneamiento -------------------------------------------------------------------

    #[test]
    fn sanitize_body_drops_out_of_range_markers() {
        let body =
            format!("Hola {{{{img:9}}}}\n{{{{img:0}}}}\n### Evidencias\n{{{{img:1}}}}\n{HIDDEN}");
        let out = sanitize_body(&body, "EJ-1", 1);
        assert!(!out.contains("img:9") && !out.contains("img:0"), "{out}");
        assert_eq!(count(&out, "{{img:1}}"), 1);
        assert!(
            out.starts_with("Hola\n"),
            "el texto de la línea se conserva: {out}"
        );
        assert_eq!(
            out,
            format!("Hola\n### Evidencias\n{{{{img:1}}}}\n{HIDDEN}"),
            "una línea que solo tenía un marcador inválido desaparece"
        );
    }

    #[test]
    fn sanitize_body_appends_missing_chosen_images_under_evidencias() {
        let body = format!(
            "Pasos\n\n### Evidencias\n\n**1.** Login\n{{{{img:1}}}}\n\n### Notas\n\nnada\n\n{HIDDEN}"
        );
        let out = sanitize_body(&body, "EJ-1", 3);
        assert_eq!(
            out,
            format!(
                "Pasos\n\n### Evidencias\n\n**1.** Login\n{{{{img:1}}}}\n\n**2.**\n{{{{img:2}}}}\n\n**3.**\n{{{{img:3}}}}\n\n### Notas\n\nnada\n\n{HIDDEN}"
            )
        );
    }

    #[test]
    fn sanitize_body_creates_the_evidencias_heading_when_missing() {
        assert_eq!(
            sanitize_body("Hola", "EJ-1", 2),
            format!(
                "Hola\n\n### Evidencias\n\n**1.**\n{{{{img:1}}}}\n\n**2.**\n{{{{img:2}}}}\n\n{HIDDEN}"
            )
        );
        // Con el comentario oculto al final, la sección va antes que él.
        assert_eq!(
            sanitize_body(&format!("Hola\n\n{HIDDEN}\n"), "EJ-1", 1),
            format!("Hola\n\n### Evidencias\n\n**1.**\n{{{{img:1}}}}\n\n{HIDDEN}\n")
        );
    }

    #[test]
    fn sanitize_body_restores_the_hidden_marker() {
        assert_eq!(
            sanitize_body("Hola\n\n", "EJ-1", 0),
            format!("Hola\n\n{HIDDEN}")
        );
        let kept = format!("Hola\n\n{HIDDEN}");
        assert_eq!(sanitize_body(&kept, "EJ-1", 0), kept);
        // El de otro caso no cuenta.
        let other = "Hola\n\n<!-- qapture:case=EJ-2;v=1 -->";
        assert!(sanitize_body(other, "EJ-1", 0).ends_with(&format!("\n\n{HIDDEN}")));
    }

    #[test]
    fn sanitize_body_keeps_one_marker_per_image() {
        let body = format!(
            "{{{{img:2}}}} y {{{{img:1}}}} {{{{img:1}}}}\n{{{{ img : 1 }}}}\n### Evidencias\n{{{{IMG:2}}}}\n{HIDDEN}"
        );
        let out = sanitize_body(&body, "EJ-1", 2);
        assert_eq!(count(&out, "{{img:1}}"), 1, "{out}");
        assert_eq!(count(&out, "{{img:2}}"), 1, "{out}");
        assert!(!out.contains("{{ img") && !out.contains("IMG"), "{out}");
        assert_eq!(
            out,
            format!("{{{{img:2}}}} y {{{{img:1}}}}\n### Evidencias\n{HIDDEN}")
        );
    }

    #[test]
    fn fill_images_uses_alt_code_and_n() {
        let urls = vec!["https://x/1.png".to_string(), "https://x/2.png".to_string()];
        assert_eq!(
            fill_images("a {{img:1}}\n{{img:2}}", "EJ-014", Some(&urls)),
            "a ![EJ-014 · 1](https://x/1.png)\n![EJ-014 · 2](https://x/2.png)"
        );
    }

    #[test]
    fn fill_images_assisted_leaves_the_pending_text() {
        assert_eq!(
            fill_images("**1.**\n{{img:1}}\n\n**2.**\n{{img:2}}", "EJ-014", None),
            "**1.**\n> 📎 Evidencia 1: pendiente de pegar\n\n**2.**\n> 📎 Evidencia 2: pendiente de pegar"
        );
    }

    // --- orquestación ------------------------------------------------------------------

    const EVID: &str = "/repos/Ps3udoDev/evidencias";
    const ISSUES: &str = "/repos/Ps3udoDev/testing/issues";

    struct Fixture {
        _tmp: TempDir,
        state: AppState,
        case_id: String,
        code: String,
        evidence: Vec<String>,
    }

    fn now() -> NaiveDateTime {
        chrono::NaiveDate::from_ymd_opt(2026, 11, 2)
            .unwrap()
            .and_hms_opt(15, 30, 12)
            .unwrap()
    }

    /// Proyecto «Recibos» vinculado a `Ps3udoDev/testing` (y `Ps3udoDev/otro`), con `shots`
    /// capturas cuyo PNG original y renderizado tienen bytes distintos.
    fn fixture(api: &str, evidence_repo: Option<&str>, shots: usize) -> Fixture {
        let tmp = TempDir::new();
        let conn = db::open(&tmp.path().join(db::DB_FILE)).unwrap();
        let github = GithubState::for_tests(api, api);
        github.tokens.set("t0k").unwrap();
        let state = AppState::new(conn, tmp.files(), tmp.path(), None, false, github);
        let mut settings = Settings::default();
        settings.profile.tester = "Ana Pérez".into();
        state.set_settings(settings);
        let (case_id, code, evidence) = {
            let conn = state.db();
            let p = test_support::project(&conn);
            let c = test_support::case(&conn, &p);
            let linked = |s: &str, is_default| LinkedRepo {
                repo: RepoRef::parse(s).unwrap(),
                is_default,
                default_labels: vec![],
            };
            save_project_github(
                &conn,
                &p.id,
                &ProjectGithub {
                    repos: vec![
                        linked("Ps3udoDev/testing", true),
                        linked("Ps3udoDev/otro", false),
                    ],
                    evidence_repo: evidence_repo.map(|r| RepoRef::parse(r).unwrap()),
                    ..ProjectGithub::default()
                },
            )
            .unwrap();
            let mut ids = Vec::new();
            fs::create_dir_all(state.files().project_dir(&p.id)).unwrap();
            for i in 0..shots {
                let id = format!("ev{i}");
                let orig = EvidenceFiles::original_rel(&p.id, &id);
                let render = EvidenceFiles::render_rel(&p.id, &id);
                fs::write(state.files().absolute(&orig), format!("ORIG-{i}")).unwrap();
                fs::write(state.files().absolute(&render), format!("RENDER-{i}")).unwrap();
                conn.execute(
                    "INSERT INTO evidence (id, case_id, position, description, original_path,
                       rendered_path, annotations, width, height, captured_at, updated_at)
                     VALUES (?1, ?2, ?3, 'd', ?4, ?5, '{}', 1, 1, '', '')",
                    params![id, c.id, i as i64, orig, render],
                )
                .unwrap();
                ids.push(id);
            }
            (c.id, c.code, ids)
        };
        Fixture {
            _tmp: tmp,
            state,
            case_id,
            code,
            evidence,
        }
    }

    impl Fixture {
        fn input(&self, evidence_ids: Vec<String>) -> ReportInput {
            let mut body = String::from("Pasos\n\n### Evidencias");
            for n in 1..=evidence_ids.len() {
                body.push_str(&format!("\n\n**{n}.** d\n{{{{img:{n}}}}}"));
            }
            body.push_str(&format!("\n\n<!-- qapture:case={};v=1 -->", self.code));
            ReportInput {
                case_id: self.case_id.clone(),
                repo: RepoRef::parse("Ps3udoDev/testing").unwrap(),
                title: format!("[{}] Falla", self.code),
                body,
                labels: vec!["bug".into()],
                assignees: vec![],
                milestone: None,
                evidence_ids,
            }
        }

        fn link(&self) -> Option<CaseIssueLink> {
            find_link(
                &self.state.db(),
                &self.case_id,
                &RepoRef::parse("Ps3udoDev/testing").unwrap(),
            )
            .unwrap()
        }

        fn pending_body(&self, shots: usize) -> String {
            let mut body = String::from("Pasos\n\n### Evidencias");
            for n in 1..=shots {
                body.push_str(&format!(
                    "\n\n**{n}.** d\n> 📎 Evidencia {n}: pendiente de pegar"
                ));
            }
            body.push_str(&format!("\n\n<!-- qapture:case={};v=1 -->", self.code));
            body
        }
    }

    fn issue_json() -> String {
        json!({"number": 45, "node_id": "I_kw", "html_url": "https://github.com/Ps3udoDev/testing/issues/45", "state": "open"}).to_string()
    }

    async fn mock_issue(s: &mut ServerGuard, body: Option<String>, times: usize) -> Mock {
        let mut m = s.mock("POST", ISSUES);
        if let Some(body) = body {
            m = m.match_body(Matcher::PartialJson(json!({ "body": body })));
        }
        m.with_status(201)
            .with_body(issue_json())
            .expect(times)
            .create_async()
            .await
    }

    async fn mock_evidence_repo(s: &mut ServerGuard, push: bool) -> Mock {
        s.mock("GET", EVID)
            .with_body(json!({"default_branch":"main","permissions":{"push":push}}).to_string())
            .create_async()
            .await
    }

    async fn mock_ref_reads(s: &mut ServerGuard) -> (Mock, Mock) {
        let r = s
            .mock("GET", format!("{EVID}/git/ref/heads/main").as_str())
            .with_body(r#"{"object":{"sha":"head1"}}"#)
            .create_async()
            .await;
        let c = s
            .mock("GET", format!("{EVID}/git/commits/head1").as_str())
            .with_body(r#"{"sha":"head1","tree":{"sha":"tree0"}}"#)
            .create_async()
            .await;
        (r, c)
    }

    /// blobs (uno por captura, comprobando los bytes del PNG renderizado), tree, commit y ref.
    async fn mock_upload(s: &mut ServerGuard, f: &Fixture, shots: usize) -> Vec<Mock> {
        let mut mocks = Vec::new();
        let mut tree = Vec::new();
        let dir = format!("recibos/{}/2026-11-02_153012_ana-perez", f.code);
        for i in 0..shots {
            mocks.push(
                s.mock("POST", format!("{EVID}/git/blobs").as_str())
                    .match_body(Matcher::Json(json!({
                        "content": STANDARD.encode(format!("RENDER-{i}")),
                        "encoding": "base64"
                    })))
                    .with_status(201)
                    .with_body(json!({ "sha": format!("blob{i}") }).to_string())
                    .expect(1)
                    .create_async()
                    .await,
            );
            tree.push(json!({
                "path": format!("{dir}/{}.png", i + 1),
                "mode": "100644", "type": "blob", "sha": format!("blob{i}")
            }));
        }
        mocks.push(
            s.mock("POST", format!("{EVID}/git/trees").as_str())
                .match_body(Matcher::Json(json!({ "base_tree": "tree0", "tree": tree })))
                .with_status(201)
                .with_body(r#"{"sha":"tree1"}"#)
                .expect(1)
                .create_async()
                .await,
        );
        mocks.push(
            s.mock("POST", format!("{EVID}/git/commits").as_str())
                .match_body(Matcher::Json(json!({
                    "message": format!("qapture: {} Aplicación de recibos (Ana Pérez)", f.code),
                    "tree": "tree1",
                    "parents": ["head1"]
                })))
                .with_status(201)
                .with_body(r#"{"sha":"newc"}"#)
                .expect(1)
                .create_async()
                .await,
        );
        mocks.push(
            s.mock("PATCH", format!("{EVID}/git/refs/heads/main").as_str())
                .with_body("{}")
                .expect(1)
                .create_async()
                .await,
        );
        mocks
    }

    fn uploaded_body(f: &Fixture, server: &str, shots: usize) -> String {
        let dir = format!("recibos/{}/2026-11-02_153012_ana-perez", f.code);
        let mut body = String::from("Pasos\n\n### Evidencias");
        for n in 1..=shots {
            body.push_str(&format!(
                "\n\n**{n}.** d\n![{} · {n}]({server}/Ps3udoDev/evidencias/blob/newc/{dir}/{n}.png?raw=true)",
                f.code
            ));
        }
        body.push_str(&format!("\n\n<!-- qapture:case={};v=1 -->", f.code));
        body
    }

    #[tokio::test]
    async fn create_uploads_rendered_pngs_and_links_by_sha() {
        let mut s = mockito::Server::new_async().await;
        let f = fixture(&s.url(), Some("Ps3udoDev/evidencias"), 2);
        let _repo = mock_evidence_repo(&mut s, true).await;
        let _reads = mock_ref_reads(&mut s).await;
        let upload = mock_upload(&mut s, &f, 2).await;
        let expected_body = uploaded_body(&f, &s.url(), 2);
        let issue = mock_issue(&mut s, Some(expected_body), 1).await;

        let outcome = create_issue(&f.state, f.input(f.evidence.clone()), now())
            .await
            .unwrap();
        for m in upload {
            m.assert_async().await;
        }
        issue.assert_async().await;
        assert_eq!(outcome.mode, UploadMode::Repo);
        assert!(outcome.pending.is_empty());
        let expected = CaseIssueLink {
            case_id: f.case_id.clone(),
            owner: "Ps3udoDev".into(),
            repo: "testing".into(),
            issue_number: 45,
            node_id: "I_kw".into(),
            html_url: "https://github.com/Ps3udoDev/testing/issues/45".into(),
            direction: "exported".into(),
            issue_state: "open".into(),
        };
        assert_eq!(outcome.link, expected);
        assert_eq!(f.link(), Some(expected));
    }

    #[tokio::test]
    async fn only_the_chosen_images_are_numbered_in_case_order() {
        let mut s = mockito::Server::new_async().await;
        let f = fixture(&s.url(), Some("Ps3udoDev/evidencias"), 3);
        let _repo = mock_evidence_repo(&mut s, true).await;
        let _reads = mock_ref_reads(&mut s).await;
        // Se eligen la 3.ª y la 1.ª (en ese orden): suben como 1.png (ev0) y 2.png (ev2).
        let dir = format!("recibos/{}/2026-11-02_153012_ana-perez", f.code);
        let mut blobs = Vec::new();
        for (i, shot) in [0, 2].into_iter().enumerate() {
            blobs.push(
                s.mock("POST", format!("{EVID}/git/blobs").as_str())
                    .match_body(Matcher::Json(json!({
                        "content": STANDARD.encode(format!("RENDER-{shot}")),
                        "encoding": "base64"
                    })))
                    .with_status(201)
                    .with_body(json!({ "sha": format!("blob{i}") }).to_string())
                    .expect(1)
                    .create_async()
                    .await,
            );
        }
        let tree = s
            .mock("POST", format!("{EVID}/git/trees").as_str())
            .match_body(Matcher::PartialJson(json!({ "tree": [
                {"path": format!("{dir}/1.png"), "sha": "blob0"},
                {"path": format!("{dir}/2.png"), "sha": "blob1"}
            ]})))
            .with_status(201)
            .with_body(r#"{"sha":"tree1"}"#)
            .expect(1)
            .create_async()
            .await;
        let _c = s
            .mock("POST", format!("{EVID}/git/commits").as_str())
            .with_status(201)
            .with_body(r#"{"sha":"newc"}"#)
            .create_async()
            .await;
        let _p = s
            .mock("PATCH", format!("{EVID}/git/refs/heads/main").as_str())
            .with_body("{}")
            .create_async()
            .await;
        let _i = mock_issue(&mut s, None, 1).await;
        let chosen = vec![f.evidence[2].clone(), f.evidence[0].clone()];
        create_issue(&f.state, f.input(chosen), now())
            .await
            .unwrap();
        for m in blobs {
            m.assert_async().await;
        }
        tree.assert_async().await;
    }

    #[tokio::test]
    async fn create_without_evidence_repo_is_assisted() {
        let mut s = mockito::Server::new_async().await;
        let f = fixture(&s.url(), None, 2);
        let issue = mock_issue(&mut s, Some(f.pending_body(2)), 1).await;
        let outcome = create_issue(&f.state, f.input(f.evidence.clone()), now())
            .await
            .unwrap();
        issue.assert_async().await;
        assert_eq!(
            outcome.mode,
            UploadMode::Assisted(AssistedReason::NoEvidenceRepo)
        );
        assert_eq!(outcome.pending, f.evidence);
        assert!(f.link().is_some());
    }

    #[tokio::test]
    async fn create_without_push_is_assisted_without_uploading() {
        let mut s = mockito::Server::new_async().await;
        let f = fixture(&s.url(), Some("Ps3udoDev/evidencias"), 1);
        let _repo = mock_evidence_repo(&mut s, false).await;
        let blobs = s
            .mock("POST", format!("{EVID}/git/blobs").as_str())
            .expect(0)
            .create_async()
            .await;
        let issue = mock_issue(&mut s, Some(f.pending_body(1)), 1).await;
        let outcome = create_issue(&f.state, f.input(f.evidence.clone()), now())
            .await
            .unwrap();
        blobs.assert_async().await;
        issue.assert_async().await;
        assert_eq!(
            outcome.mode,
            UploadMode::Assisted(AssistedReason::CannotWrite(
                RepoRef::parse("Ps3udoDev/evidencias").unwrap()
            ))
        );
    }

    #[tokio::test]
    async fn create_with_403_on_upload_is_assisted_and_still_creates_the_issue() {
        let mut s = mockito::Server::new_async().await;
        let f = fixture(&s.url(), Some("Ps3udoDev/evidencias"), 2);
        let _repo = mock_evidence_repo(&mut s, true).await;
        let _reads = mock_ref_reads(&mut s).await;
        let _b = s
            .mock("POST", format!("{EVID}/git/blobs").as_str())
            .with_status(403)
            .with_body("{}")
            .create_async()
            .await;
        let issue = mock_issue(&mut s, Some(f.pending_body(2)), 1).await;
        let outcome = create_issue(&f.state, f.input(f.evidence.clone()), now())
            .await
            .unwrap();
        issue.assert_async().await;
        assert_eq!(
            outcome.mode,
            UploadMode::Assisted(AssistedReason::CannotWrite(
                RepoRef::parse("Ps3udoDev/evidencias").unwrap()
            ))
        );
        assert_eq!(outcome.pending, f.evidence);
        assert!(f.link().is_some());
    }

    #[tokio::test]
    async fn a_404_on_the_evidence_repo_means_the_app_is_not_installed() {
        let mut s = mockito::Server::new_async().await;
        let f = fixture(&s.url(), Some("Ps3udoDev/evidencias"), 1);
        let _repo = s.mock("GET", EVID).with_status(404).create_async().await;
        let _issue = mock_issue(&mut s, Some(f.pending_body(1)), 1).await;
        let outcome = create_issue(&f.state, f.input(f.evidence.clone()), now())
            .await
            .unwrap();
        assert_eq!(
            outcome.mode,
            UploadMode::Assisted(AssistedReason::NotInstalled(
                RepoRef::parse("Ps3udoDev/evidencias").unwrap()
            ))
        );
    }

    #[tokio::test]
    async fn create_with_a_5xx_on_upload_creates_nothing() {
        let mut s = mockito::Server::new_async().await;
        let f = fixture(&s.url(), Some("Ps3udoDev/evidencias"), 1);
        let _repo = mock_evidence_repo(&mut s, true).await;
        let _reads = mock_ref_reads(&mut s).await;
        let _b = s
            .mock("POST", format!("{EVID}/git/blobs").as_str())
            .with_status(502)
            .create_async()
            .await;
        let issue = mock_issue(&mut s, None, 0).await;
        let err = create_issue(&f.state, f.input(f.evidence.clone()), now())
            .await
            .unwrap_err();
        assert_eq!(err.code(), "io");
        issue.assert_async().await;
        assert_eq!(f.link(), None);
    }

    #[tokio::test]
    async fn a_401_on_upload_aborts_with_gh_auth() {
        let mut s = mockito::Server::new_async().await;
        let f = fixture(&s.url(), Some("Ps3udoDev/evidencias"), 1);
        let _repo = s.mock("GET", EVID).with_status(401).create_async().await;
        let issue = mock_issue(&mut s, None, 0).await;
        let err = create_issue(&f.state, f.input(f.evidence.clone()), now())
            .await
            .unwrap_err();
        assert_eq!(err.code(), "gh_auth");
        issue.assert_async().await;
        assert_eq!(f.link(), None);
    }

    #[tokio::test]
    async fn create_never_writes_outside_the_configured_evidence_repo() {
        let mut s = mockito::Server::new_async().await;
        let f = fixture(&s.url(), Some("Ps3udoDev/evidencias"), 2);
        // Primero los «atrapa todo» de escritura: cualquier petición que no tenga un mock
        // específico (creado después) cae aquí y el test falla.
        let mut traps = Vec::new();
        for method in ["POST", "PATCH", "PUT", "DELETE"] {
            traps.push(
                s.mock(method, Matcher::Any)
                    .with_status(500)
                    .expect(0)
                    .create_async()
                    .await,
            );
        }
        let _repo = mock_evidence_repo(&mut s, true).await;
        let _reads = mock_ref_reads(&mut s).await;
        let mut allowed = Vec::new();
        for path in ["blobs", "trees", "commits"] {
            allowed.push(
                s.mock("POST", Matcher::Exact(format!("{EVID}/git/{path}")))
                    .with_status(201)
                    .with_body(r#"{"sha":"x"}"#)
                    .create_async()
                    .await,
            );
        }
        allowed.push(
            s.mock(
                "PATCH",
                Matcher::Exact(format!("{EVID}/git/refs/heads/main")),
            )
            .with_body("{}")
            .create_async()
            .await,
        );
        let issue = mock_issue(&mut s, None, 1).await;
        let outcome = create_issue(&f.state, f.input(f.evidence.clone()), now())
            .await
            .unwrap();
        assert_eq!(outcome.mode, UploadMode::Repo);
        for trap in traps {
            trap.assert_async().await;
        }
        issue.assert_async().await;
    }

    #[tokio::test]
    async fn a_second_create_for_the_same_case_and_repo_conflicts() {
        let mut s = mockito::Server::new_async().await;
        let f = fixture(&s.url(), None, 0);
        let issue = mock_issue(&mut s, None, 1).await;
        create_issue(&f.state, f.input(vec![]), now())
            .await
            .unwrap();
        let err = create_issue(&f.state, f.input(vec![]), now())
            .await
            .unwrap_err();
        assert_eq!(err.code(), "gh_conflict");
        assert_eq!(
            err.user_message(),
            "El caso ya tiene un issue en Ps3udoDev/testing (#45)."
        );
        issue.assert_async().await;
    }

    #[tokio::test]
    async fn a_create_already_in_flight_for_the_same_case_and_repo_conflicts() {
        let mut s = mockito::Server::new_async().await;
        let f = fixture(&s.url(), None, 0);
        let issue = mock_issue(&mut s, None, 0).await;
        let repo = RepoRef::parse("ps3udodev/Testing").unwrap();
        let hold = InFlight::claim("create", &f.case_id, &repo).unwrap();
        let err = create_issue(&f.state, f.input(vec![]), now())
            .await
            .unwrap_err();
        assert_eq!(err.code(), "gh_conflict");
        assert_eq!(
            err.user_message(),
            "Ya se está creando un issue de este caso en Ps3udoDev/testing."
        );
        issue.assert_async().await;
        assert!(
            InFlight::claim("create", &f.case_id, &repo).is_none(),
            "el intento fallido no borra la reserva ajena"
        );
        drop(hold);
        assert!(
            InFlight::claim("create", &f.case_id, &repo).is_some(),
            "se libera"
        );
    }

    #[tokio::test]
    async fn a_comment_already_in_flight_for_the_same_case_and_repo_conflicts() {
        let mut s = mockito::Server::new_async().await;
        let f = fixture(&s.url(), None, 0);
        insert_link(
            &f.state.db(),
            &CaseIssueLink {
                case_id: f.case_id.clone(),
                owner: "Ps3udoDev".into(),
                repo: "testing".into(),
                issue_number: 45,
                node_id: "I_kw".into(),
                html_url: "https://github.com/Ps3udoDev/testing/issues/45".into(),
                direction: "exported".into(),
                issue_state: "open".into(),
            },
        )
        .unwrap();
        let post = s
            .mock("POST", format!("{ISSUES}/45/comments").as_str())
            .expect(0)
            .create_async()
            .await;
        let repo = RepoRef::parse("ps3udodev/Testing").unwrap();
        let hold = InFlight::claim("comment", &f.case_id, &repo).unwrap();
        let err = comment_issue(&f.state, f.input(vec![]), now())
            .await
            .unwrap_err();
        assert_eq!(err.code(), "gh_conflict");
        assert_eq!(
            err.user_message(),
            "Ya se está comentando este caso en Ps3udoDev/testing."
        );
        post.assert_async().await;
        assert!(
            InFlight::claim("create", &f.case_id, &repo).is_some(),
            "crear y comentar no se bloquean entre sí"
        );
        drop(hold);
        assert!(
            InFlight::claim("comment", &f.case_id, &repo).is_some(),
            "se libera"
        );
    }

    #[tokio::test]
    async fn without_chosen_images_nothing_is_uploaded_and_the_body_is_sanitized() {
        let mut s = mockito::Server::new_async().await;
        let f = fixture(&s.url(), Some("Ps3udoDev/evidencias"), 1);
        let repo = s.mock("GET", EVID).expect(0).create_async().await;
        let mut input = f.input(vec![]);
        input.body = "Hola {{img:1}}".into();
        let issue = mock_issue(
            &mut s,
            Some(format!("Hola\n\n<!-- qapture:case={};v=1 -->", f.code)),
            1,
        )
        .await;
        let outcome = create_issue(&f.state, input, now()).await.unwrap();
        assert_eq!(outcome.mode, UploadMode::Repo);
        assert!(outcome.pending.is_empty());
        repo.assert_async().await;
        issue.assert_async().await;
    }

    #[tokio::test]
    async fn create_revalidates_its_input() {
        let s = mockito::Server::new_async().await;
        let f = fixture(&s.url(), None, 1);
        type Change = Box<dyn Fn(&mut ReportInput)>;
        let cases: Vec<(Change, &str)> = vec![
            (
                Box::new(|i| i.title = "  ".into()),
                "«Título» es obligatorio.",
            ),
            (
                Box::new(|i| i.title = "x".repeat(257)),
                "«Título» supera 256 caracteres.",
            ),
            (
                Box::new(|i| i.body = "x".repeat(65_537)),
                "La descripción supera 65 536 caracteres.",
            ),
            (
                Box::new(|i| i.labels = vec!["l".into(); 11]),
                "Se permiten hasta 10 etiquetas.",
            ),
            (
                Box::new(|i| i.assignees = vec!["a".into(); 11]),
                "Se permiten hasta 10 asignados.",
            ),
            (
                Box::new(|i| i.evidence_ids = vec!["ajena".into()]),
                "Una de las capturas elegidas no pertenece al caso.",
            ),
        ];
        for (change, message) in cases {
            let mut input = f.input(f.evidence.clone());
            change(&mut input);
            let err = create_issue(&f.state, input, now()).await.unwrap_err();
            assert_eq!(err.code(), "validation", "{message}");
            assert_eq!(err.user_message(), message);
        }
    }

    #[tokio::test]
    async fn comment_requires_an_existing_link() {
        let mut s = mockito::Server::new_async().await;
        let f = fixture(&s.url(), None, 1);
        let err = comment_issue(&f.state, f.input(f.evidence.clone()), now())
            .await
            .unwrap_err();
        assert_eq!(err.code(), "gh_not_found");
        assert_eq!(
            err.user_message(),
            "El caso no tiene un issue en Ps3udoDev/testing."
        );

        insert_link(
            &f.state.db(),
            &CaseIssueLink {
                case_id: f.case_id.clone(),
                owner: "Ps3udoDev".into(),
                repo: "testing".into(),
                issue_number: 45,
                node_id: "I_kw".into(),
                html_url: "https://github.com/Ps3udoDev/testing/issues/45".into(),
                direction: "exported".into(),
                issue_state: "open".into(),
            },
        )
        .unwrap();
        let comment = s
            .mock("POST", format!("{ISSUES}/45/comments").as_str())
            .match_body(Matcher::Json(json!({ "body": f.pending_body(1) })))
            .with_status(201)
            .with_body(
                r#"{"html_url":"https://github.com/Ps3udoDev/testing/issues/45#issuecomment-9"}"#,
            )
            .expect(1)
            .create_async()
            .await;
        let mut input = f.input(f.evidence.clone());
        input.title = String::new();
        let (url, mode, pending) = comment_issue(&f.state, input, now()).await.unwrap();
        comment.assert_async().await;
        assert_eq!(
            url,
            "https://github.com/Ps3udoDev/testing/issues/45#issuecomment-9"
        );
        assert_eq!(mode, UploadMode::Assisted(AssistedReason::NoEvidenceRepo));
        assert_eq!(pending, f.evidence);
    }
}
