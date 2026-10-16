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
            let _ = fs::remove_file(self.absolute(rel));
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
        let _ = fs::remove_dir_all(self.project_dir(project_id));
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
                    fs::remove_dir_all(&path)
                } else {
                    fs::remove_file(&path)
                };
                removed += 1;
                continue;
            }
            for file in fs::read_dir(&path)? {
                let file = file?;
                let rel = format!(
                    "{EVIDENCE_DIR}/{name}/{}",
                    file.file_name().to_string_lossy()
                );
                if !referenced.contains(&rel) {
                    let _ = fs::remove_file(file.path());
                    removed += 1;
                }
            }
        }
        Ok(removed)
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
}
