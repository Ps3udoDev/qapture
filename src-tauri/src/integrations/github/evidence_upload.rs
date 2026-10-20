//! Subida de capturas al repo de evidencias: todas las imágenes en un solo commit
//! (Git Data API: blobs -> tree -> commit -> ref). La ruta solo se genera aquí.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::Deserialize;
use serde_json::json;

use super::client::GhClient;
use super::issues::repo_info;
use crate::error::{AppError, AppResult, GhErrorKind, gh};
use crate::repo::integrations::RepoRef;

const MAX_ATTEMPTS: usize = 3;
const MAX_SLUG: usize = 60;

pub struct Upload {
    pub path: String,
    pub png: Vec<u8>,
}

fn fold(c: char) -> char {
    match c {
        'á' | 'à' | 'ä' | 'â' | 'ã' => 'a',
        'é' | 'è' | 'ë' | 'ê' => 'e',
        'í' | 'ì' | 'ï' | 'î' => 'i',
        'ó' | 'ò' | 'ö' | 'ô' | 'õ' => 'o',
        'ú' | 'ù' | 'ü' | 'û' => 'u',
        'ñ' => 'n',
        'ç' => 'c',
        other => other,
    }
}

/// Minúsculas, tildes plegadas, todo lo que no sea `[a-z0-9]` pasa a `-`; máximo 60 caracteres.
pub fn slug(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars().flat_map(char::to_lowercase).map(fold) {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    let trimmed = out.trim_matches('-');
    let cut: String = trimmed.chars().take(MAX_SLUG).collect();
    let cut = cut.trim_end_matches('-');
    if cut.is_empty() {
        "sin-nombre".to_string()
    } else {
        cut.to_string()
    }
}

/// `<slug-proyecto>/<code>/<AAAA-MM-DD_HHmmss>_<slug-tester>`; el código va tal cual.
pub fn evidence_dir(
    project_name: &str,
    code: &str,
    at: chrono::NaiveDateTime,
    tester: &str,
) -> String {
    format!(
        "{}/{}/{}_{}",
        slug(project_name),
        code,
        at.format("%Y-%m-%d_%H%M%S"),
        slug(tester)
    )
}

fn encode_segment(segment: &str) -> String {
    let mut out = String::new();
    for b in segment.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// `<web>/<o>/<r>/blob/<sha>/<ruta>?raw=true` (nunca raw.githubusercontent.com: con repo privado da 404).
pub fn image_url(web_base: &str, repo: &RepoRef, sha: &str, path: &str) -> String {
    let encoded: Vec<String> = path.split('/').map(encode_segment).collect();
    format!(
        "{}/{}/{}/blob/{}/{}?raw=true",
        web_base.trim_end_matches('/'),
        repo.owner,
        repo.repo,
        sha,
        encoded.join("/")
    )
}

#[derive(Deserialize)]
struct RefPayload {
    object: ShaObject,
}

#[derive(Deserialize)]
struct ShaObject {
    sha: String,
}

#[derive(Deserialize)]
struct CommitPayload {
    #[serde(default)]
    tree: Option<ShaObject>,
}

#[derive(Deserialize)]
struct Sha {
    sha: String,
}

fn empty_repo(repo: &RepoRef) -> AppError {
    gh(
        GhErrorKind::Validation,
        format!(
            "El repo de evidencias {} está vacío: crea un primer archivo (por ejemplo, un README) y vuelve a intentar.",
            repo.full_name()
        ),
    )
}

/// 404 (NotFound) o 409 (cae al error genérico `GitHub respondió 409`) al leer la ref.
fn is_missing_ref(err: &AppError) -> bool {
    match err {
        AppError::GitHub { kind, .. } => *kind == GhErrorKind::NotFound,
        AppError::Io(e) => e.to_string() == "GitHub respondió 409",
        _ => false,
    }
}

fn is_rejected_update(err: &AppError) -> bool {
    matches!(
        err,
        AppError::GitHub {
            kind: GhErrorKind::Validation,
            ..
        }
    )
}

/// Crea un único commit con todos los archivos en la rama predeterminada y devuelve su SHA.
/// Un 422 al mover la ref (otro commit llegó antes) reintenta desde la lectura de la ref.
pub async fn commit_files(
    client: &GhClient,
    token: &str,
    repo: &RepoRef,
    files: &[Upload],
    message: &str,
) -> AppResult<String> {
    let branch = repo_info(client, token, repo).await?.default_branch;
    let root = format!("/repos/{}/{}/git", repo.owner, repo.repo);
    let mut attempt = 0;
    loop {
        attempt += 1;
        let head: RefPayload = client
            .get_json(token, &format!("{root}/ref/heads/{branch}"), Some(repo))
            .await
            .map_err(|e| {
                if is_missing_ref(&e) {
                    empty_repo(repo)
                } else {
                    e
                }
            })?;
        let parent: CommitPayload = client
            .get_json(
                token,
                &format!("{root}/commits/{}", head.object.sha),
                Some(repo),
            )
            .await?;
        let base_tree = parent.tree.map(|t| t.sha).unwrap_or_default();

        // En serie, para respetar los límites secundarios de GitHub.
        let mut entries = Vec::with_capacity(files.len());
        for file in files {
            let blob: Sha = client
                .post_json(
                    token,
                    &format!("{root}/blobs"),
                    &json!({ "content": STANDARD.encode(&file.png), "encoding": "base64" }),
                    Some(repo),
                )
                .await?;
            entries.push(json!({
                "path": file.path, "mode": "100644", "type": "blob", "sha": blob.sha
            }));
        }
        let tree: Sha = client
            .post_json(
                token,
                &format!("{root}/trees"),
                &json!({ "base_tree": base_tree, "tree": entries }),
                Some(repo),
            )
            .await?;
        let commit: Sha = client
            .post_json(
                token,
                &format!("{root}/commits"),
                &json!({ "message": message, "tree": tree.sha, "parents": [head.object.sha] }),
                Some(repo),
            )
            .await?;
        let update: AppResult<serde_json::Value> = client
            .patch_json(
                token,
                &format!("{root}/refs/heads/{branch}"),
                &json!({ "sha": commit.sha, "force": false }),
                Some(repo),
            )
            .await;
        match update {
            Ok(_) => return Ok(commit.sha),
            Err(e) if is_rejected_update(&e) && attempt < MAX_ATTEMPTS => continue,
            Err(e) => return Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::{Matcher, Mock, ServerGuard};

    fn repo() -> RepoRef {
        RepoRef::parse("Ps3udoDev/evidencias").unwrap()
    }

    fn files() -> Vec<Upload> {
        vec![
            Upload {
                path: "p/EJ-1/d/1.png".into(),
                png: vec![1, 2, 3],
            },
            Upload {
                path: "p/EJ-1/d/2.png".into(),
                png: vec![4, 5],
            },
        ]
    }

    async fn mock_repo(s: &mut ServerGuard) -> Mock {
        s.mock("GET", "/repos/Ps3udoDev/evidencias")
            .with_body(r#"{"default_branch":"main","permissions":{"push":true}}"#)
            .create_async()
            .await
    }

    const GIT: &str = "/repos/Ps3udoDev/evidencias/git";

    async fn mock_read(s: &mut ServerGuard, times: usize) -> (Mock, Mock) {
        let r = s
            .mock("GET", format!("{GIT}/ref/heads/main").as_str())
            .with_body(r#"{"object":{"sha":"head1"}}"#)
            .expect(times)
            .create_async()
            .await;
        let c = s
            .mock("GET", format!("{GIT}/commits/head1").as_str())
            .with_body(r#"{"sha":"head1","tree":{"sha":"tree0"}}"#)
            .expect(times)
            .create_async()
            .await;
        (r, c)
    }

    #[test]
    fn slug_follows_the_table() {
        assert_eq!(slug("Ana Pérez"), "ana-perez");
        assert_eq!(slug("GMX — Portal Clientes"), "gmx-portal-clientes");
        assert_eq!(slug("Ñandú_Ç"), "nandu-c");
        assert_eq!(slug("***"), "sin-nombre");
        let long = format!("{}-{}", "a".repeat(59), "b".repeat(20));
        assert_eq!(slug(&long), "a".repeat(59));
        assert_eq!(slug(&"x".repeat(80)).len(), 60);
    }

    #[test]
    fn evidence_dir_has_the_agreed_shape() {
        let at = chrono::NaiveDate::from_ymd_opt(2026, 11, 2)
            .unwrap()
            .and_hms_opt(15, 30, 12)
            .unwrap();
        assert_eq!(
            evidence_dir("GMX — Portal Clientes", "EJ-014", at, "Ana Pérez"),
            "gmx-portal-clientes/EJ-014/2026-11-02_153012_ana-perez"
        );
    }

    #[test]
    fn image_url_uses_blob_and_raw() {
        let url = image_url("https://github.com", &repo(), "abc", "p/EJ 1#/d/1.png");
        assert_eq!(
            url,
            "https://github.com/Ps3udoDev/evidencias/blob/abc/p/EJ%201%23/d/1.png?raw=true"
        );
        assert!(url.starts_with("https://github.com/") && url.ends_with("?raw=true"));
        assert!(!url.contains("raw.githubusercontent.com"));
    }

    #[tokio::test]
    async fn commit_files_creates_one_commit_for_all_images() {
        let mut s = mockito::Server::new_async().await;
        let _r = mock_repo(&mut s).await;
        let (r, c) = mock_read(&mut s, 1).await;
        let b1 = s
            .mock("POST", format!("{GIT}/blobs").as_str())
            .match_body(Matcher::Json(
                json!({"content": STANDARD.encode([1u8, 2, 3]), "encoding": "base64"}),
            ))
            .with_status(201)
            .with_body(r#"{"sha":"blob1"}"#)
            .expect(1)
            .create_async()
            .await;
        let b2 = s
            .mock("POST", format!("{GIT}/blobs").as_str())
            .match_body(Matcher::Json(
                json!({"content": STANDARD.encode([4u8, 5]), "encoding": "base64"}),
            ))
            .with_status(201)
            .with_body(r#"{"sha":"blob2"}"#)
            .expect(1)
            .create_async()
            .await;
        let t = s
            .mock("POST", format!("{GIT}/trees").as_str())
            .match_body(Matcher::Json(json!({
                "base_tree": "tree0",
                "tree": [
                    {"path":"p/EJ-1/d/1.png","mode":"100644","type":"blob","sha":"blob1"},
                    {"path":"p/EJ-1/d/2.png","mode":"100644","type":"blob","sha":"blob2"}
                ]
            })))
            .with_status(201)
            .with_body(r#"{"sha":"tree1"}"#)
            .expect(1)
            .create_async()
            .await;
        let cm = s
            .mock("POST", format!("{GIT}/commits").as_str())
            .match_body(Matcher::Json(
                json!({"message":"qapture: EJ-1 t (ana)","tree":"tree1","parents":["head1"]}),
            ))
            .with_status(201)
            .with_body(r#"{"sha":"newc"}"#)
            .expect(1)
            .create_async()
            .await;
        let p = s
            .mock("PATCH", format!("{GIT}/refs/heads/main").as_str())
            .match_body(Matcher::Json(json!({"sha":"newc","force":false})))
            .with_body("{}")
            .expect(1)
            .create_async()
            .await;
        let client = GhClient::new(&s.url(), &s.url());
        let sha = commit_files(&client, "t0k", &repo(), &files(), "qapture: EJ-1 t (ana)")
            .await
            .unwrap();
        assert_eq!(sha, "newc");
        for m in [r, c, b1, b2, t, cm, p] {
            m.assert_async().await;
        }
    }

    async fn pipeline_after_read(s: &mut ServerGuard) -> Vec<Mock> {
        let mut mocks = Vec::new();
        for (path, body) in [
            ("blobs", r#"{"sha":"blob"}"#),
            ("trees", r#"{"sha":"tree1"}"#),
            ("commits", r#"{"sha":"newc"}"#),
        ] {
            mocks.push(
                s.mock("POST", format!("{GIT}/{path}").as_str())
                    .with_status(201)
                    .with_body(body)
                    .create_async()
                    .await,
            );
        }
        mocks
    }

    #[tokio::test]
    async fn a_non_fast_forward_ref_update_is_retried() {
        let mut s = mockito::Server::new_async().await;
        let _r = mock_repo(&mut s).await;
        let (r, _c) = mock_read(&mut s, 2).await;
        let _pipe = pipeline_after_read(&mut s).await;
        let _p1 = s
            .mock("PATCH", format!("{GIT}/refs/heads/main").as_str())
            .with_status(422)
            .with_body(r#"{"message":"Update is not a fast forward"}"#)
            .expect(1)
            .create_async()
            .await;
        let _p2 = s
            .mock("PATCH", format!("{GIT}/refs/heads/main").as_str())
            .with_body("{}")
            .expect(1)
            .create_async()
            .await;
        let client = GhClient::new(&s.url(), &s.url());
        let sha = commit_files(&client, "t0k", &repo(), &files(), "m")
            .await
            .unwrap();
        assert_eq!(sha, "newc");
        r.assert_async().await;
    }

    #[tokio::test]
    async fn gives_up_after_three_rejections() {
        let mut s = mockito::Server::new_async().await;
        let _r = mock_repo(&mut s).await;
        let (r, _c) = mock_read(&mut s, 3).await;
        let _pipe = pipeline_after_read(&mut s).await;
        let p = s
            .mock("PATCH", format!("{GIT}/refs/heads/main").as_str())
            .with_status(422)
            .with_body(r#"{"message":"Update is not a fast forward"}"#)
            .expect(3)
            .create_async()
            .await;
        let client = GhClient::new(&s.url(), &s.url());
        let err = commit_files(&client, "t0k", &repo(), &files(), "m")
            .await
            .unwrap_err();
        assert_eq!(err.code(), "gh_validation");
        r.assert_async().await;
        p.assert_async().await;
    }

    #[tokio::test]
    async fn an_empty_evidence_repo_reports_a_clear_error() {
        for status in [409usize, 404] {
            let mut s = mockito::Server::new_async().await;
            let _r = mock_repo(&mut s).await;
            let _g = s
                .mock("GET", format!("{GIT}/ref/heads/main").as_str())
                .with_status(status)
                .with_body(r#"{"message":"Git Repository is empty."}"#)
                .create_async()
                .await;
            let client = GhClient::new(&s.url(), &s.url());
            let err = commit_files(&client, "t0k", &repo(), &files(), "m")
                .await
                .unwrap_err();
            assert_eq!(err.code(), "gh_validation");
            assert_eq!(
                err.user_message(),
                "El repo de evidencias Ps3udoDev/evidencias está vacío: crea un primer archivo (por ejemplo, un README) y vuelve a intentar."
            );
        }
    }

    #[tokio::test]
    async fn upload_errors_keep_their_kind() {
        let mut s = mockito::Server::new_async().await;
        let _r = mock_repo(&mut s).await;
        let (_r2, _c) = mock_read(&mut s, 1).await;
        let _b = s
            .mock("POST", format!("{GIT}/blobs").as_str())
            .with_status(403)
            .with_body("{}")
            .create_async()
            .await;
        let client = GhClient::new(&s.url(), &s.url());
        let err = commit_files(&client, "t0k", &repo(), &files(), "m")
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            AppError::GitHub {
                kind: GhErrorKind::Forbidden,
                ..
            }
        ));
    }
}
