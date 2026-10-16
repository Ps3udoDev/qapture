use tauri::State;

use crate::{dto::TemplateDto, error::AppResult, repo::templates, state::AppState};

#[tauri::command(async)]
pub fn list_templates(state: State<'_, AppState>) -> AppResult<Vec<TemplateDto>> {
    templates::list(&state.db())
}

#[tauri::command(async)]
pub fn save_template_from_project(
    state: State<'_, AppState>,
    project_id: String,
    name: String,
    test_type: String,
) -> AppResult<TemplateDto> {
    templates::save_from_project(&state.db(), &project_id, &name, &test_type)
}

#[tauri::command(async)]
pub fn delete_template(state: State<'_, AppState>, id: String) -> AppResult<()> {
    templates::delete(&state.db(), &id)
}
