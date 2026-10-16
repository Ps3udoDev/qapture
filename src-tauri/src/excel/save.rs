use std::{
    collections::HashSet,
    fs::{self, OpenOptions},
    io::ErrorKind,
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard},
};

use rust_xlsxwriter::Workbook;

use crate::{
    db::new_id,
    error::{AppError, AppResult, validation},
};

/// `ERROR_SHARING_VIOLATION` (32) y `ERROR_LOCK_VIOLATION` (33): Windows los devuelve cuando
/// Excel tiene abierto el archivo (spec §8).
pub fn is_locked(error: &std::io::Error) -> bool {
    matches!(error.raw_os_error(), Some(32 | 33))
}

fn locked(path: &Path) -> AppError {
    AppError::FileLocked(path.display().to_string())
}

/// Falla antes de generar nada si el destino existe y no se puede sobrescribir.
pub fn ensure_writable(dest: &Path) -> AppResult<()> {
    if !dest.exists() {
        return Ok(());
    }
    match OpenOptions::new().append(true).open(dest) {
        Ok(_) => Ok(()),
        Err(e) if is_locked(&e) => Err(locked(dest)),
        Err(e) if e.kind() == ErrorKind::PermissionDenied => Err(validation(
            "No se puede sobrescribir el archivo: es de solo lectura o no tienes permisos.",
        )),
        Err(e) => Err(e.into()),
    }
}

/// Escribe el libro en un temporal junto al destino y lo renombra: nunca deja un xlsx a medias
/// ni temporales sueltos.
pub fn save_atomic(workbook: &mut Workbook, dest: &Path) -> AppResult<()> {
    ensure_writable(dest)?;
    let tmp = dest.with_extension(format!("{}.tmp", new_id()));
    if let Err(error) = workbook.save(&tmp) {
        let _ = fs::remove_file(&tmp);
        return Err(error.into());
    }
    if let Err(error) = fs::rename(&tmp, dest) {
        let _ = fs::remove_file(&tmp);
        return Err(
            if is_locked(&error) || (error.kind() == ErrorKind::PermissionDenied && dest.exists()) {
                locked(dest)
            } else {
                error.into()
            },
        );
    }
    Ok(())
}

/// El diálogo «Guardar como» deja escribir cualquier nombre: el libro siempre termina en `.xlsx`.
pub fn with_xlsx_extension(path: PathBuf) -> PathBuf {
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("xlsx"))
    {
        return path;
    }
    let mut raw = path.into_os_string();
    raw.push(".xlsx");
    PathBuf::from(raw)
}

/// Archivos exportados en esta sesión: lo único que «Abrir archivo» y «Abrir carpeta» aceptan
/// (la ventana no tiene acceso genérico al disco, spec §9).
#[derive(Default)]
pub struct RecentExports {
    paths: Mutex<HashSet<PathBuf>>,
}

impl RecentExports {
    fn lock(&self) -> MutexGuard<'_, HashSet<PathBuf>> {
        self.paths.lock().unwrap_or_else(|p| p.into_inner())
    }

    pub fn remember(&self, path: &Path) {
        self.lock().insert(path.to_path_buf());
    }

    pub fn resolve(&self, raw: &str) -> AppResult<PathBuf> {
        let path = PathBuf::from(raw);
        if !self.lock().contains(&path) {
            return Err(validation(
                "Solo se pueden abrir archivos exportados en esta sesión.",
            ));
        }
        if !path.is_file() {
            return Err(AppError::NotFound(
                "No se encontró el archivo exportado. Puede que se haya movido o eliminado.".into(),
            ));
        }
        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::storage::TempDir;

    fn book(text: &str) -> Workbook {
        let mut workbook = Workbook::new();
        workbook.add_worksheet().write_string(0, 0, text).unwrap();
        workbook
    }

    fn files_in(dir: &Path) -> Vec<String> {
        let mut names: Vec<_> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn writes_and_replaces_without_leftovers() {
        let tmp = TempDir::new();
        let dest = tmp.path().join("matriz.xlsx");
        save_atomic(&mut book("uno"), &dest).unwrap();
        save_atomic(&mut book("dos"), &dest).unwrap();
        assert_eq!(files_in(tmp.path()), ["matriz.xlsx"]);
        assert_eq!(
            crate::excel::testing::Book::at(&dest).text("Sheet1", 0, 0),
            "dos"
        );
    }

    #[cfg(windows)]
    #[test]
    fn a_file_open_in_excel_is_reported_and_left_intact() {
        use std::os::windows::fs::OpenOptionsExt;

        let tmp = TempDir::new();
        let dest = tmp.path().join("matriz.xlsx");
        save_atomic(&mut book("original"), &dest).unwrap();
        let before = fs::read(&dest).unwrap();
        // Excel abre el libro sin compartir escritura: se simula con share_mode(0).
        let _excel = OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(&dest)
            .unwrap();
        let error = save_atomic(&mut book("nuevo"), &dest).unwrap_err();
        assert_eq!(error.code(), "file_locked");
        assert_eq!(
            error.user_message(),
            "Cierra el archivo en Excel y vuelve a intentar."
        );
        drop(_excel);
        assert_eq!(fs::read(&dest).unwrap(), before);
        assert_eq!(files_in(tmp.path()), ["matriz.xlsx"]);
    }

    #[test]
    fn read_only_files_are_not_overwritten() {
        let tmp = TempDir::new();
        let dest = tmp.path().join("matriz.xlsx");
        save_atomic(&mut book("original"), &dest).unwrap();
        let mut permissions = fs::metadata(&dest).unwrap().permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&dest, permissions.clone()).unwrap();
        let error = save_atomic(&mut book("nuevo"), &dest).unwrap_err();
        assert_eq!(
            error.to_string(),
            "No se puede sobrescribir el archivo: es de solo lectura o no tienes permisos."
        );
        #[allow(clippy::permissions_set_readonly_false)]
        permissions.set_readonly(false);
        fs::set_permissions(&dest, permissions).unwrap();
    }

    #[test]
    fn a_missing_folder_fails_without_leaving_files() {
        let tmp = TempDir::new();
        let dest = tmp.path().join("no-existe").join("matriz.xlsx");
        assert!(save_atomic(&mut book("x"), &dest).is_err());
        assert!(files_in(tmp.path()).is_empty());
    }

    #[test]
    fn the_extension_is_always_xlsx() {
        let p = |s: &str| PathBuf::from(s);
        assert_eq!(
            with_xlsx_extension(p("C:\\a\\matriz.xlsx")),
            p("C:\\a\\matriz.xlsx")
        );
        assert_eq!(
            with_xlsx_extension(p("C:\\a\\matriz.XLSX")),
            p("C:\\a\\matriz.XLSX")
        );
        assert_eq!(
            with_xlsx_extension(p("C:\\a\\matriz")),
            p("C:\\a\\matriz.xlsx")
        );
        assert_eq!(with_xlsx_extension(p("C:\\a\\v1.0")), p("C:\\a\\v1.0.xlsx"));
    }

    #[test]
    fn only_files_exported_in_this_session_can_be_opened() {
        let tmp = TempDir::new();
        let dest = tmp.path().join("matriz.xlsx");
        let recent = RecentExports::default();
        let raw = dest.to_string_lossy().into_owned();
        assert_eq!(
            recent.resolve(&raw).unwrap_err().to_string(),
            "Solo se pueden abrir archivos exportados en esta sesión."
        );
        recent.remember(&dest);
        assert_eq!(recent.resolve(&raw).unwrap_err().code(), "not_found");
        save_atomic(&mut book("x"), &dest).unwrap();
        assert_eq!(recent.resolve(&raw).unwrap(), dest);
        assert!(recent.resolve("C:\\Windows\\notepad.exe").is_err());
    }
}
