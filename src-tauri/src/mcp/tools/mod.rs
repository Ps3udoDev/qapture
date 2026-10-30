pub mod read;
use super::host::AgentHost;
use crate::state::AppState;
use serde_json::{Map, Value};
pub struct ToolContext<'a> {
    pub state: &'a AppState,
    pub host: &'a dyn AgentHost,
    pub client: &'a str,
}
#[derive(Debug)]
pub enum ToolOutput {
    Json(Value),
    Content(Vec<Value>),
}
#[derive(Default)]
pub struct Meta {
    pub project_id: Option<String>,
    pub case_id: Option<String>,
    pub detail: String,
}
pub fn call(ctx: &ToolContext<'_>, name: &str, args: Value) -> (Result<ToolOutput, String>, Meta) {
    let result = args
        .as_object()
        .ok_or_else(|| "Los argumentos deben ser un objeto.".into())
        .and_then(|a| dispatch(ctx, name, a));
    let meta = activity_meta(ctx, name, &args, &result);
    (result, meta)
}

/// Los ids salen de entidades de la base; el detalle nunca copia argumentos del agente.
fn activity_meta(
    ctx: &ToolContext<'_>,
    name: &str,
    args: &Value,
    result: &Result<ToolOutput, String>,
) -> Meta {
    let mut meta = Meta {
        detail: name.into(),
        ..Default::default()
    };
    let output = match result {
        Ok(ToolOutput::Json(value)) => value,
        _ => &Value::Null,
    };
    let conn = ctx.state.db();
    let case_id = output["caseId"]
        .as_str()
        .or_else(|| {
            matches!(name, "create_case" | "update_case" | "get_case")
                .then(|| output["id"].as_str())
                .flatten()
        })
        .or_else(|| args["caseId"].as_str());
    let case_id = case_id.map(str::to_owned).or_else(|| {
        let evidence_id = args["evidenceId"].as_str()?;
        crate::repo::evidence::get(&conn, ctx.state.files(), evidence_id)
            .ok()
            .map(|e| e.case_id)
    });
    if let Some(case) = case_id.and_then(|id| crate::repo::cases::get(&conn, &id).ok()) {
        meta.project_id = Some(case.project_id);
        meta.case_id = Some(case.id);
    } else {
        let project_id = if matches!(name, "create_project" | "get_project") {
            output["id"].as_str()
        } else {
            None
        }
        .or_else(|| args["projectId"].as_str());
        if let Some(project) = project_id.and_then(|id| crate::repo::projects::get(&conn, id).ok())
        {
            meta.project_id = Some(project.id);
        }
    }
    meta
}
fn dispatch(
    ctx: &ToolContext<'_>,
    name: &str,
    a: &Map<String, Value>,
) -> Result<ToolOutput, String> {
    match name {
        "qapture_status" => read::status(ctx, a),
        "list_templates" => read::list_templates(ctx, a),
        "list_projects" => read::list_projects(ctx, a),
        "get_project" => read::get_project(ctx, a),
        "list_cases" => read::list_cases(ctx, a),
        "get_case" => read::get_case(ctx, a),
        "get_evidence_image" => read::get_evidence_image(ctx, a),
        _ => Err("Herramienta desconocida.".into()),
    }
}
