//! El archivo se elige en Rust y permanece en memoria; el webview solo recibe un token.
use std::{fs, io::Read};

use tauri::{State, WebviewWindow};

use super::data::pick_file;
use crate::{
    db::new_id,
    domain::text::fold,
    dto::{
        GuionImportInput, GuionInspectDto, GuionPreviewDto, GuionSheetDto, ImportProfileDto,
        ImportSummaryDto,
    },
    error::{AppResult, validation},
    repo::guion,
    state::{AppState, PendingGuion},
    transfer::guion::{
        import,
        profile::ProfileDefinition,
        read::{self, GuionBook},
    },
};

fn suggested_profile(book: &GuionBook, profiles: &[ImportProfileDto]) -> Option<String> {
    profiles
        .iter()
        .find(|profile| {
            let def = &profile.definition;
            def.header_row > 0
                && book.sheet(&def.sheet).is_some_and(|sheet| {
                    !def.columns.is_empty()
                        && def.columns.iter().all(|column| {
                            read::column_index(&column.column).is_some_and(|col| {
                                fold(&sheet.cell(def.header_row - 1, col).text())
                                    == fold(&column.header)
                            })
                        })
                })
        })
        .map(|profile| profile.id.clone())
}

fn inspection(pending: &PendingGuion, profiles: &[ImportProfileDto]) -> GuionInspectDto {
    GuionInspectDto {
        token: pending.token.clone(),
        file_name: pending.file_name.clone(),
        sheets: pending
            .book
            .sheets
            .iter()
            .map(|sheet| {
                let suggested_header_row = read::suggest_header_row(sheet);
                GuionSheetDto {
                    name: sheet.name.clone(),
                    rows: sheet
                        .rows
                        .iter()
                        .take(200)
                        .map(|row| row.iter().take(52).map(|cell| cell.text()).collect())
                        .collect(),
                    suggested_header_row,
                    cover: suggested_header_row
                        .map(|row| read::detect_cover(sheet, row))
                        .unwrap_or_default(),
                }
            })
            .collect(),
        suggested_profile_id: suggested_profile(&pending.book, profiles),
    }
}

#[tauri::command(async)]
pub fn guion_inspect(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> AppResult<Option<GuionInspectDto>> {
    state.ensure_ready()?;
    let Some(path) = pick_file(
        &window,
        "Importar guion de pruebas",
        "Libro de Excel",
        "xlsx",
    ) else {
        return Ok(None);
    };
    // El límite de lectura también cubre un archivo que crezca tras consultar sus metadatos.
    let file = fs::File::open(&path)?;
    if file.metadata()?.len() > read::MAX_FILE_BYTES {
        return Err(validation("El archivo supera 50 MB."));
    }
    let mut bytes = Vec::new();
    file.take(read::MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > read::MAX_FILE_BYTES {
        return Err(validation("El archivo supera 50 MB."));
    }
    let book = read::read(&bytes)?;
    let pending = PendingGuion {
        token: new_id(),
        file_name: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        bytes,
        book,
    };
    let profiles = guion::list_profiles(&state.db())?;
    let dto = inspection(&pending, &profiles);
    state.set_guion(pending);
    Ok(Some(dto))
}

#[tauri::command(async)]
pub fn guion_preview(
    state: State<'_, AppState>,
    token: String,
    profile: ProfileDefinition,
) -> AppResult<GuionPreviewDto> {
    state.ensure_ready()?;
    state.with_guion(&token, |pending| {
        Ok(import::plan(&pending.book, &profile).preview)
    })
}

#[tauri::command(async)]
pub fn guion_import(
    state: State<'_, AppState>,
    input: GuionImportInput,
) -> AppResult<ImportSummaryDto> {
    state.ensure_ready()?;
    let summary = state.with_guion(&input.token, |pending| {
        import::import(
            &state.db(),
            state.files(),
            import::ImportRequest {
                book: &pending.book,
                bytes: &pending.bytes,
                file_name: &pending.file_name,
                def: &input.profile,
                project_name: &input.project_name,
                save_profile_as: input.save_profile_as.as_deref(),
            },
        )
    })?;
    // with_guion ya liberó el mutex: clear_guion no vuelve a bloquearlo mientras está tomado.
    state.clear_guion();
    Ok(summary)
}

#[tauri::command(async)]
pub fn import_profiles_list(state: State<'_, AppState>) -> AppResult<Vec<ImportProfileDto>> {
    state.ensure_ready()?;
    guion::list_profiles(&state.db())
}

#[tauri::command(async)]
pub fn import_profile_delete(state: State<'_, AppState>, id: String) -> AppResult<()> {
    state.ensure_ready()?;
    guion::delete_profile(&state.db(), &id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        excel::guion_fixture,
        repo::{guion, test_support},
        transfer::guion::{profile, read},
    };

    #[test]
    fn suggests_a_profile_whose_headers_match() {
        let conn = test_support::conn();
        let profile =
            guion::create_profile(&conn, "Perfil sintético", &profile::tests::reference()).unwrap();
        let profiles = vec![profile.clone()];
        let mut book = read::read(&guion_fixture::build()).unwrap();
        assert_eq!(suggested_profile(&book, &profiles), Some(profile.id));
        book.sheets[0].rows[guion_fixture::HEADER_ROW as usize][2] =
            read::Cell::Text("otro".into());
        assert_eq!(suggested_profile(&book, &profiles), None);
    }

    #[test]
    fn inspection_limits_rows_and_columns_and_handles_missing_headers() {
        let pending = PendingGuion {
            token: "token".into(),
            file_name: "guion.xlsx".into(),
            bytes: vec![],
            book: GuionBook {
                sheets: vec![read::GuionSheet {
                    name: "Hoja".into(),
                    rows: vec![vec![read::Cell::Number(1.0); 60]; 210],
                    merged: vec![],
                }],
            },
        };
        let dto = inspection(&pending, &[]);
        assert_eq!(dto.sheets[0].rows.len(), 200);
        assert!(dto.sheets[0].rows.iter().all(|row| row.len() == 52));
        assert_eq!(dto.sheets[0].suggested_header_row, None);
        assert_eq!(dto.sheets[0].cover, profile::CoverCells::default());
        assert_eq!(dto.suggested_profile_id, None);
    }
}
