use rust_xlsxwriter::{Chart, ChartFormat, ChartSolidFill, ChartType, Worksheet};

use super::{
    data::{ExportData, Kpis},
    styles::{self, Styles},
};
use crate::{domain::status::STATUSES, dto::FieldDto, error::AppResult};

pub const SHEET: &str = "Resumen";
pub const STATUS_ROW: u32 = 3;
/// Debajo del gráfico (que ocupa unas 15 filas desde `STATUS_ROW`).
pub const GROUPS_ROW: u32 = 21;
pub const NO_VALUE: &str = "(sin valor)";
/// Tablas por grupo, si el proyecto tiene esas columnas visibles (spec §7).
const GROUPS: [&str; 2] = ["modulo", "entorno"];

pub fn write(ws: &mut Worksheet, data: &ExportData, styles: &Styles) -> AppResult<()> {
    ws.set_name(SHEET)?;
    ws.set_screen_gridlines(false);
    ws.set_column_width(0, 24)?;
    for col in 1..=7 {
        ws.set_column_width(col, 13)?;
    }
    ws.set_row_height(0, 26)?;
    ws.merge_range(0, 0, 0, 7, "Resumen de resultados", &styles.title)?;
    let p = &data.project;
    let heading = if p.client.is_empty() {
        p.name.clone()
    } else {
        format!("{} · {}", p.client, p.name)
    };
    ws.merge_range(1, 0, 1, 7, &heading, &styles.subtitle)?;

    let k = Kpis::of(data);
    for (col, title) in ["Estado", "Ejecuciones", "% del total"]
        .into_iter()
        .enumerate()
    {
        ws.write_string_with_format(STATUS_ROW, col as u16, title, &styles.header)?;
    }
    let share = |n: usize| {
        if k.total == 0 {
            0.0
        } else {
            n as f64 / k.total as f64
        }
    };
    for (i, status) in STATUSES.iter().enumerate() {
        let row = STATUS_ROW + 1 + i as u32;
        ws.write_string_with_format(row, 0, *status, &styles.value)?;
        ws.write_number_with_format(row, 1, k.by_status[i] as f64, &styles.number)?;
        ws.write_number_with_format(row, 2, share(k.by_status[i]), &styles.percent)?;
    }
    let total = STATUS_ROW + 7;
    ws.write_string_with_format(total, 0, "Total", &styles.total)?;
    ws.write_number_with_format(total, 1, k.total as f64, &styles.total)?;
    ws.write_number_with_format(total, 2, share(k.total), &styles.total)?;
    let rate = total + 1;
    ws.write_string_with_format(rate, 0, "% de éxito", &styles.label)?;
    match k.success_rate {
        Some(r) => ws.write_number_with_format(rate, 1, r, &styles.percent)?,
        None => ws.write_string_with_format(rate, 1, "—", &styles.number)?,
    };

    let mut chart = Chart::new(ChartType::Column);
    chart
        .add_series()
        .set_name("Ejecuciones")
        .set_categories((SHEET, STATUS_ROW + 1, 0, STATUS_ROW + 6, 0))
        .set_values((SHEET, STATUS_ROW + 1, 1, STATUS_ROW + 6, 1))
        .set_format(
            ChartFormat::new().set_solid_fill(ChartSolidFill::new().set_color(styles::BLUE)),
        );
    chart.title().set_name("Ejecuciones por estado");
    chart.legend().set_hidden();
    ws.insert_chart(STATUS_ROW, 4, &chart)?;

    let mut row = GROUPS_ROW;
    for key in GROUPS {
        if let Some(field) = data.field(key) {
            row = group_table(ws, row, data, field, styles)? + 3;
        }
    }
    ws.set_paper_size(9).set_print_fit_to_pages(1, 0);
    Ok(())
}

/// Conteo por estado de cada valor de la columna; devuelve la última fila usada.
fn group_table(
    ws: &mut Worksheet,
    start: u32,
    data: &ExportData,
    field: &FieldDto,
    styles: &Styles,
) -> AppResult<u32> {
    ws.write_string_with_format(start, 0, &field.label, &styles.header)?;
    for (i, status) in STATUSES.iter().enumerate() {
        ws.write_string_with_format(start, 1 + i as u16, *status, &styles.header)?;
    }
    ws.write_string_with_format(start, 7, "Total", &styles.header)?;
    ws.set_row_height(start, 28)?;
    let mut groups: Vec<(String, [usize; 6])> = Vec::new();
    for c in &data.cases {
        let raw = ExportData::value(&c.case, field).trim();
        let name = if raw.is_empty() { NO_VALUE } else { raw };
        let index = match groups.iter().position(|(g, _)| g == name) {
            Some(i) => i,
            None => {
                groups.push((name.to_string(), [0; 6]));
                groups.len() - 1
            }
        };
        if let Some(s) = STATUSES.iter().position(|s| *s == c.case.status) {
            groups[index].1[s] += 1;
        }
    }
    // Alfabético sin distinguir mayúsculas; «(sin valor)» siempre al final.
    groups.sort_by_key(|(name, _)| (name == NO_VALUE, name.to_lowercase()));
    let mut row = start;
    for (name, counts) in &groups {
        row += 1;
        ws.write_string_with_format(row, 0, name, &styles.value)?;
        for (i, n) in counts.iter().enumerate() {
            ws.write_number_with_format(row, 1 + i as u16, *n as f64, &styles.number)?;
        }
        ws.write_number_with_format(row, 7, counts.iter().sum::<usize>() as f64, &styles.total)?;
    }
    Ok(row)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::excel::testing::{Book, case, fixture};
    use calamine::Data;

    fn render(data: &ExportData) -> Book {
        let mut workbook = styles::workbook().unwrap();
        write(workbook.add_worksheet(), data, &Styles::new()).unwrap();
        Book::save(&mut workbook)
    }

    #[test]
    fn status_table_with_totals_rate_and_a_native_chart() {
        let book = render(&fixture());
        assert_eq!(
            book.row_texts(SHEET, STATUS_ROW, 3),
            ["Estado", "Ejecuciones", "% del total"]
        );
        assert_eq!(book.text(SHEET, STATUS_ROW + 4, 0), "Aprobado");
        assert_eq!(book.cell(SHEET, STATUS_ROW + 4, 1), Data::Float(2.0));
        assert_eq!(book.cell(SHEET, STATUS_ROW + 4, 2), Data::Float(0.5));
        assert_eq!(book.text(SHEET, STATUS_ROW + 7, 0), "Total");
        assert_eq!(book.cell(SHEET, STATUS_ROW + 7, 1), Data::Float(4.0));
        assert_eq!(book.cell(SHEET, STATUS_ROW + 8, 1), Data::Float(2.0 / 3.0));
        assert_eq!(book.charts(SHEET), 1);
        let chart = book.part("xl/charts/chart1.xml");
        assert!(chart.contains("<c:barDir val=\"col\"/>"));
        assert!(chart.contains("Resumen!$A$5:$A$10"));
        assert!(chart.contains("Resumen!$B$5:$B$10"));
    }

    #[test]
    fn groups_by_module_and_environment_with_blank_values_last() {
        let book = render(&fixture());
        assert_eq!(book.text(SHEET, GROUPS_ROW, 0), "Módulo");
        assert_eq!(book.text(SHEET, GROUPS_ROW, 7), "Total");
        let modules: Vec<_> = (1..=3)
            .map(|i| book.text(SHEET, GROUPS_ROW + i, 0))
            .collect();
        assert_eq!(modules, ["Cobranzas", "Pólizas", NO_VALUE]);
        assert_eq!(
            book.cell(SHEET, GROUPS_ROW + 1, 4),
            Data::Float(2.0),
            "Cobranzas · Aprobado"
        );
        assert_eq!(book.cell(SHEET, GROUPS_ROW + 1, 7), Data::Float(2.0));
        let env = GROUPS_ROW + 3 + 3;
        assert_eq!(book.text(SHEET, env, 0), "Entorno");
        assert_eq!(book.text(SHEET, env + 1, 0), "QA");
        assert_eq!(book.cell(SHEET, env + 1, 7), Data::Float(3.0));
    }

    #[test]
    fn without_those_columns_there_are_no_group_tables() {
        let mut data = fixture();
        data.project
            .fields
            .retain(|f| f.key != "modulo" && f.key != "entorno");
        let book = render(&data);
        assert_eq!(book.text(SHEET, GROUPS_ROW, 0), "");
    }

    #[test]
    fn an_empty_project_has_zero_shares_and_no_rate() {
        let mut data = fixture();
        data.cases.clear();
        let book = render(&data);
        assert_eq!(book.cell(SHEET, STATUS_ROW + 1, 2), Data::Float(0.0));
        assert_eq!(book.text(SHEET, STATUS_ROW + 8, 1), "—");
        data.cases
            .push(case(&data.project, "EJ-001", "Bloqueado", &[]));
        assert_eq!(
            render(&data).cell(SHEET, STATUS_ROW + 8, 1),
            Data::Float(0.0)
        );
    }
}
