use serde_json::{Value, json};

pub const APP_UNAVAILABLE: &str =
    "No se encontró Qapture y no se pudo abrir. Ábrelo y vuelve a intentar.";
/// Tras perder la conexión el puente no vuelve a abrir Qapture (el instalador pudo cerrarla).
pub const APP_CLOSED: &str = "Qapture se cerró. Ábrelo y vuelve a intentar.";
pub fn app_updated(app: &str) -> String {
    format!(
        "Qapture se actualizó a {app}. Vuelve a conectar el servidor MCP en tu agente (en Claude Code: /mcp)."
    )
}

/// Servidor mínimo legible incluso si la app no está disponible.
pub fn respond(line: &str, message: &str) -> Option<String> {
    let request: Value = match serde_json::from_str(line) {
        Ok(request) => request,
        Err(_) => return Some(
            json!({"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":"JSON inválido."}})
                .to_string(),
        ),
    };
    if request["jsonrpc"] != "2.0" || !request["method"].is_string() || !request.is_object() {
        return Some(json!({"jsonrpc":"2.0","id":null,"error":{"code":-32600,"message":"Solicitud inválida."}}).to_string());
    }
    let id = request.get("id")?.clone();
    let result = match request["method"].as_str()? {
        "initialize" => {
            let requested = request["params"]["protocolVersion"].as_str().unwrap_or("");
            let version = if ["2025-06-18", "2025-03-26", "2024-11-05"].contains(&requested) { requested } else { "2025-06-18" };
            json!({"protocolVersion":version,"serverInfo":{"name":"qapture","title":"Qapture","version":env!("CARGO_PKG_VERSION")},"capabilities":{"tools":{"listChanged":true},"prompts":{"listChanged":false}},"instructions":message})
        }
        "tools/list" => json!({"tools":[{"name":"qapture_status","description":"Estado de Qapture.","inputSchema":{"type":"object","properties":{},"additionalProperties":false},"annotations":{"readOnlyHint":true,"destructiveHint":false,"openWorldHint":false}}]}),
        "tools/call" => json!({"content":[{"type":"text","text":message}],"isError":true}),
        "ping" => json!({}),
        _ => return Some(json!({"jsonrpc":"2.0","id":id,"error":{"code":-32601,"message":"Método desconocido."}}).to_string()),
    };
    Some(json!({"jsonrpc":"2.0","id":id,"result":result}).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fallback_answers_initialize_and_lists_only_status() {
        let init: Value = serde_json::from_str(
            &respond(
                r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#,
                "warning",
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(init["result"]["protocolVersion"], "2025-06-18");
        let tools: Value = serde_json::from_str(
            &respond(
                r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
                "warning",
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(tools["result"]["tools"].as_array().unwrap().len(), 1);
        assert_eq!(tools["result"]["tools"][0]["name"], "qapture_status");
    }
    #[test]
    fn fallback_tool_call_returns_the_message_as_error() {
        let response: Value = serde_json::from_str(
            &respond(
                r#"{"jsonrpc":"2.0","id":2,"method":"tools/call"}"#,
                "warning",
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(response["result"]["isError"], true);
        assert_eq!(response["result"]["content"][0]["text"], "warning");
        assert!(
            respond(
                r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
                "warning"
            )
            .is_none()
        );
    }
}
