use chrono::Datelike;
use rust_xlsxwriter::{
    ConditionalFormatCell, ConditionalFormatCellRule, DataValidation, ExcelDateTime, Note, Url,
    Worksheet,
};

use super::{
    data::{ExportCase, ExportData, parse_iso},
    evidence_sheet,
    styles::{RowStyles, STATUS_COLORS, Styles, status_format, wrapped_height},
};
use crate::{
    domain::{fields::FieldType, status::STATUSES},
    dto::FieldDto,
    error::AppResult,
};

pub const SHEET: &str = "Matriz";
pub const HEADER_ROW: u32 = 3;
pub const FIRST_ROW: u32 = 4;
/// Filas con formato y listas desplegables que trae la plantilla vacía.
pub const TEMPLATE_ROWS: u32 = 100;
/// Excel no acepta listas de validación de más de 255 caracteres en total.
const MAX_LIST_CHARS: usize = 255;

pub fn row_of(case_index: usize) -> u32 {
    FIRST_ROW + case_index as u32
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Column<'a> {
    Code,
    Title,
    Field(&'a FieldDto),
    Status,
    Evidence,
    Issue,
}

impl Column<'_> {
    pub fn label(&self) -> &str {
        match self {
            Self::Code => "ID",
            Self::Title => "Caso de prueba",
            Self::Field(f) => &f.label,
            Self::Status => "Estado",
            Self::Evidence => "Evidencias",
            Self::Issue => "Issue",
        }
    }

    fn width(&self) -> f64 {
        match self {
            Self::Code => 12.0,
            Self::Title => 36.0,
            Self::Field(f) => f.width.clamp(6, 80) as f64,
            Self::Status => 16.0,
            Self::Evidence | Self::Issue => 22.0,
        }
    }
}

/// ID · Caso de prueba · columnas visibles del proyecto (en su orden) · Estado · Evidencias
/// · Issue (solo si algún caso tiene issues vinculados).
pub fn columns(data: &ExportData) -> Vec<Column<'_>> {
    let mut cols = vec![Column::Code, Column::Title];
    cols.extend(data.project.fields.iter().map(Column::Field));
    cols.extend([Column::Status, Column::Evidence]);
    if data.cases.iter().any(|c| !c.issues.is_empty()) {
        cols.push(Column::Issue);
    }
    cols
}

/// `links`: fila donde empieza cada caso en la hoja Evidencias (`None` si el libro no la incluye).
pub fn write(
    ws: &mut Worksheet,
    data: &ExportData,
    links: Option<&[u32]>,
    template: bool,
    styles: &Styles,
) -> AppResult<()> {
    ws.set_name(SHEET)?;
    ws.set_screen_gridlines(false);
    let mut cols = columns(data);
    if template {
        cols.retain(|c| *c != Column::Issue);
    }
    let last_col = (cols.len() - 1) as u16;
    for (i, c) in cols.iter().enumerate() {
        ws.set_column_width(i as u16, c.width())?;
    }
    let p = &data.project;
    let title = if p.client.is_empty() {
        p.name.clone()
    } else {
        format!("{} · {}", p.client, p.name)
    };
    ws.set_row_height(0, 26)?;
    ws.merge_range(0, 0, 0, last_col, &title, &styles.title)?;
    ws.merge_range(
        1,
        0,
        1,
        last_col,
        "Una fila por ejecución y entorno. «Ver evidencias» lleva a las capturas del caso.",
        &styles.subtitle,
    )?;
    ws.set_row_height(HEADER_ROW, 32)?;
    for (i, c) in cols.iter().enumerate() {
        ws.write_string_with_format(HEADER_ROW, i as u16, c.label(), &styles.header)?;
    }

    let rows = if template {
        TEMPLATE_ROWS
    } else {
        data.cases.len().max(1) as u32
    };
    let last_row = FIRST_ROW + rows - 1;
    if template {
        for i in 0..TEMPLATE_ROWS as usize {
            for col in 0..=last_col {
                ws.write_blank(row_of(i), col, &styles.row(i).text)?;
            }
        }
    } else if data.cases.is_empty() {
        ws.merge_range(
            FIRST_ROW,
            0,
            FIRST_ROW,
            last_col,
            "No hay casos registrados.",
            &styles.row(0).text,
        )?;
    } else {
        for (i, c) in data.cases.iter().enumerate() {
            write_case(ws, i, c, &cols, links, styles)?;
        }
    }
    validations(ws, &cols, last_row)?;
    ws.autofilter(HEADER_ROW, 0, last_row, last_col)?;
    ws.set_freeze_panes(FIRST_ROW, 2)?;
    ws.set_landscape()
        .set_paper_size(9)
        .set_print_fit_to_pages(1, 0);
    ws.set_repeat_rows(HEADER_ROW, HEADER_ROW)?;
    ws.set_footer("&C&\"Arial\"&8Página &P de &N");
    Ok(())
}

fn write_case(
    ws: &mut Worksheet,
    index: usize,
    c: &ExportCase,
    cols: &[Column],
    links: Option<&[u32]>,
    styles: &Styles,
) -> AppResult<()> {
    let row = row_of(index);
    let s = styles.row(index);
    let mut texts: Vec<(&str, f64)> = Vec::new();
    for (i, col) in cols.iter().enumerate() {
        let at = i as u16;
        match col {
            Column::Code => {
                ws.write_string_with_format(row, at, &c.case.code, &s.text)?;
            }
            Column::Title => {
                ws.write_string_with_format(row, at, &c.case.title, &s.text)?;
                texts.push((&c.case.title, col.width()));
            }
            Column::Status => {
                ws.write_string_with_format(row, at, &c.case.status, &s.text)?;
            }
            Column::Field(f) => {
                let value = ExportData::value(&c.case, f);
                write_value(ws, row, at, f, value, s)?;
                if f.field_type.is_textual() {
                    texts.push((value, col.width()));
                }
            }
            Column::Issue => match c.issues.as_slice() {
                [] => {
                    ws.write_blank(row, at, &s.text)?;
                }
                [(text, url)] => {
                    let link = Url::new(url).set_text(text);
                    ws.write_url_with_format(row, at, link, &s.link)?;
                }
                many => {
                    let text: Vec<&str> = many.iter().map(|(t, _)| t.as_str()).collect();
                    ws.write_string_with_format(row, at, text.join(", "), &s.text)?;
                    let urls: Vec<&str> = many.iter().map(|(_, u)| u.as_str()).collect();
                    ws.insert_note(row, at, &Note::new(urls.join("\n")))?;
                }
            },
            Column::Evidence => {
                let count = c.evidence.len();
                match links.and_then(|l| l.get(index)) {
                    Some(target) => {
                        let text = if count == 0 {
                            "Ver sección".to_string()
                        } else {
                            format!("Ver evidencias ({count})")
                        };
                        let url = Url::new(format!(
                            "internal:'{}'!A{}",
                            evidence_sheet::SHEET,
                            target + 1
                        ))
                        .set_text(text);
                        ws.write_url_with_format(row, at, url, &s.link)?;
                    }
                    None => {
                        ws.write_number_with_format(row, at, count as f64, &s.number)?;
                    }
                }
            }
        }
    }
    ws.set_row_height(row, wrapped_height(texts, 18.0))?;
    Ok(())
}

/// Fechas como fecha real de Excel, números como número y sí/no como texto; si un valor
/// viejo no cumple su tipo se escribe tal cual, sin perderlo.
fn write_value(
    ws: &mut Worksheet,
    row: u32,
    col: u16,
    field: &FieldDto,
    value: &str,
    s: &RowStyles,
) -> AppResult<()> {
    match field.field_type {
        FieldType::Date => {
            if let Some(date) = parse_iso(value).and_then(|d| {
                ExcelDateTime::from_ymd(d.year() as u16, d.month() as u8, d.day() as u8).ok()
            }) {
                ws.write_datetime_with_format(row, col, &date, &s.date)?;
                return Ok(());
            }
        }
        FieldType::Number => {
            if let Some(n) = value.parse::<f64>().ok().filter(|n| n.is_finite()) {
                ws.write_number_with_format(row, col, n, &s.number)?;
                return Ok(());
            }
        }
        FieldType::Boolean => {
            let text = match value {
                "true" => "Sí",
                "false" => "No",
                other => other,
            };
            ws.write_string_with_format(row, col, text, &s.text)?;
            return Ok(());
        }
        _ => {}
    }
    if value.is_empty() {
        ws.write_blank(row, col, &s.text)?;
    } else {
        ws.write_string_with_format(row, col, value, &s.text)?;
    }
    Ok(())
}

fn fits_in_list(options: &[String]) -> bool {
    let chars: usize =
        options.iter().map(|o| o.chars().count()).sum::<usize>() + options.len().saturating_sub(1);
    chars <= MAX_LIST_CHARS && options.iter().all(|o| !o.contains(','))
}

fn validations(ws: &mut Worksheet, cols: &[Column], last_row: u32) -> AppResult<()> {
    for (i, col) in cols.iter().enumerate() {
        let at = i as u16;
        let list: Option<Vec<String>> = match col {
            Column::Status => Some(STATUSES.iter().map(|s| s.to_string()).collect()),
            Column::Field(f) if f.field_type == FieldType::List && !f.options.is_empty() => {
                Some(f.options.clone())
            }
            Column::Field(f) if f.field_type == FieldType::Boolean => {
                Some(vec!["Sí".into(), "No".into()])
            }
            _ => None,
        };
        if let Some(options) = list.filter(|o| fits_in_list(o)) {
            let rule = DataValidation::new()
                .allow_list_strings(&options)?
                .set_error_title("Valor no válido")?
                .set_error_message("Elige un valor de la lista.")?;
            ws.add_data_validation(FIRST_ROW, at, last_row, at, &rule)?;
        }
        if *col == Column::Status {
            for (status, fill, font) in STATUS_COLORS {
                let rule = ConditionalFormatCell::new()
                    .set_rule(ConditionalFormatCellRule::EqualTo(format!("\"{status}\"")))
                    .set_format(status_format(fill, font));
                ws.add_conditional_format(FIRST_ROW, at, last_row, at, &rule)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::excel::{
        styles,
        testing::{Book, fixture},
    };
    use calamine::{Data, DataType};

    fn with_issues(case: usize, issues: &[(&str, &str)]) -> ExportData {
        let mut data = fixture();
        data.cases[case].issues = issues
            .iter()
            .map(|(t, u)| (t.to_string(), u.to_string()))
            .collect();
        data
    }

    fn sheet_rels(book: &Book) -> String {
        book.part("xl/worksheets/_rels/sheet1.xml.rels")
    }

    #[test]
    fn no_links_no_issue_column() {
        let book = render(&fixture(), None, false);
        assert_eq!(book.row_texts(SHEET, HEADER_ROW, 12), HEADERS);
        assert_eq!(book.cell(SHEET, HEADER_ROW, 12), Data::Empty);
        let data = with_issues(0, &[("app#1", "https://github.com/o/app/issues/1")]);
        let template = render(&data, None, true);
        assert_eq!(template.cell(SHEET, HEADER_ROW, 12), Data::Empty);
    }

    #[test]
    fn one_link_is_a_hyperlink_with_repo_and_number() {
        let data = with_issues(1, &[("app#7", "https://github.com/o/app/issues/7")]);
        let book = render(&data, None, false);
        assert_eq!(book.text(SHEET, row_of(1), 12), "app#7");
        assert_eq!(book.text(SHEET, row_of(0), 12), "");
        assert!(sheet_rels(&book).contains("https://github.com/o/app/issues/7"));
        assert!(!book.has_part("xl/comments1.xml"));
    }

    #[test]
    fn several_links_are_text_with_a_note_listing_urls() {
        let data = with_issues(
            0,
            &[
                ("app#7", "https://github.com/o/app/issues/7"),
                ("web#2", "https://github.com/o/web/issues/2"),
            ],
        );
        let book = render(&data, None, false);
        assert_eq!(book.text(SHEET, row_of(0), 12), "app#7, web#2");
        assert!(!sheet_rels(&book).contains("github.com"));
        let note = book.part("xl/comments1.xml");
        assert!(note.contains("https://github.com/o/app/issues/7"));
        assert!(note.contains("https://github.com/o/web/issues/2"));
    }

    #[test]
    fn issue_column_goes_after_evidencias() {
        let data = with_issues(0, &[("app#7", "https://github.com/o/app/issues/7")]);
        let book = render(&data, None, false);
        let mut headers = HEADERS.to_vec();
        headers.push("Issue");
        assert_eq!(book.row_texts(SHEET, HEADER_ROW, 13), headers);
    }

    fn render(data: &ExportData, links: Option<&[u32]>, template: bool) -> Book {
        let mut workbook = styles::workbook().unwrap();
        write(
            workbook.add_worksheet(),
            data,
            links,
            template,
            &Styles::new(),
        )
        .unwrap();
        Book::save(&mut workbook)
    }

    const HEADERS: [&str; 12] = [
        "ID",
        "Caso de prueba",
        "Entorno",
        "Módulo",
        "Resultado esperado",
        "Resultado obtenido",
        "Fecha de ejecución",
        "Severidad",
        "Aceptado",
        "Horas",
        "Estado",
        "Evidencias",
    ];

    #[test]
    fn headers_are_the_visible_columns_in_order_and_one_row_per_case() {
        let data = fixture();
        let book = render(&data, None, false);
        assert_eq!(book.row_texts(SHEET, HEADER_ROW, 12), HEADERS);
        assert_eq!(book.text(SHEET, 0, 0), "ACME · Recibos");
        assert_eq!(book.height(SHEET), FIRST_ROW + 4);
        let codes: Vec<_> = (0..4).map(|i| book.text(SHEET, row_of(i), 0)).collect();
        assert_eq!(codes, ["EJ-001", "EJ-002", "EJ-003", "EJ-004"]);
        assert_eq!(book.text(SHEET, row_of(2), 10), "Fallido");
    }

    #[test]
    fn values_keep_their_excel_types() {
        let book = render(&fixture(), None, false);
        let row = row_of(0);
        let date = book.cell(SHEET, row, 6);
        assert_eq!(
            date.get_datetime().map(|d| d.as_f64()),
            Some(46311.0),
            "16/10/2026"
        );
        assert_eq!(book.cell(SHEET, row, 9), Data::Float(1.5));
        assert_eq!(book.text(SHEET, row, 8), "Sí");
        assert_eq!(book.text(SHEET, row, 7), "Alta");
        assert_eq!(book.text(SHEET, row, 4), "Se aplica el recibo");
        assert_eq!(book.cell(SHEET, row_of(1), 6), Data::Empty);
    }

    #[test]
    fn values_that_do_not_match_their_type_are_kept_as_text() {
        let mut data = fixture();
        let values = &mut data.cases[0].case.values;
        values.insert("fld-fecha_ejecucion".into(), "ayer".into());
        values.insert("fld-horas".into(), "NaN".into());
        data.cases[1]
            .case
            .values
            .insert("fld-fecha_ejecucion".into(), "0001-01-01".into());
        let book = render(&data, None, false);
        assert_eq!(book.text(SHEET, row_of(0), 6), "ayer");
        assert_eq!(book.text(SHEET, row_of(0), 9), "NaN");
        assert_eq!(book.text(SHEET, row_of(1), 6), "0001-01-01");
    }

    #[test]
    fn evidence_column_links_to_each_case_block() {
        let book = render(&fixture(), Some(&[3, 20, 40, 50]), false);
        assert_eq!(book.text(SHEET, row_of(0), 11), "Ver evidencias (2)");
        assert_eq!(book.text(SHEET, row_of(2), 11), "Ver sección");
        let links = book.hyperlinks(SHEET);
        assert_eq!(links.len(), 4);
        assert_eq!(links[0], ("L5".to_string(), "'Evidencias'!A4".to_string()));
        assert_eq!(links[3], ("L8".to_string(), "'Evidencias'!A51".to_string()));
    }

    #[test]
    fn without_evidence_sheet_the_column_is_a_count() {
        let book = render(&fixture(), None, false);
        assert_eq!(book.cell(SHEET, row_of(0), 11), Data::Float(2.0));
        assert!(book.hyperlinks(SHEET).is_empty());
    }

    #[test]
    fn lists_status_colors_filter_freeze_and_print_setup() {
        let book = render(&fixture(), None, false);
        let xml = book.sheet_xml(SHEET);
        assert_eq!(
            xml.matches("<dataValidation ").count(),
            3,
            "Severidad, Aceptado y Estado"
        );
        assert!(xml.contains("sqref=\"K5:K8\""));
        assert!(xml.contains(r#"<formula1>"No ejecutado,En ejecución,Por documentar,Aprobado,Fallido,Bloqueado"</formula1>"#));
        assert!(xml.contains(r#"<formula1>"Sí,No"</formula1>"#));
        assert_eq!(xml.matches("<cfRule ").count(), 3);
        assert!(xml.contains("<autoFilter ref=\"A4:L8\""));
        assert!(xml.contains("xSplit=\"2\" ySplit=\"4\""));
        assert!(xml.contains("state=\"frozen\""));
        assert!(xml.contains("orientation=\"landscape\""));
        assert!(xml.contains("paperSize=\"9\""));
        assert!(xml.contains("fitToHeight=\"0\""));
        assert!(book.part("xl/workbook.xml").contains("Matriz!$4:$4"));
    }

    #[test]
    fn long_or_comma_lists_skip_the_dropdown() {
        let mut data = fixture();
        let severidad = data
            .project
            .fields
            .iter_mut()
            .find(|f| f.key == "severidad")
            .unwrap();
        severidad.options = (0..30).map(|i| format!("Opción número {i}")).collect();
        let book = render(&data, None, false);
        assert_eq!(book.sheet_xml(SHEET).matches("<dataValidation ").count(), 2);
        let severidad = data
            .project
            .fields
            .iter_mut()
            .find(|f| f.key == "severidad")
            .unwrap();
        severidad.options = vec!["Alta, urgente".into(), "Baja".into()];
        let book = render(&data, None, false);
        assert_eq!(book.sheet_xml(SHEET).matches("<dataValidation ").count(), 2);
    }

    #[test]
    fn a_project_without_cases_says_so() {
        let mut data = fixture();
        data.cases.clear();
        let book = render(&data, None, false);
        assert_eq!(book.text(SHEET, FIRST_ROW, 0), "No hay casos registrados.");
        assert!(book.sheet_xml(SHEET).contains("<autoFilter ref=\"A4:L5\""));
    }

    #[test]
    fn template_has_headers_and_empty_formatted_rows() {
        let book = render(&fixture(), None, true);
        assert_eq!(book.row_texts(SHEET, HEADER_ROW, 12), HEADERS);
        assert_eq!(book.text(SHEET, FIRST_ROW, 0), "");
        let xml = book.sheet_xml(SHEET);
        assert!(xml.contains("sqref=\"K5:K104\""));
        assert!(xml.contains("<autoFilter ref=\"A4:L104\""));
    }
}
