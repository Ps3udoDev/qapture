//! Actualizaciones (F7): revisión manual y periódica contra el `latest.json` del último release.
//! La versión pospuesta («Después») la guarda el front; aquí solo se busca, se avisa y se instala.

use std::{thread, time::Duration};

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::UpdaterExt;

use crate::{
    dto::{UpdateInfoDto, UpdateProgressDto},
    error::{AppResult, validation},
    state::AppState,
    system::{runtime, tray},
};

/// Payload `UpdateInfoDto`: el front abre el diálogo (si la versión no se pospuso).
pub const AVAILABLE_EVENT: &str = "update:available";
/// Payload `UpdateInfoDto`: clic en la bandeja; el front abre el diálogo aunque se haya pospuesto.
pub const SHOW_EVENT: &str = "update:show";
/// Payload `UpdateProgressDto` mientras se descarga.
pub const PROGRESS_EVENT: &str = "update:progress";

const CONNECTION_FAILED: &str = "No se pudo buscar actualizaciones. Revisa tu conexión a internet.";
const BAD_SIGNATURE: &str =
    "La actualización descargada no pasó la verificación de firma y no se instaló.";
const PORTABLE: &str = "Estás usando la versión portable: descarga el ZIP nuevo y reemplaza Qapture.exe. Tus datos en .\\data se conservan.";

const FIRST_CHECK_AFTER: Duration = Duration::from_secs(15);
const CHECK_EVERY: Duration = Duration::from_secs(6 * 60 * 60);

#[derive(Debug, Clone, PartialEq)]
pub enum CheckOutcome {
    UpToDate,
    Available(UpdateInfoDto),
    Failed(String),
}

// Hoy solo la usan los tests: la versión pospuesta vive en el front (Task 4), que aplica la misma regla.
#[cfg_attr(not(test), allow(dead_code))]
fn parse_version(version: &str) -> Option<[u64; 3]> {
    let mut parts = version.trim().trim_start_matches('v').split('.');
    let mut out = [0; 3];
    for slot in &mut out {
        *slot = parts.next()?.parse().ok()?;
    }
    parts.next().is_none().then_some(out)
}

/// `true` si no hay versión pospuesta o si `found` es más nueva (X.Y.Z numérico, no texto).
#[cfg_attr(not(test), allow(dead_code))]
pub fn should_prompt(found: &str, dismissed: Option<&str>) -> bool {
    let Some(dismissed) = dismissed else {
        return true;
    };
    match (parse_version(found), parse_version(dismissed)) {
        (Some(found), Some(dismissed)) => found > dismissed,
        _ => found != dismissed,
    }
}

/// Cualquier error al consultar (sin red, GitHub sin `latest.json`…) es un fallo, nunca una
/// actualización; el detalle técnico solo va al log.
pub fn check_outcome(result: Result<Option<UpdateInfoDto>, String>) -> CheckOutcome {
    match result {
        Ok(Some(info)) => CheckOutcome::Available(info),
        Ok(None) => CheckOutcome::UpToDate,
        Err(_) => CheckOutcome::Failed(CONNECTION_FAILED.into()),
    }
}

/// Texto del ítem de la bandeja con una versión pendiente (en portable no se instala: se descarga).
pub fn tray_label(version: &str, portable: bool) -> String {
    if portable {
        format!("Descargar actualización {version}")
    } else {
        format!("Instalar actualización {version}")
    }
}

/// Mensaje de un error al descargar o instalar.
pub fn error_message(detail: &str) -> String {
    let detail_lower = detail.to_lowercase();
    if detail_lower.contains("signature") || detail_lower.contains("minisign") {
        BAD_SIGNATURE.into()
    } else {
        format!(
            "No se pudo instalar la actualización: {}.",
            detail.trim_end_matches('.')
        )
    }
}

fn up_to_date_message(app: &AppHandle) -> String {
    format!(
        "Qapture está al día (versión {}).",
        app.package_info().version
    )
}

fn info_of(update: &tauri_plugin_updater::Update, portable: bool) -> UpdateInfoDto {
    UpdateInfoDto {
        version: update.version.clone(),
        current_version: update.current_version.clone(),
        notes: update.body.clone().unwrap_or_default(),
        // El texto tal cual viene en `latest.json` (RFC 3339).
        date: update.raw_json["pub_date"]
            .as_str()
            .map(str::to_string)
            .or_else(|| update.date.map(|d| d.to_string())),
        portable,
    }
}

async fn fetch(app: &AppHandle) -> Result<Option<UpdateInfoDto>, String> {
    let portable = app.state::<AppState>().is_portable();
    let update = app
        .updater()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| e.to_string())?;
    Ok(update.map(|u| info_of(&u, portable)))
}

/// Revisa y, si hay una versión nueva, la deja pendiente (estado y texto de la bandeja).
async fn check(app: &AppHandle) -> CheckOutcome {
    let result = fetch(app).await;
    if let Err(detail) = &result {
        eprintln!("No se pudo buscar actualizaciones: {detail}");
    }
    let outcome = check_outcome(result);
    if let CheckOutcome::Available(info) = &outcome {
        app.state::<AppState>()
            .set_pending_update(Some(info.clone()));
        tray::set_update_label(app, &tray_label(&info.version, info.portable));
    }
    outcome
}

fn announce(app: &AppHandle, info: UpdateInfoDto) {
    let _ = app.emit_to("main", AVAILABLE_EVENT, info);
}

/// Pedido explícito (bandeja): el diálogo se abre aunque la versión se haya pospuesto.
fn show(app: &AppHandle, info: UpdateInfoDto) {
    let _ = app.emit_to("main", SHOW_EVENT, info);
}

/// Revisión periódica: 15 s después de arrancar y luego cada 6 h (la app puede quedar días en la
/// bandeja). Un fallo solo queda en el log. En debug no arranca, para no consultar GitHub.
pub fn spawn(app: AppHandle) {
    if cfg!(debug_assertions) {
        return;
    }
    thread::spawn(move || {
        thread::sleep(FIRST_CHECK_AFTER);
        loop {
            if let CheckOutcome::Available(info) = tauri::async_runtime::block_on(check(&app)) {
                announce(&app, info);
            }
            thread::sleep(CHECK_EVERY);
        }
    });
}

/// Ítem de la bandeja: con una versión pendiente abre el diálogo (aunque se haya pospuesto); si no,
/// revisa y avisa del resultado.
pub fn check_from_tray(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let info = match app.state::<AppState>().pending_update() {
            Some(info) => info,
            None => match check(&app).await {
                CheckOutcome::Available(info) => info,
                CheckOutcome::UpToDate => {
                    return runtime::notify(&app, &up_to_date_message(&app));
                }
                CheckOutcome::Failed(message) => return runtime::notify(&app, &message),
            },
        };
        tray::show_main(&app);
        show(&app, info);
    });
}

/// Revisión manual (Ajustes › Acerca de): los errores de red llevan el mensaje fijo.
pub async fn check_now(app: &AppHandle) -> AppResult<Option<UpdateInfoDto>> {
    match check(app).await {
        CheckOutcome::Available(info) => Ok(Some(info)),
        CheckOutcome::UpToDate => Ok(None),
        CheckOutcome::Failed(message) => Err(validation(message)),
    }
}

/// Descarga (con progreso), instala y reinicia. En portable no se instala nada.
pub async fn install(app: &AppHandle) -> AppResult<()> {
    if app.state::<AppState>().is_portable() {
        return Err(validation(PORTABLE));
    }
    let update = app
        .updater()
        .map_err(|e| validation(error_message(&e.to_string())))?
        .check()
        .await
        .map_err(|e| {
            eprintln!("No se pudo buscar actualizaciones: {e}");
            validation(CONNECTION_FAILED)
        })?
        .ok_or_else(|| validation(up_to_date_message(app)))?;
    let mut downloaded: u64 = 0;
    update
        .download_and_install(
            |chunk, total| {
                downloaded += chunk as u64;
                let progress = UpdateProgressDto { downloaded, total };
                let _ = app.emit_to("main", PROGRESS_EVENT, progress);
            },
            || {
                if let Some(runtime) = app.try_state::<crate::mcp::McpRuntime>() {
                    runtime.disconnect_all();
                }
            },
        )
        .await
        .map_err(|e| {
            eprintln!("No se pudo instalar la actualización: {e}");
            validation(error_message(&e.to_string()))
        })?;
    app.restart()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(version: &str) -> UpdateInfoDto {
        UpdateInfoDto {
            version: version.into(),
            current_version: "1.0.0".into(),
            notes: String::new(),
            date: None,
            portable: false,
        }
    }

    #[test]
    fn a_dismissed_version_is_not_prompted_again_but_a_newer_one_is() {
        assert!(should_prompt("1.0.1", None));
        assert!(!should_prompt("1.0.1", Some("1.0.1")));
        assert!(should_prompt("1.0.2", Some("1.0.1")));
        assert!(should_prompt("1.0.10", Some("1.0.9")), "numérico, no texto");
        assert!(!should_prompt("1.0.9", Some("1.0.10")), "una más vieja no");
        assert!(should_prompt("2.0.0", Some("1.9.9")));
    }

    #[test]
    fn offline_is_a_failure_not_an_update() {
        assert_eq!(
            check_outcome(Err("error sending request".into())),
            CheckOutcome::Failed(
                "No se pudo buscar actualizaciones. Revisa tu conexión a internet.".into()
            )
        );
        assert_eq!(check_outcome(Ok(None)), CheckOutcome::UpToDate);
        assert_eq!(
            check_outcome(Ok(Some(info("1.0.1")))),
            CheckOutcome::Available(info("1.0.1"))
        );
    }

    #[test]
    fn a_bad_signature_has_its_own_message() {
        assert_eq!(
            error_message("signature verification failed"),
            "La actualización descargada no pasó la verificación de firma y no se instaló."
        );
        assert_eq!(
            error_message("The signature verification failed"),
            "La actualización descargada no pasó la verificación de firma y no se instaló.",
            "texto real de minisign-verify"
        );
        assert_eq!(
            error_message("minisign: invalid encoding in public key"),
            "La actualización descargada no pasó la verificación de firma y no se instaló.",
            "errores de minisign aunque no digan «signature»"
        );
        assert_eq!(
            error_message("disk full"),
            "No se pudo instalar la actualización: disk full."
        );
    }

    #[test]
    fn the_tray_label_says_download_in_portable() {
        assert_eq!(tray_label("1.0.1", false), "Instalar actualización 1.0.1");
        assert_eq!(tray_label("1.0.1", true), "Descargar actualización 1.0.1");
    }
}
