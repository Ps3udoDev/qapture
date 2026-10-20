//! Operaciones de issues sobre la API de GitHub: info del repo, crear, comentar y metadatos.

// Se usa desde la Task 3/4 (comandos de reporte).
#![allow(dead_code)]

use serde::{Deserialize, Serialize};

use super::client::GhClient;
use crate::error::AppResult;
use crate::repo::integrations::RepoRef;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoInfo {
    pub default_branch: String,
    pub can_push: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct NewIssue {
    pub title: String,
    pub body: String,
    pub labels: Vec<String>,
    pub assignees: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub milestone: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct IssueRef {
    pub number: u64,
    pub node_id: String,
    pub html_url: String,
    pub state: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RepoMeta {
    pub labels: Vec<String>,
    pub milestones: Vec<(u64, String)>,
    pub assignees: Vec<String>,
}

fn base(repo: &RepoRef) -> String {
    format!("/repos/{}/{}", repo.owner, repo.repo)
}

#[derive(Deserialize, Default)]
struct Permissions {
    #[serde(default)]
    push: bool,
}

#[derive(Deserialize)]
struct RepoPayload {
    default_branch: String,
    #[serde(default)]
    permissions: Permissions,
}

#[derive(Deserialize)]
struct CommentPayload {
    html_url: String,
}

#[derive(Deserialize)]
struct Label {
    name: String,
}

#[derive(Deserialize)]
struct Milestone {
    number: u64,
    title: String,
}

#[derive(Deserialize)]
struct User {
    login: String,
}

/// `GET /repos/{o}/{r}`: rama predeterminada y permiso de escritura (`permissions.push`).
pub async fn repo_info(client: &GhClient, token: &str, repo: &RepoRef) -> AppResult<RepoInfo> {
    let payload: RepoPayload = client.get_json(token, &base(repo), Some(repo)).await?;
    Ok(RepoInfo {
        default_branch: payload.default_branch,
        can_push: payload.permissions.push,
    })
}

pub async fn create(
    client: &GhClient,
    token: &str,
    repo: &RepoRef,
    issue: &NewIssue,
) -> AppResult<IssueRef> {
    client
        .post_json(token, &format!("{}/issues", base(repo)), issue, Some(repo))
        .await
}

/// Devuelve la `html_url` del comentario creado.
pub async fn comment(
    client: &GhClient,
    token: &str,
    repo: &RepoRef,
    number: u64,
    body: &str,
) -> AppResult<String> {
    let payload: CommentPayload = client
        .post_json(
            token,
            &format!("{}/issues/{number}/comments", base(repo)),
            &serde_json::json!({ "body": body }),
            Some(repo),
        )
        .await?;
    Ok(payload.html_url)
}

pub async fn repo_meta(client: &GhClient, token: &str, repo: &RepoRef) -> AppResult<RepoMeta> {
    let root = base(repo);
    let labels: Vec<Label> = client
        .get_paginated(token, &format!("{root}/labels"), None)
        .await?;
    let milestones: Vec<Milestone> = client
        .get_paginated(token, &format!("{root}/milestones?state=open"), None)
        .await?;
    let assignees: Vec<User> = client
        .get_paginated(token, &format!("{root}/assignees"), None)
        .await?;
    Ok(RepoMeta {
        labels: labels.into_iter().map(|l| l.name).collect(),
        milestones: milestones
            .into_iter()
            .map(|m| (m.number, m.title))
            .collect(),
        assignees: assignees.into_iter().map(|u| u.login).collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Matcher;
    use serde_json::json;

    fn repo() -> RepoRef {
        RepoRef::parse("Ps3udoDev/testing").unwrap()
    }

    fn issue_json() -> String {
        json!({"number": 45, "node_id": "I_kw", "html_url": "https://github.com/Ps3udoDev/testing/issues/45", "state": "open", "extra": 1}).to_string()
    }

    #[tokio::test]
    async fn repo_info_reads_push_permission_and_default_branch() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("GET", "/repos/Ps3udoDev/testing")
            .with_body(r#"{"default_branch":"develop","permissions":{"push":true,"pull":true}}"#)
            .create_async()
            .await;
        let client = GhClient::new(&server.url(), &server.url());
        let info = repo_info(&client, "t0k", &repo()).await.unwrap();
        assert_eq!(
            info,
            RepoInfo {
                default_branch: "develop".into(),
                can_push: true
            }
        );
    }

    #[tokio::test]
    async fn repo_info_defaults_to_no_push_without_permissions() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("GET", "/repos/Ps3udoDev/testing")
            .with_body(r#"{"default_branch":"main"}"#)
            .create_async()
            .await;
        let client = GhClient::new(&server.url(), &server.url());
        assert!(!repo_info(&client, "t0k", &repo()).await.unwrap().can_push);
    }

    #[tokio::test]
    async fn create_issue_returns_number_and_url() {
        let mut server = mockito::Server::new_async().await;
        let m = server
            .mock("POST", "/repos/Ps3udoDev/testing/issues")
            .match_body(Matcher::Json(json!({
                "title": "[EJ-1] Falla",
                "body": "cuerpo",
                "labels": ["bug"],
                "assignees": ["ana"],
                "milestone": 3
            })))
            .with_status(201)
            .with_body(issue_json())
            .create_async()
            .await;
        let client = GhClient::new(&server.url(), &server.url());
        let issue = NewIssue {
            title: "[EJ-1] Falla".into(),
            body: "cuerpo".into(),
            labels: vec!["bug".into()],
            assignees: vec!["ana".into()],
            milestone: Some(3),
        };
        let created = create(&client, "t0k", &repo(), &issue).await.unwrap();
        assert_eq!(created.number, 45);
        assert_eq!(created.node_id, "I_kw");
        assert_eq!(created.state, "open");
        assert_eq!(
            created.html_url,
            "https://github.com/Ps3udoDev/testing/issues/45"
        );
        m.assert_async().await;
    }

    #[tokio::test]
    async fn create_issue_omits_empty_milestone() {
        let mut server = mockito::Server::new_async().await;
        let m = server
            .mock("POST", "/repos/Ps3udoDev/testing/issues")
            .match_body(Matcher::Json(json!({
                "title": "t", "body": "b", "labels": [], "assignees": []
            })))
            .with_status(201)
            .with_body(issue_json())
            .create_async()
            .await;
        let client = GhClient::new(&server.url(), &server.url());
        let issue = NewIssue {
            title: "t".into(),
            body: "b".into(),
            labels: vec![],
            assignees: vec![],
            milestone: None,
        };
        create(&client, "t0k", &repo(), &issue).await.unwrap();
        m.assert_async().await;
    }

    #[tokio::test]
    async fn comment_returns_html_url() {
        let mut server = mockito::Server::new_async().await;
        let m = server
            .mock("POST", "/repos/Ps3udoDev/testing/issues/45/comments")
            .match_body(Matcher::Json(json!({ "body": "re-test" })))
            .with_status(201)
            .with_body(
                r#"{"html_url":"https://github.com/Ps3udoDev/testing/issues/45#issuecomment-9"}"#,
            )
            .create_async()
            .await;
        let client = GhClient::new(&server.url(), &server.url());
        let url = comment(&client, "t0k", &repo(), 45, "re-test")
            .await
            .unwrap();
        assert_eq!(
            url,
            "https://github.com/Ps3udoDev/testing/issues/45#issuecomment-9"
        );
        m.assert_async().await;
    }

    #[tokio::test]
    async fn repo_meta_collects_labels_milestones_and_assignees() {
        let mut server = mockito::Server::new_async().await;
        let next = format!(
            "<{}/repos/Ps3udoDev/testing/labels?per_page=100&page=2>; rel=\"next\"",
            server.url()
        );
        let _l1 = server
            .mock("GET", "/repos/Ps3udoDev/testing/labels?per_page=100")
            .with_header("link", &next)
            .with_body(r#"[{"name":"bug"},{"name":"ui"}]"#)
            .create_async()
            .await;
        let _l2 = server
            .mock("GET", "/repos/Ps3udoDev/testing/labels?per_page=100&page=2")
            .with_body(r#"[{"name":"qa"}]"#)
            .create_async()
            .await;
        let _m = server
            .mock(
                "GET",
                "/repos/Ps3udoDev/testing/milestones?state=open&per_page=100",
            )
            .with_body(r#"[{"number":3,"title":"v1.1"},{"number":4,"title":"v1.2"}]"#)
            .create_async()
            .await;
        let _a = server
            .mock("GET", "/repos/Ps3udoDev/testing/assignees?per_page=100")
            .with_body(r#"[{"login":"ana"},{"login":"luis"}]"#)
            .create_async()
            .await;
        let client = GhClient::new(&server.url(), &server.url());
        let meta = repo_meta(&client, "t0k", &repo()).await.unwrap();
        assert_eq!(meta.labels, vec!["bug", "ui", "qa"]);
        assert_eq!(
            meta.milestones,
            vec![(3, "v1.1".to_string()), (4, "v1.2".to_string())]
        );
        assert_eq!(meta.assignees, vec!["ana", "luis"]);
    }
}
