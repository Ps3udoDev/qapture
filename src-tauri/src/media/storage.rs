use std::{
    collections::HashSet,
    fs,
    path::{MAIN_SEPARATOR_STR, Path, PathBuf},
};

use crate::{
    db::new_id,
    error::{AppResult, validation},
};

pub const EVIDENCE_DIR: &str = "evidence";
pub const STAGING_DIR: &str = "_staging";
pub const UNUSED_DIR: &str = "evidence-sin-uso";

/// `<datos>/evidence/<proyecto>/<id>.orig.png` y `.render.png`, más `evidence/_staging/`
/// para las imágenes que todavía están abiertas en el editor.
#[derive(Debug, Clone)]
pub struct EvidenceFiles {
    data_dir: PathBuf,
}

impl EvidenceFiles {
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            data_dir: data_dir.into(),
        }
    }

    pub fn root(&self) -> PathBuf {
        self.data_dir.join(EVIDENCE_DIR)
    }

    pub fn staging_dir(&self) -> PathBuf {
        self.root().join(STAGING_DIR)
    }

    pub fn project_dir(&self, project_id: &str) -> PathBuf {
        self.root().join(project_id)
    }

    pub fn original_rel(project_id: &str, id: &str) -> String {
        format!("{EVIDENCE_DIR}/{project_id}/{id}.orig.png")
    }

    pub fn render_rel(project_id: &str, id: &str) -> String {
        format!("{EVIDENCE_DIR}/{project_id}/{id}.render.png")
    }

    pub fn absolute(&self, rel: &str) -> PathBuf {
        self.data_dir.join(rel.replace('/', MAIN_SEPARATOR_STR))
    }

    pub fn display(&self, rel: &str) -> String {
        self.absolute(rel).to_string_lossy().into_owned()
    }

    pub fn stage(&self, png: &[u8]) -> AppResult<PathBuf> {
        let dir = self.staging_dir();
        fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{}.png", new_id()));
        write_atomic(&path, png)?;
        Ok(path)
    }

    /// Valida una ruta que llega del webview: debe ser un archivo existente dentro de
    /// `evidence/` (o de `_staging/` si `staged_only`), resolviendo `..` y enlaces.
    pub fn resolve_inside(&self, raw: &str, staged_only: bool) -> AppResult<PathBuf> {
        let invalid = || validation("La ruta de la imagen no es válida.");
        let base = if staged_only {
            self.staging_dir()
        } else {
            self.root()
        };
        let base = fs::canonicalize(base).map_err(|_| invalid())?;
        let path = fs::canonicalize(raw).map_err(|_| invalid())?;
        if path.starts_with(&base) && path.is_file() {
            Ok(path)
        } else {
            Err(invalid())
        }
    }

    pub fn discard_staged(&self, raw: &str) -> AppResult<()> {
        fs::remove_file(self.resolve_inside(raw, true)?)?;
        Ok(())
    }

    pub fn clear_staging(&self) -> AppResult<()> {
        let dir = self.staging_dir();
        if dir.exists() {
            fs::remove_dir_all(dir)?;
        }
        Ok(())
    }

    /// Borrado de mejor esfuerzo: la base ya no referencia estos archivos y `sweep` limpia lo que quede.
    pub fn remove_files(&self, rels: &[String]) {
        for rel in rels {
            let _ = remove_file(&self.absolute(rel));
        }
    }

    pub fn remove_project(&self, project_id: &str) {
        let safe = !project_id.is_empty()
            && project_id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
        if !safe {
            return;
        }
        let dir = self.project_dir(project_id);
        if !dir.exists() {
            return;
        }
        if fs::remove_dir_all(&dir).is_err() {
            clear_readonly_recursive(&dir);
            let _ = fs::remove_dir_all(&dir);
        }
    }

    /// Borra lo que la base de datos no referencia: restos de fallos, cierres bruscos
    /// o proyectos eliminados. `referenced` son rutas relativas (`evidence/<p>/<archivo>`).
    pub fn sweep(
        &self,
        referenced: &HashSet<String>,
        projects: &HashSet<String>,
    ) -> AppResult<usize> {
        let root = self.root();
        if !root.exists() {
            return Ok(0);
        }
        let mut removed = 0;
        for entry in fs::read_dir(&root)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let path = entry.path();
            if name == STAGING_DIR {
                continue;
            }
            if !path.is_dir() || !projects.contains(&name) {
                let _ = if path.is_dir() {
                    if fs::remove_dir_all(&path).is_err() {
                        clear_readonly_recursive(&path);
                        fs::remove_dir_all(&path)
                    } else {
                        Ok(())
                    }
                } else {
                    remove_file(&path)
                };
                removed += 1;
                continue;
            }
            for file_path in collect_files_recursive(&path) {
                let Ok(rel_from_data) = file_path.strip_prefix(&self.data_dir) else {
                    continue;
                };
                let rel = rel_from_data.to_string_lossy().replace('\\', "/");
                if !referenced.contains(&rel) {
                    let _ = remove_file(&file_path);
                    removed += 1;
                }
            }
            remove_empty_subdirs(&path);
        }
        Ok(removed)
    }

    /// Como `sweep`, pero mueve lo no referenciado a `evidence-sin-uso/<stamp>/<proyecto>/` en vez
    /// de borrarlo: restaurar un respaldo o recuperar la base nunca debe perder capturas. Si un
    /// movimiento falla, devuelve a su sitio lo ya movido antes de devolver el error.
    pub fn set_aside(
        &self,
        referenced: &HashSet<String>,
        projects: &HashSet<String>,
        stamp: &str,
    ) -> AppResult<SetAside> {
        let mut aside = SetAside::default();
        match self.move_unreferenced(referenced, projects, stamp, &mut aside) {
            Ok(()) => Ok(aside),
            Err(error) => {
                aside.undo();
                Err(error)
            }
        }
    }

    fn move_unreferenced(
        &self,
        referenced: &HashSet<String>,
        projects: &HashSet<String>,
        stamp: &str,
        aside: &mut SetAside,
    ) -> AppResult<()> {
        let root = self.root();
        if !root.exists() {
            return Ok(());
        }
        let target = self.data_dir.join(UNUSED_DIR).join(stamp);
        for entry in fs::read_dir(&root)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let path = entry.path();
            if name == STAGING_DIR || !path.is_dir() {
                continue;
            }
            let known_project = projects.contains(&name);
            for file_path in collect_files_recursive(&path) {
                let Ok(rel_from_root) = file_path.strip_prefix(&root) else {
                    continue;
                };
                let rel = format!(
                    "{EVIDENCE_DIR}/{}",
                    rel_from_root.to_string_lossy().replace('\\', "/")
                );
                if known_project && referenced.contains(&rel) {
                    continue;
                }
                let dest = target.join(rel_from_root);
                if let Some(parent) = dest.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::rename(&file_path, &dest)?;
                aside.moves.push((file_path, dest));
            }
            if !known_project {
                remove_empty_subdirs(&path);
                let _ = fs::remove_dir(&path);
            } else {
                remove_empty_subdirs(&path);
            }
        }
        Ok(())
    }
}

/// Capturas que `set_aside` movió a `evidence-sin-uso/`, para poder deshacerlo si la operación
/// que lo pidió falla después.
#[derive(Debug, Default)]
pub struct SetAside {
    /// (ruta original en `evidence/`, ruta en `evidence-sin-uso/`)
    moves: Vec<(PathBuf, PathBuf)>,
}

impl SetAside {
    pub fn count(&self) -> usize {
        self.moves.len()
    }

    /// Devuelve cada archivo a `evidence/`. El que no se pueda devolver se queda en
    /// `evidence-sin-uso/`: nunca se borra.
    pub fn undo(self) {
        for (original, moved_to) in self.moves.into_iter().rev() {
            let back = original
                .parent()
                .map_or(Ok(()), fs::create_dir_all)
                .and_then(|()| fs::rename(&moved_to, &original));
            match back {
                Ok(()) => {
                    if let Some(parent) = moved_to.parent() {
                        let mut current = parent;
                        while current.file_name().is_some_and(|name| name != UNUSED_DIR) {
                            if fs::remove_dir(current).is_err() {
                                break;
                            }
                            if let Some(next) = current.parent() {
                                current = next;
                            } else {
                                break;
                            }
                        }
                    }
                }
                Err(error) => eprintln!(
                    "No se pudo devolver {} a {}: {error}",
                    moved_to.display(),
                    original.display()
                ),
            }
        }
    }
}

/// Si existe el archivo, quita la marca de solo lectura si la tiene y lo borra.
#[allow(clippy::permissions_set_readonly_false)]
pub(crate) fn remove_file(path: &Path) -> std::io::Result<()> {
    if path.exists() {
        if let Ok(meta) = fs::metadata(path) {
            let mut permissions = meta.permissions();
            if permissions.readonly() {
                permissions.set_readonly(false);
                let _ = fs::set_permissions(path, permissions);
            }
        }
        fs::remove_file(path)
    } else {
        Ok(())
    }
}

#[allow(clippy::permissions_set_readonly_false)]
fn clear_readonly_recursive(dir: &Path) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                clear_readonly_recursive(&path);
            }
            if let Ok(meta) = fs::metadata(&path) {
                let mut p = meta.permissions();
                if p.readonly() {
                    p.set_readonly(false);
                    let _ = fs::set_permissions(&path, p);
                }
            }
        }
    }
}

fn collect_files_recursive(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_files_recursive_into(dir, &mut files);
    files
}

fn collect_files_recursive_into(dir: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_files_recursive_into(&path, out);
            } else if path.is_file() {
                out.push(path);
            }
        }
    }
}

fn remove_empty_subdirs(dir: &Path) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                remove_empty_subdirs(&path);
                let _ = fs::remove_dir(&path);
            }
        }
    }
}

/// Escribe en un temporal del mismo directorio y renombra: nunca deja un PNG a medias.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> AppResult<()> {
    let tmp = path.with_extension(format!("{}.tmp", new_id()));
    fs::write(&tmp, bytes)?;
    if let Err(error) = fs::rename(&tmp, path) {
        let _ = fs::remove_file(&tmp);
        return Err(error.into());
    }
    Ok(())
}

#[cfg(test)]
pub(crate) struct TempDir(PathBuf);

#[cfg(test)]
impl TempDir {
    pub fn new() -> Self {
        let path = std::env::temp_dir().join(format!("qapture-test-{}", new_id()));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }

    pub fn files(&self) -> EvidenceFiles {
        EvidenceFiles::new(&self.0)
    }
}

#[cfg(test)]
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put(files: &EvidenceFiles, rel: &str) {
        let path = files.absolute(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b"x").unwrap();
    }

    #[test]
    fn stage_writes_inside_staging_and_can_be_discarded() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let staged = files.stage(b"png").unwrap();
        assert!(staged.starts_with(files.staging_dir()));
        let raw = staged.to_string_lossy().into_owned();
        assert!(files.resolve_inside(&raw, true).is_ok());
        assert!(files.resolve_inside(&raw, false).is_ok());
        files.discard_staged(&raw).unwrap();
        assert!(!staged.exists());
    }

    #[test]
    fn resolve_rejects_paths_outside_evidence() {
        let tmp = TempDir::new();
        let files = tmp.files();
        files.stage(b"x").unwrap();
        let db = tmp.path().join("qapture.db");
        fs::write(&db, b"db").unwrap();
        let sneaky = files.staging_dir().join("..").join("..").join("qapture.db");
        for raw in [
            db.to_string_lossy().into_owned(),
            sneaky.to_string_lossy().into_owned(),
            "no-existe.png".to_string(),
            files.root().to_string_lossy().into_owned(),
        ] {
            assert_eq!(
                files.resolve_inside(&raw, false).unwrap_err().to_string(),
                "La ruta de la imagen no es válida.",
                "{raw}"
            );
        }
        let rel = EvidenceFiles::original_rel("p1", "e1");
        put(&files, &rel);
        let inside = files.display(&rel);
        assert!(files.resolve_inside(&inside, false).is_ok());
        assert!(
            files.resolve_inside(&inside, true).is_err(),
            "solo _staging"
        );
    }

    #[test]
    fn write_atomic_replaces_without_leftovers() {
        let tmp = TempDir::new();
        let path = tmp.path().join("a.png");
        write_atomic(&path, b"uno").unwrap();
        write_atomic(&path, b"dos").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"dos");
        assert_eq!(fs::read_dir(tmp.path()).unwrap().count(), 1);
    }

    #[test]
    fn set_aside_moves_unreferenced_files_instead_of_deleting_them() {
        let tmp = TempDir::new();
        let files = tmp.files();
        files.stage(b"x").unwrap();
        let keep = EvidenceFiles::original_rel("p1", "e1");
        let extra = EvidenceFiles::render_rel("p1", "e2");
        let gone = EvidenceFiles::original_rel("p2", "e3");
        for rel in [&keep, &extra, &gone] {
            put(&files, rel);
        }
        let moved = files
            .set_aside(
                &HashSet::from([keep.clone()]),
                &HashSet::from(["p1".to_string()]),
                "20261016-100000",
            )
            .unwrap();
        assert_eq!(moved.count(), 2);
        assert!(files.absolute(&keep).exists());
        let target = tmp.path().join(UNUSED_DIR).join("20261016-100000");
        assert!(target.join("p1").join("e2.render.png").exists());
        assert!(target.join("p2").join("e3.orig.png").exists());
        assert!(!files.project_dir("p2").exists());
        assert!(files.staging_dir().exists(), "_staging no se toca");
        assert_eq!(
            files
                .set_aside(&HashSet::new(), &HashSet::new(), "otra")
                .unwrap()
                .count(),
            1,
            "solo queda e1"
        );
    }

    #[test]
    fn a_failed_set_aside_puts_back_what_it_already_moved() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let rels = [
            EvidenceFiles::original_rel("p1", "e1"),
            EvidenceFiles::render_rel("p1", "e1"),
            EvidenceFiles::original_rel("p2", "e3"),
        ];
        for rel in &rels {
            put(&files, rel);
        }
        // Una carpeta con el nombre de un archivo destino hace fallar ese movimiento.
        let target = tmp.path().join(UNUSED_DIR).join("20261016-100000");
        fs::create_dir_all(target.join("p2").join("e3.orig.png")).unwrap();
        assert!(
            files
                .set_aside(&HashSet::new(), &HashSet::new(), "20261016-100000")
                .is_err()
        );
        for rel in &rels {
            assert!(files.absolute(rel).is_file(), "{rel} volvió a evidence/");
        }
        assert!(!target.join("p1").exists());
    }

    #[test]
    fn clear_staging_and_sweep_remove_unreferenced_files() {
        let tmp = TempDir::new();
        let files = tmp.files();
        files.stage(b"x").unwrap();
        let keep = EvidenceFiles::original_rel("p1", "e1");
        let orphan = EvidenceFiles::render_rel("p1", "e2");
        let ghost = EvidenceFiles::original_rel("p2", "e3");
        for rel in [&keep, &orphan, &ghost] {
            put(&files, rel);
        }
        files.clear_staging().unwrap();
        assert!(!files.staging_dir().exists());
        let removed = files
            .sweep(
                &HashSet::from([keep.clone()]),
                &HashSet::from(["p1".to_string()]),
            )
            .unwrap();
        assert_eq!(removed, 2);
        assert!(files.absolute(&keep).exists());
        assert!(!files.absolute(&orphan).exists());
        assert!(!files.project_dir("p2").exists());
    }

    #[test]
    fn remove_project_ignores_unsafe_ids() {
        let tmp = TempDir::new();
        let files = tmp.files();
        files.stage(b"x").unwrap();
        for id in ["", "..", "a/b", "a\\b", "C:"] {
            files.remove_project(id);
        }
        assert!(files.staging_dir().exists());
    }

    #[test]
    fn readonly_files_are_removed() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let rel1 = "evidence/p1/fixtures/f1/a.txt".to_string();
        let path1 = files.absolute(&rel1);
        fs::create_dir_all(path1.parent().unwrap()).unwrap();
        fs::write(&path1, b"x").unwrap();
        let mut p = fs::metadata(&path1).unwrap().permissions();
        p.set_readonly(true);
        fs::set_permissions(&path1, p).unwrap();
        assert!(fs::metadata(&path1).unwrap().permissions().readonly());

        files.remove_files(&[rel1]);
        assert!(!path1.exists());

        let rel2 = "evidence/p1/fixtures/f2/b.txt".to_string();
        let path2 = files.absolute(&rel2);
        fs::create_dir_all(path2.parent().unwrap()).unwrap();
        fs::write(&path2, b"y").unwrap();
        let mut p = fs::metadata(&path2).unwrap().permissions();
        p.set_readonly(true);
        fs::set_permissions(&path2, p).unwrap();

        files.remove_project("p1");
        assert!(!files.project_dir("p1").exists());
    }

    #[test]
    fn sweep_handles_nested_files() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let keep_rel = "evidence/p1/fixtures/f1/keep.csv".to_string();
        let orphan_rel = "evidence/p1/fixtures/f2/orphan.csv".to_string();
        put(&files, &keep_rel);
        put(&files, &orphan_rel);

        let removed = files
            .sweep(
                &HashSet::from([keep_rel.clone()]),
                &HashSet::from(["p1".to_string()]),
            )
            .unwrap();
        assert_eq!(removed, 1);
        assert!(files.absolute(&keep_rel).exists());
        assert!(!files.absolute(&orphan_rel).exists());
        assert!(!files.project_dir("p1").join("fixtures").join("f2").exists());
        assert!(files.project_dir("p1").join("fixtures").join("f1").exists());
    }

    #[test]
    fn set_aside_handles_nested_files() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let keep_rel = "evidence/p1/fixtures/f1/keep.csv".to_string();
        let orphan_rel = "evidence/p1/fixtures/f2/orphan.csv".to_string();
        put(&files, &keep_rel);
        put(&files, &orphan_rel);

        let aside = files
            .set_aside(
                &HashSet::from([keep_rel.clone()]),
                &HashSet::from(["p1".to_string()]),
                "20261016-110000",
            )
            .unwrap();
        assert_eq!(aside.count(), 1);
        assert!(files.absolute(&keep_rel).exists());
        assert!(!files.absolute(&orphan_rel).exists());

        let target = tmp.path().join(UNUSED_DIR).join("20261016-110000");
        let moved_file = target
            .join("p1")
            .join("fixtures")
            .join("f2")
            .join("orphan.csv");
        assert!(moved_file.exists());

        aside.undo();
        assert!(files.absolute(&orphan_rel).exists());
        assert!(!target.exists());
    }
}
