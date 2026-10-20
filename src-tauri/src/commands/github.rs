//! Cuenta de GitHub (Device Flow), repos accesibles y configuración GitHub del proyecto.
//! El spec §5.2 llama `gh_project_repos_get/set` a los dos últimos; aquí son
//! `gh_project_github_*` porque guardan repos, ruteo y repo de evidencias.

use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};

use chrono::NaiveDateTime;
use rusqlite::Connection;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_opener::OpenerExt;

use crate::{
    commands::settings::opener_error,
    dto::{
        CaseIssueLinkDto, DeviceCodeDto, GhAccountDto, GhConnectFailedDto, GhRepoDto, MilestoneDto,
        ProjectGithubDto, ProjectGithubInput, RepoMetaDto, ReportInputDto, ReportResultDto,
    },
    error::{AppError, AppResult, GhErrorKind, validation},
    integrations::github::{
        auth::{self, DeviceCode},
        issues, project_config,
        report::{self, AssistedReason, ReportInput, UploadMode},
        repos,
    },
    repo::{
        self,
        integrations::{CaseIssueLink, GhAccountRow, RepoRef},
    },
    state::{AppState, GithubState},
};

pub const GH_CONNECTED_EVENT: &str = "gh:connected";
pub const GH_CONNECT_FAILED_EVENT: &str = "gh:connect-failed";

fn account_dto(row: GhAccountRow, needs_reconnect: bool) -> GhAccountDto {
    GhAccountDto {
        login: row.login,
        name: row.name,
        avatar_data_url: row.avatar_data,
        connected_at: row.connected_at,
        needs_reconnect,
    }
}

/// Un `gh_auth` de una llamada al API (401 o sin token) pide reconectar la cuenta.
fn mark_auth<T>(github: &GithubState, result: AppResult<T>) -> AppResult<T> {
    if let Err(AppError::GitHub {
        kind: GhErrorKind::Auth,
        ..
    }) = &result
    {
        github.needs_reconnect.store(true, Ordering::SeqCst);
    }
    result
}

/// Resultado de un Device Flow que no se canceló.
#[derive(Debug)]
pub(crate) enum FlowOutcome {
    Connected(GhAccountDto),
    Failed(String),
}

/// Token y cuenta guardados antes de conectar, para deshacer un reconectar cancelado.
struct Previous {
    token: Option<String>,
    account: Option<GhAccountRow>,
}

impl Previous {
    fn take(github: &GithubState, conn: &Mutex<Connection>) -> Self {
        let guard = conn.lock().unwrap_or_else(|p| p.into_inner());
        Self {
            token: github.tokens.get().ok().flatten(),
            account: repo::integrations::get_account(&guard).ok().flatten(),
        }
    }

    /// Deja token y cuenta como estaban (o sin nada, si no había).
    fn restore(self, github: &GithubState, conn: &Mutex<Connection>) {
        let _ = match &self.token {
            Some(token) => github.tokens.set(token),
            None => github.tokens.delete(),
        };
        let guard = conn.lock().unwrap_or_else(|p| p.into_inner());
        let _ = match &self.account {
            Some(row) => repo::integrations::save_account(&guard, row),
            None => repo::integrations::delete_account(&guard),
        };
    }
}

/// Con el token ya obtenido: si el flujo se canceló se descarta sin guardarlo; si no, se
/// guarda token y cuenta. Si falla después de guardar el token, se borra (no queda huérfano).
pub(crate) async fn after_token(
    github: &GithubState,
    conn: &Mutex<Connection>,
    token: String,
    cancel: &AtomicBool,
) -> Option<FlowOutcome> {
    if cancel.load(Ordering::SeqCst) {
        return None;
    }
    let previous = Previous::take(github, conn);
    match auth::complete(&github.client, github.tokens.as_ref(), conn, token).await {
        Ok(row) => {
            if cancel.load(Ordering::SeqCst) {
                // Cancelado mientras se guardaba: vuelve la cuenta anterior (si era un
                // reconectar) y no se avisa.
                previous.restore(github, conn);
                return None;
            }
            github.needs_reconnect.store(false, Ordering::SeqCst);
            Some(FlowOutcome::Connected(account_dto(row, false)))
        }
        Err(error) => {
            // Un fallo del llavero ocurre al guardar: no hay token nuevo que borrar (y el
            // anterior, si lo hay, sigue siendo válido).
            if !matches!(
                error,
                AppError::GitHub {
                    kind: GhErrorKind::Keyring,
                    ..
                }
            ) {
                let _ = github.tokens.delete();
            }
            (!cancel.load(Ordering::SeqCst)).then(|| FlowOutcome::Failed(error.user_message()))
        }
    }
}

/// Sondea hasta el final del Device Flow. `None` si se canceló (no se emite nada).
pub(crate) async fn finish_flow(
    github: &GithubState,
    conn: &Mutex<Connection>,
    code: &DeviceCode,
    cancel: &AtomicBool,
) -> Option<FlowOutcome> {
    match auth::poll(&github.client, code, cancel).await {
        Ok(Some(token)) => after_token(github, conn, token, cancel).await,
        Ok(None) => None,
        Err(error) => {
            (!cancel.load(Ordering::SeqCst)).then(|| FlowOutcome::Failed(error.user_message()))
        }
    }
}

/// `needsReconnect` también si hay cuenta pero el llavero no tiene token (o no responde).
pub(crate) fn account(state: &AppState) -> AppResult<Option<GhAccountDto>> {
    let Some(row) = repo::integrations::get_account(&state.db())? else {
        return Ok(None);
    };
    let github = state.github();
    let has_token = matches!(github.tokens.get(), Ok(Some(_)));
    let needs_reconnect = github.needs_reconnect.load(Ordering::SeqCst) || !has_token;
    Ok(Some(account_dto(row, needs_reconnect)))
}

/// Borra token y cuenta. Los vínculos de los proyectos se conservan. La cuenta se borra
/// aunque el llavero falle; después se devuelve ese error.
pub(crate) fn disconnect(state: &AppState) -> AppResult<()> {
    let github = state.github();
    github.cancel_flow();
    let tokens = github.tokens.delete();
    repo::integrations::delete_account(&state.db())?;
    github.needs_reconnect.store(false, Ordering::SeqCst);
    tokens
}

pub(crate) async fn list_repos(state: &AppState) -> AppResult<Vec<GhRepoDto>> {
    let github = state.github();
    let result = match github.token() {
        Ok(token) => repos::list_accessible(&github.client, &token).await,
        Err(error) => Err(error),
    };
    mark_auth(github, result)
}

pub(crate) fn project_github_get(
    state: &AppState,
    project_id: &str,
) -> AppResult<ProjectGithubDto> {
    let conn = state.db();
    repo::projects::ensure_exists(&conn, project_id)?;
    let fields = repo::fields::list_for_project(&conn, project_id)?;
    let config = repo::integrations::get_project_github(&conn, project_id)?;
    Ok(project_config::to_dto(&config, &fields))
}

pub(crate) fn project_github_set(
    state: &AppState,
    project_id: &str,
    input: &ProjectGithubInput,
) -> AppResult<ProjectGithubDto> {
    let conn = state.db();
    repo::projects::ensure_exists(&conn, project_id)?;
    let fields = repo::fields::list_for_project(&conn, project_id)?;
    let config = project_config::validate(input, &fields)?;
    repo::integrations::save_project_github(&conn, project_id, &config)?;
    Ok(project_config::to_dto(&config, &fields))
}

// --- reportar casos como issues (F8b) ------------------------------------------------------

fn link_dto(link: &CaseIssueLink) -> CaseIssueLinkDto {
    CaseIssueLinkDto {
        case_id: link.case_id.clone(),
        owner: link.owner.clone(),
        repo: link.repo.clone(),
        full_name: link.repo_ref().full_name(),
        issue_number: link.issue_number,
        html_url: link.html_url.clone(),
        direction: link.direction.clone(),
        issue_state: link.issue_state.clone(),
    }
}

/// Textos fijos de la UI para el panel del modo asistido.
fn assisted_text(reason: &AssistedReason) -> String {
    match reason {
        AssistedReason::NoEvidenceRepo => "No hay repo de evidencias configurado.".to_string(),
        AssistedReason::CannotWrite(repo) => {
            format!("Tu cuenta no puede escribir en {}.", repo.full_name())
        }
        AssistedReason::NotInstalled(repo) => {
            format!("Qapture no está instalada en {}.", repo.full_name())
        }
    }
}

fn result_dto(
    link: &CaseIssueLink,
    mode: &UploadMode,
    pending: Vec<String>,
    comment_url: Option<String>,
) -> ReportResultDto {
    let (upload_mode, assisted_reason) = match mode {
        UploadMode::Repo => ("repo", None),
        UploadMode::Assisted(reason) => ("assisted", Some(assisted_text(reason))),
    };
    ReportResultDto {
        link: link_dto(link),
        upload_mode: upload_mode.to_string(),
        assisted_reason,
        pending_evidence: pending,
        comment_url,
    }
}

fn parse_repo(text: &str) -> AppResult<RepoRef> {
    RepoRef::parse(text.trim())
        .ok_or_else(|| validation("El repositorio debe tener el formato dueño/repositorio."))
}

/// El repo del reporte debe ser uno de los vinculados al proyecto del caso (sin distinguir
/// mayúsculas): el WebView no puede apuntar a un repo cualquiera.
fn linked_repo(state: &AppState, case_id: &str, text: &str) -> AppResult<RepoRef> {
    let repo = parse_repo(text)?;
    let conn = state.db();
    let case = repo::cases::get(&conn, case_id)?;
    let config = repo::integrations::get_project_github(&conn, &case.project_id)?;
    // Se devuelve el repo con la escritura guardada, no la que mandó el WebView.
    config
        .repos
        .into_iter()
        .map(|l| l.repo)
        .find(|l| {
            l.owner.eq_ignore_ascii_case(&repo.owner) && l.repo.eq_ignore_ascii_case(&repo.repo)
        })
        .ok_or_else(|| {
            validation(format!(
                "«{}» no está vinculado al proyecto.",
                repo.full_name()
            ))
        })
}

fn report_input(state: &AppState, dto: ReportInputDto) -> AppResult<ReportInput> {
    let repo = linked_repo(state, &dto.case_id, &dto.repo)?;
    Ok(ReportInput {
        case_id: dto.case_id,
        repo,
        title: dto.title,
        body: dto.body,
        labels: dto.labels,
        assignees: dto.assignees,
        milestone: dto.milestone,
        evidence_ids: dto.evidence_ids,
    })
}

pub(crate) async fn repo_meta(state: &AppState, repo: &str) -> AppResult<RepoMetaDto> {
    let github = state.github();
    let result = async {
        let repo = parse_repo(repo)?;
        let token = github.token()?;
        let meta = issues::repo_meta(&github.client, &token, &repo).await?;
        Ok(RepoMetaDto {
            labels: meta.labels,
            milestones: meta
                .milestones
                .into_iter()
                .map(|(number, title)| MilestoneDto { number, title })
                .collect(),
            assignees: meta.assignees,
        })
    }
    .await;
    mark_auth(github, result)
}

pub(crate) fn project_links(
    state: &AppState,
    project_id: &str,
) -> AppResult<Vec<CaseIssueLinkDto>> {
    let conn = state.db();
    repo::projects::ensure_exists(&conn, project_id)?;
    let links = repo::integrations::links_for_project(&conn, project_id)?;
    Ok(links.iter().map(link_dto).collect())
}

pub(crate) async fn create_issue(
    state: &AppState,
    input: ReportInputDto,
    now: NaiveDateTime,
) -> AppResult<ReportResultDto> {
    let github = state.github();
    let result = async {
        let input = report_input(state, input)?;
        let outcome = report::create_issue(state, input, now).await?;
        Ok(result_dto(
            &outcome.link,
            &outcome.mode,
            outcome.pending,
            None,
        ))
    }
    .await;
    mark_auth(github, result)
}

pub(crate) async fn comment_issue(
    state: &AppState,
    input: ReportInputDto,
    now: NaiveDateTime,
) -> AppResult<ReportResultDto> {
    let github = state.github();
    let result = async {
        let input = report_input(state, input)?;
        let (case_id, repo) = (input.case_id.clone(), input.repo.clone());
        let (url, mode, pending) = report::comment_issue(state, input, now).await?;
        let link = repo::integrations::find_link(&state.db(), &case_id, &repo)?
            .ok_or_else(|| AppError::NotFound("El caso no tiene un issue en ese repo.".into()))?;
        Ok(result_dto(&link, &mode, pending, Some(url)))
    }
    .await;
    mark_auth(github, result)
}

/// El PNG renderizado de la captura, como RGBA para el portapapeles.
pub(crate) fn evidence_rgba(state: &AppState, evidence_id: &str) -> AppResult<(u32, u32, Vec<u8>)> {
    let shot = repo::evidence::get(&state.db(), state.files(), evidence_id)?;
    let bytes = std::fs::read(&shot.rendered_path)?;
    let image = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)
        .map_err(|_| validation("La captura no se pudo leer."))?
        .into_rgba8();
    let (width, height) = image.dimensions();
    Ok((width, height, image.into_raw()))
}

/// La `html_url` guardada del vínculo (caso, repo): nunca una URL del WebView.
pub(crate) fn stored_issue_url(state: &AppState, case_id: &str, repo: &str) -> AppResult<String> {
    let repo = parse_repo(repo)?;
    repo::integrations::find_link(&state.db(), case_id, &repo)?
        .map(|link| link.html_url)
        .ok_or_else(|| AppError::NotFound("El caso no tiene un issue en ese repo.".into()))
}

/// Pide el código del Device Flow y deja el sondeo corriendo. Un segundo inicio cancela el
/// anterior. Falla antes de que el front abra el navegador si el llavero no responde.
#[tauri::command(async)]
pub async fn gh_connect_start(
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<DeviceCodeDto> {
    let github = state.github();
    github.tokens.get()?;
    let cancel = github.begin_flow();
    let code = auth::start(&github.client).await?;
    let dto = DeviceCodeDto {
        user_code: code.user_code.clone(),
        verification_uri: code.verification_uri.clone(),
        expires_in: code.expires_in,
        interval: code.interval,
    };
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>();
        let outcome = finish_flow(state.github(), state.db_mutex(), &code, &cancel).await;
        if cancel.load(Ordering::SeqCst) {
            return;
        }
        match outcome {
            Some(FlowOutcome::Connected(account)) => {
                let _ = app.emit_to("main", GH_CONNECTED_EVENT, account);
            }
            Some(FlowOutcome::Failed(message)) => {
                let _ = app.emit_to(
                    "main",
                    GH_CONNECT_FAILED_EVENT,
                    GhConnectFailedDto { message },
                );
            }
            None => {}
        }
    });
    Ok(dto)
}

#[tauri::command]
pub fn gh_connect_cancel(state: State<'_, AppState>) {
    state.github().cancel_flow();
}

#[tauri::command(async)]
pub fn gh_account(state: State<'_, AppState>) -> AppResult<Option<GhAccountDto>> {
    account(&state)
}

#[tauri::command(async)]
pub fn gh_disconnect(state: State<'_, AppState>) -> AppResult<()> {
    disconnect(&state)
}

#[tauri::command(async)]
pub async fn gh_list_repos(state: State<'_, AppState>) -> AppResult<Vec<GhRepoDto>> {
    list_repos(&state).await
}

#[tauri::command(async)]
pub fn gh_project_github_get(
    state: State<'_, AppState>,
    project_id: String,
) -> AppResult<ProjectGithubDto> {
    project_github_get(&state, &project_id)
}

#[tauri::command(async)]
pub fn gh_project_github_set(
    state: State<'_, AppState>,
    project_id: String,
    config: ProjectGithubInput,
) -> AppResult<ProjectGithubDto> {
    project_github_set(&state, &project_id, &config)
}

#[tauri::command(async)]
pub async fn gh_repo_meta(state: State<'_, AppState>, repo: String) -> AppResult<RepoMetaDto> {
    repo_meta(&state, &repo).await
}

#[tauri::command(async)]
pub fn gh_project_links(
    state: State<'_, AppState>,
    project_id: String,
) -> AppResult<Vec<CaseIssueLinkDto>> {
    project_links(&state, &project_id)
}

#[tauri::command(async)]
pub async fn gh_create_issue(
    state: State<'_, AppState>,
    input: ReportInputDto,
) -> AppResult<ReportResultDto> {
    create_issue(&state, input, chrono::Local::now().naive_local()).await
}

#[tauri::command(async)]
pub async fn gh_comment_issue(
    state: State<'_, AppState>,
    input: ReportInputDto,
) -> AppResult<ReportResultDto> {
    comment_issue(&state, input, chrono::Local::now().naive_local()).await
}

/// Pone el PNG renderizado de la captura en el portapapeles del sistema.
#[tauri::command(async)]
pub fn gh_copy_evidence(
    app: AppHandle,
    state: State<'_, AppState>,
    evidence_id: String,
) -> AppResult<()> {
    let (width, height, rgba) = evidence_rgba(&state, &evidence_id)?;
    app.clipboard()
        .write_image(&tauri::image::Image::new(&rgba, width, height))
        .map_err(|error| AppError::Io(std::io::Error::other(error.to_string())))
}

/// Abre la `html_url` guardada del vínculo con el navegador.
#[tauri::command(async)]
pub fn gh_open_issue(
    app: AppHandle,
    state: State<'_, AppState>,
    case_id: String,
    repo: String,
) -> AppResult<()> {
    let url = stored_issue_url(&state, &case_id, &repo)?;
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(opener_error)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::{
        db,
        dto::LinkedRepoDto,
        integrations::github::{client::GhClient, token_store::MemoryTokenStore},
        media::storage::TempDir,
        repo::integrations::{get_account, save_account},
    };

    fn state_at(api: &str) -> (TempDir, AppState) {
        state_with(GithubState::for_tests(api, api))
    }

    fn state_with(github: GithubState) -> (TempDir, AppState) {
        let tmp = TempDir::new();
        let conn = db::open(&tmp.path().join(db::DB_FILE)).unwrap();
        let state = AppState::new(conn, tmp.files(), tmp.path(), None, false, github);
        (tmp, state)
    }

    fn fresh() -> (TempDir, AppState) {
        state_at("http://127.0.0.1:9")
    }

    fn row() -> GhAccountRow {
        GhAccountRow {
            login: "Ps3udoDev".into(),
            name: Some("Pseudo".into()),
            avatar_data: None,
            connected_at: "2026-10-19T12:00:00+00:00".into(),
        }
    }

    /// Proyecto con «Entorno» convertida en lista Front/Back.
    fn project_with_routing_field(state: &AppState) -> (String, String) {
        let conn = state.db();
        let p = repo::test_support::project(&conn);
        let field_id = repo::test_support::field_id(&p, "entorno");
        conn.execute(
            "UPDATE project_field SET type = 'list', options = '[\"Front\",\"Back\"]' WHERE id = ?1",
            [&field_id],
        )
        .unwrap();
        (p.id, field_id)
    }

    fn input(field_id: &str) -> ProjectGithubInput {
        ProjectGithubInput {
            repos: vec![LinkedRepoDto {
                full_name: "TEAMS4SOFT/GMX-frontend".into(),
                is_default: true,
                default_labels: vec!["bug".into()],
            }],
            routing_field_id: Some(field_id.into()),
            routing_map: BTreeMap::from([(
                "Front".to_string(),
                "TEAMS4SOFT/GMX-frontend".to_string(),
            )]),
            evidence_repo: None,
        }
    }

    #[tokio::test]
    async fn a_401_marks_the_account_for_reconnect() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("GET", "/user/installations?per_page=100")
            .with_status(401)
            .create_async()
            .await;
        let (_tmp, state) = state_at(&server.url());
        save_account(&state.db(), &row()).unwrap();
        state.github().tokens.set("revocado").unwrap();
        assert!(!account(&state).unwrap().unwrap().needs_reconnect);

        let error = list_repos(&state).await.unwrap_err();
        assert_eq!(error.code(), "gh_auth");
        assert!(state.github().needs_reconnect.load(Ordering::SeqCst));
        assert!(account(&state).unwrap().unwrap().needs_reconnect);
    }

    #[tokio::test]
    async fn listing_without_token_is_gh_auth_and_other_errors_do_not_mark() {
        let (_tmp, state) = fresh();
        assert_eq!(list_repos(&state).await.unwrap_err().code(), "gh_auth");
        assert!(state.github().needs_reconnect.load(Ordering::SeqCst));

        let (_tmp, offline) = fresh();
        offline.github().tokens.set("tok").unwrap();
        assert_eq!(list_repos(&offline).await.unwrap_err().code(), "gh_offline");
        assert!(!offline.github().needs_reconnect.load(Ordering::SeqCst));
    }

    #[test]
    fn account_without_token_needs_reconnect() {
        let (_tmp, state) = fresh();
        assert_eq!(account(&state).unwrap(), None);
        save_account(&state.db(), &row()).unwrap();
        let dto = account(&state).unwrap().unwrap();
        assert!(dto.needs_reconnect);
        assert_eq!(dto.login, "Ps3udoDev");
        state.github().tokens.set("tok").unwrap();
        assert!(!account(&state).unwrap().unwrap().needs_reconnect);
    }

    #[test]
    fn disconnect_keeps_project_links() {
        let (_tmp, state) = fresh();
        let (project_id, field_id) = project_with_routing_field(&state);
        let saved = project_github_set(&state, &project_id, &input(&field_id)).unwrap();
        save_account(&state.db(), &row()).unwrap();
        state.github().tokens.set("tok").unwrap();
        state.github().needs_reconnect.store(true, Ordering::SeqCst);

        disconnect(&state).unwrap();
        assert_eq!(state.github().tokens.get().unwrap(), None);
        assert_eq!(get_account(&state.db()).unwrap(), None);
        assert!(!state.github().needs_reconnect.load(Ordering::SeqCst));
        assert_eq!(account(&state).unwrap(), None);
        assert_eq!(project_github_get(&state, &project_id).unwrap(), saved);
    }

    #[test]
    fn disconnect_removes_the_account_even_if_the_keyring_fails() {
        let (_tmp, state) = state_with(GithubState::new(
            GhClient::new("http://127.0.0.1:9", "http://127.0.0.1:9"),
            Box::new(MemoryTokenStore::failing()),
        ));
        save_account(&state.db(), &row()).unwrap();
        state.github().needs_reconnect.store(true, Ordering::SeqCst);
        let flow = state.github().begin_flow();

        let error = disconnect(&state).unwrap_err();
        assert_eq!(error.code(), "gh_keyring");
        assert_eq!(get_account(&state.db()).unwrap(), None);
        assert!(!state.github().needs_reconnect.load(Ordering::SeqCst));
        assert!(flow.load(Ordering::SeqCst), "el flujo en curso se cancela");
    }

    #[test]
    fn routing_on_an_archived_field_is_reported_stale() {
        let (_tmp, state) = fresh();
        let (project_id, field_id) = project_with_routing_field(&state);
        let saved = project_github_set(&state, &project_id, &input(&field_id)).unwrap();
        assert!(!saved.routing_stale);

        state
            .db()
            .execute(
                "UPDATE project_field SET archived = 1 WHERE id = ?1",
                [&field_id],
            )
            .unwrap();
        let loaded = project_github_get(&state, &project_id).unwrap();
        assert!(loaded.routing_stale);
        assert_eq!(
            loaded.routing_map, saved.routing_map,
            "la config se conserva"
        );
        assert_eq!(loaded.repos, saved.repos);

        // Igual con una opción que ya no existe.
        state
            .db()
            .execute(
                "UPDATE project_field SET archived = 0, options = '[\"Frontend\",\"Back\"]' WHERE id = ?1",
                [&field_id],
            )
            .unwrap();
        assert!(
            project_github_get(&state, &project_id)
                .unwrap()
                .routing_stale
        );
    }

    #[test]
    fn set_rejects_an_unknown_project() {
        let (_tmp, state) = fresh();
        let error = project_github_set(&state, "no-existe", &input("x")).unwrap_err();
        assert_eq!(error.code(), "not_found");
        assert_eq!(
            project_github_get(&state, "no-existe").unwrap_err().code(),
            "not_found"
        );
    }

    #[test]
    fn set_reports_validation_errors() {
        let (_tmp, state) = fresh();
        let (project_id, field_id) = project_with_routing_field(&state);
        let mut bad = input(&field_id);
        bad.repos[0].is_default = false;
        assert_eq!(
            project_github_set(&state, &project_id, &bad)
                .unwrap_err()
                .user_message(),
            "Marca un repositorio como predeterminado."
        );
    }

    async fn user_mocks(server: &mut mockito::ServerGuard, status: usize) -> mockito::Mock {
        server
            .mock("GET", "/user")
            .with_status(status)
            .with_body(r#"{"login":"Ps3udoDev","name":null,"avatar_url":null}"#)
            .create_async()
            .await
    }

    #[tokio::test]
    async fn a_token_after_cancelling_is_discarded() {
        let mut server = mockito::Server::new_async().await;
        let m = user_mocks(&mut server, 200).await.expect(0);
        let (_tmp, state) = state_at(&server.url());
        let cancel = AtomicBool::new(true);
        let outcome = after_token(state.github(), state.db_mutex(), "tok".into(), &cancel).await;
        assert!(outcome.is_none());
        assert_eq!(state.github().tokens.get().unwrap(), None);
        assert_eq!(get_account(&state.db()).unwrap(), None);
        m.assert_async().await;
    }

    /// `/user` que cancela el flujo justo mientras se guarda la cuenta nueva.
    async fn user_mock_that_cancels(
        server: &mut mockito::ServerGuard,
        cancel: &std::sync::Arc<AtomicBool>,
    ) -> mockito::Mock {
        let flag = cancel.clone();
        server
            .mock("GET", "/user")
            .with_body_from_request(move |_| {
                flag.store(true, Ordering::SeqCst);
                br#"{"login":"Otra","name":null,"avatar_url":null}"#.to_vec()
            })
            .create_async()
            .await
    }

    #[tokio::test]
    async fn cancelling_a_reconnect_while_saving_keeps_the_previous_account() {
        let mut server = mockito::Server::new_async().await;
        let cancel = std::sync::Arc::new(AtomicBool::new(false));
        let _m = user_mock_that_cancels(&mut server, &cancel).await;
        let (_tmp, state) = state_at(&server.url());
        save_account(&state.db(), &row()).unwrap();
        state.github().tokens.set("anterior").unwrap();

        let outcome = after_token(state.github(), state.db_mutex(), "nuevo".into(), &cancel).await;
        assert!(outcome.is_none());
        assert_eq!(get_account(&state.db()).unwrap(), Some(row()));
        assert_eq!(
            state.github().tokens.get().unwrap().as_deref(),
            Some("anterior")
        );
    }

    #[tokio::test]
    async fn cancelling_a_first_connect_while_saving_leaves_nothing() {
        let mut server = mockito::Server::new_async().await;
        let cancel = std::sync::Arc::new(AtomicBool::new(false));
        let _m = user_mock_that_cancels(&mut server, &cancel).await;
        let (_tmp, state) = state_at(&server.url());

        let outcome = after_token(state.github(), state.db_mutex(), "nuevo".into(), &cancel).await;
        assert!(outcome.is_none());
        assert_eq!(get_account(&state.db()).unwrap(), None);
        assert_eq!(state.github().tokens.get().unwrap(), None);
    }

    #[tokio::test]
    async fn connecting_saves_the_account_and_clears_reconnect() {
        let mut server = mockito::Server::new_async().await;
        let _m = user_mocks(&mut server, 200).await;
        let (_tmp, state) = state_at(&server.url());
        state.github().needs_reconnect.store(true, Ordering::SeqCst);
        let cancel = AtomicBool::new(false);
        let outcome = after_token(state.github(), state.db_mutex(), "tok".into(), &cancel).await;
        let Some(FlowOutcome::Connected(dto)) = outcome else {
            panic!("se esperaba la cuenta conectada: {outcome:?}");
        };
        assert_eq!(dto.login, "Ps3udoDev");
        assert!(!dto.needs_reconnect);
        assert!(!state.github().needs_reconnect.load(Ordering::SeqCst));
        assert_eq!(state.github().tokens.get().unwrap().as_deref(), Some("tok"));
    }

    #[tokio::test]
    async fn a_failure_after_storing_the_token_removes_it() {
        let mut server = mockito::Server::new_async().await;
        let _m = user_mocks(&mut server, 502).await;
        let (_tmp, state) = state_at(&server.url());
        let cancel = AtomicBool::new(false);
        let outcome = after_token(state.github(), state.db_mutex(), "tok".into(), &cancel).await;
        assert!(matches!(outcome, Some(FlowOutcome::Failed(ref m)) if !m.is_empty()));
        assert_eq!(
            state.github().tokens.get().unwrap(),
            None,
            "sin token huérfano"
        );
        assert_eq!(get_account(&state.db()).unwrap(), None);
        assert!(
            !state.github().needs_reconnect.load(Ordering::SeqCst),
            "un fallo del Device Flow no pide reconectar"
        );
    }

    #[tokio::test]
    async fn a_denied_authorization_reports_its_message() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("POST", "/login/oauth/access_token")
            .with_body(r#"{"error":"access_denied"}"#)
            .create_async()
            .await;
        let (_tmp, state) = state_at(&server.url());
        let code = DeviceCode {
            device_code: "dc".into(),
            user_code: "ABCD-1234".into(),
            verification_uri: "https://github.com/login/device".into(),
            expires_in: 900,
            interval: 0,
        };
        let cancel = AtomicBool::new(false);
        let outcome = finish_flow(state.github(), state.db_mutex(), &code, &cancel).await;
        assert!(
            matches!(outcome, Some(FlowOutcome::Failed(ref m)) if m == "Se canceló la autorización en GitHub.")
        );
        assert!(!state.github().needs_reconnect.load(Ordering::SeqCst));
    }

    // --- reportar (F8b) -------------------------------------------------------------------

    use crate::repo::integrations::{LinkedRepo, ProjectGithub, insert_link, save_project_github};

    /// Caso de un proyecto vinculado a `Ps3udoDev/testing`, con token y una captura real.
    fn report_fixture(api: &str) -> (TempDir, AppState, String, String) {
        let (tmp, state) = state_at(api);
        state.github().tokens.set("t0k").unwrap();
        let (case_id, ev_id) = {
            let conn = state.db();
            let p = repo::test_support::project(&conn);
            let c = repo::test_support::case(&conn, &p);
            save_project_github(
                &conn,
                &p.id,
                &ProjectGithub {
                    repos: vec![LinkedRepo {
                        repo: RepoRef::parse("Ps3udoDev/testing").unwrap(),
                        is_default: true,
                        default_labels: vec![],
                    }],
                    ..ProjectGithub::default()
                },
            )
            .unwrap();
            std::fs::create_dir_all(state.files().project_dir(&p.id)).unwrap();
            let rel = crate::media::storage::EvidenceFiles::render_rel(&p.id, "ev0");
            let mut png = std::io::Cursor::new(Vec::new());
            image::RgbaImage::from_pixel(2, 1, image::Rgba([1, 2, 3, 255]))
                .write_to(&mut png, image::ImageFormat::Png)
                .unwrap();
            std::fs::write(state.files().absolute(&rel), png.into_inner()).unwrap();
            conn.execute(
                "INSERT INTO evidence (id, case_id, position, description, original_path,
                   rendered_path, annotations, width, height, captured_at, updated_at)
                 VALUES ('ev0', ?1, 0, 'd', ?2, ?2, '{}', 2, 1, '', '')",
                rusqlite::params![c.id, rel],
            )
            .unwrap();
            (c.id, "ev0".to_string())
        };
        (tmp, state, case_id, ev_id)
    }

    fn report_dto(case_id: &str, repo: &str) -> ReportInputDto {
        ReportInputDto {
            case_id: case_id.into(),
            repo: repo.into(),
            title: "[EJ-1] Falla".into(),
            body: "Pasos".into(),
            labels: vec![],
            assignees: vec![],
            milestone: None,
            evidence_ids: vec![],
        }
    }

    fn at() -> NaiveDateTime {
        chrono::NaiveDate::from_ymd_opt(2026, 11, 2)
            .unwrap()
            .and_hms_opt(15, 30, 12)
            .unwrap()
    }

    fn stored_link(case_id: &str) -> CaseIssueLink {
        CaseIssueLink {
            case_id: case_id.into(),
            owner: "Ps3udoDev".into(),
            repo: "testing".into(),
            issue_number: 45,
            node_id: "I_kw".into(),
            html_url: "https://github.com/Ps3udoDev/testing/issues/45".into(),
            direction: "exported".into(),
            issue_state: "open".into(),
        }
    }

    #[test]
    fn open_issue_uses_the_stored_url() {
        let (_tmp, state, case_id, _) = report_fixture("http://127.0.0.1:9");
        assert_eq!(
            stored_issue_url(&state, &case_id, "Ps3udoDev/testing")
                .unwrap_err()
                .code(),
            "not_found"
        );
        insert_link(&state.db(), &stored_link(&case_id)).unwrap();
        assert_eq!(
            stored_issue_url(&state, &case_id, "ps3udodev/TESTING").unwrap(),
            "https://github.com/Ps3udoDev/testing/issues/45"
        );
        assert_eq!(
            stored_issue_url(&state, &case_id, "https://evil.example/x")
                .unwrap_err()
                .code(),
            "validation"
        );
        let project_id = repo::cases::get(&state.db(), &case_id).unwrap().project_id;
        let links = project_links(&state, &project_id).unwrap();
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].full_name, "Ps3udoDev/testing");
    }

    #[test]
    fn copy_evidence_rejects_an_unknown_id() {
        let (_tmp, state, _, ev_id) = report_fixture("http://127.0.0.1:9");
        assert_eq!(
            evidence_rgba(&state, "no-existe").unwrap_err().code(),
            "not_found"
        );
        let (w, h, rgba) = evidence_rgba(&state, &ev_id).unwrap();
        assert_eq!((w, h), (2, 1));
        assert_eq!(rgba, vec![1, 2, 3, 255, 1, 2, 3, 255]);
    }

    #[test]
    fn report_maps_assisted_reasons_to_fixed_texts() {
        let repo = RepoRef::parse("Ps3udoDev/evidencias").unwrap();
        assert_eq!(
            assisted_text(&AssistedReason::NoEvidenceRepo),
            "No hay repo de evidencias configurado."
        );
        assert_eq!(
            assisted_text(&AssistedReason::CannotWrite(repo.clone())),
            "Tu cuenta no puede escribir en Ps3udoDev/evidencias."
        );
        assert_eq!(
            assisted_text(&AssistedReason::NotInstalled(repo)),
            "Qapture no está instalada en Ps3udoDev/evidencias."
        );
        let link = stored_link("case-1");
        let repo_mode = result_dto(&link, &UploadMode::Repo, vec![], None);
        assert_eq!(repo_mode.upload_mode, "repo");
        assert_eq!(repo_mode.assisted_reason, None);
        let assisted = result_dto(
            &link,
            &UploadMode::Assisted(AssistedReason::NoEvidenceRepo),
            vec!["ev0".into()],
            Some("https://c".into()),
        );
        assert_eq!(assisted.upload_mode, "assisted");
        assert_eq!(assisted.pending_evidence, ["ev0"]);
        assert_eq!(assisted.comment_url.as_deref(), Some("https://c"));
        assert_eq!(assisted.link.full_name, "Ps3udoDev/testing");
    }

    #[tokio::test]
    async fn a_401_while_reporting_marks_reconnect() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("POST", "/repos/Ps3udoDev/testing/issues")
            .with_status(401)
            .create_async()
            .await;
        let (_tmp, state, case_id, _) = report_fixture(&server.url());
        let error = create_issue(&state, report_dto(&case_id, "Ps3udoDev/testing"), at())
            .await
            .unwrap_err();
        assert_eq!(error.code(), "gh_auth");
        assert!(state.github().needs_reconnect.load(Ordering::SeqCst));

        // También al leer los metadatos del repo.
        state
            .github()
            .needs_reconnect
            .store(false, Ordering::SeqCst);
        let _l = server
            .mock("GET", "/repos/Ps3udoDev/testing/labels?per_page=100")
            .with_status(401)
            .create_async()
            .await;
        assert_eq!(
            repo_meta(&state, "Ps3udoDev/testing")
                .await
                .unwrap_err()
                .code(),
            "gh_auth"
        );
        assert!(state.github().needs_reconnect.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn reporting_to_a_repo_not_linked_to_the_project_is_rejected() {
        let mut server = mockito::Server::new_async().await;
        let post = server
            .mock("POST", mockito::Matcher::Any)
            .expect(0)
            .create_async()
            .await;
        let (_tmp, state, case_id, _) = report_fixture(&server.url());
        for run in [
            create_issue(&state, report_dto(&case_id, "otro/repo"), at()).await,
            comment_issue(&state, report_dto(&case_id, "otro/repo"), at()).await,
        ] {
            let error = run.unwrap_err();
            assert_eq!(error.code(), "validation");
            assert_eq!(
                error.user_message(),
                "«otro/repo» no está vinculado al proyecto."
            );
        }
        post.assert_async().await;
        assert!(!state.github().needs_reconnect.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn creating_returns_the_link_and_commenting_adds_the_comment_url() {
        let mut server = mockito::Server::new_async().await;
        let _issue = server
            .mock("POST", "/repos/Ps3udoDev/testing/issues")
            .with_status(201)
            .with_body(r#"{"number":45,"node_id":"I_kw","html_url":"https://github.com/Ps3udoDev/testing/issues/45","state":"open"}"#)
            .create_async()
            .await;
        let _comment = server
            .mock("POST", "/repos/Ps3udoDev/testing/issues/45/comments")
            .with_status(201)
            .with_body(
                r#"{"html_url":"https://github.com/Ps3udoDev/testing/issues/45#issuecomment-9"}"#,
            )
            .create_async()
            .await;
        let (_tmp, state, case_id, _) = report_fixture(&server.url());
        let created = create_issue(&state, report_dto(&case_id, "ps3udodev/TESTING"), at())
            .await
            .unwrap();
        assert_eq!(created.upload_mode, "repo");
        assert_eq!(created.link.issue_number, 45);
        assert_eq!(created.comment_url, None);

        let commented = comment_issue(&state, report_dto(&case_id, "Ps3udoDev/testing"), at())
            .await
            .unwrap();
        assert_eq!(commented.link, created.link);
        assert_eq!(
            commented.comment_url.as_deref(),
            Some("https://github.com/Ps3udoDev/testing/issues/45#issuecomment-9")
        );
    }
}
