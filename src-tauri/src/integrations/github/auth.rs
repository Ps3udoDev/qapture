//! OAuth Device Flow de GitHub y cuenta conectada. El token solo pasa por el almacén del sistema.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use base64::{Engine, engine::general_purpose::STANDARD};
use rusqlite::Connection;
use serde::Deserialize;
use serde_json::Value;

use super::client::GhClient;
use super::config;
use super::token_store::TokenStore;
use crate::error::{AppResult, GhErrorKind, gh};
use crate::repo::integrations::{GhAccountRow, save_account};

const MSG_EXPIRED: &str = "El código venció. Vuelve a pulsar «Conectar con GitHub».";
const MSG_DENIED: &str = "Se canceló la autorización en GitHub.";
const MSG_DISABLED: &str = "La App de GitHub no tiene habilitado el Device Flow.";
const GRANT_TYPE: &str = "urn:ietf:params:oauth:grant-type:device_code";

#[derive(Debug, Clone, Deserialize)]
pub struct DeviceCode {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub enum PollStep {
    Wait(Duration),
    Token(String),
    Fail(String),
}

pub async fn start(client: &GhClient) -> AppResult<DeviceCode> {
    client
        .post_form_web("/login/device/code", &[("client_id", config::CLIENT_ID)])
        .await
}

/// Interpreta una respuesta de `/login/oauth/access_token`.
pub fn next_step(body: &Value, interval: Duration) -> PollStep {
    if let Some(token) = body.get("access_token").and_then(Value::as_str) {
        return PollStep::Token(token.to_string());
    }
    match body.get("error").and_then(Value::as_str) {
        Some("authorization_pending") => PollStep::Wait(interval),
        Some("slow_down") => PollStep::Wait(
            body.get("interval")
                .and_then(Value::as_u64)
                .map(Duration::from_secs)
                .unwrap_or(interval + Duration::from_secs(5)),
        ),
        Some("expired_token") => PollStep::Fail(MSG_EXPIRED.into()),
        Some("access_denied") => PollStep::Fail(MSG_DENIED.into()),
        Some("device_flow_disabled") => PollStep::Fail(MSG_DISABLED.into()),
        Some(other) => PollStep::Fail(
            body.get("error_description")
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
                .unwrap_or(other)
                .to_string(),
        ),
        None => PollStep::Fail("Respuesta inesperada de GitHub.".into()),
    }
}

/// Sondea hasta obtener el token. `Ok(None)` si se canceló.
pub async fn poll(
    client: &GhClient,
    code: &DeviceCode,
    cancel: &AtomicBool,
) -> AppResult<Option<String>> {
    let started = Instant::now();
    let limit = Duration::from_secs(code.expires_in);
    let mut interval = Duration::from_secs(code.interval);
    loop {
        if cancel.load(Ordering::SeqCst) {
            return Ok(None);
        }
        if started.elapsed() >= limit {
            return Err(gh(GhErrorKind::Auth, MSG_EXPIRED));
        }
        let body: Value = client
            .post_form_web(
                "/login/oauth/access_token",
                &[
                    ("client_id", config::CLIENT_ID),
                    ("device_code", &code.device_code),
                    ("grant_type", GRANT_TYPE),
                ],
            )
            .await?;
        // Un token que llega tras cancelar se descarta.
        if cancel.load(Ordering::SeqCst) {
            return Ok(None);
        }
        match next_step(&body, interval) {
            PollStep::Token(token) => return Ok(Some(token)),
            PollStep::Fail(message) => return Err(gh(GhErrorKind::Auth, message)),
            PollStep::Wait(wait) => {
                interval = wait;
                tokio::time::sleep(wait).await;
            }
        }
    }
}

#[derive(Deserialize)]
struct UserDto {
    login: String,
    name: Option<String>,
    avatar_url: Option<String>,
}

async fn fetch_avatar(client: &GhClient, url: &str) -> Option<String> {
    let sep = if url.contains('?') { '&' } else { '?' };
    let bytes = client.get_bytes(&format!("{url}{sep}s=64")).await.ok()?;
    if bytes.is_empty() {
        return None;
    }
    Some(format!("data:image/png;base64,{}", STANDARD.encode(bytes)))
}

pub async fn fetch_account(client: &GhClient, token: &str) -> AppResult<GhAccountRow> {
    let user: UserDto = client.get_json(token, "/user", None).await?;
    let avatar_data = match user.avatar_url.as_deref() {
        Some(url) => fetch_avatar(client, url).await,
        None => None,
    };
    Ok(GhAccountRow {
        login: user.login,
        name: user.name,
        avatar_data,
        connected_at: chrono::Utc::now().to_rfc3339(),
    })
}

/// Guarda el token, lee la cuenta y la persiste. Nunca retiene el candado de la base durante un `.await`.
pub async fn complete(
    client: &GhClient,
    tokens: &dyn TokenStore,
    conn: &Mutex<Connection>,
    token: String,
) -> AppResult<GhAccountRow> {
    tokens.set(&token)?;
    let account = fetch_account(client, &token).await?;
    let guard = conn.lock().unwrap_or_else(|p| p.into_inner());
    save_account(&guard, &account)?;
    Ok(account)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::integrations::github::token_store::MemoryTokenStore;
    use crate::repo::integrations::get_account;
    use crate::state::GithubState;
    use serde_json::json;
    use std::sync::Arc;

    fn code() -> DeviceCode {
        DeviceCode {
            device_code: "dc".into(),
            user_code: "ABCD-1234".into(),
            verification_uri: "https://github.com/login/device".into(),
            expires_in: 900,
            interval: 0,
        }
    }

    #[test]
    fn next_step_handles_every_device_flow_answer() {
        let i = Duration::from_secs(5);
        assert_eq!(
            next_step(&json!({"error": "authorization_pending"}), i),
            PollStep::Wait(i)
        );
        assert_eq!(
            next_step(&json!({"error": "slow_down"}), i),
            PollStep::Wait(Duration::from_secs(10))
        );
        assert_eq!(
            next_step(&json!({"error": "slow_down", "interval": 15}), i),
            PollStep::Wait(Duration::from_secs(15))
        );
        assert_eq!(
            next_step(&json!({"access_token": "tok"}), i),
            PollStep::Token("tok".into())
        );
        assert_eq!(
            next_step(&json!({"error": "expired_token"}), i),
            PollStep::Fail("El código venció. Vuelve a pulsar «Conectar con GitHub».".into())
        );
        assert_eq!(
            next_step(&json!({"error": "access_denied"}), i),
            PollStep::Fail("Se canceló la autorización en GitHub.".into())
        );
        assert_eq!(
            next_step(&json!({"error": "device_flow_disabled"}), i),
            PollStep::Fail("La App de GitHub no tiene habilitado el Device Flow.".into())
        );
        assert_eq!(
            next_step(
                &json!({"error": "boom", "error_description": "Algo falló"}),
                i
            ),
            PollStep::Fail("Algo falló".into())
        );
    }

    #[tokio::test]
    async fn poll_returns_the_token_after_pending() {
        let mut server = mockito::Server::new_async().await;
        let _pending = server
            .mock("POST", "/login/oauth/access_token")
            .with_body(r#"{"error":"authorization_pending"}"#)
            .expect(1)
            .create_async()
            .await;
        let _token = server
            .mock("POST", "/login/oauth/access_token")
            .with_body(r#"{"access_token":"tok123"}"#)
            .create_async()
            .await;
        let client = GhClient::new(&server.url(), &server.url());
        let cancel = AtomicBool::new(false);
        let token = poll(&client, &code(), &cancel).await.unwrap();
        assert_eq!(token.as_deref(), Some("tok123"));
    }

    #[tokio::test]
    async fn poll_stops_when_cancelled() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("POST", "/login/oauth/access_token")
            .with_body(r#"{"error":"authorization_pending"}"#)
            .create_async()
            .await;
        let client = GhClient::new(&server.url(), &server.url());
        let cancel = Arc::new(AtomicBool::new(false));
        let flag = cancel.clone();
        let dc = code();
        let (r, _) = tokio::join!(poll(&client, &dc, &cancel), async move {
            tokio::time::sleep(Duration::from_millis(30)).await;
            flag.store(true, Ordering::SeqCst);
        });
        assert_eq!(r.unwrap(), None);
    }

    #[test]
    fn a_second_start_cancels_the_first_poll() {
        let state = GithubState::for_tests("http://127.0.0.1:9", "http://127.0.0.1:9");
        let first = state.begin_flow();
        assert!(!first.load(Ordering::SeqCst));
        let second = state.begin_flow();
        assert!(first.load(Ordering::SeqCst));
        assert!(!second.load(Ordering::SeqCst));
        state.cancel_flow();
        assert!(second.load(Ordering::SeqCst));
    }

    async fn user_mocks(
        server: &mut mockito::ServerGuard,
        avatar_status: usize,
    ) -> Vec<mockito::Mock> {
        let body = format!(
            r#"{{"login":"Ps3udoDev","name":"Pseudo","avatar_url":"{}/av?v=4"}}"#,
            server.url()
        );
        vec![
            server
                .mock("GET", "/user")
                .with_body(body)
                .create_async()
                .await,
            server
                .mock("GET", "/av?v=4&s=64")
                .with_status(avatar_status)
                .with_body("PNGDATA")
                .create_async()
                .await,
        ]
    }

    #[tokio::test]
    async fn complete_saves_token_then_account() {
        let mut server = mockito::Server::new_async().await;
        let _m = user_mocks(&mut server, 200).await;
        let client = GhClient::new(&server.url(), &server.url());
        let tokens = MemoryTokenStore::default();
        let conn = Mutex::new(db::open_in_memory().unwrap());
        let row = complete(&client, &tokens, &conn, "tok".into())
            .await
            .unwrap();
        assert_eq!(row.login, "Ps3udoDev");
        assert_eq!(tokens.get().unwrap().as_deref(), Some("tok"));
        let saved = get_account(&conn.lock().unwrap()).unwrap().unwrap();
        assert_eq!(saved.login, "Ps3udoDev");
        assert_eq!(
            saved.avatar_data.as_deref(),
            Some("data:image/png;base64,UE5HREFUQQ==")
        );
    }

    #[tokio::test]
    async fn keyring_failure_leaves_no_account_row() {
        let mut server = mockito::Server::new_async().await;
        let _m = user_mocks(&mut server, 200).await;
        let client = GhClient::new(&server.url(), &server.url());
        let tokens = MemoryTokenStore::failing();
        let conn = Mutex::new(db::open_in_memory().unwrap());
        let err = complete(&client, &tokens, &conn, "tok".into())
            .await
            .unwrap_err();
        assert_eq!(err.code(), "gh_keyring");
        assert!(get_account(&conn.lock().unwrap()).unwrap().is_none());
    }

    #[tokio::test]
    async fn avatar_failure_does_not_block_connecting() {
        let mut server = mockito::Server::new_async().await;
        let _m = user_mocks(&mut server, 500).await;
        let client = GhClient::new(&server.url(), &server.url());
        let tokens = MemoryTokenStore::default();
        let conn = Mutex::new(db::open_in_memory().unwrap());
        complete(&client, &tokens, &conn, "tok".into())
            .await
            .unwrap();
        let saved = get_account(&conn.lock().unwrap()).unwrap().unwrap();
        assert_eq!(saved.login, "Ps3udoDev");
        assert_eq!(saved.avatar_data, None);
    }
}
