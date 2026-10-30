use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Map, Value, json};

use super::{ToolContext, ToolOutput};
use crate::{
    domain::status::{DEFAULT_STATUS, STATUSES},
    dto::EvidenceDto,
    mcp::{fields::normalize_label, images},
    repo,
};

pub(crate) fn required<'a>(args: &'a Map<String, Value>, key: &str) -> Result<&'a str, String> {
    args.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("Falta «{key}» o no es texto."))
}

pub(crate) fn evidence_metadata(evidence: &EvidenceDto) -> Value {
    let mut value = json!(evidence);
    if let Some(object) = value.as_object_mut() {
        object.remove("originalPath");
        object.remove("renderedPath");
    }
    value
}

pub fn status(ctx: &ToolContext<'_>, _args: &Map<String, Value>) -> Result<ToolOutput, String> {
    let account = repo::integrations::get_account(&ctx.state.db()).map_err(|e| e.to_string())?;
    let github = match account {
        Some(account) => json!({"connected":true,"login":account.login,"name":account.name}),
        None => json!({"connected":false}),
    };
    Ok(ToolOutput::Json(
        json!({"version":env!("CARGO_PKG_VERSION"), "agents":ctx.state.settings().agents, "recovery":ctx.state.recovery().is_some(),"github":github}),
    ))
}

pub fn list_templates(
    ctx: &ToolContext<'_>,
    _args: &Map<String, Value>,
) -> Result<ToolOutput, String> {
    Ok(ToolOutput::Json(json!(
        repo::templates::list(&ctx.state.db()).map_err(|e| e.to_string())?
    )))
}

pub fn list_projects(
    ctx: &ToolContext<'_>,
    _args: &Map<String, Value>,
) -> Result<ToolOutput, String> {
    Ok(ToolOutput::Json(json!(
        repo::projects::list(&ctx.state.db()).map_err(|e| e.to_string())?
    )))
}

pub fn get_project(ctx: &ToolContext<'_>, args: &Map<String, Value>) -> Result<ToolOutput, String> {
    let project = repo::projects::get(&ctx.state.db(), required(args, "projectId")?)
        .map_err(|e| e.to_string())?;
    let mut value = json!(project);
    value["statuses"] = json!(STATUSES);
    value["defaultStatus"] = json!(DEFAULT_STATUS);
    Ok(ToolOutput::Json(value))
}

pub fn list_cases(ctx: &ToolContext<'_>, args: &Map<String, Value>) -> Result<ToolOutput, String> {
    let project_id = required(args, "projectId")?;
    let limit = match args.get("limit") {
        Some(value) => value
            .as_u64()
            .filter(|n| *n > 0)
            .ok_or("«limit» debe ser un entero positivo.")?
            .min(500) as usize,
        None => 100,
    };
    let offset = match args.get("cursor") {
        Some(value) => value
            .as_str()
            .and_then(|s| s.parse::<usize>().ok())
            .ok_or("«cursor» no es válido.")?,
        None => 0,
    };
    let status = args
        .get("status")
        .map(|_| required(args, "status"))
        .transpose()?;
    let text = args
        .get("text")
        .map(|_| required(args, "text").map(normalize_label))
        .transpose()?;
    let cases = repo::cases::list(&ctx.state.db(), project_id).map_err(|e| e.to_string())?;
    let cases: Vec<_> = cases
        .into_iter()
        .filter(|case| {
            status.is_none_or(|s| case.status == s)
                && text.as_ref().is_none_or(|text| {
                    std::iter::once(&case.code)
                        .chain(std::iter::once(&case.title))
                        .chain(case.values.values())
                        .any(|s| normalize_label(s).contains(text))
                })
        })
        .collect();
    let end = offset.saturating_add(limit).min(cases.len());
    let page = cases.get(offset..end).unwrap_or_default();
    let mut value = json!({"cases": page});
    if end < cases.len() {
        value["nextCursor"] = json!(end.to_string());
    }
    Ok(ToolOutput::Json(value))
}

pub fn get_case(ctx: &ToolContext<'_>, args: &Map<String, Value>) -> Result<ToolOutput, String> {
    let conn = ctx.state.db();
    let case = if args.contains_key("caseId") {
        repo::cases::get(&conn, required(args, "caseId")?).map_err(|e| e.to_string())?
    } else {
        let code = required(args, "code")?;
        repo::cases::list(&conn, required(args, "projectId")?)
            .map_err(|e| e.to_string())?
            .into_iter()
            .find(|case| case.code == code)
            .ok_or("El caso no existe.")?
    };
    let evidence =
        repo::evidence::list(&conn, ctx.state.files(), &case.id).map_err(|e| e.to_string())?;
    let mut value = json!(case);
    value["evidence"] = json!(evidence.iter().map(evidence_metadata).collect::<Vec<_>>());
    Ok(ToolOutput::Json(value))
}

pub fn get_evidence_image(
    ctx: &ToolContext<'_>,
    args: &Map<String, Value>,
) -> Result<ToolOutput, String> {
    let evidence = repo::evidence::get(
        &ctx.state.db(),
        ctx.state.files(),
        required(args, "evidenceId")?,
    )
    .map_err(|e| e.to_string())?;
    let variant = args
        .get("variant")
        .map(|_| required(args, "variant"))
        .transpose()?
        .unwrap_or("render");
    let path = match variant {
        "render" => evidence.rendered_path,
        "original" => evidence.original_path,
        _ => return Err("«variant» debe ser «render» u «original».".into()),
    };
    let bytes = std::fs::read(path).map_err(|_| "No se pudo leer la imagen.".to_string())?;
    let png = images::thumbnail_png(&bytes, 1568).map_err(|e| e.to_string())?;
    Ok(ToolOutput::Content(vec![
        json!({"type":"image","mimeType":"image/png","data":STANDARD.encode(png)}),
    ]))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::{
        media::storage::TempDir,
        repo::test_support,
        state::{AppState, GithubState},
    };

    pub fn state(tmp: &TempDir) -> AppState {
        AppState::new(
            test_support::conn(),
            tmp.files(),
            tmp.path(),
            None,
            false,
            GithubState::for_tests("http://127.0.0.1:9", "http://127.0.0.1:9"),
        )
    }

    pub fn output(result: Result<ToolOutput, String>) -> Value {
        match result.unwrap() {
            ToolOutput::Json(value) => value,
            _ => panic!("expected JSON"),
        }
    }

    #[test]
    fn get_project_includes_fields_and_statuses() {
        let tmp = TempDir::new();
        let state = state(&tmp);
        let project = test_support::project(&state.db());
        let host = crate::mcp::host::RecordingHost::default();
        let ctx = ToolContext {
            state: &state,
            host: &host,
            client: "test",
        };
        let value = output(get_project(
            &ctx,
            json!({"projectId":project.id}).as_object().unwrap(),
        ));
        assert_eq!(value["statuses"], json!(STATUSES));
        assert_eq!(value["defaultStatus"], DEFAULT_STATUS);
        assert_eq!(
            value["fields"].as_array().unwrap().len(),
            project.fields.len()
        );
    }

    #[test]
    fn list_cases_filters_by_status_and_text_and_paginates() {
        let tmp = TempDir::new();
        let state = state(&tmp);
        let project = test_support::project(&state.db());
        for i in 0..7 {
            repo::cases::create(
                &state.db(),
                &project.id,
                &crate::dto::CaseInput {
                    title: format!("Módulo {i}"),
                    status: if i == 6 { "Fallido" } else { DEFAULT_STATUS }.into(),
                    values: std::collections::BTreeMap::from([
                        (test_support::field_id(&project, "entorno"), "QA".into()),
                        (test_support::field_id(&project, "modulo"), "Pagos".into()),
                    ]),
                },
            )
            .unwrap();
        }
        let host = crate::mcp::host::RecordingHost::default();
        let ctx = ToolContext {
            state: &state,
            host: &host,
            client: "test",
        };
        let args = json!({"projectId":project.id,"limit":3,"text":"MODULO"});
        let page = output(list_cases(&ctx, args.as_object().unwrap()));
        assert_eq!(page["cases"].as_array().unwrap().len(), 3);
        assert_eq!(page["nextCursor"], "3");
        let last = output(list_cases(
            &ctx,
            json!({"projectId":project.id,"limit":3,"cursor":"6"})
                .as_object()
                .unwrap(),
        ));
        assert_eq!(last["cases"].as_array().unwrap().len(), 1);
        assert!(last.get("nextCursor").is_none());
        for text in ["EJ-007", "pagos"] {
            let filtered = output(list_cases(
                &ctx,
                json!({"projectId":project.id,"status":"Fallido","text":text})
                    .as_object()
                    .unwrap(),
            ));
            assert_eq!(filtered["cases"].as_array().unwrap().len(), 1);
        }
        let outside = output(list_cases(
            &ctx,
            json!({"projectId":project.id,"cursor":"999"})
                .as_object()
                .unwrap(),
        ));
        assert!(outside["cases"].as_array().unwrap().is_empty());
    }

    #[test]
    fn list_cases_caps_limit_at_500() {
        let tmp = TempDir::new();
        let state = state(&tmp);
        let project = test_support::project(&state.db());
        {
            let db = state.db();
            for i in 0..501 {
                db.execute("INSERT INTO test_case(id,project_id,code,title,status,created_at,updated_at) VALUES (?1,?2,?1,'Case','No ejecutado','','')",rusqlite::params![format!("EJ-{i:03}"),project.id]).unwrap();
            }
        }
        let host = crate::mcp::host::RecordingHost::default();
        let ctx = ToolContext {
            state: &state,
            host: &host,
            client: "test",
        };
        let page = output(list_cases(
            &ctx,
            json!({"projectId":project.id,"limit":1000})
                .as_object()
                .unwrap(),
        ));
        assert_eq!(page["cases"].as_array().unwrap().len(), 500);
        assert_eq!(page["nextCursor"], "500");
    }

    #[test]
    fn get_case_by_project_and_code_and_evidence_image_are_safe() {
        let tmp = TempDir::new();
        let state = state(&tmp);
        let case = {
            let db = state.db();
            test_support::case(&db, &test_support::project(&db))
        };
        let host = crate::mcp::host::RecordingHost::default();
        let ctx = ToolContext {
            state: &state,
            host: &host,
            client: "test",
        };
        let png = crate::media::raster::sample_png(2400, 1200);
        let staged = repo::evidence::stage(state.files(), &png).unwrap();
        let ev = repo::evidence::add(
            &state.db(),
            state.files(),
            &crate::dto::NewEvidenceMeta {
                case_id: case.id.clone(),
                staged_path: staged.path,
                description: "Pantalla".into(),
                annotations: r#"{"version":1,"shapes":[]}"#.into(),
            },
            &png,
        )
        .unwrap();
        let value = output(get_case(
            &ctx,
            json!({"projectId":case.project_id,"code":case.code})
                .as_object()
                .unwrap(),
        ));
        assert_eq!(value["id"], case.id);
        assert_eq!(value["evidence"][0]["description"], "Pantalla");
        assert!(value["evidence"][0].get("originalPath").is_none());
        assert!(value["evidence"][0].get("renderedPath").is_none());
        let ToolOutput::Content(content) =
            get_evidence_image(&ctx, json!({"evidenceId":ev.id}).as_object().unwrap()).unwrap()
        else {
            panic!("expected image")
        };
        assert_eq!(content[0]["type"], "image");
        assert_eq!(content[0]["mimeType"], "image/png");
        let bytes = STANDARD
            .decode(content[0]["data"].as_str().unwrap())
            .unwrap();
        let image = image::load_from_memory(&bytes).unwrap();
        assert_eq!((image.width(), image.height()), (1568, 784));
        std::fs::write(&ev.rendered_path, crate::media::raster::sample_png(4, 4)).unwrap();
        let ToolOutput::Content(content) = get_evidence_image(
            &ctx,
            json!({"evidenceId":ev.id,"variant":"original"})
                .as_object()
                .unwrap(),
        )
        .unwrap() else {
            panic!("expected image")
        };
        let image = image::load_from_memory(
            &STANDARD
                .decode(content[0]["data"].as_str().unwrap())
                .unwrap(),
        )
        .unwrap();
        assert_eq!((image.width(), image.height()), (1568, 784));
    }

    #[test]
    fn status_exposes_account_identity_without_private_fields() {
        let tmp = TempDir::new();
        let state = state(&tmp);
        let host = crate::mcp::host::RecordingHost::default();
        repo::integrations::save_account(
            &state.db(),
            &repo::integrations::GhAccountRow {
                login: "qa".into(),
                name: Some("QA".into()),
                avatar_data: Some("private-avatar".into()),
                connected_at: "date".into(),
            },
        )
        .unwrap();
        let value = output(status(
            &ToolContext {
                state: &state,
                host: &host,
                client: "test",
            },
            &Map::new(),
        ));
        assert_eq!(value["github"]["login"], "qa");
        assert!(!value.to_string().contains("private-avatar"));
        assert!(value["github"].get("token").is_none());
    }

    #[test]
    fn metadata_never_contains_internal_paths() {
        let ev = EvidenceDto {
            id: "id".into(),
            case_id: "case".into(),
            position: 0,
            description: "desc".into(),
            original_path: "secret".into(),
            rendered_path: "secret".into(),
            annotations: "{}".into(),
            width: 1,
            height: 1,
            captured_at: "".into(),
            updated_at: "".into(),
        };
        let value = evidence_metadata(&ev);
        assert!(value.get("originalPath").is_none());
        assert!(value.get("renderedPath").is_none());
        assert_eq!(value["description"], "desc");
    }
}
