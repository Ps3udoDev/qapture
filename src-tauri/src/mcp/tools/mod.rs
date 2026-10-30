use super::host::AgentHost;
use crate::state::AppState;
use serde_json::{Value, json};

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

pub fn call(ctx: &ToolContext<'_>, name: &str, _args: Value) -> (Result<ToolOutput, String>, Meta) {
    let meta = Meta {
        detail: name.into(),
        ..Default::default()
    };
    let result = if name == "qapture_status" {
        crate::repo::integrations::get_account(&ctx.state.db())
            .map_err(|error| error.to_string())
            .map(|account| {
                let github = account.map_or_else(|| json!({"connected": false}), |account| json!({"connected": true, "login": account.login}));
                ToolOutput::Json(json!({"version": env!("CARGO_PKG_VERSION"), "recovery": ctx.state.recovery().is_some(), "agents": ctx.state.settings().agents, "github": github}))
            })
    } else {
        Err("Herramienta aún no disponible.".into())
    };
    (result, meta)
}
