use tauri::AppHandle;

use crate::{dto::UpdateInfoDto, error::AppResult, system::updater};

/// Revisión manual: `None` si Qapture está al día.
#[tauri::command]
pub async fn check_update(app: AppHandle) -> AppResult<Option<UpdateInfoDto>> {
    updater::check_now(&app).await
}

/// Descarga con `update:progress`, instala y reinicia (en portable responde con el aviso).
#[tauri::command]
pub async fn install_update(app: AppHandle) -> AppResult<()> {
    updater::install(&app).await
}
