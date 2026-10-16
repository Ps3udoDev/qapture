use tauri::State;

use crate::{
    dto::{DocVersionDto, DocVersionInput},
    error::AppResult,
    repo::doc_versions,
    state::AppState,
};

#[tauri::command(async)]
pub fn list_doc_versions(
    state: State<'_, AppState>,
    project_id: String,
) -> AppResult<Vec<DocVersionDto>> {
    doc_versions::list(&state.db(), &project_id)
}

#[tauri::command(async)]
pub fn create_doc_version(
    state: State<'_, AppState>,
    project_id: String,
    input: DocVersionInput,
) -> AppResult<DocVersionDto> {
    doc_versions::create(&state.db(), &project_id, &input)
}

#[tauri::command(async)]
pub fn update_doc_version(
    state: State<'_, AppState>,
    id: String,
    input: DocVersionInput,
) -> AppResult<DocVersionDto> {
    doc_versions::update(&state.db(), &id, &input)
}

#[tauri::command(async)]
pub fn delete_doc_version(state: State<'_, AppState>, id: String) -> AppResult<()> {
    doc_versions::delete(&state.db(), &id)
}
