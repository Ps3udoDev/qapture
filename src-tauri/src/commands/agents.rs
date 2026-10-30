use crate::{
    dto::{ActivityDto, AgentClientConfigDto},
    error::AppResult,
    mcp::McpRuntime,
    repo,
    state::AppState,
};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager, State};

pub fn bridge_path(exe: &Path, data_dir: &Path, appimage: bool) -> PathBuf {
    if appimage {
        return data_dir.join("bin/qapture-mcp");
    }
    exe.parent()
        .unwrap_or(Path::new("."))
        .join(if cfg!(windows) {
            "qapture-mcp.exe"
        } else {
            "qapture-mcp"
        })
}

fn client_config(path: &Path, listening: bool) -> AgentClientConfigDto {
    let path = path.to_string_lossy().into_owned();
    // TOML literal strings avoid Windows backslash escaping. Triple quotes also support apostrophes.
    let toml = if path.contains('\'') {
        format!("'''{path}'''")
    } else {
        format!("'{path}'")
    };
    AgentClientConfigDto {
        bridge_path: path.clone(),
        listening,
        claude_code: format!("claude mcp add qapture -- \"{path}\""),
        codex: format!("[mcp_servers.qapture]\ncommand = {toml}"),
        json: serde_json::to_string_pretty(
            &serde_json::json!({"mcpServers":{"qapture":{"command":path}}}),
        )
        .expect("serializar strings"),
    }
}

#[tauri::command]
pub fn mcp_client_config(app: AppHandle) -> AppResult<AgentClientConfigDto> {
    let runtime = app.state::<McpRuntime>();
    let path = bridge_path(
        &std::env::current_exe()?,
        runtime.data_dir(),
        std::env::var_os("APPIMAGE").is_some(),
    );
    Ok(client_config(&path, runtime.listening()))
}

#[tauri::command]
pub fn mcp_activity(
    state: State<'_, AppState>,
    limit: u32,
    client: Option<String>,
) -> AppResult<Vec<ActivityDto>> {
    repo::agent_activity::list(&state.db(), limit.min(2000), client.as_deref())
}

#[tauri::command]
pub fn mcp_regenerate_key(runtime: State<'_, McpRuntime>) -> AppResult<()> {
    runtime.regenerate_key()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn client_config_uses_the_sibling_bridge_path() {
        let exe = Path::new("/install/qapture");
        assert_eq!(
            bridge_path(exe, Path::new("/data"), false),
            Path::new("/install").join(if cfg!(windows) {
                "qapture-mcp.exe"
            } else {
                "qapture-mcp"
            })
        );
    }
    #[test]
    fn json_config_escapes_backslashes() {
        let config = client_config(Path::new(r"C:\Qapture\qapture-mcp.exe"), true);
        let json: serde_json::Value = serde_json::from_str(&config.json).unwrap();
        assert_eq!(
            json["mcpServers"]["qapture"]["command"],
            r"C:\Qapture\qapture-mcp.exe"
        );
        assert!(config.json.contains(r"C:\\Qapture\\qapture-mcp.exe"));
    }
    #[test]
    fn codex_config_uses_a_literal_string() {
        let config = client_config(Path::new(r"C:\Qapture\qapture-mcp.exe"), true);
        assert_eq!(
            config.codex,
            "[mcp_servers.qapture]\ncommand = 'C:\\Qapture\\qapture-mcp.exe'"
        );
    }
    #[test]
    fn regenerate_key_changes_the_file_and_disconnects() {
        let dir = crate::media::storage::TempDir::new();
        let old = crate::mcp::endpoint::load_or_create_key(dir.path()).unwrap();
        let runtime = McpRuntime::new(dir.path().to_path_buf());
        let mut disconnected = runtime.disconnect_events();
        runtime.regenerate_key().unwrap();
        assert!(disconnected.try_recv().is_ok());
        assert_ne!(old, crate::mcp::endpoint::read_key(dir.path()).unwrap());
    }
}
