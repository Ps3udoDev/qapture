//! Modo portable: con `portable.flag` junto al exe, los datos viven en `<exe>/data`.

use std::{
    fs,
    path::{Path, PathBuf},
};

pub const PORTABLE_FLAG: &str = "portable.flag";
const DATA_FOLDER: &str = "data";
const PROBE_FILE: &str = ".qapture-write-test";

/// Mismo identificador que `app.path().app_data_dir()` (tauri.conf.json).
/// Verificación manual: comparar ambas rutas en una instalación normal.
pub fn default_app_data_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    let base = std::env::var_os("APPDATA").map(PathBuf::from);
    #[cfg(unix)]
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")));
    base.map(|base| base.join("com.teams4soft.qapture"))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataLocation {
    pub dir: PathBuf,
    pub portable: bool,
}

pub fn resolve_data_dir(exe_dir: &Path, app_data: &Path) -> Result<DataLocation, String> {
    if exe_dir.join(PORTABLE_FLAG).is_file() {
        Ok(DataLocation {
            dir: exe_dir.join(DATA_FOLDER),
            portable: true,
        })
    } else {
        Ok(DataLocation {
            dir: app_data.to_path_buf(),
            portable: false,
        })
    }
}

/// Crea la carpeta y comprueba que se puede escribir; sin alternativa silenciosa a otra ruta.
pub fn ensure_writable(dir: &Path) -> Result<(), String> {
    let probe = dir.join(PROBE_FILE);
    let result = fs::create_dir_all(dir)
        .and_then(|_| fs::write(&probe, b"ok"))
        .and_then(|_| fs::remove_file(&probe));
    result.map_err(|_| {
        format!(
            "No se puede escribir en {}. Copia Qapture a una carpeta con permisos de escritura (no en Archivos de programa).",
            dir.display()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::storage::TempDir;

    #[test]
    fn default_app_data_dir_uses_the_identifier() {
        assert!(
            default_app_data_dir()
                .unwrap()
                .ends_with("com.teams4soft.qapture")
        );
    }

    #[test]
    fn without_the_flag_data_stays_in_app_data() {
        let tmp = TempDir::new();
        let exe = tmp.path().join("exe");
        fs::create_dir_all(&exe).unwrap();
        let app_data = tmp.path().join("appdata");
        assert_eq!(
            resolve_data_dir(&exe, &app_data).unwrap(),
            DataLocation {
                dir: app_data,
                portable: false
            }
        );
    }

    #[test]
    fn with_the_flag_data_goes_next_to_the_exe() {
        let tmp = TempDir::new();
        let exe = tmp.path().join("exe");
        fs::create_dir_all(&exe).unwrap();
        fs::write(exe.join(PORTABLE_FLAG), b"").unwrap();
        assert_eq!(
            resolve_data_dir(&exe, &tmp.path().join("appdata")).unwrap(),
            DataLocation {
                dir: exe.join("data"),
                portable: true
            }
        );
    }

    #[test]
    fn a_read_only_folder_is_an_error_not_a_fallback() {
        let tmp = TempDir::new();
        let file = tmp.path().join("archivo.txt");
        fs::write(&file, b"x").unwrap();
        let dir = file.join("data");
        assert_eq!(
            ensure_writable(&dir).unwrap_err(),
            format!(
                "No se puede escribir en {}. Copia Qapture a una carpeta con permisos de escritura (no en Archivos de programa).",
                dir.display()
            )
        );
        let ok = tmp.path().join("ok").join("data");
        assert!(ensure_writable(&ok).is_ok());
        assert!(
            !ok.join(PROBE_FILE).exists(),
            "el archivo de prueba se borra"
        );
    }
}
