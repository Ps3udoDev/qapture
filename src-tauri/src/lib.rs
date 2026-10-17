mod backup;
mod capture;
mod commands;
mod db;
mod domain;
mod dto;
mod error;
mod excel;
mod media;
mod repo;
mod settings;
mod state;
mod system;
mod transfer;

use tauri::Manager;
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_global_shortcut::ShortcutState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            system::tray::show_main(app);
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state != ShortcutState::Pressed {
                        return;
                    }
                    // Solo la copia en memoria de los ajustes: ni la base ni el registrador.
                    let current = app.state::<state::AppState>().settings().shortcuts;
                    let pressed = system::runtime::canonical_of(shortcut);
                    if let Some(action) = system::shortcuts::action_for(&pressed, &current) {
                        system::runtime::dispatch(app, action);
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![system::runtime::HIDDEN_ARG]),
        ))
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let files = media::storage::EvidenceFiles::new(&data_dir);
            let state = match db::recovery::open_or_recover(&data_dir.join(db::DB_FILE))? {
                db::recovery::Startup::Ready(conn) => {
                    // Restos de sesiones anteriores: capturas sin guardar y archivos sin fila en la db.
                    if let Err(error) = files.clear_staging() {
                        eprintln!("No se pudo limpiar evidence/_staging: {error}");
                    }
                    if let Err(error) = repo::evidence::sweep_orphans(&conn, &files) {
                        eprintln!("No se pudieron limpiar evidencias huérfanas: {error}");
                    }
                    state::AppState::new(conn, files, &data_dir, None)
                }
                db::recovery::Startup::Recovery(detail) => {
                    // Nada de limpiezas ni respaldos: la base vacía en memoria no refleja los datos reales.
                    eprintln!("Base de datos dañada, se abre el modo recuperación: {detail}");
                    state::AppState::new(db::open_in_memory()?, files, &data_dir, Some(detail))
                }
            };
            app.manage(state);
            let handle = app.handle().clone();
            let state = app.state::<state::AppState>();
            // Registrador real y ajustes saneados: `reload_settings` deja la copia en memoria
            // antes de registrar (en recuperación, los valores por defecto de la base en memoria);
            // lo que el SO rechace queda para el aviso del front.
            state.install_registrar(Box::new(system::runtime::TauriRegistrar {
                app: handle.clone(),
            }));
            let loaded = system::runtime::reload_settings(&state);
            let args: Vec<String> = std::env::args().collect();
            let plan =
                system::runtime::startup_plan(state.recovery().is_some(), &args, &loaded.system);
            // Inicio con Windows, al arrancar (salvo en recuperación) y cada vez que se guarda.
            if plan.sync_autostart {
                system::runtime::sync_autostart(&handle, loaded.system.autostart);
            }
            let hook_app = handle.clone();
            state.set_on_saved(Box::new(move |saved| {
                system::runtime::sync_autostart(&hook_app, saved.system.autostart);
            }));
            system::daily_backup::spawn(handle.clone());
            system::tray::setup_tray(app)?;
            // La ventana arranca oculta (tauri.conf.json); se muestra salvo inicio oculto.
            if plan.show {
                system::tray::show_main(&handle);
            }
            // Con la ventana oculta nadie vería el toast del front: aviso nativo por cada atajo caído.
            for message in
                system::runtime::startup_notifications(&state.shortcut_failures(), !plan.show)
            {
                system::runtime::notify(&handle, &message);
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if capture::windows::overlay_index(window.label()).is_some() {
                // Un overlay cerrado por fuera (Alt+F4, WebView caído) cancela toda la captura.
                if let tauri::WindowEvent::Destroyed = event {
                    let app = window.app_handle();
                    app.state::<state::AppState>()
                        .capture()
                        .overlay_closed(&capture::windows::TauriHost { app });
                }
                return;
            }
            if window.label() != "main" {
                return;
            }
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let app = window.app_handle();
                let prefs = app.state::<state::AppState>().settings().system;
                api.prevent_close();
                match system::runtime::close_action(&prefs) {
                    system::runtime::CloseAction::Hide => {
                        let _ = window.hide();
                    }
                    system::runtime::CloseAction::Exit => app.exit(0),
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::templates::list_templates,
            commands::templates::save_template_from_project,
            commands::templates::delete_template,
            commands::projects::list_projects,
            commands::projects::get_project,
            commands::projects::create_project,
            commands::projects::update_project,
            commands::projects::delete_project,
            commands::projects::save_project_fields,
            commands::cases::list_cases,
            commands::cases::create_case,
            commands::cases::update_case,
            commands::cases::duplicate_case,
            commands::cases::delete_case,
            commands::evidence::stage_evidence_image,
            commands::evidence::discard_staged_image,
            commands::evidence::read_evidence_image,
            commands::evidence::list_evidence,
            commands::evidence::add_evidence,
            commands::evidence::update_evidence,
            commands::evidence::update_evidence_description,
            commands::evidence::move_evidence,
            commands::evidence::reorder_evidence,
            commands::evidence::delete_evidence,
            commands::capture::start_capture,
            commands::capture::capture_frame,
            commands::capture::overlay_ready,
            commands::capture::finish_capture,
            commands::capture::cancel_capture,
            commands::doc_versions::list_doc_versions,
            commands::doc_versions::create_doc_version,
            commands::doc_versions::update_doc_version,
            commands::doc_versions::delete_doc_version,
            commands::export::export_project,
            commands::export::export_template,
            commands::export::open_export,
            commands::export::reveal_export,
            commands::data::get_startup_status,
            commands::data::import_legacy,
            commands::data::import_package,
            commands::data::export_package,
            commands::data::list_backups,
            commands::data::create_backup,
            commands::data::restore_backup,
            commands::data::recover_database,
            commands::settings::get_settings,
            commands::settings::save_settings,
            commands::settings::pick_export_folder,
            commands::settings::open_data_folder,
            commands::settings::hide_main_window,
            commands::settings::open_link,
        ])
        .run(tauri::generate_context!())
        .expect("no se pudo iniciar Qapture");
}
