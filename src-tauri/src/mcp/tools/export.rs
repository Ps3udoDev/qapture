use std::path::{Path, PathBuf};

use chrono::Local;
use serde_json::{Map, Value, json};

use super::{ToolContext, ToolOutput, read::required};
use crate::{
    dto::ExportOptionsInput,
    excel::{
        self,
        data::{self, ExportData},
    },
    transfer::package,
};

pub fn unique_path(dir: &Path, file_name: &str) -> PathBuf {
    let original = dir.join(file_name);
    if !original.exists() {
        return original;
    }
    let file = Path::new(file_name);
    let stem = file.file_stem().unwrap_or_default().to_string_lossy();
    let extension = file
        .extension()
        .map(|s| format!(".{}", s.to_string_lossy()))
        .unwrap_or_default();
    for n in 2u64.. {
        let candidate = dir.join(format!("{stem} ({n}){extension}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    unreachable!()
}

fn folder(ctx: &ToolContext<'_>, args: &Map<String, Value>) -> Result<PathBuf, String> {
    let settings = ctx.state.settings();
    let dir = if args.contains_key("folder") {
        PathBuf::from(required(args, "folder")?)
    } else if !settings.export.default_folder.trim().is_empty() {
        PathBuf::from(settings.export.default_folder)
    } else {
        ctx.host.default_export_dir()
    };
    if !dir.is_absolute() {
        return Err("La ruta debe ser absoluta.".into());
    }
    Ok(dir)
}

fn confirm(ctx: &ToolContext<'_>, project: &str, folder: &Path) -> Result<(), String> {
    if ctx.state.settings().agents.confirm_export
        && !ctx.host.confirm(
            "Exportar",
            &format!(
                "Un agente quiere exportar «{project}» a {}. ¿Permitir?",
                folder.display()
            ),
        )
    {
        return Err("El tester rechazó la exportación.".into());
    }
    std::fs::create_dir_all(folder)
        .map_err(|_| "No se pudo crear la carpeta de exportación.".to_string())
}

pub fn export_excel(
    ctx: &ToolContext<'_>,
    args: &Map<String, Value>,
) -> Result<ToolOutput, String> {
    let settings = ctx.state.settings();
    let tester = args
        .get("tester")
        .map(|_| required(args, "tester"))
        .transpose()?
        .unwrap_or(&settings.profile.tester);
    if tester.trim().is_empty() {
        return Err("Configura tu nombre en Ajustes › Perfil o envía «tester».".into());
    }
    let folder = folder(ctx, args)?;
    let data = ExportData::load(
        &ctx.state.db(),
        ctx.state.files(),
        required(args, "projectId")?,
    )
    .map_err(|e| e.to_string())?;
    let input = ExportOptionsInput {
        tester: tester.into(),
        doc_version_id: data.versions.last().map(|v| v.id.clone()),
        include_evidence: true,
        include_summary: settings.export.include_summary,
        image_max_width: settings.export.image_max_width,
    };
    let today = Local::now().date_naive();
    let options = if data.versions.is_empty() {
        // MCP exports may precede the document's first revision; the UI still
        // requires a revision through its existing resolve_options path.
        let tester =
            crate::domain::text::required_text("Tester", tester, 100).map_err(|e| e.to_string())?;
        if !data::IMAGE_WIDTH_RANGE.contains(&input.image_max_width) {
            return Err(format!(
                "El ancho máximo de imagen debe estar entre {} y {} px.",
                data::IMAGE_WIDTH_RANGE.start(),
                data::IMAGE_WIDTH_RANGE.end()
            ));
        }
        data::ExportSettings {
            tester,
            version: None,
            include_evidence: true,
            include_summary: input.include_summary,
            image_max_width: input.image_max_width,
            today,
            template: false,
        }
    } else {
        data::resolve_options(&data, &input, false, today).map_err(|e| e.to_string())?
    };
    confirm(ctx, &data.project.name, &folder)?;
    // Hold the DB mutex across name selection and saving to serialize concurrent agent exports.
    let _guard = ctx.state.db();
    let path = unique_path(&folder, &data::suggested_file_name(&data.project, &options));
    let result = excel::export(&data, &options, &path, &|_| {}).map_err(|e| e.to_string())?;
    Ok(ToolOutput::Json(json!(result)))
}

pub fn export_package(
    ctx: &ToolContext<'_>,
    args: &Map<String, Value>,
) -> Result<ToolOutput, String> {
    let folder = folder(ctx, args)?;
    let export = package::collect(
        &ctx.state.db(),
        ctx.state.files(),
        required(args, "projectId")?,
    )
    .map_err(|e| e.to_string())?;
    confirm(ctx, &export.doc.project.name, &folder)?;
    let _guard = ctx.state.db();
    let path = unique_path(
        &folder,
        &package::suggested_file_name(
            &export.doc.project.client,
            &export.doc.project.name,
            Local::now().date_naive(),
        ),
    );
    package::write(&export, &path).map_err(|e| e.to_string())?;
    Ok(ToolOutput::Json(
        json!({"path":path.to_string_lossy(),"cases":export.doc.cases.len(),"evidence":export.evidence_count(),"missingImages":export.missing_images}),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        dto::DocVersionInput,
        mcp::{
            host::RecordingHost,
            tools::read::tests::{output, state},
        },
        media::storage::TempDir,
        repo::{self, test_support},
    };

    #[test]
    fn export_never_overwrites_an_existing_file() {
        let tmp = TempDir::new();
        let state = state(&tmp);
        let p = test_support::project(&state.db());
        repo::doc_versions::create(
            &state.db(),
            &p.id,
            &DocVersionInput {
                version: "1.0".into(),
                date: "2026-10-29".into(),
                author: "Tester".into(),
                reviewer: String::new(),
                approver: String::new(),
                changes: String::new(),
            },
        )
        .unwrap();
        let host = RecordingHost {
            dir: tmp.path().join("exports"),
            ..Default::default()
        };
        let ctx = ToolContext {
            state: &state,
            host: &host,
            client: "test",
        };
        let args = json!({"projectId":p.id,"tester":"QA"});
        let first = output(export_excel(&ctx, args.as_object().unwrap()));
        let path = first["path"].as_str().unwrap();
        let original = std::fs::read(path).unwrap();
        let second = output(export_excel(&ctx, args.as_object().unwrap()));
        assert!(second["path"].as_str().unwrap().ends_with(" (2).xlsx"));
        assert_eq!(std::fs::read(path).unwrap(), original);
    }

    #[test]
    fn export_without_tester_asks_for_it() {
        let tmp = TempDir::new();
        let state = state(&tmp);
        let host = RecordingHost::default();
        let ctx = ToolContext {
            state: &state,
            host: &host,
            client: "test",
        };
        assert_eq!(
            export_excel(&ctx, &Map::new()).unwrap_err(),
            "Configura tu nombre en Ajustes › Perfil o envía «tester»."
        );
    }

    #[test]
    fn export_without_document_versions_keeps_cases_and_evidence() {
        let tmp = TempDir::new();
        let state = state(&tmp);
        let case = {
            let db = state.db();
            test_support::case(&db, &test_support::project(&db))
        };
        let host = RecordingHost {
            dir: tmp.path().join("exports"),
            ..Default::default()
        };
        let ctx = ToolContext {
            state: &state,
            host: &host,
            client: "test",
        };
        crate::mcp::tools::write::attach_png(
            &ctx,
            &case.id,
            &crate::media::raster::sample_png(4, 4),
            "Captura",
        )
        .unwrap();
        let result = output(export_excel(
            &ctx,
            json!({"projectId":case.project_id,"tester":"QA"})
                .as_object()
                .unwrap(),
        ));
        assert_eq!(result["cases"], 1);
        assert_eq!(result["evidence"], 1);
        assert!(Path::new(result["path"].as_str().unwrap()).is_file());
    }

    #[test]
    fn export_excel_asks_for_confirmation_when_configured() {
        let tmp = TempDir::new();
        let state = state(&tmp);
        let project = test_support::project(&state.db());
        let host = RecordingHost {
            allow: false,
            dir: tmp.path().join("exports"),
            ..Default::default()
        };
        let ctx = ToolContext {
            state: &state,
            host: &host,
            client: "test",
        };
        assert_eq!(
            export_excel(
                &ctx,
                json!({"projectId":project.id,"tester":"QA"})
                    .as_object()
                    .unwrap()
            )
            .unwrap_err(),
            "El tester rechazó la exportación."
        );
        assert!(!host.dir.exists());
    }

    #[test]
    fn export_package_writes_a_qapture_in_the_folder_and_asks_permission() {
        let tmp = TempDir::new();
        let state = state(&tmp);
        let p = test_support::project(&state.db());
        let host = RecordingHost {
            allow: false,
            dir: tmp.path().join("exports"),
            ..Default::default()
        };
        let args = json!({"projectId":p.id});
        assert_eq!(
            export_package(
                &ToolContext {
                    state: &state,
                    host: &host,
                    client: "test"
                },
                args.as_object().unwrap()
            )
            .unwrap_err(),
            "El tester rechazó la exportación."
        );
        assert!(!host.dir.exists());
        let mut settings = state.settings();
        settings.agents.confirm_export = false;
        state.set_settings(settings);
        let result = output(export_package(
            &ToolContext {
                state: &state,
                host: &host,
                client: "test",
            },
            args.as_object().unwrap(),
        ));
        assert!(Path::new(result["path"].as_str().unwrap()).is_file());
        assert!(result["path"].as_str().unwrap().ends_with(".qapture"));
    }

    #[test]
    fn export_uses_folder_then_settings_then_documents() {
        let tmp = TempDir::new();
        let state = state(&tmp);
        let host = RecordingHost {
            dir: tmp.path().join("documents"),
            ..Default::default()
        };
        let ctx = ToolContext {
            state: &state,
            host: &host,
            client: "test",
        };
        assert_eq!(folder(&ctx, &Map::new()).unwrap(), host.dir);
        let mut settings = state.settings();
        settings.export.default_folder = tmp.path().join("settings").to_string_lossy().into_owned();
        state.set_settings(settings.clone());
        assert_eq!(
            folder(&ctx, &Map::new()).unwrap(),
            PathBuf::from(settings.export.default_folder)
        );
        let explicit = tmp.path().join("explicit");
        assert_eq!(
            folder(&ctx, json!({"folder":explicit}).as_object().unwrap()).unwrap(),
            explicit
        );
    }
}
