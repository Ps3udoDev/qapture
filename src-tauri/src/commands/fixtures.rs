use tauri::{AppHandle, State, ipc::Request};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_opener::OpenerExt;

use super::frame;
use crate::{
    commands::settings::opener_error,
    dto::{FixtureDto, NewFixtureMeta},
    error::{AppError, AppResult},
    repo,
    state::AppState,
};

#[tauri::command(async)]
pub fn fixture_add(state: State<'_, AppState>, request: Request<'_>) -> AppResult<FixtureDto> {
    state.ensure_ready()?;
    let (meta, bytes) = frame::split::<NewFixtureMeta>(frame::raw(&request)?)?;
    state.with_conn(|conn| {
        repo::fixtures::add(
            conn,
            state.files(),
            &meta.case_id,
            &meta.file_name,
            &meta.note,
            bytes,
        )
    })
}

#[tauri::command(async)]
pub fn fixture_list(state: State<'_, AppState>, case_id: String) -> AppResult<Vec<FixtureDto>> {
    state.ensure_ready()?;
    state.with_conn(|conn| repo::fixtures::list(conn, &case_id))
}

#[tauri::command(async)]
pub fn fixture_remove(state: State<'_, AppState>, id: String) -> AppResult<()> {
    state.ensure_ready()?;
    state.with_conn(|conn| repo::fixtures::remove(conn, state.files(), &id))
}

#[tauri::command(async)]
pub fn fixture_reveal(app: AppHandle, state: State<'_, AppState>, id: String) -> AppResult<()> {
    state.ensure_ready()?;
    let path = state.with_conn(|conn| repo::fixtures::absolute(conn, state.files(), &id))?;
    app.opener().reveal_item_in_dir(path).map_err(opener_error)
}

#[tauri::command(async)]
pub fn fixture_copy_path(app: AppHandle, state: State<'_, AppState>, id: String) -> AppResult<()> {
    state.ensure_ready()?;
    let path = state.with_conn(|conn| repo::fixtures::absolute(conn, state.files(), &id))?;
    app.clipboard()
        .write_text(path.to_string_lossy())
        .map_err(|e| AppError::Io(std::io::Error::other(e.to_string())))
}
