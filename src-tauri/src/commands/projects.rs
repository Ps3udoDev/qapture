use tauri::State;

use crate::{
    dto::{FieldDto, FieldInput, NewProjectInput, ProjectDto, ProjectMetaInput, ProjectSummaryDto},
    error::AppResult,
    repo::{fields, projects},
    state::AppState,
};

#[tauri::command(async)]
pub fn list_projects(state: State<'_, AppState>) -> AppResult<Vec<ProjectSummaryDto>> {
    projects::list(&state.db())
}

#[tauri::command(async)]
pub fn get_project(state: State<'_, AppState>, id: String) -> AppResult<ProjectDto> {
    projects::get(&state.db(), &id)
}

#[tauri::command(async)]
pub fn create_project(state: State<'_, AppState>, input: NewProjectInput) -> AppResult<ProjectDto> {
    projects::create(&state.db(), input)
}

#[tauri::command(async)]
pub fn update_project(
    state: State<'_, AppState>,
    id: String,
    input: ProjectMetaInput,
) -> AppResult<ProjectDto> {
    projects::update_meta(&state.db(), &id, &input)
}

#[tauri::command(async)]
pub fn delete_project(state: State<'_, AppState>, id: String) -> AppResult<()> {
    projects::delete(&state.db(), &id)
}

#[tauri::command(async)]
pub fn save_project_fields(
    state: State<'_, AppState>,
    project_id: String,
    fields: Vec<FieldInput>,
) -> AppResult<Vec<FieldDto>> {
    fields::save(&state.db(), &project_id, fields)
}
