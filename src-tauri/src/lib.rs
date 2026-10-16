mod commands;
mod db;
mod domain;
mod dto;
mod error;
mod repo;
mod state;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let conn = db::open(&data_dir.join("qapture.db"))?;
            app.manage(state::AppState::new(conn));
            Ok(())
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
        ])
        .run(tauri::generate_context!())
        .expect("no se pudo iniciar Qapture");
}
