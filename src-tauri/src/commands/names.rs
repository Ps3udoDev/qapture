// Lista única de comandos propios. build.rs la incluye (`include!`) para generar los permisos
// `allow-*`. Al agregar un comando: regístralo aquí, en `generate_handler!` (lib.rs) y en la
// capability de su ventana (capabilities/*.json). Los tests de abajo verifican las tres cosas.

pub const APP_COMMANDS: &[&str] = &[
    "mcp_client_config",
    "mcp_activity",
    "mcp_regenerate_key",
    "list_templates",
    "save_template_from_project",
    "delete_template",
    "list_projects",
    "get_project",
    "create_project",
    "update_project",
    "delete_project",
    "save_project_fields",
    "list_cases",
    "create_case",
    "update_case",
    "duplicate_case",
    "delete_case",
    "stage_evidence_image",
    "discard_staged_image",
    "read_evidence_image",
    "list_evidence",
    "add_evidence",
    "update_evidence",
    "update_evidence_description",
    "move_evidence",
    "reorder_evidence",
    "delete_evidence",
    "start_capture",
    "capture_frame",
    "overlay_ready",
    "finish_capture",
    "cancel_capture",
    "list_doc_versions",
    "create_doc_version",
    "update_doc_version",
    "delete_doc_version",
    "export_project",
    "export_template",
    "open_export",
    "reveal_export",
    "get_startup_status",
    "import_legacy",
    "import_package",
    "export_package",
    "list_backups",
    "create_backup",
    "restore_backup",
    "recover_database",
    "get_settings",
    "save_settings",
    "pick_export_folder",
    "open_data_folder",
    "hide_main_window",
    "open_link",
    "check_update",
    "install_update",
    "gh_connect_start",
    "gh_connect_cancel",
    "gh_account",
    "gh_disconnect",
    "gh_list_repos",
    "gh_project_github_get",
    "gh_project_github_set",
    "gh_repo_meta",
    "gh_project_links",
    "gh_create_issue",
    "gh_comment_issue",
    "gh_copy_evidence",
    "gh_open_issue",
];

/// Lo único que puede invocar una ventana overlay (spec §9). El resto es de la principal.
pub const OVERLAY_COMMANDS: &[&str] = &[
    "capture_frame",
    "overlay_ready",
    "finish_capture",
    "cancel_capture",
];

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    fn capability(file: &str) -> (Vec<String>, BTreeSet<String>) {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("capabilities")
            .join(file);
        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let strings = |key: &str| -> Vec<String> {
            json[key]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap().to_string())
                .collect()
        };
        (
            strings("windows"),
            strings("permissions").into_iter().collect(),
        )
    }

    fn allow<'a>(names: impl IntoIterator<Item = &'a &'a str>) -> BTreeSet<String> {
        names
            .into_iter()
            .map(|n| format!("allow-{}", n.replace('_', "-")))
            .collect()
    }

    #[test]
    fn command_names_are_unique_and_overlay_commands_are_app_commands() {
        let unique: BTreeSet<_> = APP_COMMANDS.iter().collect();
        assert_eq!(unique.len(), APP_COMMANDS.len());
        assert!(OVERLAY_COMMANDS.iter().all(|c| APP_COMMANDS.contains(c)));
    }

    #[test]
    fn main_window_gets_core_and_every_non_overlay_command() {
        let (windows, permissions) = capability("main.json");
        assert_eq!(windows, ["main"]);
        let mut expected = allow(
            APP_COMMANDS
                .iter()
                .filter(|c| !OVERLAY_COMMANDS.contains(c)),
        );
        expected.insert("core:default".into());
        assert_eq!(permissions, expected);
    }

    #[test]
    fn overlay_can_only_call_the_capture_commands() {
        let (windows, permissions) = capability("overlay.json");
        assert_eq!(windows, ["overlay-*"]);
        assert_eq!(permissions, allow(OVERLAY_COMMANDS));
    }

    #[test]
    fn every_registered_handler_is_listed_and_vice_versa() {
        let lib = include_str!("../lib.rs");
        let registered: BTreeSet<&str> = lib
            .lines()
            .map(str::trim)
            .filter(|l| l.starts_with("commands::") && l.ends_with(','))
            .map(|l| l.trim_end_matches(',').rsplit("::").next().unwrap())
            .collect();
        assert_eq!(
            registered,
            APP_COMMANDS.iter().copied().collect::<BTreeSet<_>>()
        );
    }
}
