use std::{collections::HashSet, fs};

use rusqlite::{Connection, OptionalExtension, Row, params};

use super::{cases, projects};
use crate::{
    db::{new_id, now},
    domain::text::required_text,
    dto::{EvidenceDto, EvidenceUpdateMeta, NewEvidenceMeta, StagedImageDto},
    error::{AppError, AppResult, validation},
    media::{
        raster,
        storage::{EvidenceFiles, write_atomic},
    },
};

pub const MAX_DESCRIPTION: usize = 2000;
pub const MAX_ANNOTATIONS_BYTES: usize = 512 * 1024;

const COLUMNS: &str = "id, case_id, position, description, original_path, rendered_path,
    annotations, width, height, captured_at, updated_at";

fn not_found() -> AppError {
    AppError::NotFound("La evidencia no existe.".into())
}

fn from_row(files: &EvidenceFiles, r: &Row) -> rusqlite::Result<EvidenceDto> {
    Ok(EvidenceDto {
        id: r.get("id")?,
        case_id: r.get("case_id")?,
        position: r.get("position")?,
        description: r.get("description")?,
        original_path: files.display(&r.get::<_, String>("original_path")?),
        rendered_path: files.display(&r.get::<_, String>("rendered_path")?),
        annotations: r.get("annotations")?,
        width: r.get("width")?,
        height: r.get("height")?,
        captured_at: r.get("captured_at")?,
        updated_at: r.get("updated_at")?,
    })
}

fn validate_description(raw: &str) -> AppResult<String> {
    required_text("Descripción", raw, MAX_DESCRIPTION)
}

/// Rust solo exige la envoltura versionada; la forma de cada anotación la valida el dominio TS.
fn validate_annotations(raw: &str) -> AppResult<String> {
    if raw.len() > MAX_ANNOTATIONS_BYTES {
        return Err(validation("Las anotaciones superan 512 KB."));
    }
    let invalid = || validation("Las anotaciones no tienen un formato válido.");
    let value: serde_json::Value = serde_json::from_str(raw).map_err(|_| invalid())?;
    let ok = value.get("version").and_then(serde_json::Value::as_i64) == Some(1)
        && value.get("shapes").is_some_and(serde_json::Value::is_array);
    if ok {
        Ok(raw.to_string())
    } else {
        Err(invalid())
    }
}

fn case_project(conn: &Connection, case_id: &str) -> AppResult<String> {
    conn.query_row(
        "SELECT project_id FROM test_case WHERE id = ?1",
        [case_id],
        |r| r.get(0),
    )
    .optional()?
    .ok_or_else(|| AppError::NotFound("El caso no existe.".into()))
}

struct Stored {
    case_id: String,
    project_id: String,
    original_rel: String,
    render_rel: String,
}

fn stored(conn: &Connection, id: &str) -> AppResult<Stored> {
    conn.query_row(
        "SELECT e.case_id, c.project_id, e.original_path, e.rendered_path
         FROM evidence e JOIN test_case c ON c.id = e.case_id WHERE e.id = ?1",
        [id],
        |r| {
            Ok(Stored {
                case_id: r.get(0)?,
                project_id: r.get(1)?,
                original_rel: r.get(2)?,
                render_rel: r.get(3)?,
            })
        },
    )
    .optional()?
    .ok_or_else(not_found)
}

fn ids_in(conn: &Connection, case_id: &str) -> AppResult<Vec<String>> {
    let mut stmt =
        conn.prepare("SELECT id FROM evidence WHERE case_id = ?1 ORDER BY position, captured_at")?;
    let ids = stmt
        .query_map([case_id], |r| r.get(0))?
        .collect::<Result<Vec<String>, _>>()?;
    Ok(ids)
}

fn set_positions(conn: &Connection, ids: &[String]) -> AppResult<()> {
    let mut stmt = conn.prepare("UPDATE evidence SET position = ?1 WHERE id = ?2")?;
    for (position, id) in ids.iter().enumerate() {
        stmt.execute(params![position as i64, id])?;
    }
    Ok(())
}

fn compact(conn: &Connection, case_id: &str) -> AppResult<()> {
    set_positions(conn, &ids_in(conn, case_id)?)
}

pub fn get(conn: &Connection, files: &EvidenceFiles, id: &str) -> AppResult<EvidenceDto> {
    conn.query_row(
        &format!("SELECT {COLUMNS} FROM evidence WHERE id = ?1"),
        [id],
        |r| from_row(files, r),
    )
    .optional()?
    .ok_or_else(not_found)
}

pub fn list(
    conn: &Connection,
    files: &EvidenceFiles,
    case_id: &str,
) -> AppResult<Vec<EvidenceDto>> {
    case_project(conn, case_id)?;
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLUMNS} FROM evidence WHERE case_id = ?1 ORDER BY position, captured_at"
    ))?;
    let rows = stmt
        .query_map([case_id], |r| from_row(files, r))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn stage(files: &EvidenceFiles, bytes: &[u8]) -> AppResult<StagedImageDto> {
    let image = raster::normalize(bytes)?;
    let path = files.stage(&image.png)?;
    Ok(StagedImageDto {
        path: path.to_string_lossy().into_owned(),
        width: image.width.into(),
        height: image.height.into(),
    })
}

pub fn add(
    conn: &Connection,
    files: &EvidenceFiles,
    meta: &NewEvidenceMeta,
    render: &[u8],
) -> AppResult<EvidenceDto> {
    let description = validate_description(&meta.description)?;
    let annotations = validate_annotations(&meta.annotations)?;
    let project_id = case_project(conn, &meta.case_id)?;
    let staged = files.resolve_inside(&meta.staged_path, true)?;
    let (width, height) = raster::dimensions(&fs::read(&staged)?)?;
    raster::validate_render(render)?;

    let id = new_id();
    let original_rel = EvidenceFiles::original_rel(&project_id, &id);
    let render_rel = EvidenceFiles::render_rel(&project_id, &id);
    let original = files.absolute(&original_rel);
    let rendered = files.absolute(&render_rel);
    fs::create_dir_all(files.project_dir(&project_id))?;
    write_atomic(&rendered, render)?;
    if let Err(error) = fs::rename(&staged, &original) {
        let _ = fs::remove_file(&rendered);
        return Err(error.into());
    }
    let saved = (|| -> AppResult<()> {
        let tx = conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO evidence (id, case_id, position, description, original_path, rendered_path,
                                   annotations, width, height, captured_at, updated_at)
             VALUES (?1, ?2, (SELECT COUNT(*) FROM evidence WHERE case_id = ?2), ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
            params![id, meta.case_id, description, original_rel, render_rel, annotations, width, height, now()],
        )?;
        projects::touch(&tx, &project_id)?;
        tx.commit()?;
        Ok(())
    })();
    if let Err(error) = saved {
        // Devolver la imagen a _staging permite reintentar «Guardar» desde el editor.
        let _ = fs::rename(&original, &staged);
        let _ = fs::remove_file(&rendered);
        return Err(error);
    }
    get(conn, files, &id)
}

pub fn update(
    conn: &Connection,
    files: &EvidenceFiles,
    meta: &EvidenceUpdateMeta,
    render: &[u8],
) -> AppResult<EvidenceDto> {
    let s = stored(conn, &meta.evidence_id)?;
    let description = validate_description(&meta.description)?;
    let annotations = validate_annotations(&meta.annotations)?;
    raster::validate_render(render)?;
    let rendered = files.absolute(&s.render_rel);
    let backup = rendered.with_extension("png.bak");
    let had_render = rendered.exists();
    if had_render {
        fs::rename(&rendered, &backup)?;
    }
    let restore = || {
        if had_render {
            let _ = fs::rename(&backup, &rendered);
        } else {
            let _ = fs::remove_file(&rendered);
        }
    };
    if let Err(error) = write_atomic(&rendered, render) {
        restore();
        return Err(error);
    }
    let saved = (|| -> AppResult<()> {
        let tx = conn.unchecked_transaction()?;
        tx.execute(
            "UPDATE evidence SET description = ?1, annotations = ?2, updated_at = ?3 WHERE id = ?4",
            params![description, annotations, now(), meta.evidence_id],
        )?;
        projects::touch(&tx, &s.project_id)?;
        tx.commit()?;
        Ok(())
    })();
    match saved {
        Ok(()) => {
            let _ = fs::remove_file(&backup);
        }
        Err(error) => {
            restore();
            return Err(error);
        }
    }
    get(conn, files, &meta.evidence_id)
}

pub fn update_description(
    conn: &Connection,
    files: &EvidenceFiles,
    id: &str,
    description: &str,
) -> AppResult<EvidenceDto> {
    let s = stored(conn, id)?;
    let description = validate_description(description)?;
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "UPDATE evidence SET description = ?1, updated_at = ?2 WHERE id = ?3",
        params![description, now(), id],
    )?;
    projects::touch(&tx, &s.project_id)?;
    tx.commit()?;
    get(conn, files, id)
}

pub fn move_to_case(
    conn: &Connection,
    files: &EvidenceFiles,
    id: &str,
    target_case_id: &str,
) -> AppResult<EvidenceDto> {
    let s = stored(conn, id)?;
    if case_project(conn, target_case_id)? != s.project_id {
        return Err(validation(
            "Solo puedes mover evidencias entre casos de la misma matriz.",
        ));
    }
    if target_case_id != s.case_id {
        let tx = conn.unchecked_transaction()?;
        tx.execute(
            "UPDATE evidence SET case_id = ?1, updated_at = ?2,
                    position = (SELECT COUNT(*) FROM evidence WHERE case_id = ?1)
             WHERE id = ?3",
            params![target_case_id, now(), id],
        )?;
        compact(&tx, &s.case_id)?;
        projects::touch(&tx, &s.project_id)?;
        tx.commit()?;
    }
    get(conn, files, id)
}

pub fn reorder(
    conn: &Connection,
    files: &EvidenceFiles,
    case_id: &str,
    ordered: &[String],
) -> AppResult<Vec<EvidenceDto>> {
    let project_id = case_project(conn, case_id)?;
    let mut current = ids_in(conn, case_id)?;
    current.sort();
    let mut requested = ordered.to_vec();
    requested.sort();
    if current != requested {
        return Err(validation(
            "La lista de evidencias cambió. Recarga el caso.",
        ));
    }
    let tx = conn.unchecked_transaction()?;
    set_positions(&tx, ordered)?;
    projects::touch(&tx, &project_id)?;
    tx.commit()?;
    list(conn, files, case_id)
}

pub fn delete(conn: &Connection, files: &EvidenceFiles, id: &str) -> AppResult<()> {
    let s = stored(conn, id)?;
    let tx = conn.unchecked_transaction()?;
    tx.execute("DELETE FROM evidence WHERE id = ?1", [id])?;
    compact(&tx, &s.case_id)?;
    projects::touch(&tx, &s.project_id)?;
    tx.commit()?;
    files.remove_files(&[s.original_rel, s.render_rel]);
    Ok(())
}

fn case_files(conn: &Connection, case_id: &str) -> AppResult<Vec<String>> {
    let mut stmt =
        conn.prepare("SELECT original_path, rendered_path FROM evidence WHERE case_id = ?1")?;
    let mut out = Vec::new();
    for row in stmt.query_map([case_id], |r| {
        Ok([r.get::<_, String>(0)?, r.get::<_, String>(1)?])
    })? {
        out.extend(row?);
    }
    Ok(out)
}

pub fn delete_case(conn: &Connection, files: &EvidenceFiles, case_id: &str) -> AppResult<()> {
    let paths = case_files(conn, case_id)?;
    cases::delete(conn, case_id)?;
    files.remove_files(&paths);
    Ok(())
}

pub fn delete_project(conn: &Connection, files: &EvidenceFiles, project_id: &str) -> AppResult<()> {
    projects::delete(conn, project_id)?;
    files.remove_project(project_id);
    Ok(())
}

/// Al arrancar: borra archivos de `evidence/` que la base no referencia.
pub fn sweep_orphans(conn: &Connection, files: &EvidenceFiles) -> AppResult<usize> {
    let mut referenced = HashSet::new();
    let mut stmt = conn.prepare("SELECT original_path, rendered_path FROM evidence")?;
    for row in stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))? {
        let (original, rendered) = row?;
        referenced.insert(original);
        referenced.insert(rendered);
    }
    let mut stmt = conn.prepare("SELECT id FROM project")?;
    let projects = stmt
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<Result<HashSet<_>, _>>()?;
    files.sweep(&referenced, &projects)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        dto::{CaseDto, ProjectDto},
        media::{raster::sample_png, storage::TempDir},
        repo::{
            cases as case_repo,
            test_support::{case, conn, other_project, project},
        },
    };

    const ANN: &str = r#"{"version":1,"crop":null,"shapes":[]}"#;

    struct Env {
        _tmp: TempDir,
        files: EvidenceFiles,
        conn: Connection,
        project: ProjectDto,
        case: CaseDto,
    }

    fn env() -> Env {
        let tmp = TempDir::new();
        let files = tmp.files();
        let conn = conn();
        let project = project(&conn);
        let case = case(&conn, &project);
        Env {
            _tmp: tmp,
            files,
            conn,
            project,
            case,
        }
    }

    fn staged(e: &Env) -> StagedImageDto {
        stage(&e.files, &sample_png(800, 600)).unwrap()
    }

    fn meta(case_id: &str, staged: &StagedImageDto, description: &str) -> NewEvidenceMeta {
        NewEvidenceMeta {
            case_id: case_id.into(),
            staged_path: staged.path.clone(),
            description: description.into(),
            annotations: ANN.into(),
        }
    }

    fn add_one(e: &Env, case_id: &str, description: &str) -> EvidenceDto {
        let s = staged(e);
        add(
            &e.conn,
            &e.files,
            &meta(case_id, &s, description),
            &sample_png(800, 600),
        )
        .unwrap()
    }

    fn project_files(e: &Env) -> usize {
        fs::read_dir(e.files.project_dir(&e.project.id))
            .map(|d| d.count())
            .unwrap_or(0)
    }

    fn descriptions(e: &Env, case_id: &str) -> Vec<String> {
        list(&e.conn, &e.files, case_id)
            .unwrap()
            .into_iter()
            .map(|x| x.description)
            .collect()
    }

    #[test]
    fn add_moves_the_original_writes_the_render_and_counts() {
        let e = env();
        let s = staged(&e);
        let first = add(
            &e.conn,
            &e.files,
            &meta(&e.case.id, &s, "  Pantalla de recibos  "),
            &sample_png(400, 300),
        )
        .unwrap();
        assert_eq!((first.position, first.width, first.height), (0, 800, 600));
        assert_eq!(first.description, "Pantalla de recibos");
        assert!(std::path::Path::new(&first.original_path).exists());
        assert!(std::path::Path::new(&first.rendered_path).exists());
        assert!(
            !std::path::Path::new(&s.path).exists(),
            "el staging se consume"
        );
        let second = add_one(&e, &e.case.id, "Segunda");
        assert_eq!(second.position, 1);
        assert_eq!(
            descriptions(&e, &e.case.id),
            ["Pantalla de recibos", "Segunda"]
        );
        assert_eq!(
            case_repo::get(&e.conn, &e.case.id).unwrap().evidence_count,
            2
        );
    }

    #[test]
    fn add_validates_without_touching_files() {
        let e = env();
        let s = staged(&e);
        let render = sample_png(10, 10);
        let blank = meta(&e.case.id, &s, "  ");
        assert_eq!(
            add(&e.conn, &e.files, &blank, &render)
                .unwrap_err()
                .to_string(),
            "«Descripción» es obligatorio."
        );
        let mut bad_ann = meta(&e.case.id, &s, "ok");
        bad_ann.annotations = r#"{"version":2,"shapes":[]}"#.into();
        assert_eq!(
            add(&e.conn, &e.files, &bad_ann, &render)
                .unwrap_err()
                .to_string(),
            "Las anotaciones no tienen un formato válido."
        );
        let mut outside = meta(&e.case.id, &s, "ok");
        let db = e._tmp.path().join("qapture.db");
        fs::write(&db, b"db").unwrap();
        outside.staged_path = db.to_string_lossy().into_owned();
        assert_eq!(
            add(&e.conn, &e.files, &outside, &render)
                .unwrap_err()
                .to_string(),
            "La ruta de la imagen no es válida."
        );
        assert!(add(&e.conn, &e.files, &meta(&e.case.id, &s, "ok"), b"no-png").is_err());
        assert_eq!(
            add(&e.conn, &e.files, &meta("no-existe", &s, "ok"), &render)
                .unwrap_err()
                .code(),
            "not_found"
        );
        assert!(std::path::Path::new(&s.path).exists());
        assert_eq!(project_files(&e), 0);
    }

    #[test]
    fn add_rolls_back_files_when_the_db_fails_and_can_retry() {
        let e = env();
        let s = staged(&e);
        e.conn
            .execute_batch("CREATE TRIGGER fail_insert BEFORE INSERT ON evidence BEGIN SELECT RAISE(ABORT, 'falla simulada'); END;")
            .unwrap();
        assert!(
            add(
                &e.conn,
                &e.files,
                &meta(&e.case.id, &s, "ok"),
                &sample_png(10, 10)
            )
            .is_err()
        );
        assert_eq!(project_files(&e), 0, "sin archivos huérfanos");
        assert!(
            std::path::Path::new(&s.path).exists(),
            "el staging vuelve para reintentar"
        );
        e.conn.execute_batch("DROP TRIGGER fail_insert;").unwrap();
        add(
            &e.conn,
            &e.files,
            &meta(&e.case.id, &s, "ok"),
            &sample_png(10, 10),
        )
        .unwrap();
        assert_eq!(project_files(&e), 2);
    }

    #[test]
    fn add_leaves_no_row_when_files_fail() {
        let e = env();
        let s = staged(&e);
        fs::create_dir_all(e.files.root()).unwrap();
        fs::write(e.files.project_dir(&e.project.id), b"soy un archivo").unwrap();
        assert!(
            add(
                &e.conn,
                &e.files,
                &meta(&e.case.id, &s, "ok"),
                &sample_png(10, 10)
            )
            .is_err()
        );
        assert_eq!(list(&e.conn, &e.files, &e.case.id).unwrap().len(), 0);
        assert!(std::path::Path::new(&s.path).exists());
    }

    #[test]
    fn update_replaces_the_render_and_keeps_the_original() {
        let e = env();
        let ev = add_one(&e, &e.case.id, "Antes");
        let original = fs::read(&ev.original_path).unwrap();
        let new_render = sample_png(20, 20);
        let ann = r#"{"version":1,"crop":null,"shapes":[{"id":"s1"}]}"#;
        let up = EvidenceUpdateMeta {
            evidence_id: ev.id.clone(),
            description: "Después".into(),
            annotations: ann.into(),
        };
        let saved = update(&e.conn, &e.files, &up, &new_render).unwrap();
        assert_eq!(
            (saved.description.as_str(), saved.annotations.as_str()),
            ("Después", ann)
        );
        assert_eq!(fs::read(&ev.rendered_path).unwrap(), new_render);
        assert_eq!(fs::read(&ev.original_path).unwrap(), original);
        assert_eq!(project_files(&e), 2, "sin .bak ni .tmp");

        e.conn
            .execute_batch("CREATE TRIGGER fail_update BEFORE UPDATE ON evidence BEGIN SELECT RAISE(ABORT, 'falla simulada'); END;")
            .unwrap();
        assert!(update(&e.conn, &e.files, &up, &sample_png(30, 30)).is_err());
        assert_eq!(
            fs::read(&ev.rendered_path).unwrap(),
            new_render,
            "se restaura el render previo"
        );
        assert_eq!(project_files(&e), 2);
    }

    #[test]
    fn update_description_trims_and_validates() {
        let e = env();
        let ev = add_one(&e, &e.case.id, "Antes");
        assert_eq!(
            update_description(&e.conn, &e.files, &ev.id, "  Nueva ")
                .unwrap()
                .description,
            "Nueva"
        );
        assert!(update_description(&e.conn, &e.files, &ev.id, "").is_err());
        assert_eq!(
            update_description(&e.conn, &e.files, "x", "a")
                .unwrap_err()
                .code(),
            "not_found"
        );
    }

    #[test]
    fn move_and_reorder_keep_positions_compact() {
        let e = env();
        let other = case(&e.conn, &e.project);
        let a = add_one(&e, &e.case.id, "A");
        let b = add_one(&e, &e.case.id, "B");
        let c = add_one(&e, &e.case.id, "C");
        let moved = move_to_case(&e.conn, &e.files, &b.id, &other.id).unwrap();
        assert_eq!(
            (moved.case_id.as_str(), moved.position),
            (other.id.as_str(), 0)
        );
        assert_eq!(descriptions(&e, &e.case.id), ["A", "C"]);
        assert_eq!(list(&e.conn, &e.files, &e.case.id).unwrap()[1].position, 1);

        let foreign = case(&e.conn, &other_project(&e.conn));
        assert_eq!(
            move_to_case(&e.conn, &e.files, &a.id, &foreign.id)
                .unwrap_err()
                .to_string(),
            "Solo puedes mover evidencias entre casos de la misma matriz."
        );

        let reordered =
            reorder(&e.conn, &e.files, &e.case.id, &[c.id.clone(), a.id.clone()]).unwrap();
        assert_eq!(
            reordered
                .iter()
                .map(|x| x.description.as_str())
                .collect::<Vec<_>>(),
            ["C", "A"]
        );
        for stale in [
            vec![c.id.clone()],
            vec![c.id.clone(), c.id.clone()],
            vec![c.id.clone(), b.id.clone()],
        ] {
            assert_eq!(
                reorder(&e.conn, &e.files, &e.case.id, &stale)
                    .unwrap_err()
                    .to_string(),
                "La lista de evidencias cambió. Recarga el caso."
            );
        }
    }

    #[test]
    fn delete_removes_files_and_compacts() {
        let e = env();
        let a = add_one(&e, &e.case.id, "A");
        add_one(&e, &e.case.id, "B");
        delete(&e.conn, &e.files, &a.id).unwrap();
        assert!(!std::path::Path::new(&a.original_path).exists());
        assert!(!std::path::Path::new(&a.rendered_path).exists());
        let rest = list(&e.conn, &e.files, &e.case.id).unwrap();
        assert_eq!((rest.len(), rest[0].position), (1, 0));
        assert_eq!(
            delete(&e.conn, &e.files, &a.id).unwrap_err().code(),
            "not_found"
        );
    }

    #[test]
    fn deleting_a_case_or_project_removes_its_files() {
        let e = env();
        let ev = add_one(&e, &e.case.id, "A");
        let keep_case = case(&e.conn, &e.project);
        let kept = add_one(&e, &keep_case.id, "B");
        delete_case(&e.conn, &e.files, &e.case.id).unwrap();
        assert!(!std::path::Path::new(&ev.original_path).exists());
        assert!(std::path::Path::new(&kept.original_path).exists());
        delete_project(&e.conn, &e.files, &e.project.id).unwrap();
        assert!(!e.files.project_dir(&e.project.id).exists());
    }

    #[test]
    fn sweep_orphans_keeps_referenced_files() {
        let e = env();
        let ev = add_one(&e, &e.case.id, "A");
        let stray = e.files.project_dir(&e.project.id).join("perdido.orig.png");
        fs::write(&stray, b"x").unwrap();
        fs::create_dir_all(e.files.project_dir("proyecto-borrado")).unwrap();
        assert_eq!(sweep_orphans(&e.conn, &e.files).unwrap(), 2);
        assert!(std::path::Path::new(&ev.original_path).exists());
        assert!(std::path::Path::new(&ev.rendered_path).exists());
        assert!(!stray.exists());
    }
}
