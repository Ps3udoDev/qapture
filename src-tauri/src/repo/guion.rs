//! Perfiles reutilizables y trazabilidad del libro de origen.
use rusqlite::{Connection, OptionalExtension, params};

use crate::{
    db::{new_id, now},
    domain::text::required_text,
    dto::ImportProfileDto,
    error::{AppError, AppResult, validation},
    transfer::guion::profile::ProfileDefinition,
};

pub fn create_profile(
    conn: &Connection,
    name: &str,
    def: &ProfileDefinition,
) -> AppResult<ImportProfileDto> {
    let name = required_text("Nombre del perfil", name, 100)?;
    // SQLite NOCASE solo cubre ASCII; los nombres en español también deben
    // ignorar mayúsculas sin perder la distinción entre letras acentuadas.
    let wanted = name.to_lowercase();
    let mut stmt = conn.prepare("SELECT name FROM import_profile")?;
    let mut taken = false;
    for row in stmt.query_map([], |r| r.get::<_, String>(0))? {
        if row?.to_lowercase() == wanted {
            taken = true;
            break;
        }
    }
    if taken {
        return Err(validation(format!("Ya existe un perfil llamado «{name}».")));
    }
    let id = new_id();
    let stamp = now();
    conn.execute(
        "INSERT INTO import_profile (id, name, definition, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?4)",
        params![id, name, serde_json::to_string(def)?, stamp],
    )?;
    Ok(ImportProfileDto {
        id,
        name,
        definition: def.clone(),
        created_at: stamp.clone(),
        updated_at: stamp,
    })
}

pub fn list_profiles(conn: &Connection) -> AppResult<Vec<ImportProfileDto>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, definition, created_at, updated_at FROM import_profile ORDER BY name COLLATE NOCASE",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
            r.get::<_, String>(4)?,
        ))
    })?;
    let mut profiles = Vec::new();
    for row in rows {
        let (id, name, json, created_at, updated_at) = row?;
        match serde_json::from_str(&json) {
            Ok(definition) => profiles.push(ImportProfileDto {
                id,
                name,
                definition,
                created_at,
                updated_at,
            }),
            Err(_) => eprintln!("Se omitió el perfil {id}: su definición no se puede leer."),
        }
    }
    profiles.sort_by_key(|profile| profile.name.to_lowercase());
    Ok(profiles)
}

pub fn delete_profile(conn: &Connection, id: &str) -> AppResult<()> {
    if conn.execute("DELETE FROM import_profile WHERE id = ?1", [id])? == 0 {
        return Err(AppError::NotFound("El perfil no existe.".into()));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)] // El exportador al formato de origen (G4) consume estos datos.
pub struct ProjectSource {
    pub file_name: String,
    pub stored_path: String,
    pub sha256: String,
    pub sheet: String,
    pub header_row: u32,
    pub profile_id: Option<String>,
    pub definition: ProfileDefinition,
    pub imported_at: String,
}

#[allow(dead_code)] // Consumido por G4; probado desde G1.
pub fn source(conn: &Connection, project_id: &str) -> AppResult<Option<ProjectSource>> {
    let row = conn.query_row(
        "SELECT file_name, stored_path, sha256, sheet, header_row, profile_id, definition, imported_at FROM project_source WHERE project_id = ?1",
        [project_id],
        |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, String>(3)?, r.get::<_, u32>(4)?, r.get::<_, Option<String>>(5)?, r.get::<_, String>(6)?, r.get::<_, String>(7)?)),
    ).optional()?;
    row.map(
        |(file_name, stored_path, sha256, sheet, header_row, profile_id, json, imported_at)| {
            Ok(ProjectSource {
                file_name,
                stored_path,
                sha256,
                sheet,
                header_row,
                profile_id,
                definition: serde_json::from_str(&json)?,
                imported_at,
            })
        },
    )
    .transpose()
}

#[allow(dead_code)] // Consumido por G4; probado desde G1.
pub fn case_rows(conn: &Connection, project_id: &str) -> AppResult<Vec<(String, u32)>> {
    let mut stmt = conn.prepare(
        "SELECT c.id, s.row FROM test_case c JOIN case_source s ON s.case_id = c.id WHERE c.project_id = ?1 ORDER BY c.rowid",
    )?;
    Ok(stmt
        .query_map([project_id], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<Vec<_>, _>>()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        repo::{cases, projects, test_support},
        transfer::guion::profile::tests::reference,
    };

    fn insert_source(conn: &Connection, project_id: &str, profile_id: Option<&str>) {
        conn.execute(
            "INSERT INTO project_source (project_id, file_name, stored_path, sha256, sheet, header_row, profile_id, definition, imported_at) VALUES (?1, 'guion.xlsx', ?2, 'hash', 'Guión de Pruebas', 15, ?3, ?4, '')",
            params![project_id, format!("evidence/{project_id}/guion.xlsx"), profile_id, serde_json::to_string(&reference()).unwrap()],
        ).unwrap();
    }

    #[test]
    fn profiles_are_created_listed_and_deleted() {
        let conn = test_support::conn();
        let b = create_profile(&conn, "  cliente B · v1 ", &reference()).unwrap();
        create_profile(&conn, "Cliente A · guion v3", &reference()).unwrap();
        assert_eq!(b.name, "cliente B · v1");
        let names: Vec<String> = list_profiles(&conn)
            .unwrap()
            .into_iter()
            .map(|p| p.name)
            .collect();
        assert_eq!(names, ["Cliente A · guion v3", "cliente B · v1"]);
        assert_eq!(
            create_profile(&conn, "CLIENTE B · V1", &reference())
                .unwrap_err()
                .to_string(),
            "Ya existe un perfil llamado «CLIENTE B · V1»."
        );
        delete_profile(&conn, &b.id).unwrap();
        assert_eq!(
            delete_profile(&conn, &b.id).unwrap_err().to_string(),
            "El perfil no existe."
        );
        assert_eq!(list_profiles(&conn).unwrap().len(), 1);
    }

    #[test]
    fn profile_names_are_required_and_limited() {
        let conn = test_support::conn();
        assert_eq!(
            create_profile(&conn, "   ", &reference())
                .unwrap_err()
                .to_string(),
            "«Nombre del perfil» es obligatorio."
        );
        assert_eq!(
            create_profile(&conn, &"á".repeat(101), &reference())
                .unwrap_err()
                .to_string(),
            "«Nombre del perfil» supera 100 caracteres."
        );
        assert!(list_profiles(&conn).unwrap().is_empty());
    }

    #[test]
    fn unreadable_profiles_are_skipped_without_hiding_valid_ones() {
        let conn = test_support::conn();
        let valid = create_profile(&conn, "Válido", &reference()).unwrap();
        let broken = create_profile(&conn, "Dañado", &reference()).unwrap();
        conn.execute(
            "UPDATE import_profile SET definition = '{' WHERE id = ?1",
            [&broken.id],
        )
        .unwrap();
        assert_eq!(list_profiles(&conn).unwrap(), [valid]);
        delete_profile(&conn, &broken.id).unwrap();
    }

    #[test]
    fn profile_names_ignore_unicode_case_but_keep_accents() {
        let conn = test_support::conn();
        create_profile(&conn, "GUION ÁREA", &reference()).unwrap();
        assert_eq!(
            create_profile(&conn, "guion área", &reference())
                .unwrap_err()
                .to_string(),
            "Ya existe un perfil llamado «guion área»."
        );
        // Acentos distintos siguen siendo nombres distintos, solo se ignoran mayúsculas.
        create_profile(&conn, "guion area", &reference()).unwrap();
        create_profile(&conn, "Guion Ábaco", &reference()).unwrap();
        let names: Vec<String> = list_profiles(&conn)
            .unwrap()
            .into_iter()
            .map(|p| p.name)
            .collect();
        assert_eq!(names, ["guion area", "Guion Ábaco", "GUION ÁREA"]);
    }

    #[test]
    fn deleting_a_profile_keeps_the_project_source() {
        let conn = test_support::conn();
        let p = test_support::project(&conn);
        let profile = create_profile(&conn, "Perfil", &reference()).unwrap();
        insert_source(&conn, &p.id, Some(&profile.id));
        delete_profile(&conn, &profile.id).unwrap();
        let saved = source(&conn, &p.id).unwrap().unwrap();
        assert_eq!(saved.profile_id, None);
        assert_eq!(saved.definition, reference());
        assert_eq!(saved.file_name, "guion.xlsx");
        assert_eq!(saved.stored_path, format!("evidence/{}/guion.xlsx", p.id));
        assert_eq!(saved.sha256, "hash");
        assert_eq!(saved.sheet, "Guión de Pruebas");
        assert_eq!(saved.header_row, 15);
        assert_eq!(saved.imported_at, "");
    }

    #[test]
    fn source_rows_go_away_with_their_project_and_cases() {
        let conn = test_support::conn();
        let p = test_support::project(&conn);
        let first = test_support::case(&conn, &p);
        let second = test_support::case(&conn, &p);
        insert_source(&conn, &p.id, None);
        // Inserción de origen invertida: se conserva el orden de test_case, no el de case_source.
        conn.execute(
            "INSERT INTO case_source (case_id, row) VALUES (?1, 17), (?2, 16)",
            params![second.id, first.id],
        )
        .unwrap();
        assert_eq!(
            case_rows(&conn, &p.id).unwrap(),
            [(first.id.clone(), 16), (second.id.clone(), 17)]
        );
        conn.execute(
            "INSERT INTO case_fixture (id, case_id, file_name, stored_path, size, sha256, note, position, added_at) VALUES ('fixture', ?1, 'datos.csv', 'evidence/datos.csv', 1, 'hash', '', 0, '')",
            [&first.id],
        ).unwrap();
        cases::delete(&conn, &first.id).unwrap();
        let fixtures: i64 = conn
            .query_row("SELECT COUNT(*) FROM case_fixture", [], |r| r.get(0))
            .unwrap();
        assert_eq!(fixtures, 0);
        assert_eq!(case_rows(&conn, &p.id).unwrap(), [(second.id, 17)]);
        projects::delete(&conn, &p.id).unwrap();
        assert!(source(&conn, &p.id).unwrap().is_none());
        assert!(case_rows(&conn, &p.id).unwrap().is_empty());
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM case_source", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }
}
