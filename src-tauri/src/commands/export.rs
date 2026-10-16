use std::path::PathBuf;

use tauri::{AppHandle, Emitter, State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

use crate::{
    dto::{ExportOptionsInput, ExportProgressDto, ExportResultDto},
    error::{AppError, AppResult},
    excel::{
        self,
        data::{ExportData, resolve_options, suggested_file_name},
        save::with_xlsx_extension,
    },
    state::AppState,
};

pub const EXPORT_PROGRESS_EVENT: &str = "export:progress";

/// `None`: el usuario cerró «Guardar como» sin elegir archivo.
fn run(
    app: &AppHandle,
    window: &WebviewWindow,
    state: &AppState,
    project_id: &str,
    options: &ExportOptionsInput,
    template: bool,
) -> AppResult<Option<ExportResultDto>> {
    let today = chrono::Local::now().date_naive();
    // La base se lee de una vez y se suelta antes del diálogo y de generar el libro.
    let data = ExportData::load(&state.db(), state.files(), project_id)?;
    let settings = resolve_options(&data, options, template, today)?;
    let name = suggested_file_name(&data.project, &settings);
    let Some(dest) = ask_destination(window, &name) else {
        return Ok(None);
    };
    let progress = |p: ExportProgressDto| {
        let _ = app.emit_to("main", EXPORT_PROGRESS_EVENT, p);
    };
    let result = excel::export(&data, &settings, &dest, &progress)?;
    state.exports().remember(&dest);
    Ok(Some(result))
}

fn ask_destination(window: &WebviewWindow, name: &str) -> Option<PathBuf> {
    let path = window
        .dialog()
        .file()
        .set_parent(window)
        .set_title("Guardar Excel")
        .set_file_name(name)
        .add_filter("Libro de Excel", &["xlsx"])
        .blocking_save_file()?
        .into_path()
        .ok()?;
    Some(with_xlsx_extension(path))
}

fn opener_error(error: tauri_plugin_opener::Error) -> AppError {
    AppError::Io(std::io::Error::other(error.to_string()))
}

#[tauri::command(async)]
pub fn export_project(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, AppState>,
    project_id: String,
    options: ExportOptionsInput,
) -> AppResult<Option<ExportResultDto>> {
    run(&app, &window, &state, &project_id, &options, false)
}

#[tauri::command(async)]
pub fn export_template(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, AppState>,
    project_id: String,
    options: ExportOptionsInput,
) -> AppResult<Option<ExportResultDto>> {
    run(&app, &window, &state, &project_id, &options, true)
}

#[tauri::command(async)]
pub fn open_export(app: AppHandle, state: State<'_, AppState>, path: String) -> AppResult<()> {
    let path = state.exports().resolve(&path)?;
    app.opener()
        .open_path(path.to_string_lossy(), None::<&str>)
        .map_err(opener_error)
}

#[tauri::command(async)]
pub fn reveal_export(app: AppHandle, state: State<'_, AppState>, path: String) -> AppResult<()> {
    let path = state.exports().resolve(&path)?;
    app.opener().reveal_item_in_dir(path).map_err(opener_error)
}
