use std::path::Path;

use serde::Deserialize;
use tauri::{AppHandle, State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

use crate::{
    dto::{SettingsDto, ShortcutFailureDto},
    error::{AppError, AppResult, validation},
    repo,
    settings::{self, Settings},
    state::AppState,
    system::shortcuts,
};

const RELEASES_URL: &str = "https://github.com/Ps3udoDev/qapture/releases";
const DOCS_URL: &str = "https://github.com/Ps3udoDev/qapture#readme";
const IMAGE_WIDTH: std::ops::RangeInclusive<u32> = 400..=1600;

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LinkTarget {
    Releases,
    Docs,
}

impl LinkTarget {
    fn url(self) -> &'static str {
        match self {
            Self::Releases => RELEASES_URL,
            Self::Docs => DOCS_URL,
        }
    }
}

fn opener_error(error: tauri_plugin_opener::Error) -> AppError {
    AppError::Io(std::io::Error::other(error.to_string()))
}

/// `repo::settings::load` no comprueba rangos: se ajusta el ancho de imagen al rango válido y,
/// si algún texto sigue sin validar, el perfil vuelve a sus valores por defecto. Así la interfaz
/// nunca recibe datos fuera de rango. Unos atajos guardados inválidos vuelven a los de por
/// defecto, así nunca se registran en el SO.
pub(crate) fn sanitized(mut loaded: Settings) -> Settings {
    loaded.export.image_max_width = loaded
        .export
        .image_max_width
        .clamp(*IMAGE_WIDTH.start(), *IMAGE_WIDTH.end());
    loaded.shortcuts = shortcuts::validate(&loaded.shortcuts).unwrap_or_default();
    match settings::validate(&loaded) {
        Ok(valid) => valid,
        Err(_) => Settings {
            profile: Default::default(),
            ..loaded
        },
    }
}

fn dto(state: &AppState, settings: Settings, app_version: &str) -> SettingsDto {
    SettingsDto {
        settings,
        shortcut_failures: state.shortcut_failures(),
        app_version: app_version.to_string(),
        data_dir: state
            .db_path()
            .parent()
            .map(|dir| dir.to_string_lossy().into_owned())
            .unwrap_or_default(),
    }
}

/// Guarda los ajustes sin Tauri para poder probarla: valida, aplica los atajos (con rollback
/// si el SO rechaza alguno) y solo entonces persiste, actualiza la copia en memoria y avisa.
pub(crate) fn save(state: &AppState, input: Settings, app_version: &str) -> AppResult<SettingsDto> {
    state.ensure_ready()?;
    let mut wanted = settings::validate(&input)?;
    let folder = &wanted.export.default_folder;
    if !folder.is_empty() && !Path::new(folder).is_dir() {
        return Err(validation("La carpeta de exportación no existe."));
    }
    wanted.shortcuts = shortcuts::validate(&wanted.shortcuts)?;

    let current = state.settings();
    shortcuts::apply(
        &mut **state.registrar(),
        &current.shortcuts,
        &wanted.shortcuts,
    )?;
    let saved = repo::settings::save(&state.db(), &wanted);
    if let Err(error) = saved {
        // Mejor esfuerzo: dejar el SO como estaba, igual que lo guardado.
        let _ = shortcuts::apply(
            &mut **state.registrar(),
            &wanted.shortcuts,
            &current.shortcuts,
        );
        return Err(error);
    }
    state.set_settings(wanted.clone());
    // Un fallo de arranque solo sigue vigente si su combinación no cambió.
    let still_failing: Vec<ShortcutFailureDto> = state
        .shortcut_failures()
        .into_iter()
        .filter(|f| f.action.accel(&wanted.shortcuts) == f.accelerator)
        .collect();
    state.set_shortcut_failures(still_failing);
    state.notify_saved(&wanted);
    Ok(dto(state, wanted, app_version))
}

#[tauri::command(async)]
pub fn get_settings(app: AppHandle, state: State<'_, AppState>) -> AppResult<SettingsDto> {
    let loaded = repo::settings::load(&state.db())?;
    let version = app.package_info().version.to_string();
    Ok(dto(&state, sanitized(loaded), &version))
}

#[tauri::command(async)]
pub fn save_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: Settings,
) -> AppResult<SettingsDto> {
    save(&state, settings, &app.package_info().version.to_string())
}

/// `None`: el usuario cerró el diálogo sin elegir carpeta.
#[tauri::command(async)]
pub fn pick_export_folder(window: WebviewWindow) -> AppResult<Option<String>> {
    Ok(window
        .dialog()
        .file()
        .set_parent(&window)
        .set_title("Seleccionar carpeta de exportación")
        .blocking_pick_folder()
        .and_then(|path| path.into_path().ok())
        .map(|path| path.to_string_lossy().into_owned()))
}

#[tauri::command(async)]
pub fn open_data_folder(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    let dir = state.db_path().parent().unwrap_or(state.db_path());
    app.opener()
        .open_path(dir.to_string_lossy(), None::<&str>)
        .map_err(opener_error)
}

#[tauri::command]
pub fn hide_main_window(window: WebviewWindow) -> AppResult<()> {
    window
        .hide()
        .map_err(|error| AppError::Io(std::io::Error::other(error.to_string())))
}

#[tauri::command(async)]
pub fn open_link(app: AppHandle, target: LinkTarget) -> AppResult<()> {
    app.opener()
        .open_url(target.url(), None::<&str>)
        .map_err(opener_error)
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeSet,
        sync::{Arc, Mutex},
    };

    use super::*;
    use crate::{db, media::storage::TempDir, system::shortcuts::ShortcutRegistrar};

    #[derive(Clone, Default)]
    struct FakeRegistrar {
        registered: Arc<Mutex<BTreeSet<String>>>,
        refuse: Arc<Mutex<BTreeSet<String>>>,
    }

    impl ShortcutRegistrar for FakeRegistrar {
        fn register(&mut self, accel: &str) -> Result<(), String> {
            if self.refuse.lock().unwrap().contains(accel) {
                return Err("ocupada".into());
            }
            self.registered.lock().unwrap().insert(accel.to_string());
            Ok(())
        }
        fn unregister(&mut self, accel: &str) {
            self.registered.lock().unwrap().remove(accel);
        }
    }

    fn state_with(recovery: Option<String>) -> (TempDir, AppState, FakeRegistrar) {
        let tmp = TempDir::new();
        let conn = db::open(&tmp.path().join(db::DB_FILE)).unwrap();
        let state = AppState::new(conn, tmp.files(), tmp.path(), recovery);
        let fake = FakeRegistrar::default();
        let mut boot = fake.clone();
        shortcuts::register_all(&mut boot, &state.settings().shortcuts);
        state.install_registrar(Box::new(fake.clone()));
        (tmp, state, fake)
    }

    fn with(f: impl FnOnce(&mut Settings)) -> Settings {
        let mut s = Settings::default();
        f(&mut s);
        s
    }

    #[test]
    fn save_persists_validated_settings_and_applies_shortcuts() {
        let (_tmp, state, fake) = state_with(None);
        let dto = save(
            &state,
            with(|s| {
                s.profile.tester = "  Ana ".into();
                s.shortcuts.toggle = "ctrl+shift+f12".into();
            }),
            "0.5.0",
        )
        .unwrap();
        assert_eq!(dto.settings.profile.tester, "Ana");
        assert_eq!(dto.settings.shortcuts.toggle, "Ctrl+Shift+F12");
        assert_eq!(dto.app_version, "0.5.0");
        assert_eq!(repo::settings::load(&state.db()).unwrap(), dto.settings);
        assert_eq!(state.settings(), dto.settings);
        let registered = fake.registered.lock().unwrap();
        assert!(registered.contains("Ctrl+Shift+F12"));
        assert!(!registered.contains("Ctrl+Alt+Q"));
    }

    #[test]
    fn a_refused_shortcut_saves_nothing() {
        let (_tmp, state, fake) = state_with(None);
        fake.refuse.lock().unwrap().insert("Ctrl+Shift+F12".into());
        let error = save(
            &state,
            with(|s| {
                s.profile.tester = "Ana".into();
                s.shortcuts.toggle = "Ctrl+Shift+F12".into();
            }),
            "0.5.0",
        )
        .unwrap_err();
        assert_eq!(
            error.user_message(),
            "No se pudo registrar Ctrl+Shift+F12 para «Mostrar/ocultar»: otra aplicación ya la usa. Se conservan los atajos anteriores."
        );
        assert_eq!(
            repo::settings::load(&state.db()).unwrap(),
            Settings::default()
        );
        assert_eq!(state.settings(), Settings::default());
        assert!(fake.registered.lock().unwrap().contains("Ctrl+Alt+Q"));
    }

    #[test]
    fn a_missing_export_folder_is_rejected() {
        let (tmp, state, _fake) = state_with(None);
        let error = save(
            &state,
            with(|s| s.export.default_folder = "C:\\no\\existe".into()),
            "0.5.0",
        )
        .unwrap_err();
        assert_eq!(error.user_message(), "La carpeta de exportación no existe.");
        let existing = tmp.path().to_string_lossy().into_owned();
        let ok = save(
            &state,
            with(|s| s.export.default_folder = existing),
            "0.5.0",
        );
        assert!(ok.is_ok(), "una carpeta existente se acepta");
    }

    #[test]
    fn saving_is_blocked_in_recovery_mode() {
        let (_tmp, state, _fake) = state_with(Some("base dañada".into()));
        let error = save(&state, Settings::default(), "0.5.0").unwrap_err();
        assert_eq!(
            error.user_message(),
            "Qapture está en modo recuperación: restaura un respaldo o empieza con una base vacía."
        );
    }

    #[test]
    fn saving_notifies_the_hook_and_drops_stale_failures() {
        let (_tmp, state, _fake) = state_with(None);
        state.set_shortcut_failures(vec![
            ShortcutFailureDto {
                action: shortcuts::ShortcutAction::Capture,
                accelerator: "Ctrl+Alt+S".into(),
            },
            ShortcutFailureDto {
                action: shortcuts::ShortcutAction::Paste,
                accelerator: "Ctrl+Alt+V".into(),
            },
        ]);
        let seen = Arc::new(Mutex::new(None));
        let sink = seen.clone();
        state.set_on_saved(Box::new(move |s| {
            *sink.lock().unwrap() = Some(s.shortcuts.paste.clone());
        }));
        let dto = save(
            &state,
            with(|s| s.shortcuts.paste = "Ctrl+Alt+P".into()),
            "0.5.0",
        )
        .unwrap();
        assert_eq!(seen.lock().unwrap().as_deref(), Some("Ctrl+Alt+P"));
        assert_eq!(dto.shortcut_failures.len(), 1);
        assert_eq!(dto.shortcut_failures[0].accelerator, "Ctrl+Alt+S");
    }

    #[test]
    fn link_target_and_action_wire_names() {
        for (raw, ok) in [
            ("releases", true),
            ("docs", true),
            ("other", false),
            ("Docs", false),
        ] {
            assert_eq!(
                serde_json::from_value::<LinkTarget>(serde_json::json!(raw)).is_ok(),
                ok,
                "{raw}"
            );
        }
        assert_eq!(
            serde_json::to_value(shortcuts::ShortcutAction::NewCase).unwrap(),
            serde_json::json!("newCase")
        );
    }
    #[test]
    fn loaded_settings_are_clamped_into_range() {
        let loaded = with(|s| {
            s.export.image_max_width = 5000;
            s.profile.tester = "a".repeat(200);
        });
        let fixed = sanitized(loaded);
        assert_eq!(fixed.export.image_max_width, 1600);
        assert_eq!(fixed.profile.tester, "");
    }

    #[test]
    fn an_invalid_stored_shortcut_set_falls_back_to_defaults() {
        let fixed = sanitized(with(|s| {
            s.shortcuts.capture = "Ctrl+Alt+Q".into();
            s.profile.tester = "Ana".into();
        }));
        assert_eq!(fixed.shortcuts, settings::Shortcuts::default());
        assert_eq!(fixed.profile.tester, "Ana");
        let fixed = sanitized(with(|s| s.shortcuts.toggle = "alt+ctrl+p".into()));
        assert_eq!(
            fixed.shortcuts.toggle, "Ctrl+Alt+P",
            "uno válido solo se normaliza"
        );
    }
}
