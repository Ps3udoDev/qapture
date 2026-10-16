use tauri::{AppHandle, State, Window, ipc::Response};

use crate::{
    capture::{
        crop::Selection,
        screen::XcapScreen,
        windows::{TauriHost, overlay_index},
    },
    error::{AppResult, validation},
    state::AppState,
};

/// El monitor sale del label de la ventana que llama, nunca de un argumento: un overlay solo
/// puede leer y recortar su propio monitor.
fn caller(window: &Window) -> AppResult<usize> {
    overlay_index(window.label())
        .ok_or_else(|| validation("Solo el overlay de captura puede usar este comando."))
}

/// UC-11. F6 lo invoca también desde el atajo global.
#[tauri::command(async)]
pub fn start_capture(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    state.capture().start(&TauriHost { app: &app }, &XcapScreen)
}

#[tauri::command(async)]
pub fn capture_frame(window: Window, state: State<'_, AppState>) -> AppResult<Response> {
    Ok(Response::new(state.capture().frame(caller(&window)?)?))
}

#[tauri::command(async)]
pub fn overlay_ready(app: AppHandle, window: Window, state: State<'_, AppState>) -> AppResult<()> {
    state
        .capture()
        .overlay_ready(&TauriHost { app: &app }, caller(&window)?)
}

#[tauri::command(async)]
pub fn finish_capture(
    app: AppHandle,
    window: Window,
    state: State<'_, AppState>,
    selection: Option<Selection>,
) -> AppResult<()> {
    let index = caller(&window)?;
    state
        .capture()
        .finish(&TauriHost { app: &app }, state.files(), index, selection)
        .map(|_| ())
}

#[tauri::command(async)]
pub fn cancel_capture(app: AppHandle, window: Window, state: State<'_, AppState>) -> AppResult<()> {
    caller(&window)?;
    state.capture().cancel(&TauriHost { app: &app });
    Ok(())
}
