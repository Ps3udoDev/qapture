use tauri::State;

use crate::{
    dto::{CaseDto, CaseInput},
    error::AppResult,
    repo::{cases, evidence},
    state::AppState,
};

#[tauri::command(async)]
pub fn list_cases(state: State<'_, AppState>, project_id: String) -> AppResult<Vec<CaseDto>> {
    cases::list(&state.db(), &project_id)
}

#[tauri::command(async)]
pub fn create_case(
    state: State<'_, AppState>,
    project_id: String,
    input: CaseInput,
) -> AppResult<CaseDto> {
    cases::create(&state.db(), &project_id, &input)
}

#[tauri::command(async)]
pub fn update_case(
    state: State<'_, AppState>,
    case_id: String,
    input: CaseInput,
) -> AppResult<CaseDto> {
    cases::update(&state.db(), &case_id, &input)
}

#[tauri::command(async)]
pub fn duplicate_case(state: State<'_, AppState>, case_id: String) -> AppResult<CaseDto> {
    cases::duplicate(&state.db(), &case_id)
}

#[tauri::command(async)]
pub fn delete_case(state: State<'_, AppState>, case_id: String) -> AppResult<()> {
    evidence::delete_case(&state.db(), state.files(), &case_id)
}
