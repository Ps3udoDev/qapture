use std::{fs, path::PathBuf};

use chrono::{Local, NaiveDateTime};
use tauri::{AppHandle, State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;

use crate::{
    db::recovery,
    dto::{
        BackupDto, ImportSummaryDto, PackageExportResultDto, RestoreResultDto, StartupStatusDto,
    },
    error::{AppResult, validation},
    state::AppState,
    system::runtime,
    transfer::{legacy, package},
};

fn now() -> NaiveDateTime {
    Local::now().naive_local()
}

/// `None`: el usuario cerró el diálogo sin elegir archivo.
fn pick_file(
    window: &WebviewWindow,
    title: &str,
    filter: &str,
    extension: &str,
) -> Option<PathBuf> {
    window
        .dialog()
        .file()
        .set_parent(window)
        .set_title(title)
        .add_filter(filter, &[extension])
        .blocking_pick_file()?
        .into_path()
        .ok()
}

#[tauri::command(async)]
pub fn get_startup_status(state: State<'_, AppState>) -> AppResult<StartupStatusDto> {
    let detail = state.recovery();
    Ok(StartupStatusDto {
        recovery: detail.is_some(),
        detail: detail.unwrap_or_default(),
    })
}

#[tauri::command(async)]
pub fn import_legacy(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> AppResult<Option<ImportSummaryDto>> {
    state.ensure_ready()?;
    let Some(path) = pick_file(
        &window,
        "Importar Bitácora v1",
        "Respaldo de la Bitácora v1",
        "json",
    ) else {
        return Ok(None);
    };
    if fs::metadata(&path)?.len() > legacy::MAX_FILE_BYTES {
        return Err(validation("El archivo supera 200 MB."));
    }
    let bytes = fs::read(&path)?;
    legacy::import(&state.db(), state.files(), &bytes).map(Some)
}

#[tauri::command(async)]
pub fn import_package(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> AppResult<Option<ImportSummaryDto>> {
    state.ensure_ready()?;
    let Some(path) = pick_file(
        &window,
        "Importar paquete",
        "Paquete de Qapture",
        package::EXTENSION,
    ) else {
        return Ok(None);
    };
    package::import(&state.db(), state.files(), &path).map(Some)
}

#[tauri::command(async)]
pub fn export_package(
    window: WebviewWindow,
    state: State<'_, AppState>,
    project_id: String,
) -> AppResult<Option<PackageExportResultDto>> {
    state.ensure_ready()?;
    // La base se lee de una vez y se suelta antes del diálogo y de escribir el ZIP.
    let export = package::collect(&state.db(), state.files(), &project_id)?;
    let name = package::suggested_file_name(
        &export.doc.project.client,
        &export.doc.project.name,
        Local::now().date_naive(),
    );
    let Some(dest) = window
        .dialog()
        .file()
        .set_parent(&window)
        .set_title("Exportar paquete")
        .set_file_name(&name)
        .add_filter("Paquete de Qapture", &[package::EXTENSION])
        .blocking_save_file()
        .and_then(|p| p.into_path().ok())
    else {
        return Ok(None);
    };
    let dest = package::with_package_extension(dest);
    package::write(&export, &dest)?;
    // «Abrir carpeta» reutiliza `reveal_export`, que solo acepta rutas exportadas en la sesión.
    state.exports().remember(&dest);
    Ok(Some(PackageExportResultDto {
        path: dest.to_string_lossy().into_owned(),
        file_name: dest
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        cases: export.doc.cases.len() as i64,
        evidence: export.evidence_count(),
        missing_images: export.missing_images,
    }))
}

/// También en modo recuperación: la pantalla de recuperación lista los respaldos.
#[tauri::command(async)]
pub fn list_backups(state: State<'_, AppState>) -> AppResult<Vec<BackupDto>> {
    state.backups().list()
}

#[tauri::command(async)]
pub fn create_backup(state: State<'_, AppState>) -> AppResult<BackupDto> {
    state.ensure_ready()?;
    state.backups().create_manual(&state.db(), now())
}

#[tauri::command(async)]
pub fn restore_backup(
    app: AppHandle,
    state: State<'_, AppState>,
    file_name: String,
) -> AppResult<RestoreResultDto> {
    state.ensure_ready()?;
    let result = {
        let mut db = state.db();
        state
            .backups()
            .restore(&mut db, state.files(), &file_name, now())?
    };
    settings_changed_underneath(&app, &state);
    Ok(result)
}

/// La base cambió por debajo (restaurar o salir de recuperación): ajustes, atajos e inicio con
/// Windows pasan a reflejar la base nueva. Se llama sin el candado de la base.
fn settings_changed_underneath(app: &AppHandle, state: &AppState) {
    let loaded = runtime::reload_settings(state);
    runtime::sync_autostart(app, loaded.system.autostart);
}

/// Sale del modo recuperación: `fileName` restaura ese respaldo; `null` empieza con una base vacía.
#[tauri::command(async)]
pub fn recover_database(
    app: AppHandle,
    state: State<'_, AppState>,
    file_name: Option<String>,
) -> AppResult<()> {
    if state.recovery().is_none() {
        return Err(validation("Qapture no está en modo recuperación."));
    }
    let conn = recovery::recover(
        state.db_path(),
        state.backups(),
        state.files(),
        file_name.as_deref(),
        now(),
    )?;
    state.finish_recovery(conn);
    settings_changed_underneath(&app, &state);
    Ok(())
}
