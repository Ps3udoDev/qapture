use rust_xlsxwriter::{Image, Worksheet};

use super::{
    data::{ExportData, ExportSettings, Kpis, format_dmy, parse_iso, period},
    styles::{self, Styles, wrapped_height},
};
use crate::{domain::status::STATUSES, dto::DocVersionDto, error::AppResult};

pub const SHEET: &str = "Carátula";
pub const FOOTER: &str = "Teams4Soft · Inteligencia Híbrida · Confidencial";
pub const TITLE_ROW: u32 = 7;
/// Fila de la banda «Datos del proyecto»; los 8 datos van debajo.
pub const DATA_ROW: u32 = 10;

// El logo y el banner van dentro del binario (spec §7): el Excel no depende de archivos externos.
const LOGO: &[u8] = include_bytes!("../../../src/assets/brand/t4s-logo.png");
const BANNER: &[u8] = include_bytes!("../../../src/assets/brand/t4s-network-banner.png");
const LAST_COL: u16 = 7;
const WIDTHS: [f64; 8] = [16.0, 14.0, 18.0, 18.0, 18.0, 14.0, 14.0, 14.0];

pub fn write(
    ws: &mut Worksheet,
    data: &ExportData,
    settings: &ExportSettings,
    styles: &Styles,
) -> AppResult<()> {
    ws.set_name(SHEET)?;
    ws.set_screen_gridlines(false);
    ws.set_tab_color(styles::BLUE);
    for (col, width) in WIDTHS.iter().enumerate() {
        ws.set_column_width(col as u16, *width)?;
    }
    let logo = Image::new_from_buffer(LOGO)?
        .set_scale_to_size(80, 92, true)
        .set_alt_text("Teams4Soft · Inteligencia Híbrida");
    ws.insert_image_with_offset(0, 0, &logo, 8, 8)?;
    let banner = Image::new_from_buffer(BANNER)?
        .set_scale_to_size(430, 140, true)
        .set_decorative(true);
    ws.insert_image(0, 4, &banner)?;

    let title = if settings.template {
        "Plantilla de pruebas"
    } else {
        "Matriz de pruebas y evidencias"
    };
    ws.set_row_height(TITLE_ROW, 30)?;
    ws.merge_range(TITLE_ROW, 0, TITLE_ROW, LAST_COL, title, &styles.title)?;
    let p = &data.project;
    let heading = if p.client.is_empty() {
        p.name.clone()
    } else {
        format!("{} · {}", p.client, p.name)
    };
    ws.merge_range(
        TITLE_ROW + 1,
        0,
        TITLE_ROW + 1,
        LAST_COL,
        &heading,
        &styles.subtitle,
    )?;

    section(ws, DATA_ROW, "Datos del proyecto", styles)?;
    let environments = data.distinct_values("entorno").join(", ");
    let dates = period(&p.start_date, &p.end_date);
    let generated = format_dmy(settings.today);
    let rows = [
        ("Cliente", p.client.as_str()),
        ("Proyecto / Módulo", p.name.as_str()),
        ("Tipo de prueba", p.test_type.as_str()),
        ("Versión / release probado", p.release_version.as_str()),
        ("Entorno(s)", environments.as_str()),
        ("Periodo", dates.as_str()),
        ("Tester", settings.tester.as_str()),
        ("Fecha de generación", generated.as_str()),
    ];
    let mut row = DATA_ROW;
    for (label, value) in rows {
        row += 1;
        ws.set_row_height(row, wrapped_height([(value, 90.0)], 20.0))?;
        ws.merge_range(row, 0, row, 1, label, &styles.label)?;
        ws.merge_range(row, 2, row, LAST_COL, or_dash(value), &styles.value)?;
    }
    row += 2;
    if !settings.template {
        row = kpis(ws, row, data, styles)? + 2;
    }
    row = versions(ws, row, &data.versions, styles)? + 2;
    ws.merge_range(row, 0, row, LAST_COL, FOOTER, &styles.footer)?;
    ws.set_footer(format!("&C&\"Arial\"&8{FOOTER}"));
    ws.set_paper_size(9).set_print_fit_to_pages(1, 1);
    Ok(())
}

fn or_dash(value: &str) -> &str {
    if value.trim().is_empty() {
        "—"
    } else {
        value
    }
}

fn section(ws: &mut Worksheet, row: u32, title: &str, styles: &Styles) -> AppResult<()> {
    ws.set_row_height(row, 22)?;
    ws.merge_range(row, 0, row, LAST_COL, title, &styles.section)?;
    Ok(())
}

/// Bloque de KPI; devuelve la última fila que usa.
fn kpis(ws: &mut Worksheet, start: u32, data: &ExportData, styles: &Styles) -> AppResult<u32> {
    let k = Kpis::of(data);
    section(ws, start, "Resumen de resultados", styles)?;
    let (labels, values) = (start + 1, start + 2);
    ws.set_row_height(values, 34)?;
    let incidents = (k.count("Fallido") + k.count("Bloqueado")) as f64;
    let tiles = [
        ("Ejecuciones", Some(k.total as f64), &styles.kpi_value),
        ("% de éxito", k.success_rate, &styles.kpi_rate),
        ("Evidencias", Some(k.evidence as f64), &styles.kpi_value),
        ("Con incidencias", Some(incidents), &styles.kpi_value),
    ];
    for (i, (label, value, format)) in tiles.into_iter().enumerate() {
        let col = (i * 2) as u16;
        ws.merge_range(labels, col, labels, col + 1, label, &styles.kpi_label)?;
        ws.merge_range(values, col, values, col + 1, "", format)?;
        match value {
            Some(v) => ws.write_number_with_format(values, col, v, format)?,
            None => ws.write_string_with_format(values, col, "—", format)?,
        };
    }
    let (header, counts) = (start + 4, start + 5);
    ws.set_row_height(header, 28)?;
    ws.merge_range(header, 0, counts, 1, "Por estado", &styles.label)?;
    for (i, status) in STATUSES.iter().enumerate() {
        let col = 2 + i as u16;
        ws.write_string_with_format(header, col, *status, &styles.header)?;
        ws.write_number_with_format(counts, col, k.by_status[i] as f64, &styles.number)?;
    }
    Ok(counts)
}

/// Tabla de control de versiones (UC-05); devuelve la última fila que usa.
fn versions(
    ws: &mut Worksheet,
    start: u32,
    versions: &[DocVersionDto],
    styles: &Styles,
) -> AppResult<u32> {
    section(ws, start, "Control de versiones", styles)?;
    let head = start + 1;
    for (col, title) in ["Versión", "Fecha", "Elaborado", "Revisado", "Aprobado"]
        .into_iter()
        .enumerate()
    {
        ws.write_string_with_format(head, col as u16, title, &styles.header)?;
    }
    ws.merge_range(head, 5, head, LAST_COL, "Cambios", &styles.header)?;
    if versions.is_empty() {
        ws.merge_range(
            head + 1,
            0,
            head + 1,
            LAST_COL,
            "Sin versiones registradas.",
            &styles.value,
        )?;
        return Ok(head + 1);
    }
    let mut row = head;
    for v in versions {
        row += 1;
        let date = parse_iso(&v.date).map_or_else(|| v.date.clone(), format_dmy);
        let cells = [
            v.version.as_str(),
            date.as_str(),
            v.author.as_str(),
            v.reviewer.as_str(),
            v.approver.as_str(),
        ];
        for (col, text) in cells.into_iter().enumerate() {
            ws.write_string_with_format(row, col as u16, or_dash(text), &styles.value)?;
        }
        ws.merge_range(row, 5, row, LAST_COL, or_dash(&v.changes), &styles.value)?;
        ws.set_row_height(row, wrapped_height([(v.changes.as_str(), 42.0)], 20.0))?;
    }
    Ok(row)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::excel::testing::{Book, fixture, version};
    use calamine::Data;
    use chrono::NaiveDate;

    fn settings(template: bool) -> ExportSettings {
        ExportSettings {
            tester: "Ana Pérez".into(),
            version: None,
            include_evidence: true,
            include_summary: true,
            image_max_width: 940,
            today: NaiveDate::from_ymd_opt(2026, 10, 16).unwrap(),
            template,
        }
    }

    fn render(data: &ExportData, template: bool) -> Book {
        let mut workbook = styles::workbook().unwrap();
        write(
            workbook.add_worksheet(),
            data,
            &settings(template),
            &Styles::new(),
        )
        .unwrap();
        Book::save(&mut workbook)
    }

    #[test]
    fn shows_logo_banner_title_and_project_data() {
        let book = render(&fixture(), false);
        assert_eq!(book.sheet_names(), [SHEET]);
        assert_eq!(book.pictures(SHEET), 2);
        assert_eq!(
            book.text(SHEET, TITLE_ROW, 0),
            "Matriz de pruebas y evidencias"
        );
        assert_eq!(book.text(SHEET, TITLE_ROW + 1, 0), "ACME · Recibos");
        let data: Vec<_> = (1..=8)
            .map(|i| {
                (
                    book.text(SHEET, DATA_ROW + i, 0),
                    book.text(SHEET, DATA_ROW + i, 2),
                )
            })
            .collect();
        let expected = [
            ("Cliente", "ACME"),
            ("Proyecto / Módulo", "Recibos"),
            ("Tipo de prueba", "Funcional"),
            ("Versión / release probado", "2.4.0"),
            ("Entorno(s)", "QA, UAT"),
            ("Periodo", "01/10/2026 – 31/10/2026"),
            ("Tester", "Ana Pérez"),
            ("Fecha de generación", "16/10/2026"),
        ];
        for ((label, value), (exp_label, exp_value)) in data.iter().zip(expected) {
            assert_eq!((label.as_str(), value.as_str()), (exp_label, exp_value));
        }
    }

    #[test]
    fn kpis_show_totals_rate_evidence_and_each_status() {
        let book = render(&fixture(), false);
        let start = book.find_row(SHEET, "Resumen de resultados").unwrap();
        assert_eq!(
            book.row_texts(SHEET, start + 1, 8),
            [
                "Ejecuciones",
                "",
                "% de éxito",
                "",
                "Evidencias",
                "",
                "Con incidencias",
                ""
            ]
        );
        assert_eq!(book.cell(SHEET, start + 2, 0), Data::Float(4.0));
        assert_eq!(book.cell(SHEET, start + 2, 2), Data::Float(2.0 / 3.0));
        assert_eq!(book.cell(SHEET, start + 2, 4), Data::Float(3.0));
        assert_eq!(book.cell(SHEET, start + 2, 6), Data::Float(1.0));
        assert_eq!(book.text(SHEET, start + 4, 2), "No ejecutado");
        assert_eq!(book.cell(SHEET, start + 5, 2), Data::Float(1.0));
        assert_eq!(book.text(SHEET, start + 4, 5), "Aprobado");
        assert_eq!(book.cell(SHEET, start + 5, 5), Data::Float(2.0));
    }

    #[test]
    fn rate_is_a_dash_without_verdicts() {
        let mut data = fixture();
        data.cases.retain(|c| c.case.status == "No ejecutado");
        let book = render(&data, false);
        let start = book.find_row(SHEET, "Resumen de resultados").unwrap();
        assert_eq!(book.text(SHEET, start + 2, 2), "—");
    }

    #[test]
    fn version_table_lists_the_history_and_footer_closes_the_page() {
        let mut data = fixture();
        data.versions = vec![
            version("v1", "1.0", "2026-10-01"),
            version("v2", "1.1", "2026-10-15"),
        ];
        let book = render(&data, false);
        let start = book.find_row(SHEET, "Control de versiones").unwrap();
        assert_eq!(
            book.row_texts(SHEET, start + 1, 6),
            [
                "Versión",
                "Fecha",
                "Elaborado",
                "Revisado",
                "Aprobado",
                "Cambios"
            ]
        );
        assert_eq!(
            book.row_texts(SHEET, start + 2, 6),
            [
                "1.0",
                "01/10/2026",
                "Ana Pérez",
                "Luis Gómez",
                "—",
                "Versión inicial"
            ]
        );
        assert_eq!(book.text(SHEET, start + 3, 0), "1.1");
        assert_eq!(book.text(SHEET, start + 5, 0), FOOTER);
        assert!(
            book.sheet_xml(SHEET)
                .contains("Teams4Soft · Inteligencia Híbrida · Confidencial</oddFooter>")
        );
    }

    #[test]
    fn empty_values_show_a_dash_and_no_versions_is_explicit() {
        let mut data = fixture();
        data.project.client = String::new();
        data.project.start_date = String::new();
        data.project.end_date = String::new();
        let book = render(&data, false);
        assert_eq!(book.text(SHEET, TITLE_ROW + 1, 0), "Recibos");
        assert_eq!(book.text(SHEET, DATA_ROW + 1, 2), "—");
        assert_eq!(book.text(SHEET, DATA_ROW + 6, 2), "—");
        let start = book.find_row(SHEET, "Control de versiones").unwrap();
        assert_eq!(book.text(SHEET, start + 2, 0), "Sin versiones registradas.");
    }

    #[test]
    fn template_cover_has_no_results() {
        let book = render(&fixture(), true);
        assert_eq!(book.text(SHEET, TITLE_ROW, 0), "Plantilla de pruebas");
        assert!(book.find_row(SHEET, "Resumen de resultados").is_none());
        assert!(book.find_row(SHEET, "Control de versiones").is_some());
    }
}
