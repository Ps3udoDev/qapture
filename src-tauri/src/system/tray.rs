use tauri::{
    Manager,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

#[derive(Debug, PartialEq, Eq)]
enum TrayAction {
    Show,
    Hide,
    Quit,
}

fn action_for(id: &str) -> Option<TrayAction> {
    match id {
        "open" => Some(TrayAction::Show),
        "hide" => Some(TrayAction::Hide),
        "quit" => Some(TrayAction::Quit),
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

pub fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Abrir Qapture", true, None::<&str>)?;
    let hide = MenuItem::with_id(app, "hide", "Ocultar", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Salir", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &hide, &separator, &quit])?;
    let tray = TrayIconBuilder::with_id("main-tray")
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("Qapture · Teams4Soft")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match action_for(event.id.as_ref()) {
            Some(TrayAction::Show) => show_main(app),
            Some(TrayAction::Hide) => hide_main(app),
            Some(TrayAction::Quit) => app.exit(0),
            None => {}
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
    }

    #[test]
    fn unknown_menu_ids_have_no_action() {
        for id in ["", "unknown", "Open", "exit"] {
            assert_eq!(action_for(id), None);
        }
    }
}
