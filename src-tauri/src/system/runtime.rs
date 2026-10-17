//! Arranque y segundo plano: despacho de atajos y menú, inicio oculto y cierre.

use std::str::FromStr;

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_autostart::ManagerExt as _;
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Modifiers, Shortcut};
use tauri_plugin_notification::NotificationExt;

use crate::{
    capture::{
        screen::XcapScreen,
        windows::{CAPTURED_EVENT, TauriHost},
    },
    dto::CapturedEventDto,
    settings::SystemPrefs,
    state::AppState,
    system::{
        clipboard_paste::{self, NO_IMAGE, PasteOutcome},
        shortcuts::{ShortcutAction, ShortcutRegistrar},
        tray,
    },
};

/// Evento sin payload hacia `main`: el front abre el alta de un caso.
pub const NEW_CASE_EVENT: &str = "shortcut:new-case";
/// Argumento con el que el inicio con Windows lanza la app.
pub const HIDDEN_ARG: &str = "--hidden";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseAction {
    Hide,
    Exit,
}

pub fn should_start_hidden(args: &[String], prefs: &SystemPrefs) -> bool {
    prefs.start_hidden && args.iter().any(|a| a == HIDDEN_ARG)
}

pub fn close_action(prefs: &SystemPrefs) -> CloseAction {
    if prefs.close_to_tray {
        CloseAction::Hide
    } else {
        CloseAction::Exit
    }
}

/// Forma canónica (`Ctrl+Alt+Win+Q`) → formato del plugin (`Win` se llama `Super`).
pub fn to_plugin_accel(canonical: &str) -> String {
    canonical
        .split('+')
        .map(|part| if part == "Win" { "Super" } else { part })
        .collect::<Vec<_>>()
        .join("+")
}

/// Atajo pulsado → forma canónica, para buscarlo con `shortcuts::action_for`.
pub fn canonical_of(shortcut: &Shortcut) -> String {
    let mut out = String::new();
    for (flag, name) in [
        (Modifiers::CONTROL, "Ctrl"),
        (Modifiers::ALT, "Alt"),
        (Modifiers::SHIFT, "Shift"),
        (Modifiers::SUPER, "Win"),
    ] {
        if shortcut.mods.contains(flag) {
            out.push_str(name);
            out.push('+');
        }
    }
    let key = shortcut.key.to_string();
    let key = key
        .strip_prefix("Key")
        .or_else(|| key.strip_prefix("Digit"))
        .unwrap_or(&key);
    out.push_str(key);
    out
}

/// Registra en el SO a través de `tauri-plugin-global-shortcut`.
pub struct TauriRegistrar {
    pub app: AppHandle,
}

impl ShortcutRegistrar for TauriRegistrar {
    fn register(&mut self, accel: &str) -> Result<(), String> {
        let shortcut = Shortcut::from_str(&to_plugin_accel(accel)).map_err(|e| e.to_string())?;
        self.app
            .global_shortcut()
            .register(shortcut)
            .map_err(|e| e.to_string())
    }

    fn unregister(&mut self, accel: &str) {
        if let Ok(shortcut) = Shortcut::from_str(&to_plugin_accel(accel)) {
            let _ = self.app.global_shortcut().unregister(shortcut);
        }
    }
}

/// Activa o desactiva el inicio con Windows; un error solo queda en el log.
pub fn sync_autostart(app: &AppHandle, enabled: bool) {
    let manager = app.autolaunch();
    if manager.is_enabled().is_ok_and(|current| current == enabled) {
        return;
    }
    let result = if enabled {
        manager.enable()
    } else {
        manager.disable()
    };
    if let Err(error) = result {
        eprintln!("No se pudo actualizar el inicio con Windows: {error}");
    }
}

fn notify(app: &AppHandle, body: &str) {
    if let Err(error) = app
        .notification()
        .builder()
        .title("Qapture")
        .body(body)
        .show()
    {
        eprintln!("No se pudo mostrar la notificación: {error}");
    }
}

fn capture(app: &AppHandle) {
    let state = app.state::<AppState>();
    if let Err(error) = state.capture().start(&TauriHost { app }, &XcapScreen) {
        notify(app, &error.user_message());
    }
}

fn paste(app: &AppHandle) {
    let image = app
        .clipboard()
        .read_image()
        .ok()
        .map(|img| (img.width(), img.height(), img.rgba().to_vec()));
    let state = app.state::<AppState>();
    match clipboard_paste::stage_from_clipboard(state.files(), image) {
        PasteOutcome::Staged(staged) => {
            tray::show_main(app);
            let payload = CapturedEventDto {
                tmp_path: staged.path,
            };
            let _ = app.emit_to("main", CAPTURED_EVENT, payload);
        }
        PasteOutcome::NoImage => notify(app, NO_IMAGE),
        PasteOutcome::Unusable(message) => notify(app, &message),
    }
}

/// Punto único de despacho de atajos y menú de bandeja. Corre en el hilo principal: captura y
/// pegar (que esperan, leen el portapapeles y escriben en disco) van a un hilo aparte.
pub fn dispatch(app: &AppHandle, action: ShortcutAction) {
    match action {
        ShortcutAction::Toggle => tray::toggle_main(app),
        ShortcutAction::Capture => {
            let app = app.clone();
            std::thread::spawn(move || capture(&app));
        }
        ShortcutAction::Paste => {
            let app = app.clone();
            std::thread::spawn(move || paste(&app));
        }
        ShortcutAction::NewCase => {
            tray::show_main(app);
            let _ = app.emit_to("main", NEW_CASE_EVENT, ());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    fn prefs(start_hidden: bool, close_to_tray: bool) -> SystemPrefs {
        SystemPrefs {
            start_hidden,
            close_to_tray,
            ..SystemPrefs::default()
        }
    }

    #[test]
    fn starts_hidden_only_when_launched_by_autostart_with_the_option_on() {
        let on = prefs(true, true);
        assert!(should_start_hidden(
            &args(&["qapture.exe", "--hidden"]),
            &on
        ));
        assert!(!should_start_hidden(&args(&["qapture.exe"]), &on));
        assert!(!should_start_hidden(
            &args(&["qapture.exe", "--hidden"]),
            &prefs(false, true)
        ));
    }

    #[test]
    fn closing_hides_or_exits_according_to_the_setting() {
        assert_eq!(close_action(&prefs(true, true)), CloseAction::Hide);
        assert_eq!(close_action(&prefs(true, false)), CloseAction::Exit);
    }

    #[test]
    fn canonical_accelerators_convert_to_the_plugin_format() {
        for canonical in ["Ctrl+Alt+Q", "Alt+Win+1", "Ctrl+Shift+F12"] {
            let plugin = to_plugin_accel(canonical);
            assert!(
                Shortcut::from_str(&plugin).is_ok(),
                "{canonical} -> {plugin}"
            );
        }
        assert_eq!(to_plugin_accel("Alt+Win+1"), "Alt+Super+1");
    }

    #[test]
    fn a_pressed_shortcut_maps_back_to_its_canonical_form() {
        for canonical in ["Ctrl+Alt+Q", "Alt+Win+1", "Ctrl+Shift+F12", "Win+Q"] {
            let shortcut = Shortcut::from_str(&to_plugin_accel(canonical)).unwrap();
            assert_eq!(canonical_of(&shortcut), canonical);
        }
    }
}
