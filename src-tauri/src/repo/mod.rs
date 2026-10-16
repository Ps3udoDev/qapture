pub mod cases;
pub mod evidence;
pub mod fields;
pub mod projects;
pub mod templates;

#[cfg(test)]
pub(crate) mod test_support {
    use std::collections::BTreeMap;

    use rusqlite::{Connection, params};

    use crate::{
        db,
        dto::{CaseDto, CaseInput, NewProjectInput, ProjectDto, ProjectMetaInput},
    };

    pub fn conn() -> Connection {
        db::open_in_memory().expect("db en memoria")
    }

    pub fn meta(name: &str) -> ProjectMetaInput {
        ProjectMetaInput {
            name: name.into(),
            client: "ACME".into(),
            test_type: "Funcional".into(),
            release_version: "1.0".into(),
            start_date: "2026-10-01".into(),
            end_date: "2026-10-31".into(),
        }
    }

    pub fn project(conn: &Connection) -> ProjectDto {
        super::projects::create(
            conn,
            NewProjectInput {
                meta: meta("Recibos"),
                template_id: "tpl-funcional".into(),
            },
        )
        .expect("proyecto")
    }

    pub fn case(conn: &Connection, project: &ProjectDto) -> CaseDto {
        let values = BTreeMap::from([(field_id(project, "entorno"), "QA".to_string())]);
        super::cases::create(
            conn,
            &project.id,
            &CaseInput {
                title: "Aplicación de recibos".into(),
                status: "No ejecutado".into(),
                values,
            },
        )
        .expect("caso")
    }

    pub fn other_project(conn: &Connection) -> ProjectDto {
        super::projects::create(
            conn,
            NewProjectInput {
                meta: meta("Pólizas"),
                template_id: "tpl-funcional".into(),
            },
        )
        .expect("proyecto")
    }

    pub fn field_id(project: &ProjectDto, key: &str) -> String {
        project
            .fields
            .iter()
            .find(|f| f.key == key)
            .unwrap_or_else(|| panic!("falta la columna {key}"))
            .id
            .clone()
    }

    /// Inserta un caso mínimo con un valor directamente en SQL, para probar columnas
    /// sin depender del repositorio de casos.
    pub fn insert_value(conn: &Connection, project_id: &str, field_id: &str, value: &str) {
        let case_id = db::new_id();
        conn.execute(
            "INSERT INTO test_case (id, project_id, code, title, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'Caso', 'No ejecutado', '', '')",
            params![case_id, project_id, format!("T-{case_id}")],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO field_value (case_id, field_id, value) VALUES (?1, ?2, ?3)",
            params![case_id, field_id, value],
        )
        .unwrap();
    }
}
