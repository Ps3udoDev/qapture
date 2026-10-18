use super::{runtime, shortcuts::ShortcutAction, updater};
use tauri::{
    Manager,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

#[derive(Debug, PartialEq, Eq)]
pub enum TrayAction {
    Show,
    Hide,
    Quit,
    Capture,
    Paste,
    NewCase,
    CheckUpdate,
}

fn action_for(id: &str) -> Option<TrayAction> {
    match id {
        "open" => Some(TrayAction::Show),
        "hide" => Some(TrayAction::Hide),
        "quit" => Some(TrayAction::Quit),
        "capture" => Some(TrayAction::Capture),
        "paste" => Some(TrayAction::Paste),
        "new-case" => Some(TrayAction::NewCase),
        "check-update" => Some(TrayAction::CheckUpdate),
        _ => None,
    }
}

pub fn show_main(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn hide_main(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToggleDecision {
    Show,
    Hide,
}

/// `is_visible` también es cierto minimizada o tapada por otras ventanas: solo se oculta la
/// ventana que la persona tiene delante; en cualquier otro caso se trae al frente.
pub fn toggle_decision(visible: bool, focused: bool, minimized: bool) -> ToggleDecision {
    if visible && focused && !minimized {
        ToggleDecision::Hide
    } else {
        ToggleDecision::Show
    }
}

/// Atajo «Mostrar/ocultar».
pub fn toggle_main(app: &tauri::AppHandle) {
    toggle(app, false);
}

/// Clic en el icono: el propio clic le quita el foco a la ventana (pasa a la barra de tareas),
/// así que ahí el foco no cuenta y se trata como si lo tuviera.
fn toggle_from_tray(app: &tauri::AppHandle) {
    toggle(app, true);
}

fn toggle(app: &tauri::AppHandle, ignore_focus: bool) {
    if let Some(window) = app.get_webview_window("main") {
        let decision = toggle_decision(
            window.is_visible().unwrap_or(false),
            ignore_focus || window.is_focused().unwrap_or(false),
            window.is_minimized().unwrap_or(false),
        );
        match decision {
            ToggleDecision::Hide => hide_main(app),
            ToggleDecision::Show => show_main(app),
        }
    }
}

/// Punto único de despacho del menú; captura, pegar y nuevo caso usan el despacho de los atajos.
pub fn on_action(app: &tauri::AppHandle, action: TrayAction) {
    match action {
        TrayAction::Show => show_main(app),
        TrayAction::Hide => hide_main(app),
        TrayAction::Quit => app.exit(0),
        TrayAction::Capture => runtime::dispatch(app, ShortcutAction::Capture),
        TrayAction::Paste => runtime::dispatch(app, ShortcutAction::Paste),
        TrayAction::NewCase => runtime::dispatch(app, ShortcutAction::NewCase),
        TrayAction::CheckUpdate => updater::check_from_tray(app),
    }
}

/// Ítem «Buscar actualizaciones»: con una versión pendiente pasa a «Instalar actualización <v>».
struct UpdateMenuItem(MenuItem<tauri::Wry>);

pub fn set_update_label(app: &tauri::AppHandle, text: &str) {
    if let Some(item) = app.try_state::<UpdateMenuItem>()
        && let Err(error) = item.0.set_text(text)
    {
        eprintln!("No se pudo cambiar el menú de la bandeja: {error}");
    }
}

pub fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Abrir Qapture", true, None::<&str>)?;
    let hide = MenuItem::with_id(app, "hide", "Ocultar", true, None::<&str>)?;
    let first_separator = PredefinedMenuItem::separator(app)?;
    let capture = MenuItem::with_id(app, "capture", "Capturar pantalla", true, None::<&str>)?;
    let paste = MenuItem::with_id(app, "paste", "Pegar evidencia", true, None::<&str>)?;
    let new_case = MenuItem::with_id(app, "new-case", "Nuevo caso", true, None::<&str>)?;
    let check_update = MenuItem::with_id(
        app,
        "check-update",
        "Buscar actualizaciones",
        true,
        None::<&str>,
    )?;
    let second_separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Salir", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &open,
            &hide,
            &first_separator,
            &capture,
            &paste,
            &new_case,
            &second_separator,
            &check_update,
            &quit,
        ],
    )?;
    let tray = TrayIconBuilder::with_id("main-tray")
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("Qapture · Teams4Soft")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            if let Some(action) = action_for(event.id.as_ref()) {
                on_action(app, action);
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_from_tray(tray.app_handle());
            }
        })
        .build(app)?;
    app.manage(tray);
    app.manage(UpdateMenuItem(check_update));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{ToggleDecision, TrayAction, action_for, toggle_decision};

    #[test]
    fn toggle_hides_only_the_window_in_front_of_the_user() {
        assert_eq!(toggle_decision(true, true, false), ToggleDecision::Hide);
        for (visible, focused, minimized) in [
            (false, false, false),
            (true, false, false),
            (true, true, true),
            (true, false, true),
            (false, true, false),
        ] {
            assert_eq!(
                toggle_decision(visible, focused, minimized),
                ToggleDecision::Show,
                "visible={visible} focused={focused} minimized={minimized}"
            );
        }
    }

    #[test]
    fn menu_ids_map_to_the_expected_actions() {
        assert_eq!(action_for("open"), Some(TrayAction::Show));
        assert_eq!(action_for("hide"), Some(TrayAction::Hide));
        assert_eq!(action_for("quit"), Some(TrayAction::Quit));
        assert_eq!(action_for("capture"), Some(TrayAction::Capture));
        assert_eq!(action_for("paste"), Some(TrayAction::Paste));
        assert_eq!(action_for("new-case"), Some(TrayAction::NewCase));
        assert_eq!(action_for("check-update"), Some(TrayAction::CheckUpdate));
    }

    #[test]
    fn unknown_menu_ids_have_no_action() {
        for id in ["", "unknown", "Open", "exit"] {
            assert_eq!(action_for(id), None);
        }
    }
}
