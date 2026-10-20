use std::collections::BTreeMap;

use rusqlite::{Connection, OptionalExtension, params};

use crate::error::{AppError, AppResult, GhErrorKind, gh};

/// Fila única (`id = 1`) con la cuenta de GitHub conectada. El token vive en el llavero.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GhAccountRow {
    pub login: String,
    pub name: Option<String>,
    pub avatar_data: Option<String>,
    pub connected_at: String,
}

pub fn get_account(conn: &Connection) -> AppResult<Option<GhAccountRow>> {
    Ok(conn
        .query_row(
            "SELECT login, name, avatar_data, connected_at FROM gh_account WHERE id = 1",
            [],
            |r| {
                Ok(GhAccountRow {
                    login: r.get(0)?,
                    name: r.get(1)?,
                    avatar_data: r.get(2)?,
                    connected_at: r.get(3)?,
                })
            },
        )
        .optional()?)
}

/// Reemplaza la cuenta anterior: solo hay una por instalación.
pub fn save_account(conn: &Connection, account: &GhAccountRow) -> AppResult<()> {
    conn.execute(
        "INSERT INTO gh_account (id, login, name, avatar_data, connected_at)
         VALUES (1, ?1, ?2, ?3, ?4)
         ON CONFLICT(id) DO UPDATE SET login = excluded.login, name = excluded.name,
           avatar_data = excluded.avatar_data, connected_at = excluded.connected_at",
        params![
            account.login,
            account.name,
            account.avatar_data,
            account.connected_at
        ],
    )?;
    Ok(())
}

pub fn delete_account(conn: &Connection) -> AppResult<()> {
    conn.execute("DELETE FROM gh_account", [])?;
    Ok(())
}

/// `dueño/repositorio` ya validado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoRef {
    pub owner: String,
    pub repo: String,
}

impl RepoRef {
    pub fn full_name(&self) -> String {
        format!("{}/{}", self.owner, self.repo)
    }

    /// Dueño `^[A-Za-z0-9](?:[A-Za-z0-9-]{0,38})$` y nombre `^[A-Za-z0-9._-]{1,100}$`
    /// (distinto de `.` y `..`).
    pub fn parse(text: &str) -> Option<RepoRef> {
        let (owner, repo) = text.split_once('/')?;
        let owner_ok = (1..=39).contains(&owner.len())
            && owner.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
            && !owner.starts_with('-');
        let repo_ok = (1..=100).contains(&repo.len())
            && repo != "."
            && repo != ".."
            && repo
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));
        (owner_ok && repo_ok).then(|| RepoRef {
            owner: owner.to_string(),
            repo: repo.to_string(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkedRepo {
    pub repo: RepoRef,
    pub is_default: bool,
    pub default_labels: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectGithub {
    pub repos: Vec<LinkedRepo>,
    pub routing_field_id: Option<String>,
    pub routing_map: BTreeMap<String, String>,
    pub evidence_repo: Option<RepoRef>,
}

type Settings = (Option<String>, String, Option<String>, Option<String>);

/// Vacío si el proyecto no tiene filas. Los repos salen en el orden en que se guardaron.
pub fn get_project_github(conn: &Connection, project_id: &str) -> AppResult<ProjectGithub> {
    let mut stmt = conn.prepare(
        "SELECT owner, repo, is_default, default_labels FROM project_repo_link
         WHERE project_id = ?1 ORDER BY position",
    )?;
    let repos = stmt
        .query_map([project_id], |r| {
            let labels: String = r.get(3)?;
            Ok(LinkedRepo {
                repo: RepoRef {
                    owner: r.get(0)?,
                    repo: r.get(1)?,
                },
                is_default: r.get::<_, i64>(2)? != 0,
                default_labels: labels
                    .split(',')
                    .filter(|l| !l.is_empty())
                    .map(str::to_string)
                    .collect(),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let settings: Option<Settings> = conn
        .query_row(
            "SELECT routing_field_id, routing_map, evidence_owner, evidence_repo
             FROM project_gh_settings WHERE project_id = ?1",
            [project_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()?;
    let Some((routing_field_id, routing_map, evidence_owner, evidence_repo)) = settings else {
        return Ok(ProjectGithub {
            repos,
            ..ProjectGithub::default()
        });
    };
    Ok(ProjectGithub {
        repos,
        routing_field_id,
        // A propósito: un `routing_map` corrupto se lee como vacío para que la config (y el
        // proyecto) sigan cargando; el usuario lo vuelve a armar en la pestaña GitHub.
        routing_map: serde_json::from_str(&routing_map).unwrap_or_default(),
        evidence_repo: evidence_owner
            .zip(evidence_repo)
            .map(|(owner, repo)| RepoRef { owner, repo }),
    })
}

/// Una transacción: reemplaza los repos vinculados y hace upsert de los ajustes. Los repos
/// que siguen vinculados conservan `field_mapping` e `import_filter`.
pub fn save_project_github(
    conn: &Connection,
    project_id: &str,
    config: &ProjectGithub,
) -> AppResult<()> {
    let tx = conn.unchecked_transaction()?;
    // Se aparta lo que cada repo ya tenía (sin distinguir mayúsculas) y se reinserta todo.
    let mut kept: Vec<(String, String, Option<String>, Option<String>)> = Vec::new();
    {
        let mut stmt = tx.prepare(
            "SELECT owner, repo, field_mapping, import_filter FROM project_repo_link
             WHERE project_id = ?1",
        )?;
        let rows = stmt.query_map([project_id], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
        })?;
        for row in rows {
            let (owner, repo, mapping, filter): (String, String, _, _) = row?;
            kept.push((owner.to_lowercase(), repo.to_lowercase(), mapping, filter));
        }
    }
    tx.execute(
        "DELETE FROM project_repo_link WHERE project_id = ?1",
        [project_id],
    )?;
    for (position, linked) in config.repos.iter().enumerate() {
        let (owner, repo) = (
            linked.repo.owner.to_lowercase(),
            linked.repo.repo.to_lowercase(),
        );
        let previous = kept.iter().find(|k| k.0 == owner && k.1 == repo);
        tx.execute(
            "INSERT INTO project_repo_link
               (project_id, owner, repo, is_default, position, default_labels, field_mapping, import_filter)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                project_id,
                linked.repo.owner,
                linked.repo.repo,
                linked.is_default,
                position as i64,
                linked.default_labels.join(","),
                previous.and_then(|k| k.2.as_ref()),
                previous.and_then(|k| k.3.as_ref()),
            ],
        )?;
    }
    tx.execute(
        "INSERT INTO project_gh_settings
           (project_id, routing_field_id, routing_map, evidence_owner, evidence_repo)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(project_id) DO UPDATE SET routing_field_id = excluded.routing_field_id,
           routing_map = excluded.routing_map, evidence_owner = excluded.evidence_owner,
           evidence_repo = excluded.evidence_repo",
        params![
            project_id,
            config.routing_field_id,
            serde_json::to_string(&config.routing_map)?,
            config.evidence_repo.as_ref().map(|r| &r.owner),
            config.evidence_repo.as_ref().map(|r| &r.repo),
        ],
    )?;
    tx.commit()?;
    Ok(())
}

/// Vínculo caso ↔ issue (como máximo uno por caso y repo).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseIssueLink {
    pub case_id: String,
    pub owner: String,
    pub repo: String,
    pub issue_number: u64,
    pub node_id: String,
    pub html_url: String,
    /// `exported` o `imported`.
    pub direction: String,
    pub issue_state: String,
}

impl CaseIssueLink {
    pub fn repo_ref(&self) -> RepoRef {
        RepoRef {
            owner: self.owner.clone(),
            repo: self.repo.clone(),
        }
    }
}

/// `gh_conflict`: el caso ya tiene un issue en ese repo.
pub fn conflict(existing: &CaseIssueLink) -> AppError {
    gh(
        GhErrorKind::Conflict,
        format!(
            "El caso ya tiene un issue en {} (#{}).",
            existing.repo_ref().full_name(),
            existing.issue_number
        ),
    )
}

const LINK_COLUMNS: &str =
    "l.case_id, l.owner, l.repo, l.issue_number, l.node_id, l.html_url, l.direction, l.issue_state";

fn link_from_row(r: &rusqlite::Row) -> rusqlite::Result<CaseIssueLink> {
    Ok(CaseIssueLink {
        case_id: r.get(0)?,
        owner: r.get(1)?,
        repo: r.get(2)?,
        issue_number: r.get::<_, i64>(3)? as u64,
        node_id: r.get::<_, Option<String>>(4)?.unwrap_or_default(),
        html_url: r.get(5)?,
        direction: r.get(6)?,
        issue_state: r.get(7)?,
    })
}

/// Inserta el vínculo. Si el caso ya tiene uno en ese repo (sin distinguir mayúsculas, o por
/// la UNIQUE de 0005 en una carrera), devuelve `gh_conflict` con el número existente.
pub fn insert_link(conn: &Connection, link: &CaseIssueLink) -> AppResult<()> {
    let tx = conn.unchecked_transaction()?;
    if let Some(existing) = find_link(&tx, &link.case_id, &link.repo_ref())? {
        return Err(conflict(&existing));
    }
    let inserted = tx.execute(
        "INSERT INTO case_issue_link
           (case_id, owner, repo, issue_number, node_id, html_url, direction, issue_state)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            link.case_id,
            link.owner,
            link.repo,
            link.issue_number as i64,
            link.node_id,
            link.html_url,
            link.direction,
            link.issue_state,
        ],
    );
    match inserted {
        Ok(_) => {
            tx.commit()?;
            Ok(())
        }
        Err(rusqlite::Error::SqliteFailure(e, _))
            if e.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE =>
        {
            match find_link(&tx, &link.case_id, &link.repo_ref())? {
                Some(existing) => Err(conflict(&existing)),
                None => Err(conflict(link)),
            }
        }
        Err(e) => Err(e.into()),
    }
}

/// Dueño y repo se comparan sin distinguir mayúsculas, como en GitHub.
pub fn find_link(
    conn: &Connection,
    case_id: &str,
    repo: &RepoRef,
) -> AppResult<Option<CaseIssueLink>> {
    Ok(conn
        .query_row(
            &format!(
                "SELECT {LINK_COLUMNS} FROM case_issue_link l
                 WHERE l.case_id = ?1 AND lower(l.owner) = lower(?2) AND lower(l.repo) = lower(?3)"
            ),
            params![case_id, repo.owner, repo.repo],
            link_from_row,
        )
        .optional()?)
}

pub fn links_for_project(conn: &Connection, project_id: &str) -> AppResult<Vec<CaseIssueLink>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {LINK_COLUMNS} FROM case_issue_link l JOIN test_case c ON c.id = l.case_id
         WHERE c.project_id = ?1 ORDER BY c.rowid, l.rowid"
    ))?;
    let links = stmt
        .query_map([project_id], link_from_row)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(links)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        db,
        repo::test_support::{case, conn, other_project, project},
    };

    fn repo(s: &str) -> RepoRef {
        RepoRef::parse(s).unwrap()
    }

    fn link(case_id: &str, number: u64) -> CaseIssueLink {
        CaseIssueLink {
            case_id: case_id.into(),
            owner: "Ps3udoDev".into(),
            repo: "testing".into(),
            issue_number: number,
            node_id: format!("I_{number}"),
            html_url: format!("https://github.com/Ps3udoDev/testing/issues/{number}"),
            direction: "exported".into(),
            issue_state: "open".into(),
        }
    }

    #[test]
    fn links_round_trip_and_are_listed_by_project() {
        let conn = conn();
        let p = project(&conn);
        let c = case(&conn, &p);
        let other = other_project(&conn);
        let oc = case(&conn, &other);
        insert_link(&conn, &link(&c.id, 45)).unwrap();
        insert_link(&conn, &link(&oc.id, 46)).unwrap();
        assert_eq!(
            find_link(&conn, &c.id, &repo("Ps3udoDev/testing")).unwrap(),
            Some(link(&c.id, 45))
        );
        assert_eq!(
            find_link(&conn, &c.id, &repo("ps3udodev/TESTING")).unwrap(),
            Some(link(&c.id, 45)),
            "dueño y repo no distinguen mayúsculas"
        );
        assert_eq!(
            find_link(&conn, &c.id, &repo("Ps3udoDev/otro")).unwrap(),
            None
        );
        assert_eq!(
            links_for_project(&conn, &p.id).unwrap(),
            vec![link(&c.id, 45)]
        );
    }

    #[test]
    fn links_conflict_on_the_same_case_and_repo() {
        let conn = conn();
        let p = project(&conn);
        let c = case(&conn, &p);
        insert_link(&conn, &link(&c.id, 45)).unwrap();
        let err = insert_link(&conn, &link(&c.id, 99)).unwrap_err();
        assert_eq!(err.code(), "gh_conflict");
        assert_eq!(
            err.user_message(),
            "El caso ya tiene un issue en Ps3udoDev/testing (#45)."
        );
        let mut upper = link(&c.id, 99);
        upper.owner = "PS3UDODEV".into();
        assert_eq!(
            insert_link(&conn, &upper).unwrap_err().code(),
            "gh_conflict",
            "otra capitalización del mismo repo también choca"
        );
    }

    fn sample() -> ProjectGithub {
        ProjectGithub {
            repos: vec![
                LinkedRepo {
                    repo: repo("TEAMS4SOFT/GMX-frontend"),
                    is_default: true,
                    default_labels: vec!["bug".into(), "qapture".into()],
                },
                LinkedRepo {
                    repo: repo("Ps3udoDev/booknow-mcp-service"),
                    is_default: false,
                    default_labels: vec![],
                },
            ],
            routing_field_id: Some("campo-1".into()),
            routing_map: BTreeMap::from([(
                "Front".to_string(),
                "TEAMS4SOFT/GMX-frontend".to_string(),
            )]),
            evidence_repo: Some(repo("TEAMS4SOFT/qa-evidencias")),
        }
    }

    #[test]
    fn migration_0005_creates_the_github_tables() {
        let conn = db::open_in_memory().unwrap();
        for table in [
            "gh_account",
            "project_repo_link",
            "project_gh_settings",
            "case_issue_link",
        ] {
            let n: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                    [table],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(n, 1, "falta {table}");
        }
        assert_eq!(db::schema_version(&conn).unwrap(), 5);
    }

    #[test]
    fn project_github_round_trips() {
        let conn = conn();
        let p = project(&conn);
        assert_eq!(
            get_project_github(&conn, &p.id).unwrap(),
            ProjectGithub::default()
        );
        save_project_github(&conn, &p.id, &sample()).unwrap();
        assert_eq!(get_project_github(&conn, &p.id).unwrap(), sample());
    }

    #[test]
    fn saving_keeps_import_settings_of_repos_that_stay_linked() {
        let conn = conn();
        let p = project(&conn);
        let mut cfg = sample();
        save_project_github(&conn, &p.id, &cfg).unwrap();
        conn.execute(
            "UPDATE project_repo_link SET field_mapping = '{\"a\":1}' WHERE project_id = ?1 AND repo = 'GMX-frontend'",
            [&p.id],
        )
        .unwrap();
        cfg.repos.push(LinkedRepo {
            repo: repo("TEAMS4SOFT/otro"),
            is_default: false,
            default_labels: vec![],
        });
        save_project_github(&conn, &p.id, &cfg).unwrap();
        let mapping: Option<String> = conn
            .query_row(
                "SELECT field_mapping FROM project_repo_link WHERE project_id = ?1 AND repo = 'GMX-frontend'",
                [&p.id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(mapping.as_deref(), Some("{\"a\":1}"));
        assert_eq!(get_project_github(&conn, &p.id).unwrap().repos.len(), 3);
    }

    #[test]
    fn deleting_a_project_removes_its_github_config() {
        let conn = conn();
        let p = project(&conn);
        save_project_github(&conn, &p.id, &sample()).unwrap();
        conn.execute("DELETE FROM project WHERE id = ?1", [&p.id])
            .unwrap();
        for table in ["project_repo_link", "project_gh_settings"] {
            let n: i64 = conn
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
                .unwrap();
            assert_eq!(n, 0, "{table}");
        }
    }

    #[test]
    fn account_is_a_single_row() {
        let conn = conn();
        assert_eq!(get_account(&conn).unwrap(), None);
        let first = GhAccountRow {
            login: "uno".into(),
            name: Some("Uno".into()),
            avatar_data: Some("data:image/png;base64,AA".into()),
            connected_at: "2026-10-19T00:00:00.000Z".into(),
        };
        let second = GhAccountRow {
            login: "dos".into(),
            name: None,
            avatar_data: None,
            connected_at: "2026-10-19T01:00:00.000Z".into(),
        };
        save_account(&conn, &first).unwrap();
        save_account(&conn, &second).unwrap();
        let n: i64 = conn
            .query_row("SELECT COUNT(*) FROM gh_account", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1);
        assert_eq!(get_account(&conn).unwrap(), Some(second));
        delete_account(&conn).unwrap();
        assert_eq!(get_account(&conn).unwrap(), None);
    }

    #[test]
    fn repo_ref_parse() {
        assert_eq!(
            repo("TEAMS4SOFT/GMX-frontend").full_name(),
            "TEAMS4SOFT/GMX-frontend"
        );
        assert!(RepoRef::parse("Ps3udoDev/booknow-mcp-service").is_some());
        let long = format!("{}/x", "a".repeat(40));
        for bad in [
            "a/b/c",
            "/x",
            "x/",
            "-a/b",
            "a/..",
            "a/.",
            "",
            "abc",
            long.as_str(),
        ] {
            assert!(RepoRef::parse(bad).is_none(), "{bad}");
        }
    }
}
