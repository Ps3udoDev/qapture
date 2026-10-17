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
}

fn action_for(id: &str) -> Option<TrayAction> {
    match id {
        "open" => Some(TrayAction::Show),
        "hide" => Some(TrayAction::Hide),
        "quit" => Some(TrayAction::Quit),
        "capture" => Some(TrayAction::Capture),
        "paste" => Some(TrayAction::Paste),
        "new-case" => Some(TrayAction::NewCase),
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

fn toggle_main(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) {
            hide_main(app);
        } else {
            show_main(app);
        }
    }
}

/// Punto único de despacho del menú; la Task 5 conecta captura, pegar y nuevo caso.
pub fn on_action(app: &tauri::AppHandle, action: TrayAction) {
    match action {
        TrayAction::Show => show_main(app),
        TrayAction::Hide => hide_main(app),
        TrayAction::Quit => app.exit(0),
        // TODO(F6-T5): conectar con captura, portapapeles y nuevo caso.
        TrayAction::Capture | TrayAction::Paste | TrayAction::NewCase => show_main(app),
    }
}

pub fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Abrir Qapture", true, None::<&str>)?;
    let hide = MenuItem::with_id(app, "hide", "Ocultar", true, None::<&str>)?;
    let first_separator = PredefinedMenuItem::separator(app)?;
    let capture = MenuItem::with_id(app, "capture", "Capturar pantalla", true, None::<&str>)?;
    let paste = MenuItem::with_id(app, "paste", "Pegar evidencia", true, None::<&str>)?;
    let new_case = MenuItem::with_id(app, "new-case", "Nuevo caso", true, None::<&str>)?;
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
                toggle_main(tray.app_handle());
            }
        })
        .build(app)?;
    app.manage(tray);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{TrayAction, action_for};

    #[test]
    fn menu_ids_map_to_the_expected_actions() {
        assert_eq!(action_for("open"), Some(TrayAction::Show));
        assert_eq!(action_for("hide"), Some(TrayAction::Hide));
        assert_eq!(action_for("quit"), Some(TrayAction::Quit));
        assert_eq!(action_for("capture"), Some(TrayAction::Capture));
        assert_eq!(action_for("paste"), Some(TrayAction::Paste));
        assert_eq!(action_for("new-case"), Some(TrayAction::NewCase));
    }

    #[test]
    fn unknown_menu_ids_have_no_action() {
        for id in ["", "unknown", "Open", "exit"] {
            assert_eq!(action_for(id), None);
        }
    }
}
