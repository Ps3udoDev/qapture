use rust_xlsxwriter::{Format, FormatAlign, FormatBorder, FormatUnderline, Workbook};

use crate::error::AppResult;

// Paleta Teams4Soft (spec §1). Todo el libro usa Arial: es segura en Excel y WPS.
pub const FONT: &str = "Arial";
pub const BLUE: &str = "#2B629E";
pub const LIME: &str = "#BAC82D";
pub const CHARCOAL: &str = "#292B2D";
pub const CREAM: &str = "#F6F4F1";
pub const GRAY: &str = "#596069";
pub const LINE: &str = "#E3E0DA";
pub const WHITE: &str = "#FFFFFF";

/// Alto por defecto de una fila con Arial 10, en píxeles (12,75 pt).
pub const ROW_PX: u32 = 17;

pub const DATE_FORMAT: &str = "dd/mm/yyyy";
pub const PERCENT_FORMAT: &str = "0 %";

/// Formato condicional de la columna Estado (spec §7): Aprobado verde, Fallido rojo, Bloqueado amarillo.
pub const STATUS_COLORS: [(&str, &str, &str); 3] = [
    ("Aprobado", "#C6EFCE", "#006100"),
    ("Fallido", "#FFC7CE", "#9C0006"),
    ("Bloqueado", "#FFEB9C", "#9C5700"),
];

pub fn status_format(fill: &str, font: &str) -> Format {
    Format::new()
        .set_background_color(fill)
        .set_font_color(font)
}

/// Formatos de una fila de la matriz; las filas alternas llevan fondo crema.
#[derive(Clone)]
pub struct RowStyles {
    pub text: Format,
    pub date: Format,
    pub number: Format,
    pub link: Format,
}

pub struct Styles {
    pub title: Format,
    pub subtitle: Format,
    pub section: Format,
    pub header: Format,
    pub label: Format,
    pub value: Format,
    pub kpi_label: Format,
    pub kpi_value: Format,
    pub kpi_rate: Format,
    pub band: Format,
    pub band_soft: Format,
    pub text: Format,
    pub link: Format,
    pub number: Format,
    pub percent: Format,
    pub total: Format,
    pub footer: Format,
    rows: [RowStyles; 2],
}

/// Libro con Arial 10 como formato por defecto, también en las celdas vacías que el usuario
/// llene después. Arial 10 usa 7 px por dígito: ancho de columna por defecto 64 px.
pub fn workbook() -> AppResult<Workbook> {
    let mut workbook = Workbook::new();
    workbook.set_default_format(
        &Format::new().set_font_name(FONT).set_font_size(10),
        ROW_PX,
        64,
    )?;
    Ok(workbook)
}

fn base() -> Format {
    Format::new()
        .set_font_name(FONT)
        .set_font_size(10)
        .set_font_color(CHARCOAL)
}

fn boxed() -> Format {
    base().set_border(FormatBorder::Thin).set_border_color(LINE)
}

fn row_styles(fill: Option<&str>) -> RowStyles {
    let cell = || {
        let f = boxed().set_text_wrap().set_align(FormatAlign::Top);
        match fill {
            Some(color) => f.set_background_color(color),
            None => f,
        }
    };
    RowStyles {
        text: cell(),
        date: cell()
            .set_num_format(DATE_FORMAT)
            .set_align(FormatAlign::Left),
        number: cell().set_align(FormatAlign::Right),
        link: cell()
            .set_font_color(BLUE)
            .set_underline(FormatUnderline::Single),
    }
}

impl Styles {
    pub fn new() -> Self {
        let number = boxed().set_align(FormatAlign::Center);
        let kpi_value = base()
            .set_font_size(20)
            .set_bold()
            .set_font_color(BLUE)
            .set_background_color(CREAM)
            .set_align(FormatAlign::Center)
            .set_align(FormatAlign::VerticalCenter);
        Self {
            title: base().set_font_size(18).set_bold().set_font_color(BLUE),
            subtitle: base().set_italic().set_font_color(GRAY),
            section: base()
                .set_bold()
                .set_font_color(WHITE)
                .set_background_color(BLUE)
                .set_align(FormatAlign::VerticalCenter)
                .set_indent(1),
            header: base()
                .set_bold()
                .set_font_color(WHITE)
                .set_background_color(BLUE)
                .set_text_wrap()
                .set_align(FormatAlign::VerticalCenter)
                .set_border(FormatBorder::Thin)
                .set_border_color(WHITE),
            label: boxed()
                .set_bold()
                .set_font_color(BLUE)
                .set_background_color(CREAM)
                .set_align(FormatAlign::VerticalCenter)
                .set_indent(1),
            value: boxed()
                .set_text_wrap()
                .set_align(FormatAlign::VerticalCenter)
                .set_indent(1),
            kpi_label: base()
                .set_font_size(9)
                .set_font_color(GRAY)
                .set_background_color(CREAM)
                .set_align(FormatAlign::Center)
                .set_border_top(FormatBorder::Medium)
                .set_border_top_color(LIME),
            kpi_rate: kpi_value.clone().set_num_format(PERCENT_FORMAT),
            kpi_value,
            band: base()
                .set_bold()
                .set_font_color(WHITE)
                .set_background_color(BLUE)
                .set_align(FormatAlign::VerticalCenter)
                .set_indent(1),
            band_soft: base()
                .set_bold()
                .set_font_color(BLUE)
                .set_background_color(CREAM)
                .set_align(FormatAlign::VerticalCenter)
                .set_indent(1),
            text: base().set_text_wrap().set_align(FormatAlign::Top),
            link: base()
                .set_font_color(BLUE)
                .set_underline(FormatUnderline::Single),
            percent: number.clone().set_num_format(PERCENT_FORMAT),
            total: number.clone().set_bold().set_background_color(CREAM),
            number,
            footer: base()
                .set_font_size(8)
                .set_italic()
                .set_font_color(GRAY)
                .set_align(FormatAlign::Center),
            rows: [row_styles(None), row_styles(Some(CREAM))],
        }
    }

    /// Formatos de la fila de datos `index` (0 = primera fila de datos).
    pub fn row(&self, index: usize) -> &RowStyles {
        &self.rows[index % 2]
    }
}

/// Líneas que ocupa un texto envuelto en una columna de `width` caracteres (Arial 10).
pub fn text_lines(text: &str, width: f64) -> u32 {
    let per_line = (width * 1.1).max(1.0) as usize;
    text.split('\n')
        .map(|line| line.chars().count().div_ceil(per_line).max(1) as u32)
        .sum()
}

/// Alto de fila (puntos) para que se lean los textos dados; Excel admite hasta 409.
pub fn wrapped_height<'a>(texts: impl IntoIterator<Item = (&'a str, f64)>, min: f64) -> f64 {
    let lines = texts
        .into_iter()
        .map(|(text, width)| text_lines(text, width))
        .max()
        .unwrap_or(1);
    (f64::from(lines) * 13.0 + 6.0).clamp(min, 409.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::excel::testing::Book;

    #[test]
    fn every_font_is_arial_and_the_palette_is_t4s() {
        let styles = Styles::new();
        let mut workbook = workbook().unwrap();
        let ws = workbook.add_worksheet();
        let formats = [
            &styles.title,
            &styles.header,
            &styles.label,
            &styles.kpi_label,
            &styles.kpi_rate,
            &styles.band_soft,
            &styles.footer,
            &styles.row(1).date,
        ];
        for (row, format) in formats.into_iter().enumerate() {
            ws.write_string_with_format(row as u32, 0, "x", format)
                .unwrap();
        }
        let xml = Book::save(&mut workbook).part("xl/styles.xml");
        let fonts: Vec<_> = xml
            .match_indices("<name val=\"")
            .map(|(i, _)| &xml[i + 11..i + 16])
            .collect();
        assert!(!fonts.is_empty());
        assert!(fonts.iter().all(|f| *f == "Arial"), "{fonts:?}");
        for color in ["FF2B629E", "FFBAC82D", "FF292B2D", "FFF6F4F1", "FF596069"] {
            assert!(xml.contains(color), "falta {color}");
        }
        assert!(xml.contains("formatCode=\"dd/mm/yyyy\""));
        assert!(xml.contains("formatCode=\"0 %\""));
    }

    #[test]
    fn alternate_rows_switch_to_cream() {
        let styles = Styles::new();
        assert!(styles.row(0).text != styles.row(1).text);
        assert!(styles.row(0).text == styles.row(2).text);
    }

    #[test]
    fn heights_grow_with_text_and_stop_at_excel_limit() {
        assert_eq!(text_lines("", 10.0), 1);
        assert_eq!(text_lines(&"a".repeat(22), 10.0), 2);
        assert_eq!(text_lines("uno\ndos\ntres", 40.0), 3);
        assert_eq!(wrapped_height([("corto", 40.0)], 18.0), 19.0);
        assert_eq!(wrapped_height([("a\nb\nc", 40.0), ("x", 40.0)], 18.0), 45.0);
        assert_eq!(
            wrapped_height([(&"a\n".repeat(100) as &str, 40.0)], 18.0),
            409.0
        );
    }
}
