use image::ImageEncoder;
use serde_json::{Map, Value, json};

use super::{
    ToolContext, ToolOutput,
    read::{evidence_metadata, required},
    write::attach_png,
};

pub fn list_windows(
    ctx: &ToolContext<'_>,
    _args: &Map<String, Value>,
) -> Result<ToolOutput, String> {
    Ok(ToolOutput::Json(json!(
        ctx.host.screen().windows().map_err(|e| e.to_string())?
    )))
}

pub fn capture_screen(
    ctx: &ToolContext<'_>,
    args: &Map<String, Value>,
) -> Result<ToolOutput, String> {
    let case_id = required(args, "caseId")?;
    // Validate the destination before requesting permission or touching the screen.
    crate::repo::cases::get(&ctx.state.db(), case_id).map_err(|e| e.to_string())?;
    let target = args.get("target").ok_or("Falta «target».")?;
    enum Target {
        Monitor(usize),
        Window(u32),
    }
    let (target, label) = if target.as_str() == Some("primary") {
        (Target::Monitor(0), "monitor 0".to_string())
    } else if let Some(object) = target.as_object() {
        if object.len() != 1 {
            return Err("«target» debe indicar un monitor o una ventana.".into());
        }
        if let Some(monitor) = object.get("monitor") {
            let index = monitor
                .as_u64()
                .and_then(|n| usize::try_from(n).ok())
                .ok_or("«monitor» debe ser un entero no negativo.")?;
            (Target::Monitor(index), format!("monitor {index}"))
        } else if let Some(window) = object.get("window") {
            let search = window
                .as_str()
                .filter(|s| !s.trim().is_empty())
                .ok_or("«window» debe ser texto no vacío.")?;
            let needle = search.to_lowercase();
            let window = ctx
                .host
                .screen()
                .windows()
                .map_err(|e| e.to_string())?
                .into_iter()
                .find(|window| window.title.to_lowercase().contains(&needle))
                .ok_or_else(|| {
                    format!("No hay ninguna ventana cuyo título contenga «{search}».")
                })?;
            (
                Target::Window(window.id),
                format!("la ventana «{}»", window.title),
            )
        } else {
            return Err("«target» debe indicar un monitor o una ventana.".into());
        }
    } else {
        return Err("«target» debe ser «primary», un monitor o una ventana.".into());
    };
    if let Target::Monitor(index) = target {
        let count = ctx
            .host
            .screen()
            .monitor_count()
            .map_err(|e| e.to_string())?;
        if index >= count {
            return Err(format!("No existe el monitor {index}. Hay {count}."));
        }
    }
    if ctx.state.settings().agents.confirm_capture
        && !ctx.host.confirm(
            "Captura de pantalla",
            &format!("Un agente quiere capturar {label}. ¿Permitir?"),
        )
    {
        return Err("El tester rechazó la captura.".into());
    }
    let image = match target {
        Target::Monitor(index) => ctx
            .host
            .screen()
            .capture_monitor(index)
            .map_err(|e| e.to_string())?,
        Target::Window(id) => ctx
            .host
            .screen()
            .capture_window(id)
            .map_err(|e| e.to_string())?,
    };
    ctx.host.notify("Un agente capturó la pantalla.");
    let mut png = Vec::new();
    image::codecs::png::PngEncoder::new(&mut png)
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|_| "No se pudo codificar la captura.".to_string())?;
    let description = args
        .get("description")
        .map(|_| required(args, "description"))
        .transpose()?
        .unwrap_or("Captura de pantalla");
    let evidence = attach_png(ctx, case_id, &png, description)?;
    Ok(ToolOutput::Json(evidence_metadata(&evidence)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        mcp::host::RecordingHost,
        mcp::tools::read::tests::{output, state},
        media::storage::TempDir,
        repo::test_support,
    };

    struct TargetedHost {
        host: RecordingHost,
        captures: std::sync::Mutex<Vec<usize>>,
    }

    impl crate::mcp::host::AgentScreen for TargetedHost {
        fn monitors(&self) -> crate::error::AppResult<Vec<image::RgbaImage>> {
            panic!("capture must only access the selected monitor")
        }
        fn monitor_count(&self) -> crate::error::AppResult<usize> {
            Ok(2)
        }
        fn capture_monitor(&self, index: usize) -> crate::error::AppResult<image::RgbaImage> {
            self.captures.lock().unwrap().push(index);
            Ok(image::RgbaImage::from_pixel(
                4,
                4,
                image::Rgba([index as u8, 0, 0, 255]),
            ))
        }
        fn windows(&self) -> crate::error::AppResult<Vec<crate::mcp::host::WindowInfo>> {
            Ok(vec![])
        }
        fn capture_window(&self, _: u32) -> crate::error::AppResult<image::RgbaImage> {
            panic!("unexpected window capture")
        }
    }

    impl crate::mcp::host::AgentHost for TargetedHost {
        fn data_changed(&self, p: Option<&str>, c: Option<&str>, s: &str) {
            crate::mcp::host::AgentHost::data_changed(&self.host, p, c, s);
        }
        fn notify(&self, s: &str) {
            crate::mcp::host::AgentHost::notify(&self.host, s);
        }
        fn confirm(&self, t: &str, s: &str) -> bool {
            crate::mcp::host::AgentHost::confirm(&self.host, t, s)
        }
        fn screen(&self) -> &dyn crate::mcp::host::AgentScreen {
            self
        }
        fn default_export_dir(&self) -> std::path::PathBuf {
            self.host.dir.clone()
        }
    }

    #[test]
    fn captures_only_the_selected_monitor_after_approval() {
        let tmp = TempDir::new();
        let state = state(&tmp);
        let case = {
            let db = state.db();
            test_support::case(&db, &test_support::project(&db))
        };
        let mut host = TargetedHost {
            host: RecordingHost {
                allow: false,
                ..Default::default()
            },
            captures: Default::default(),
        };
        let args = json!({"caseId":case.id,"target":{"monitor":1}});
        assert!(
            capture_screen(
                &ToolContext {
                    state: &state,
                    host: &host,
                    client: "test"
                },
                args.as_object().unwrap()
            )
            .is_err()
        );
        assert!(host.captures.lock().unwrap().is_empty());
        host.host.allow = true;
        let result = output(capture_screen(
            &ToolContext {
                state: &state,
                host: &host,
                client: "test",
            },
            args.as_object().unwrap(),
        ));
        assert_eq!(*host.captures.lock().unwrap(), vec![1]);
        let evidence =
            crate::repo::evidence::get(&state.db(), state.files(), result["id"].as_str().unwrap())
                .unwrap();
        let image = image::open(evidence.original_path).unwrap().to_rgba8();
        assert_eq!(image.get_pixel(0, 0).0, [1, 0, 0, 255]);
        assert!(
            capture_screen(
                &ToolContext {
                    state: &state,
                    host: &host,
                    client: "test"
                },
                json!({"caseId":case.id,"target":{"monitor":5}})
                    .as_object()
                    .unwrap()
            )
            .is_err()
        );
        assert_eq!(*host.captures.lock().unwrap(), vec![1]);
    }

    #[test]
    fn capture_asks_for_confirmation_when_configured() {
        let tmp = TempDir::new();
        let state = state(&tmp);
        let case = {
            let db = state.db();
            test_support::case(&db, &test_support::project(&db))
        };
        let host = RecordingHost {
            allow: false,
            ..Default::default()
        };
        let ctx = ToolContext {
            state: &state,
            host: &host,
            client: "test",
        };
        assert_eq!(
            capture_screen(
                &ctx,
                json!({"caseId":case.id,"target":"primary"})
                    .as_object()
                    .unwrap()
            )
            .unwrap_err(),
            "El tester rechazó la captura."
        );
        assert!(
            crate::repo::evidence::list(&state.db(), state.files(), &case.id)
                .unwrap()
                .is_empty()
        );
        assert!(
            !host
                .calls
                .lock()
                .unwrap()
                .iter()
                .any(|s| s == "Un agente capturó la pantalla.")
        );
    }

    #[test]
    fn capture_skips_confirmation_when_disabled_and_always_notifies() {
        let tmp = TempDir::new();
        let state = state(&tmp);
        let case = {
            let db = state.db();
            test_support::case(&db, &test_support::project(&db))
        };
        let mut settings = state.settings();
        settings.agents.confirm_capture = false;
        state.set_settings(settings);
        let host = RecordingHost {
            allow: false,
            ..Default::default()
        };
        let ctx = ToolContext {
            state: &state,
            host: &host,
            client: "test",
        };
        let result = output(capture_screen(
            &ctx,
            json!({"caseId":case.id,"target":"primary"})
                .as_object()
                .unwrap(),
        ));
        assert_eq!(result["width"], 4);
        let calls = host.calls.lock().unwrap();
        assert!(calls.iter().any(|s| s == "Un agente capturó la pantalla."));
        assert!(
            !calls
                .iter()
                .any(|s| s.starts_with("Un agente quiere capturar"))
        );
    }

    #[test]
    fn capture_monitor_out_of_range_and_unknown_window_are_precise() {
        let tmp = TempDir::new();
        let state = state(&tmp);
        let case = {
            let db = state.db();
            test_support::case(&db, &test_support::project(&db))
        };
        let host = RecordingHost::default();
        let ctx = ToolContext {
            state: &state,
            host: &host,
            client: "test",
        };
        assert_eq!(
            capture_screen(
                &ctx,
                json!({"caseId":case.id,"target":{"monitor":5}})
                    .as_object()
                    .unwrap()
            )
            .unwrap_err(),
            "No existe el monitor 5. Hay 2."
        );
        assert_eq!(
            capture_screen(
                &ctx,
                json!({"caseId":case.id,"target":{"window":"missing"}})
                    .as_object()
                    .unwrap()
            )
            .unwrap_err(),
            "No hay ninguna ventana cuyo título contenga «missing»."
        );
        assert!(
            !host
                .calls
                .lock()
                .unwrap()
                .iter()
                .any(|s| s.contains("Editor de pruebas"))
        );
    }

    #[test]
    fn capture_window_by_title_contains_ignoring_case() {
        let tmp = TempDir::new();
        let state = state(&tmp);
        let case = {
            let db = state.db();
            test_support::case(&db, &test_support::project(&db))
        };
        let host = RecordingHost::default();
        let ctx = ToolContext {
            state: &state,
            host: &host,
            client: "test",
        };
        let result = output(capture_screen(
            &ctx,
            json!({"caseId":case.id,"target":{"window":"PRUEBAS"}})
                .as_object()
                .unwrap(),
        ));
        assert_eq!(result["width"], 4);
        assert!(
            host.calls
                .lock()
                .unwrap()
                .iter()
                .any(|s| s.contains("la ventana «Editor de pruebas»"))
        );
    }

    #[test]
    fn primary_target_uses_monitor_0() {
        let tmp = TempDir::new();
        let state = state(&tmp);
        let case = {
            let db = state.db();
            test_support::case(&db, &test_support::project(&db))
        };
        let host = RecordingHost::default();
        output(capture_screen(
            &ToolContext {
                state: &state,
                host: &host,
                client: "test",
            },
            json!({"caseId":case.id,"target":"primary"})
                .as_object()
                .unwrap(),
        ));
        assert!(
            host.calls
                .lock()
                .unwrap()
                .iter()
                .any(|s| s == "Un agente quiere capturar monitor 0. ¿Permitir?")
        );
    }
}
