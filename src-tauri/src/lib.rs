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
mod state;
mod system;
mod transfer;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            system::tray::show_main(app);
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
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
            system::daily_backup::spawn(app.handle().clone());
            system::tray::setup_tray(app)?;
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
                api.prevent_close();
                let _ = window.hide();
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
        ])
        .run(tauri::generate_context!())
        .expect("no se pudo iniciar Qapture");
}
