//! Cuenta de GitHub (Device Flow), repos accesibles y configuración GitHub del proyecto.
//! El spec §5.2 llama `gh_project_repos_get/set` a los dos últimos; aquí son
//! `gh_project_github_*` porque guardan repos, ruteo y repo de evidencias.

use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};

use rusqlite::Connection;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::{
    dto::{
        DeviceCodeDto, GhAccountDto, GhConnectFailedDto, GhRepoDto, ProjectGithubDto,
        ProjectGithubInput,
    },
    error::{AppError, AppResult, GhErrorKind},
    integrations::github::{
        auth::{self, DeviceCode},
        project_config, repos,
    },
    repo::{self, integrations::GhAccountRow},
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
    match auth::complete(&github.client, github.tokens.as_ref(), conn, token).await {
        Ok(row) => {
            if cancel.load(Ordering::SeqCst) {
                // Cancelado mientras se guardaba: se deshace lo guardado y no se avisa.
                let _ = github.tokens.delete();
                let guard = conn.lock().unwrap_or_else(|p| p.into_inner());
                let _ = repo::integrations::delete_account(&guard);
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

/// Borra token y cuenta. Los vínculos de los proyectos se conservan.
pub(crate) fn disconnect(state: &AppState) -> AppResult<()> {
    let github = state.github();
    github.cancel_flow();
    github.tokens.delete()?;
    repo::integrations::delete_account(&state.db())?;
    github.needs_reconnect.store(false, Ordering::SeqCst);
    Ok(())
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::{
        db,
        dto::LinkedRepoDto,
        media::storage::TempDir,
        repo::integrations::{get_account, save_account},
    };

    fn state_at(api: &str) -> (TempDir, AppState) {
        let tmp = TempDir::new();
        let conn = db::open(&tmp.path().join(db::DB_FILE)).unwrap();
        let state = AppState::new(
            conn,
            tmp.files(),
            tmp.path(),
            None,
            false,
            GithubState::for_tests(api, api),
        );
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
}
