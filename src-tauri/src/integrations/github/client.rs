//! Cliente HTTP de GitHub. El token solo viaja en el header `Authorization`; ningún error lo incluye.

use std::time::Duration;

use reqwest::header::{ACCEPT, AUTHORIZATION, HeaderMap, HeaderValue, USER_AGENT};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::config;
use crate::error::{AppError, AppResult, GhErrorKind, gh};
use crate::repo::integrations::RepoRef;

const MAX_PAGES: usize = 50;
const GENERIC_IO: &str = "respuesta inesperada de GitHub";
const NO_HTTP: &str = "no se pudo preparar la conexión con GitHub";
/// Plazo de cada solicitud, salvo los blobs.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);
/// Plazo de un `POST` de blob: un PNG de varios MB en base64 con una red lenta.
pub const BLOB_TIMEOUT: Duration = Duration::from_secs(120);

pub struct GhClient {
    api: String,
    web: String,
    /// `None` si el cliente HTTP no se pudo construir: las solicitudes fallan con un error,
    /// nunca con un pánico (se construye al arrancar la app).
    http: Option<reqwest::Client>,
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
        // Mismo patrón que tauri-plugin-updater: reqwest no trae proveedor TLS propio.
        if rustls::crypto::CryptoProvider::get_default().is_none() {
            let _ = rustls::crypto::ring::default_provider().install_default();
        }
        let http = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .ok();
        Self::with_http(api_base, web_base, http)
    }

    fn with_http(api_base: &str, web_base: &str, http: Option<reqwest::Client>) -> Self {
        Self {
            api: api_base.trim_end_matches('/').to_string(),
            web: web_base.trim_end_matches('/').to_string(),
            http,
        }
    }

    /// Cliente cuyo HTTP no se pudo construir.
    #[cfg(test)]
    pub(crate) fn without_http(api_base: &str, web_base: &str) -> Self {
        Self::with_http(api_base, web_base, None)
    }

    fn http(&self) -> AppResult<&reqwest::Client> {
        self.http.as_ref().ok_or_else(|| generic(NO_HTTP))
    }

    /// Mismo esquema, host y puerto que el API, y ruta dentro de su base. Sin credenciales.
    fn is_api_url(&self, candidate: &str) -> bool {
        let (Ok(base), Ok(url)) = (
            reqwest::Url::parse(&self.api),
            reqwest::Url::parse(candidate),
        ) else {
            return false;
        };
        let base_path = base.path().trim_end_matches('/');
        url.scheme() == base.scheme()
            && url.host_str() == base.host_str()
            && url.port_or_known_default() == base.port_or_known_default()
            && url.username().is_empty()
            && url.password().is_none()
            && (base_path.is_empty() || url.path().starts_with(&format!("{base_path}/")))
    }

    /// Base web (`https://github.com` en producción) para armar enlaces como los de las capturas.
    pub fn web_base(&self) -> &str {
        &self.web
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

    fn api_request(&self, token: &str, url: &str) -> AppResult<reqwest::RequestBuilder> {
        self.api_request_with(reqwest::Method::GET, token, url)
    }

    fn api_request_with(
        &self,
        method: reqwest::Method,
        token: &str,
        url: &str,
    ) -> AppResult<reqwest::RequestBuilder> {
        // Un token con caracteres no válidos para un header no se manda (ni se muestra).
        let mut auth =
            HeaderValue::from_str(&format!("Bearer {token}")).map_err(|_| generic(GENERIC_IO))?;
        auth.set_sensitive(true);
        Ok(self
            .http()?
            .request(method, url)
            .header(AUTHORIZATION, auth)
            .header(ACCEPT, "application/vnd.github+json")
            .header("X-GitHub-Api-Version", config::API_VERSION)
            .header(USER_AGENT, config::user_agent()))
    }

    pub async fn get_json<T: DeserializeOwned>(
        &self,
        token: &str,
        path: &str,
        repo: Option<&RepoRef>,
    ) -> AppResult<T> {
        let url = format!("{}{}", self.api, path);
        let (_, bytes) = self.send(self.api_request(token, &url)?, repo).await?;
        serde_json::from_slice(&bytes).map_err(|_| generic(GENERIC_IO))
    }

    /// Escritura al API con el plazo del cliente (20 s).
    fn write_request<B: Serialize>(
        &self,
        method: reqwest::Method,
        token: &str,
        path: &str,
        body: &B,
    ) -> AppResult<reqwest::RequestBuilder> {
        let url = format!("{}{}", self.api, path);
        Ok(self.api_request_with(method, token, &url)?.json(body))
    }

    /// `POST` de un blob de Git Data (un PNG en base64): lleva su propio plazo, `BLOB_TIMEOUT`.
    fn blob_request<B: Serialize>(
        &self,
        token: &str,
        path: &str,
        body: &B,
    ) -> AppResult<reqwest::RequestBuilder> {
        Ok(self
            .write_request(reqwest::Method::POST, token, path, body)?
            .timeout(BLOB_TIMEOUT))
    }

    async fn read_json<T: DeserializeOwned>(
        &self,
        request: reqwest::RequestBuilder,
        repo: Option<&RepoRef>,
    ) -> AppResult<T> {
        let (_, bytes) = self.send(request, repo).await?;
        serde_json::from_slice(&bytes).map_err(|_| generic(GENERIC_IO))
    }

    async fn write_json<B: Serialize, T: DeserializeOwned>(
        &self,
        method: reqwest::Method,
        token: &str,
        path: &str,
        body: &B,
        repo: Option<&RepoRef>,
    ) -> AppResult<T> {
        let request = self.write_request(method, token, path, body)?;
        self.read_json(request, repo).await
    }

    pub async fn post_json<B: Serialize, T: DeserializeOwned>(
        &self,
        token: &str,
        path: &str,
        body: &B,
        repo: Option<&RepoRef>,
    ) -> AppResult<T> {
        self.write_json(reqwest::Method::POST, token, path, body, repo)
            .await
    }

    /// Como `post_json`, con el plazo largo de los blobs.
    pub async fn post_blob_json<B: Serialize, T: DeserializeOwned>(
        &self,
        token: &str,
        path: &str,
        body: &B,
        repo: Option<&RepoRef>,
    ) -> AppResult<T> {
        let request = self.blob_request(token, path, body)?;
        self.read_json(request, repo).await
    }

    pub async fn patch_json<B: Serialize, T: DeserializeOwned>(
        &self,
        token: &str,
        path: &str,
        body: &B,
        repo: Option<&RepoRef>,
    ) -> AppResult<T> {
        self.write_json(reqwest::Method::PATCH, token, path, body, repo)
            .await
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
            let (headers, bytes) = self.send(self.api_request(token, &current)?, None).await?;
            let value: Value = serde_json::from_slice(&bytes).map_err(|_| generic(GENERIC_IO))?;
            let items = match items_key {
                Some(key) => value.get(key).cloned().unwrap_or(Value::Null),
                None => value,
            };
            let page: Vec<T> = serde_json::from_value(items).map_err(|_| generic(GENERIC_IO))?;
            out.extend(page);
            // Nunca se manda el token a un host distinto del API configurado.
            url = next_link(&headers).filter(|u| self.is_api_url(u));
        }
        Ok(out)
    }

    pub async fn post_form_web<T: DeserializeOwned>(
        &self,
        path: &str,
        form: &[(&str, &str)],
    ) -> AppResult<T> {
        let request = self
            .http()?
            .post(format!("{}{}", self.web, path))
            .header(ACCEPT, "application/json")
            .header(USER_AGENT, config::user_agent())
            .form(form);
        let (_, bytes) = self.send(request, None).await?;
        serde_json::from_slice(&bytes).map_err(|_| generic(GENERIC_IO))
    }

    pub async fn get_bytes(&self, url: &str) -> AppResult<Vec<u8>> {
        let request = self
            .http()?
            .get(url)
            .header(USER_AGENT, config::user_agent());
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

    /// Un PNG de varios MB con una red lenta no cabe en los 20 s del cliente: los blobs
    /// llevan su propio plazo y el resto de las escrituras se queda con el del cliente.
    #[test]
    fn blob_posts_get_their_own_longer_timeout() {
        let client = GhClient::new("https://api.example", "https://web.example");
        let body = json!({ "content": "", "encoding": "base64" });
        let blob = client
            .blob_request("t0k", "/repos/o/r/git/blobs", &body)
            .unwrap()
            .build()
            .unwrap();
        assert_eq!(blob.timeout(), Some(&BLOB_TIMEOUT));
        assert_eq!(BLOB_TIMEOUT, Duration::from_secs(120));
        assert_eq!(blob.method(), reqwest::Method::POST);
        assert_eq!(
            blob.url().as_str(),
            "https://api.example/repos/o/r/git/blobs"
        );
        let other = client
            .write_request(reqwest::Method::POST, "t0k", "/repos/o/r/git/trees", &body)
            .unwrap()
            .build()
            .unwrap();
        assert_eq!(other.timeout(), None, "usa el plazo de 20 s del cliente");
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

    #[test]
    fn look_alike_hosts_are_not_the_api() {
        let client = GhClient::production();
        assert!(client.is_api_url("https://api.github.com/repos?page=2"));
        assert!(!client.is_api_url("https://api.github.com.evil.example/repos"));
        assert!(!client.is_api_url("https://api.github.com@evil.example/repos"));
        assert!(!client.is_api_url("https://evil.example@api.github.com/repos"));
        assert!(!client.is_api_url("http://api.github.com/repos"));
        assert!(!client.is_api_url("https://api.github.com:444/repos"));
        assert!(!client.is_api_url("not a url"));
    }

    #[tokio::test]
    async fn a_client_that_failed_to_build_returns_errors_instead_of_panicking() {
        let client = GhClient::without_http("http://127.0.0.1:9", "http://127.0.0.1:9");
        let err = client
            .get_json::<Value>("t0k", "/x", None)
            .await
            .unwrap_err();
        assert_eq!(err.code(), "io");
        assert!(!err.user_message().is_empty());
        assert_no_token(&err);
        assert!(
            client
                .get_paginated::<Value>("t0k", "/x", None)
                .await
                .is_err()
        );
        assert!(
            client
                .post_form_web::<Value>("/login/device/code", &[])
                .await
                .is_err()
        );
        assert!(client.get_bytes("http://127.0.0.1:9/a.png").await.is_err());
    }

    #[test]
    fn authorization_header_is_marked_sensitive() {
        let client = GhClient::production();
        let request = client
            .api_request("t0k", "https://api.github.com/user")
            .unwrap()
            .build()
            .unwrap();
        assert!(request.headers()[AUTHORIZATION].is_sensitive());
    }

    #[test]
    fn production_client_builds_without_panicking() {
        let _ = GhClient::production();
        let _ = GhClient::production();
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
    async fn post_json_sends_body_and_headers() {
        let mut server = mockito::Server::new_async().await;
        let m = server
            .mock("POST", "/x")
            .match_header("authorization", "Bearer t0k")
            .match_header("x-github-api-version", "2022-11-28")
            .match_header("accept", "application/vnd.github+json")
            .match_header("content-type", "application/json")
            .match_body(mockito::Matcher::Json(json!({ "a": 1 })))
            .with_status(201)
            .with_body(r#"{"ok":true}"#)
            .create_async()
            .await;
        let client = GhClient::new(&server.url(), &server.url());
        let v: Value = client
            .post_json("t0k", "/x", &json!({ "a": 1 }), None)
            .await
            .unwrap();
        assert_eq!(v["ok"], true);
        m.assert_async().await;
    }

    #[tokio::test]
    async fn patch_json_maps_422_to_validation() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("PATCH", "/x")
            .with_status(422)
            .with_body(r#"{"message":"Update is not a fast forward"}"#)
            .create_async()
            .await;
        let client = GhClient::new(&server.url(), &server.url());
        let err = client
            .patch_json::<_, Value>("t0k", "/x", &json!({}), None)
            .await
            .unwrap_err();
        assert_eq!(err.code(), "gh_validation");
        assert_eq!(
            err.user_message(),
            "GitHub rechazó la solicitud: Update is not a fast forward."
        );
        assert_no_token(&err);
    }

    #[tokio::test]
    async fn write_errors_never_contain_the_token() {
        for status in [401usize, 403, 404, 422, 502] {
            let mut server = mockito::Server::new_async().await;
            let _m = server
                .mock("POST", "/x")
                .with_status(status)
                .with_body("t0k")
                .create_async()
                .await;
            let client = GhClient::new(&server.url(), &server.url());
            let repo = RepoRef::parse("o/r").unwrap();
            let err = client
                .post_json::<_, Value>("t0k", "/x", &json!({}), Some(&repo))
                .await
                .unwrap_err();
            assert_no_token(&err);
        }
        let offline = GhClient::new("http://127.0.0.1:9", "http://127.0.0.1:9");
        let err = offline
            .patch_json::<_, Value>("t0k", "/x", &json!({}), None)
            .await
            .unwrap_err();
        assert_no_token(&err);
        let broken = GhClient::without_http("http://127.0.0.1:9", "http://127.0.0.1:9");
        let err = broken
            .post_json::<_, Value>("t0k", "/x", &json!({}), None)
            .await
            .unwrap_err();
        assert_no_token(&err);
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
