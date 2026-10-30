use std::collections::BTreeMap;

use serde_json::{Map, Value, json};

use super::{
    ToolContext, ToolOutput,
    read::{evidence_metadata, required},
};
use crate::{
    domain::status::{DEFAULT_STATUS, STATUSES, is_status},
    dto::{CaseInput, EvidenceDto, NewEvidenceMeta, NewProjectInput, ProjectMetaInput},
    mcp::{fields, images},
    repo,
};

fn status(value: &str) -> Result<String, String> {
    if is_status(value) {
        Ok(value.to_string())
    } else {
        Err(format!(
            "«{value}» no es un estado válido. Estados: {}.",
            STATUSES.join(", ")
        ))
    }
}

fn incoming_values(
    project: &crate::dto::ProjectDto,
    args: &Map<String, Value>,
) -> Result<BTreeMap<String, String>, String> {
    match args.get("values") {
        Some(value) => fields::to_input_values(
            &project.fields,
            value.as_object().ok_or("«values» debe ser un objeto.")?,
        ),
        None => Ok(BTreeMap::new()),
    }
}

pub fn create_project(
    ctx: &ToolContext<'_>,
    args: &Map<String, Value>,
) -> Result<ToolOutput, String> {
    let meta = args
        .get("meta")
        .and_then(Value::as_object)
        .ok_or("Falta «meta».")?;
    let text = |key: &str| -> Result<String, String> {
        match meta.get(key) {
            Some(value) => value
                .as_str()
                .map(str::to_string)
                .ok_or_else(|| format!("«{key}» debe ser texto.")),
            None => Ok(String::new()),
        }
    };
    let meta = ProjectMetaInput {
        name: required(meta, "name")?.into(),
        client: text("client")?,
        test_type: text("testType")?,
        release_version: text("releaseVersion")?,
        start_date: text("startDate")?,
        end_date: text("endDate")?,
    };
    let project = repo::projects::create(
        &ctx.state.db(),
        NewProjectInput {
            template_id: required(args, "templateId")?.into(),
            meta,
        },
    )
    .map_err(|e| e.to_string())?;
    ctx.host.data_changed(
        None,
        None,
        &format!("Un agente creó el proyecto «{}»", project.name),
    );
    Ok(ToolOutput::Json(
        json!({"id":project.id,"name":project.name,"templateId":project.source_template_id,"createdAt":project.created_at}),
    ))
}

pub fn create_case(ctx: &ToolContext<'_>, args: &Map<String, Value>) -> Result<ToolOutput, String> {
    let project_id = required(args, "projectId")?;
    let conn = ctx.state.db();
    let project = repo::projects::get(&conn, project_id).map_err(|e| e.to_string())?;
    let input = CaseInput {
        title: required(args, "title")?.into(),
        status: status(
            args.get("status")
                .map(|_| required(args, "status"))
                .transpose()?
                .unwrap_or(DEFAULT_STATUS),
        )?,
        values: incoming_values(&project, args)?,
    };
    let case = repo::cases::create(&conn, project_id, &input).map_err(|e| e.to_string())?;
    drop(conn);
    ctx.host.data_changed(
        Some(project_id),
        Some(&case.id),
        &format!("Un agente creó {}", case.code),
    );
    Ok(ToolOutput::Json(json!(case)))
}

pub fn update_case(ctx: &ToolContext<'_>, args: &Map<String, Value>) -> Result<ToolOutput, String> {
    let case_id = required(args, "caseId")?;
    let conn = ctx.state.db();
    let current = repo::cases::get(&conn, case_id).map_err(|e| e.to_string())?;
    let project = repo::projects::get(&conn, &current.project_id).map_err(|e| e.to_string())?;
    let mut values = current.values.clone();
    values.retain(|id, _| project.fields.iter().any(|f| f.id == *id && !f.archived));
    values.extend(incoming_values(&project, args)?);
    let input = CaseInput {
        title: args
            .get("title")
            .map(|_| required(args, "title"))
            .transpose()?
            .unwrap_or(&current.title)
            .into(),
        status: status(
            args.get("status")
                .map(|_| required(args, "status"))
                .transpose()?
                .unwrap_or(&current.status),
        )?,
        values,
    };
    let case = repo::cases::update(&conn, case_id, &input).map_err(|e| e.to_string())?;
    drop(conn);
    ctx.host.data_changed(
        Some(&case.project_id),
        Some(case_id),
        &format!("Un agente actualizó {}", case.code),
    );
    Ok(ToolOutput::Json(json!(case)))
}

pub fn attach_png(
    ctx: &ToolContext<'_>,
    case_id: &str,
    png: &[u8],
    description: &str,
) -> Result<EvidenceDto, String> {
    let staged = repo::evidence::stage(ctx.state.files(), png).map_err(|e| e.to_string())?;
    let result = (|| {
        let render =
            std::fs::read(&staged.path).map_err(|_| "No se pudo leer la imagen.".to_string())?;
        let conn = ctx.state.db();
        let case = repo::cases::get(&conn, case_id).map_err(|e| e.to_string())?;
        let meta = NewEvidenceMeta {
            case_id: case_id.into(),
            staged_path: staged.path.clone(),
            description: description.into(),
            annotations: r#"{"version":1,"shapes":[]}"#.into(),
        };
        let evidence = repo::evidence::add(&conn, ctx.state.files(), &meta, &render)
            .map_err(|e| e.to_string())?;
        drop(conn);
        ctx.host.data_changed(
            Some(&case.project_id),
            Some(case_id),
            &format!("Un agente adjuntó una captura a {}", case.code),
        );
        Ok(evidence)
    })();
    if result.is_err() {
        let _ = ctx.state.files().discard_staged(&staged.path);
    }
    result
}

pub fn add_evidence(
    ctx: &ToolContext<'_>,
    args: &Map<String, Value>,
) -> Result<ToolOutput, String> {
    let bytes = images::load(args)?;
    let description = args
        .get("description")
        .map(|_| required(args, "description"))
        .transpose()?
        .unwrap_or("Captura adjunta");
    let evidence = attach_png(ctx, required(args, "caseId")?, &bytes, description)?;
    Ok(ToolOutput::Json(evidence_metadata(&evidence)))
}

pub fn update_evidence_description(
    ctx: &ToolContext<'_>,
    args: &Map<String, Value>,
) -> Result<ToolOutput, String> {
    let conn = ctx.state.db();
    let evidence = repo::evidence::update_description(
        &conn,
        ctx.state.files(),
        required(args, "evidenceId")?,
        required(args, "description")?,
    )
    .map_err(|e| e.to_string())?;
    let case = repo::cases::get(&conn, &evidence.case_id).map_err(|e| e.to_string())?;
    drop(conn);
    ctx.host.data_changed(
        Some(&case.project_id),
        Some(&case.id),
        &format!("Un agente actualizó {}", case.code),
    );
    Ok(ToolOutput::Json(evidence_metadata(&evidence)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        mcp::{
            host::RecordingHost,
            tools::read::tests::{output, state},
        },
        media::{raster::sample_png, storage::TempDir},
        repo::test_support,
    };
    use base64::{Engine, engine::general_purpose::STANDARD};

    #[test]
    fn create_case_defaults_to_no_ejecutado_and_resolves_labels() {
        let tmp = TempDir::new();
        let state = state(&tmp);
        let project = test_support::project(&state.db());
        let host = RecordingHost::default();
        let ctx = ToolContext {
            state: &state,
            host: &host,
            client: "test",
        };
        let case=output(create_case(&ctx,json!({"projectId":project.id,"title":"Pantalla","values":{"Entorno":"QA","MÓDULO":"A"}}).as_object().unwrap()));
        assert_eq!(case["status"], DEFAULT_STATUS);
        assert_eq!(
            case["values"][test_support::field_id(&project, "modulo")],
            "A"
        );
        let calls = host.calls.lock().unwrap();
        assert!(calls[0].contains(&project.id));
        assert!(calls[0].contains(case["id"].as_str().unwrap()));
        assert!(calls[0].contains("Un agente creó EJ-001"));
        let error = create_case(
            &ctx,
            json!({"projectId":project.id,"title":"Pantalla","status":"Hecho"})
                .as_object()
                .unwrap(),
        )
        .unwrap_err();
        assert_eq!(
            error,
            format!(
                "«Hecho» no es un estado válido. Estados: {}.",
                STATUSES.join(", ")
            )
        );
    }

    #[test]
    fn update_case_merges_values_instead_of_replacing() {
        let tmp = TempDir::new();
        let state = state(&tmp);
        let project = test_support::project(&state.db());
        let host = RecordingHost::default();
        let ctx = ToolContext {
            state: &state,
            host: &host,
            client: "test",
        };
        let case=output(create_case(&ctx,json!({"projectId":project.id,"title":"Pantalla","values":{"entorno":"QA","modulo":"A","resultado_obtenido":"X"}}).as_object().unwrap()));
        let updated = output(update_case(
            &ctx,
            json!({"caseId":case["id"],"values":{"Resultado obtenido":"Y"}})
                .as_object()
                .unwrap(),
        ));
        assert_eq!(
            updated["values"][test_support::field_id(&project, "modulo")],
            "A"
        );
        assert_eq!(
            updated["values"][test_support::field_id(&project, "resultado_obtenido")],
            "Y"
        );
        assert_eq!(updated["title"], "Pantalla");
        assert_eq!(updated["status"], DEFAULT_STATUS);
    }

    #[test]
    fn create_project_requires_an_existing_template_and_returns_metadata() {
        let tmp = TempDir::new();
        let state = state(&tmp);
        let host = RecordingHost::default();
        let ctx = ToolContext {
            state: &state,
            host: &host,
            client: "test",
        };
        assert!(
            create_project(
                &ctx,
                json!({"templateId":"missing","meta":{"name":"Proyecto"}})
                    .as_object()
                    .unwrap()
            )
            .is_err()
        );
        let result = output(create_project(
            &ctx,
            json!({"templateId":"tpl-funcional","meta":{"name":"Proyecto"}})
                .as_object()
                .unwrap(),
        ));
        assert_eq!(result["name"], "Proyecto");
        assert_eq!(result["templateId"], "tpl-funcional");
        assert!(!result["id"].as_str().unwrap().is_empty());
        assert!(!result["createdAt"].as_str().unwrap().is_empty());
        assert!(host.calls.lock().unwrap()[0].starts_with("changed:None:None:"));
    }

    #[test]
    fn add_evidence_from_path_and_base64_stores_pngs_and_empty_annotations() {
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
        let png = sample_png(8, 8);
        let path = tmp.path().join("input.png");
        std::fs::write(&path, &png).unwrap();
        for source in [
            json!({"caseId":case.id,"path":path}),
            json!({"caseId":case.id,"base64":STANDARD.encode(&png)}),
        ] {
            let result = output(add_evidence(&ctx, source.as_object().unwrap()));
            assert!(result.get("originalPath").is_none());
            assert!(result.get("renderedPath").is_none());
            let stored =
                repo::evidence::get(&state.db(), state.files(), result["id"].as_str().unwrap())
                    .unwrap();
            assert_eq!(
                serde_json::from_str::<Value>(&stored.annotations).unwrap(),
                json!({"version":1,"shapes":[]})
            );
            assert!(std::path::Path::new(&stored.original_path).is_file());
            assert!(std::path::Path::new(&stored.rendered_path).is_file());
        }
        assert_eq!(
            repo::evidence::list(&state.db(), state.files(), &case.id)
                .unwrap()
                .len(),
            2
        );
    }

    #[test]
    fn failed_attachment_discards_staging() {
        let tmp = TempDir::new();
        let state = state(&tmp);
        let host = RecordingHost::default();
        let ctx = ToolContext {
            state: &state,
            host: &host,
            client: "test",
        };
        assert!(attach_png(&ctx, "missing", &sample_png(4, 4), "Captura").is_err());
        assert_eq!(
            std::fs::read_dir(state.files().staging_dir())
                .unwrap()
                .count(),
            0
        );
    }

    #[test]
    fn a_non_image_path_is_rejected_without_echoing_it_in_activity() {
        let tmp = TempDir::new();
        let state = state(&tmp);
        let case = {
            let db = state.db();
            test_support::case(&db, &test_support::project(&db))
        };
        let mut settings = state.settings();
        settings.agents.enabled = true;
        settings.agents.write = true;
        state.set_settings(settings);
        let path = tmp.path().join("secreto.png");
        std::fs::write(&path, b"-----BEGIN OPENSSH PRIVATE KEY-----").unwrap();
        let host = RecordingHost::default();
        let mut session = crate::mcp::protocol::Session::new();
        let replies=crate::mcp::protocol::handle_line(&mut session,&state,&host,&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"add_evidence","arguments":{"caseId":case.id,"path":path}}}).to_string());
        let response: Value = serde_json::from_str(&replies[0]).unwrap();
        assert_eq!(
            response["result"]["content"][0]["text"],
            "El archivo no es una imagen PNG, JPEG o WebP."
        );
        let rows = crate::repo::agent_activity::list(&state.db(), 10, None).unwrap();
        assert_eq!(rows.len(), 1);
        let serialized = json!(rows).to_string();
        assert!(!serialized.contains("secreto"));
        assert!(!serialized.contains("OPENSSH"));
    }

    #[test]
    fn two_sessions_creating_cases_get_distinct_codes() {
        let tmp = TempDir::new();
        let state = state(&tmp);
        let project = test_support::project(&state.db());
        let host = RecordingHost::default();
        let codes = std::thread::scope(|scope| {
            let run = || {
                (0..10).map(|i| {let ctx=ToolContext {state:&state,host:&host,client:"test"};output(create_case(&ctx,json!({"projectId":project.id,"title":format!("Case {i}"),"values":{"entorno":"QA"}}).as_object().unwrap()))["code"].as_str().unwrap().to_string()}).collect::<Vec<_>>()
            };
            let a = scope.spawn(run);
            let b = scope.spawn(run);
            let mut codes = a.join().unwrap();
            codes.extend(b.join().unwrap());
            codes
        });
        let unique: std::collections::BTreeSet<_> = codes.into_iter().collect();
        assert_eq!(unique.len(), 20);
        assert!(unique.contains("EJ-001"));
        assert!(unique.contains("EJ-020"));
        assert_eq!(
            repo::cases::list(&state.db(), &project.id).unwrap().len(),
            20
        );
    }
}
