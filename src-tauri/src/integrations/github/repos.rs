//! Repos a los que la cuenta conectada llega a través de las instalaciones de la App.

use serde::Deserialize;

use super::client::GhClient;
use crate::dto::GhRepoDto;
use crate::error::AppResult;

#[derive(Deserialize)]
struct Installation {
    id: u64,
}

#[derive(Deserialize)]
struct Owner {
    login: String,
}

#[derive(Deserialize, Default)]
struct Permissions {
    #[serde(default)]
    pull: bool,
    #[serde(default)]
    push: bool,
}

#[derive(Deserialize)]
struct Repo {
    name: String,
    owner: Owner,
    #[serde(default)]
    private: bool,
    #[serde(default)]
    has_issues: bool,
    #[serde(default)]
    permissions: Permissions,
}

impl From<Repo> for GhRepoDto {
    fn from(r: Repo) -> Self {
        GhRepoDto {
            full_name: format!("{}/{}", r.owner.login, r.name),
            owner: r.owner.login,
            name: r.name,
            private: r.private,
            // Quien puede leer un repo puede abrir issues si están habilitados.
            can_write_issues: r.has_issues && r.permissions.pull,
            can_write_contents: r.permissions.push,
        }
    }
}

/// `GET /user/installations` y, por cada una, sus repos (paginados). Ordenados por `fullName`
/// sin distinguir mayúsculas y sin duplicados (un repo puede llegar por dos instalaciones).
pub async fn list_accessible(client: &GhClient, token: &str) -> AppResult<Vec<GhRepoDto>> {
    let installations: Vec<Installation> = client
        .get_paginated(token, "/user/installations", Some("installations"))
        .await?;
    let mut repos: Vec<GhRepoDto> = Vec::new();
    for installation in installations {
        let page: Vec<Repo> = client
            .get_paginated(
                token,
                &format!("/user/installations/{}/repositories", installation.id),
                Some("repositories"),
            )
            .await?;
        repos.extend(page.into_iter().map(GhRepoDto::from));
    }
    repos.sort_by_key(|r| r.full_name.to_lowercase());
    repos.dedup_by(|a, b| a.full_name.eq_ignore_ascii_case(&b.full_name));
    Ok(repos)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn repo(
        owner: &str,
        name: &str,
        has_issues: bool,
        pull: bool,
        push: bool,
    ) -> serde_json::Value {
        json!({
            "id": 1, "name": name, "full_name": format!("{owner}/{name}"),
            "owner": { "login": owner }, "private": true, "has_issues": has_issues,
            "permissions": { "admin": false, "pull": pull, "push": push }
        })
    }

    #[tokio::test]
    async fn lists_repos_of_every_installation() {
        let mut server = mockito::Server::new_async().await;
        let _inst = server
            .mock("GET", "/user/installations?per_page=100")
            .with_body(
                json!({ "total_count": 2, "installations": [{ "id": 11 }, { "id": 22 }] })
                    .to_string(),
            )
            .create_async()
            .await;
        let _a = server
            .mock("GET", "/user/installations/11/repositories?per_page=100")
            .with_body(
                json!({ "total_count": 1, "repositories": [repo("TEAMS4SOFT", "zeta", true, true, true)] })
                    .to_string(),
            )
            .create_async()
            .await;
        let next = format!(
            "<{}/user/installations/22/repositories?per_page=100&page=2>; rel=\"next\"",
            server.url()
        );
        let _b1 = server
            .mock("GET", "/user/installations/22/repositories?per_page=100")
            .with_header("link", &next)
            .with_body(
                json!({ "total_count": 3, "repositories": [
                    repo("Ps3udoDev", "booknow", false, true, false),
                    repo("teams4soft", "Zeta", true, true, true)
                ] })
                .to_string(),
            )
            .create_async()
            .await;
        let _b2 = server
            .mock("GET", "/user/installations/22/repositories?per_page=100&page=2")
            .with_body(
                json!({ "total_count": 3, "repositories": [repo("TEAMS4SOFT", "Alpha", true, true, false)] })
                    .to_string(),
            )
            .create_async()
            .await;
        let client = GhClient::new(&server.url(), &server.url());
        let repos = list_accessible(&client, "t0k").await.unwrap();
        let names: Vec<&str> = repos.iter().map(|r| r.full_name.as_str()).collect();
        assert_eq!(
            names,
            ["Ps3udoDev/booknow", "TEAMS4SOFT/Alpha", "TEAMS4SOFT/zeta"]
        );
        let booknow = &repos[0];
        assert!(!booknow.can_write_issues, "issues deshabilitados");
        assert!(!booknow.can_write_contents);
        let alpha = &repos[1];
        assert!(alpha.can_write_issues, "pull + has_issues");
        assert!(!alpha.can_write_contents, "sin push");
        assert_eq!(
            (alpha.owner.as_str(), alpha.name.as_str()),
            ("TEAMS4SOFT", "Alpha")
        );
        assert!(alpha.private);
        assert!(repos[2].can_write_contents, "push");
    }

    #[tokio::test]
    async fn a_401_is_reported_as_gh_auth() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("GET", "/user/installations?per_page=100")
            .with_status(401)
            .create_async()
            .await;
        let client = GhClient::new(&server.url(), &server.url());
        let err = list_accessible(&client, "t0k").await.unwrap_err();
        assert_eq!(err.code(), "gh_auth");
    }
}
