use rusqlite::{Connection, params};

use crate::{
    db::{new_id, now},
    dto::ActivityDto,
    error::AppResult,
};

pub struct NewActivity {
    pub client: String,
    pub tool: String,
    pub project_id: Option<String>,
    pub case_id: Option<String>,
    pub ok: bool,
    /// Solo códigos de caso, nombres de proyecto, cantidades y nombres de herramienta.
    pub detail: String,
}

/// Inserción y poda atómicas: ningún lector observa más de 2000 registros.
pub fn insert(conn: &Connection, row: &NewActivity) -> AppResult<()> {
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "INSERT INTO agent_activity (id, at, client, tool, project_id, case_id, ok, detail)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            new_id(),
            now(),
            row.client,
            row.tool,
            row.project_id,
            row.case_id,
            row.ok,
            row.detail
        ],
    )?;
    tx.execute(
        "DELETE FROM agent_activity WHERE id NOT IN
         (SELECT id FROM agent_activity ORDER BY at DESC, rowid DESC LIMIT 2000)",
        [],
    )?;
    tx.commit()?;
    Ok(())
}

pub fn list(conn: &Connection, limit: u32, client: Option<&str>) -> AppResult<Vec<ActivityDto>> {
    let mut stmt = conn.prepare(
        "SELECT id, at, client, tool, project_id, case_id, ok, detail FROM agent_activity
         WHERE (?1 IS NULL OR client = ?1) ORDER BY at DESC, rowid DESC LIMIT ?2",
    )?;
    let rows = stmt
        .query_map(params![client, limit], |row| {
            Ok(ActivityDto {
                id: row.get(0)?,
                at: row.get(1)?,
                client: row.get(2)?,
                tool: row.get(3)?,
                project_id: row.get(4)?,
                case_id: row.get(5)?,
                ok: row.get(6)?,
                detail: row.get(7)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repo::test_support::conn;

    fn activity(client: &str, tool: &str) -> NewActivity {
        NewActivity {
            client: client.into(),
            tool: tool.into(),
            project_id: Some("p-1".into()),
            case_id: Some("c-1".into()),
            ok: true,
            detail: "CP-001".into(),
        }
    }

    #[test]
    fn insert_then_list_returns_newest_first() {
        let conn = conn();
        let first = activity("codex", "create_case");
        let mut second = activity("claude-code", "update_case");
        second.ok = false;
        second.case_id = None;
        insert(&conn, &first).unwrap();
        insert(&conn, &second).unwrap();
        // Empates de timestamp se resuelven por el orden de inserción.
        conn.execute("UPDATE agent_activity SET at = '2026-10-29T10:00:00Z'", [])
            .unwrap();
        let rows = list(&conn, 2, None).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].tool, "update_case");
        assert_eq!(rows[1].tool, "create_case");
        assert!(!rows[0].ok);
        assert_eq!(rows[0].case_id, None);
        assert_eq!(rows[1].project_id.as_deref(), Some("p-1"));
        assert_eq!(rows[1].detail, "CP-001");
        assert_ne!(rows[0].id, rows[1].id);
        assert_eq!(list(&conn, 1, None).unwrap().len(), 1);
        assert!(list(&conn, 0, None).unwrap().is_empty());
    }

    #[test]
    fn list_filters_by_client() {
        let conn = conn();
        insert(&conn, &activity("codex", "create_case")).unwrap();
        insert(&conn, &activity("claude-code", "update_case")).unwrap();
        let rows = list(&conn, 100, Some("codex")).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].client, "codex");
        assert!(list(&conn, 100, Some("absent")).unwrap().is_empty());
    }

    #[test]
    fn insert_prunes_to_2000_rows() {
        let conn = conn();
        for index in 0..2005 {
            insert(&conn, &activity("codex", &format!("tool-{index}"))).unwrap();
        }
        let rows = list(&conn, u32::MAX, None).unwrap();
        assert_eq!(rows.len(), 2000);
        assert_eq!(rows.first().unwrap().tool, "tool-2004");
        assert_eq!(rows.last().unwrap().tool, "tool-5");
    }
}
