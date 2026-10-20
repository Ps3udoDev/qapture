//! Cliente HTTP de GitHub. El token solo viaja en el header `Authorization`; ningún error lo incluye.

use std::time::Duration;

use reqwest::header::{ACCEPT, AUTHORIZATION, HeaderMap, USER_AGENT};
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::config;
use crate::error::{AppError, AppResult, GhErrorKind, gh};
use crate::repo::integrations::RepoRef;

const MAX_PAGES: usize = 50;
const GENERIC_IO: &str = "respuesta inesperada de GitHub";

pub struct GhClient {
    api: String,
    web: String,
    http: reqwest::Client,
}

fn generic(detail: &str) -> AppError {
    AppError::Io(std::io::Error::other(detail.to_string()))
}

fn offline() -> AppError {
    gh(
        GhErrorKind::Offline,
        "No hay conexión con GitHub. Revisa tu red e inténtalo de nuevo.",
    )
}

fn header_i64(headers: &HeaderMap, name: &str) -> Option<i64> {
    headers.get(name)?.to_str().ok()?.trim().parse().ok()
}

fn local_hhmm(epoch: i64) -> Option<String> {
    Some(
        chrono::DateTime::from_timestamp(epoch, 0)?
            .with_timezone(&chrono::Local)
            .format("%H:%M")
            .to_string(),
    )
}

fn rate_limited(headers: &HeaderMap) -> AppError {
    let reset = header_i64(headers, "x-ratelimit-reset").or_else(|| {
        header_i64(headers, "retry-after").map(|secs| chrono::Utc::now().timestamp() + secs)
    });
    let message = match reset.and_then(local_hhmm) {
        Some(hhmm) => format!("GitHub limitó las solicitudes. Intenta de nuevo a las {hhmm}."),
        None => "GitHub limitó las solicitudes. Intenta de nuevo en unos minutos.".to_string(),
    };
    gh(GhErrorKind::RateLimited, message)
}

pub(crate) fn map_status(
    status: u16,
    headers: &HeaderMap,
    body: &str,
    repo: Option<&RepoRef>,
) -> AppError {
    match status {
        401 => gh(
            GhErrorKind::Auth,
            "Tu cuenta de GitHub no está conectada o la sesión venció. Conéctala en Ajustes › GitHub.",
        ),
        403 | 429 => {
            let exhausted = header_i64(headers, "x-ratelimit-remaining") == Some(0)
                || headers.contains_key("retry-after");
            if status == 429 || exhausted {
                rate_limited(headers)
            } else {
                let message = match repo {
                    Some(r) => format!(
                        "Tu cuenta de GitHub no tiene permiso para esta acción en {}.",
                        r.full_name()
                    ),
                    None => "Tu cuenta de GitHub no tiene permiso para esta acción.".to_string(),
                };
                gh(GhErrorKind::Forbidden, message)
            }
        }
        404 => {
            let target = repo
                .map(RepoRef::full_name)
                .unwrap_or_else(|| "el recurso".to_string());
            gh(
                GhErrorKind::NotFound,
                format!("No se encontró {target} o Qapture no está instalada en él."),
            )
        }
        422 => {
            let detail = serde_json::from_str::<Value>(body)
                .ok()
                .and_then(|v| v["message"].as_str().map(str::to_string))
                .unwrap_or_else(|| "datos no válidos".to_string());
            gh(
                GhErrorKind::Validation,
                format!("GitHub rechazó la solicitud: {detail}."),
            )
        }
        _ => generic(&format!("GitHub respondió {status}")),
    }
}

/// `Link: <url>; rel="next"` -> url.
fn next_link(headers: &HeaderMap) -> Option<String> {
    let link = headers.get("link")?.to_str().ok()?;
    link.split(',').find_map(|part| {
        let (url, rels) = part.split_once(';')?;
        rels.contains("rel=\"next\"").then(|| {
            url.trim()
                .trim_start_matches('<')
                .trim_end_matches('>')
                .to_string()
        })
    })
}

impl GhClient {
    pub fn new(api_base: &str, web_base: &str) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .build()
            .unwrap_or_default();
        Self {
            api: api_base.trim_end_matches('/').to_string(),
            web: web_base.trim_end_matches('/').to_string(),
            http,
        }
    }

    pub fn production() -> Self {
        Self::new(config::API_BASE, config::WEB_BASE)
    }

    /// Envía, traduce los fallos y devuelve headers + cuerpo de una respuesta exitosa.
    async fn send(
        &self,
        request: reqwest::RequestBuilder,
        repo: Option<&RepoRef>,
    ) -> AppResult<(HeaderMap, Vec<u8>)> {
        let response = request.send().await.map_err(|_| offline())?;
        let status = response.status();
        let headers = response.headers().clone();
        let bytes = response.bytes().await.map_err(|_| offline())?;
        if !status.is_success() {
            let body = String::from_utf8_lossy(&bytes);
            return Err(map_status(status.as_u16(), &headers, &body, repo));
        }
        Ok((headers, bytes.to_vec()))
    }

    fn api_request(&self, token: &str, url: &str) -> reqwest::RequestBuilder {
        self.http
            .get(url)
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .header(ACCEPT, "application/vnd.github+json")
            .header("X-GitHub-Api-Version", config::API_VERSION)
            .header(USER_AGENT, config::user_agent())
    }

    pub async fn get_json<T: DeserializeOwned>(
        &self,
        token: &str,
        path: &str,
        repo: Option<&RepoRef>,
    ) -> AppResult<T> {
        let url = format!("{}{}", self.api, path);
        let (_, bytes) = self.send(self.api_request(token, &url), repo).await?;
        serde_json::from_slice(&bytes).map_err(|_| generic(GENERIC_IO))
    }

    pub async fn get_paginated<T: DeserializeOwned>(
        &self,
        token: &str,
        path: &str,
        items_key: Option<&str>,
    ) -> AppResult<Vec<T>> {
        let sep = if path.contains('?') { '&' } else { '?' };
        let mut url = Some(format!("{}{}{}per_page=100", self.api, path, sep));
        let mut out = Vec::new();
        for _ in 0..MAX_PAGES {
            let Some(current) = url.take() else { break };
            let (headers, bytes) = self.send(self.api_request(token, &current), None).await?;
            let value: Value = serde_json::from_slice(&bytes).map_err(|_| generic(GENERIC_IO))?;
            let items = match items_key {
                Some(key) => value.get(key).cloned().unwrap_or(Value::Null),
                None => value,
            };
            let page: Vec<T> = serde_json::from_value(items).map_err(|_| generic(GENERIC_IO))?;
            out.extend(page);
            // Nunca se manda el token a un host distinto del API configurado.
            url = next_link(&headers).filter(|u| u.starts_with(&self.api));
        }
        Ok(out)
    }

    pub async fn post_form_web<T: DeserializeOwned>(
        &self,
        path: &str,
        form: &[(&str, &str)],
    ) -> AppResult<T> {
        let request = self
            .http
            .post(format!("{}{}", self.web, path))
            .header(ACCEPT, "application/json")
            .header(USER_AGENT, config::user_agent())
            .form(form);
        let (_, bytes) = self.send(request, None).await?;
        serde_json::from_slice(&bytes).map_err(|_| generic(GENERIC_IO))
    }

    pub async fn get_bytes(&self, url: &str) -> AppResult<Vec<u8>> {
        let request = self.http.get(url).header(USER_AGENT, config::user_agent());
        Ok(self.send(request, None).await?.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::GhErrorKind;
    use crate::repo::integrations::RepoRef;
    use serde_json::{Value, json};

    fn assert_no_token(err: &AppError) {
        assert!(!serde_json::to_string(err).unwrap().contains("t0k"));
    }

    async fn get_with(status: usize, headers: &[(&str, &str)], body: &str) -> AppError {
        let mut server = mockito::Server::new_async().await;
        let mut mock = server.mock("GET", "/x").with_status(status).with_body(body);
        for (k, v) in headers {
            mock = mock.with_header(*k, v);
        }
        let _m = mock.create_async().await;
        let client = GhClient::new(&server.url(), &server.url());
        let repo = RepoRef::parse("TEAMS4SOFT/GMX-frontend").unwrap();
        let err = client
            .get_json::<Value>("t0k", "/x", Some(&repo))
            .await
            .unwrap_err();
        assert_no_token(&err);
        err
    }

    #[tokio::test]
    async fn status_401_maps_to_auth() {
        let err = get_with(401, &[], "{}").await;
        assert_eq!(err.code(), "gh_auth");
        assert!(
            err.user_message()
                .starts_with("Tu cuenta de GitHub no está conectada")
        );
    }

    #[tokio::test]
    async fn status_403_with_rate_limit_remaining_0_maps_to_rate_limited() {
        let reset = 1_900_000_000i64;
        let err = get_with(
            403,
            &[
                ("x-ratelimit-remaining", "0"),
                ("x-ratelimit-reset", "1900000000"),
            ],
            "{}",
        )
        .await;
        assert_eq!(err.code(), "gh_rate_limited");
        let hhmm = chrono::DateTime::from_timestamp(reset, 0)
            .unwrap()
            .with_timezone(&chrono::Local)
            .format("%H:%M")
            .to_string();
        assert_eq!(
            err.user_message(),
            format!("GitHub limitó las solicitudes. Intenta de nuevo a las {hhmm}.")
        );
    }

    #[tokio::test]
    async fn status_403_otherwise_maps_to_forbidden_with_repo() {
        let err = get_with(403, &[], "{}").await;
        assert_eq!(err.code(), "gh_forbidden");
        assert_eq!(
            err.user_message(),
            "Tu cuenta de GitHub no tiene permiso para esta acción en TEAMS4SOFT/GMX-frontend."
        );
    }

    #[test]
    fn forbidden_without_repo_omits_it() {
        let err = map_status(403, &HeaderMap::new(), "", None);
        assert_eq!(
            err.user_message(),
            "Tu cuenta de GitHub no tiene permiso para esta acción."
        );
    }

    #[tokio::test]
    async fn status_404_maps_to_not_found() {
        let err = get_with(404, &[], "{}").await;
        assert_eq!(err.code(), "gh_not_found");
        assert_eq!(
            err.user_message(),
            "No se encontró TEAMS4SOFT/GMX-frontend o Qapture no está instalada en él."
        );
    }

    #[tokio::test]
    async fn status_422_maps_to_validation_with_github_message() {
        let err = get_with(422, &[], r#"{"message":"Validation Failed"}"#).await;
        assert_eq!(err.code(), "gh_validation");
        assert_eq!(
            err.user_message(),
            "GitHub rechazó la solicitud: Validation Failed."
        );
    }

    #[tokio::test]
    async fn server_errors_map_to_the_generic_error() {
        let err = get_with(502, &[], "bad gateway").await;
        assert_eq!(err.code(), "io");
    }

    #[tokio::test]
    async fn html_body_maps_to_a_friendly_error() {
        let err = get_with(200, &[], "<html><body>Sign in</body></html>").await;
        assert_eq!(err.code(), "io");
        assert!(!err.user_message().is_empty());
    }

    #[tokio::test]
    async fn connection_refused_maps_to_offline() {
        let client = GhClient::new("http://127.0.0.1:9", "http://127.0.0.1:9");
        let err = client
            .get_json::<Value>("t0k", "/x", None)
            .await
            .unwrap_err();
        assert_eq!(err.code(), "gh_offline");
        assert_eq!(
            err.user_message(),
            "No hay conexión con GitHub. Revisa tu red e inténtalo de nuevo."
        );
        assert!(matches!(
            err,
            AppError::GitHub {
                kind: GhErrorKind::Offline,
                ..
            }
        ));
        assert_no_token(&err);
    }

    #[tokio::test]
    async fn pagination_follows_link_next() {
        let mut server = mockito::Server::new_async().await;
        let page = |from: usize, n: usize| {
            Value::Array((from..from + n).map(|i| json!({ "id": i })).collect()).to_string()
        };
        let next = format!("<{}/items?per_page=100&page=2>; rel=\"next\"", server.url());
        let _p1 = server
            .mock("GET", "/items?per_page=100")
            .with_header("link", &next)
            .with_body(page(0, 100))
            .create_async()
            .await;
        let _p2 = server
            .mock("GET", "/items?per_page=100&page=2")
            .with_body(page(100, 50))
            .create_async()
            .await;
        let client = GhClient::new(&server.url(), &server.url());
        let items: Vec<Value> = client.get_paginated("t0k", "/items", None).await.unwrap();
        assert_eq!(items.len(), 150);
        assert_eq!(items[149]["id"], 149);
    }

    #[tokio::test]
    async fn pagination_unwraps_items_key() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("GET", "/installation/repositories?per_page=100")
            .with_body(r#"{"total_count":2,"repositories":[{"id":1},{"id":2}]}"#)
            .create_async()
            .await;
        let client = GhClient::new(&server.url(), &server.url());
        let items: Vec<Value> = client
            .get_paginated("t0k", "/installation/repositories", Some("repositories"))
            .await
            .unwrap();
        assert_eq!(items.len(), 2);
    }

    #[tokio::test]
    async fn pagination_ignores_next_links_to_other_hosts() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("GET", "/items?per_page=100")
            .with_header("link", "<http://evil.example/items>; rel=\"next\"")
            .with_body("[{\"id\":1}]")
            .create_async()
            .await;
        let client = GhClient::new(&server.url(), &server.url());
        let items: Vec<Value> = client.get_paginated("t0k", "/items", None).await.unwrap();
        assert_eq!(items.len(), 1);
    }

    #[tokio::test]
    async fn requests_carry_the_required_headers() {
        let mut server = mockito::Server::new_async().await;
        let m = server
            .mock("GET", "/x")
            .match_header("authorization", "Bearer t0k")
            .match_header("x-github-api-version", "2022-11-28")
            .match_header("accept", "application/vnd.github+json")
            .match_header(
                "user-agent",
                mockito::Matcher::Regex("^Qapture/".to_string()),
            )
            .with_body("{}")
            .create_async()
            .await;
        let client = GhClient::new(&server.url(), &server.url());
        client.get_json::<Value>("t0k", "/x", None).await.unwrap();
        m.assert_async().await;
    }

    #[tokio::test]
    async fn post_form_web_sends_form_and_json_accept() {
        let mut server = mockito::Server::new_async().await;
        let m = server
            .mock("POST", "/login/device/code")
            .match_header("accept", "application/json")
            .match_header(
                "user-agent",
                mockito::Matcher::Regex("^Qapture/".to_string()),
            )
            .match_body("client_id=abc")
            .with_body(r#"{"device_code":"d"}"#)
            .create_async()
            .await;
        let client = GhClient::new(&server.url(), &server.url());
        let v: Value = client
            .post_form_web("/login/device/code", &[("client_id", "abc")])
            .await
            .unwrap();
        assert_eq!(v["device_code"], "d");
        m.assert_async().await;
    }

    #[tokio::test]
    async fn get_bytes_downloads_without_a_token() {
        let mut server = mockito::Server::new_async().await;
        let m = server
            .mock("GET", "/a.png")
            .match_header("authorization", mockito::Matcher::Missing)
            .with_body(vec![1u8, 2, 3])
            .create_async()
            .await;
        let client = GhClient::new(&server.url(), &server.url());
        let bytes = client
            .get_bytes(&format!("{}/a.png", server.url()))
            .await
            .unwrap();
        assert_eq!(bytes, vec![1, 2, 3]);
        m.assert_async().await;
    }
}
