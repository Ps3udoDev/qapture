use std::fs;

use tauri::{
    State,
    ipc::{Request, Response},
};

use super::frame;
use crate::{
    dto::{EvidenceDto, EvidenceUpdateMeta, NewEvidenceMeta, StagedImageDto},
    error::AppResult,
    repo::evidence,
    state::AppState,
};

#[tauri::command(async)]
pub fn stage_evidence_image(
    state: State<'_, AppState>,
    request: Request<'_>,
) -> AppResult<StagedImageDto> {
    evidence::stage(state.files(), frame::raw(&request)?)
}

#[tauri::command(async)]
pub fn discard_staged_image(state: State<'_, AppState>, path: String) -> AppResult<()> {
    state.files().discard_staged(&path)
}

#[tauri::command(async)]
pub fn read_evidence_image(state: State<'_, AppState>, path: String) -> AppResult<Response> {
    let path = state.files().resolve_inside(&path, false)?;
    Ok(Response::new(fs::read(path)?))
}

#[tauri::command(async)]
pub fn list_evidence(state: State<'_, AppState>, case_id: String) -> AppResult<Vec<EvidenceDto>> {
    evidence::list(&state.db(), state.files(), &case_id)
}

#[tauri::command(async)]
pub fn add_evidence(state: State<'_, AppState>, request: Request<'_>) -> AppResult<EvidenceDto> {
    let (meta, render) = frame::split::<NewEvidenceMeta>(frame::raw(&request)?)?;
    evidence::add(&state.db(), state.files(), &meta, render)
}

#[tauri::command(async)]
pub fn update_evidence(state: State<'_, AppState>, request: Request<'_>) -> AppResult<EvidenceDto> {
    let (meta, render) = frame::split::<EvidenceUpdateMeta>(frame::raw(&request)?)?;
    evidence::update(&state.db(), state.files(), &meta, render)
}

#[tauri::command(async)]
pub fn update_evidence_description(
    state: State<'_, AppState>,
    evidence_id: String,
    description: String,
) -> AppResult<EvidenceDto> {
    evidence::update_description(&state.db(), state.files(), &evidence_id, &description)
}

#[tauri::command(async)]
pub fn move_evidence(
    state: State<'_, AppState>,
    evidence_id: String,
    target_case_id: String,
) -> AppResult<EvidenceDto> {
    evidence::move_to_case(&state.db(), state.files(), &evidence_id, &target_case_id)
}

#[tauri::command(async)]
pub fn reorder_evidence(
    state: State<'_, AppState>,
    case_id: String,
    ordered_ids: Vec<String>,
) -> AppResult<Vec<EvidenceDto>> {
    evidence::reorder(&state.db(), state.files(), &case_id, &ordered_ids)
}

#[tauri::command(async)]
pub fn delete_evidence(state: State<'_, AppState>, evidence_id: String) -> AppResult<()> {
    evidence::delete(&state.db(), state.files(), &evidence_id)
}
