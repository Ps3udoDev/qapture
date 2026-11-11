//! Lectura del libro de origen, sin modificar sus bytes.
use std::io::Cursor;

use calamine::{Data, Reader, SheetVisible, Xlsx};
use chrono::{Duration, NaiveDate};

use crate::{
    domain::fields::is_iso_date,
    error::{AppResult, validation},
};

pub const NOT_XLSX: &str = "El archivo no es un libro de Excel (.xlsx) válido.";
pub const PROTECTED: &str =
    "El archivo está protegido con contraseña. Quita la protección y vuelve a intentar.";
pub const MAX_FILE_BYTES: u64 = 50 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    Empty,
    Text(String),
    Number(f64),
    Date(String),
}

impl Cell {
    pub fn text(&self) -> String {
        match self {
            Self::Empty => String::new(),
            Self::Text(value) | Self::Date(value) => value.clone(),
            Self::Number(value) => value.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct GuionSheet {
    pub name: String,
    pub rows: Vec<Vec<Cell>>,
    pub merged: Vec<(u32, u32, u32, u32)>,
}

impl GuionSheet {
    pub fn cell(&self, row: u32, col: u32) -> &Cell {
        self.rows
            .get(row as usize)
            .and_then(|r| r.get(col as usize))
            .unwrap_or(&Cell::Empty)
    }
}

#[derive(Debug, Clone)]
pub struct GuionBook {
    pub sheets: Vec<GuionSheet>,
}

impl GuionBook {
    pub fn sheet(&self, name: &str) -> Option<&GuionSheet> {
        self.sheets.iter().find(|sheet| sheet.name == name)
    }
}

// El plan G1 fija estas APIs de calamine 0.36 para cargar las combinaciones.
#[allow(deprecated)]
pub fn read(bytes: &[u8]) -> AppResult<GuionBook> {
    if bytes.starts_with(&[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]) {
        let encrypted: Vec<u8> = "EncryptedPackage"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        return Err(validation(
            if bytes.windows(encrypted.len()).any(|w| w == encrypted) {
                PROTECTED
            } else {
                NOT_XLSX
            },
        ));
    }
    let mut workbook: Xlsx<_> =
        calamine::open_workbook_from_rs(Cursor::new(bytes)).map_err(|_| validation(NOT_XLSX))?;
    workbook
        .load_merged_regions()
        .map_err(|_| validation(NOT_XLSX))?;
    let names: Vec<String> = workbook
        .sheets_metadata()
        .iter()
        .filter(|sheet| sheet.visible == SheetVisible::Visible)
        .map(|sheet| sheet.name.clone())
        .collect();
    let mut sheets = Vec::new();
    for name in names {
        let range = workbook
            .worksheet_range(&name)
            .map_err(|_| validation(NOT_XLSX))?;
        let mut rows = Vec::new();
        if let Some((last_row, last_col)) = range.end() {
            for row in 0..=last_row {
                rows.push(
                    (0..=last_col)
                        .map(|col| {
                            range
                                .get_value((row, col))
                                .map(cell_from_data)
                                .unwrap_or(Cell::Empty)
                        })
                        .collect(),
                );
            }
        }
        let merged = workbook
            .merged_regions_by_sheet(&name)
            .iter()
            .map(|(_, _, d)| (d.start.0, d.start.1, d.end.0, d.end.1))
            .collect();
        sheets.push(GuionSheet { name, rows, merged });
    }
    Ok(GuionBook { sheets })
}

fn cell_from_data(value: &Data) -> Cell {
    match value {
        Data::String(value) if value.trim().is_empty() => Cell::Empty,
        Data::String(value) => Cell::Text(value.clone()),
        Data::Float(value) => Cell::Number(*value),
        Data::Int(value) => Cell::Number(*value as f64),
        Data::Bool(value) => Cell::Text(value.to_string()),
        Data::DateTime(value) => {
            let (year, month, day, ..) = value.to_ymd_hms_milli();
            Cell::Date(format!("{year:04}-{month:02}-{day:02}"))
        }
        Data::DateTimeIso(value) => match value.get(..10).filter(|date| is_iso_date(date)) {
            Some(date) => Cell::Date(date.into()),
            None => Cell::Text(value.clone()),
        },
        Data::Empty | Data::Error(_) | Data::DurationIso(_) => Cell::Empty,
    }
}

pub fn suggest_header_row(sheet: &GuionSheet) -> Option<u32> {
    for (index, row) in sheet.rows.iter().take(200).enumerate() {
        let mut consecutive = 0;
        for cell in row {
            consecutive = if matches!(cell, Cell::Text(_)) {
                consecutive + 1
            } else {
                0
            };
            if consecutive >= 4
                && sheet
                    .rows
                    .get(index + 1)
                    .is_some_and(|next| next.iter().any(|cell| *cell != Cell::Empty))
            {
                return Some(index as u32 + 1);
            }
        }
    }
    None
}

pub fn column_letter(index: u32) -> String {
    let mut n = index + 1;
    let mut letters = Vec::new();
    while n > 0 {
        letters.push((b'A' + ((n - 1) % 26) as u8) as char);
        n = (n - 1) / 26;
    }
    letters.iter().rev().collect()
}

pub fn column_index(letter: &str) -> Option<u32> {
    if letter.is_empty() || !letter.bytes().all(|b| b.is_ascii_uppercase()) {
        return None;
    }
    let mut n = 0_u32;
    for b in letter.bytes() {
        n = n.checked_mul(26)?.checked_add(u32::from(b - b'A') + 1)?;
        if n > 16_384 {
            return None;
        }
    }
    Some(n - 1)
}

pub fn parse_date(cell: &Cell) -> Option<String> {
    match cell {
        Cell::Date(date) => Some(date.clone()),
        Cell::Text(date) if is_iso_date(date) => Some(date.clone()),
        Cell::Text(date) => {
            let b = date.as_bytes();
            if b.len() != 10
                || b[2] != b'/'
                || b[5] != b'/'
                || !b
                    .iter()
                    .enumerate()
                    .all(|(i, c)| i == 2 || i == 5 || c.is_ascii_digit())
            {
                return None;
            }
            NaiveDate::parse_from_str(date, "%d/%m/%Y")
                .ok()
                .map(|d| d.format("%Y-%m-%d").to_string())
        }
        Cell::Number(serial) if serial.is_finite() && (1.0..=2_958_465.0).contains(serial) => {
            NaiveDate::from_ymd_opt(1899, 12, 30)?
                .checked_add_signed(Duration::days(serial.trunc() as i64))
                .map(|d| d.format("%Y-%m-%d").to_string())
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::excel::guion_fixture;
    #[test]
    fn lists_only_visible_sheets_with_their_cells() {
        let book = read(&guion_fixture::build()).unwrap();
        let names: Vec<&str> = book.sheets.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, [guion_fixture::SHEET]);
        let sheet = book.sheet(guion_fixture::SHEET).unwrap();
        assert_eq!(sheet.cell(guion_fixture::HEADER_ROW, 2).text(), "ID");
        assert_eq!(sheet.cell(guion_fixture::FIRST_CASE_ROW, 2).text(), "CP01");
        assert_eq!(sheet.cell(guion_fixture::FIRST_CASE_ROW, 9).text(), "1.0");
        assert_eq!(sheet.cell(9_999, 99), &Cell::Empty);
        assert!(sheet.merged.contains(&(0, 0, 0, 12)), "A1:M1");
    }

    #[test]
    fn suggests_the_header_row_after_the_cover() {
        let book = read(&guion_fixture::build()).unwrap();
        assert_eq!(suggest_header_row(&book.sheets[0]), Some(15));
    }

    #[test]
    fn dates_come_from_serials_and_day_first_text() {
        use guion_fixture::Put;
        let r = guion_fixture::FIRST_CASE_ROW;
        let bytes = guion_fixture::build_edited(&[
            (r, 8, Put::Date(2026, 11, 3)),
            (r + 1, 8, Put::Text("04/11/2026")),
            (r + 2, 8, Put::Text("2026-11-05")),
        ]);
        let book = read(&bytes).unwrap();
        let sheet = &book.sheets[0];
        assert_eq!(sheet.cell(r, 8), &Cell::Date("2026-11-03".into()));
        assert_eq!(
            sheet.cell(r + 1, 8).text(),
            "04/11/2026",
            "el texto sigue siendo texto"
        );
        assert_eq!(parse_date(sheet.cell(r + 1, 8)), Some("2026-11-04".into()));
        assert_eq!(parse_date(sheet.cell(r + 2, 8)), Some("2026-11-05".into()));
        assert_eq!(parse_date(&Cell::Text("31/02/2026".into())), None);
        assert_eq!(
            parse_date(&Cell::Number(46_329.0)),
            Some("2026-11-03".into())
        );
    }

    #[test]
    fn rejects_cfb_files_with_the_right_message() {
        let mut cfb = vec![0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
        cfb.extend_from_slice(&[0; 512]);
        assert_eq!(read(&cfb).unwrap_err().to_string(), NOT_XLSX, ".xls");
        let name: Vec<u8> = "EncryptedPackage"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        cfb.extend_from_slice(&name);
        assert_eq!(read(&cfb).unwrap_err().to_string(), PROTECTED);
        assert_eq!(read(b"hola").unwrap_err().to_string(), NOT_XLSX);
    }

    #[test]
    fn column_letters_round_trip() {
        for (i, l) in [(0, "A"), (12, "M"), (25, "Z"), (26, "AA"), (16_383, "XFD")] {
            assert_eq!(column_letter(i), l);
            assert_eq!(column_index(l), Some(i));
        }
        for bad in ["", "a", "XFE", "A1", "ÁB"] {
            assert_eq!(column_index(bad), None, "{bad}");
        }
    }

    #[test]
    fn malformed_dates_and_unicode_iso_cells_do_not_panic() {
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 0.0, 2_958_466.0] {
            assert_eq!(parse_date(&Cell::Number(value)), None);
        }
        let unicode = "aaaaaaaaaé".to_string();
        assert_eq!(
            cell_from_data(&Data::DateTimeIso(unicode.clone())),
            Cell::Text(unicode)
        );
        assert_eq!(parse_date(&Cell::Text("1/02/2026".into())), None);
        assert_eq!(cell_from_data(&Data::Float(12.0)).text(), "12");
        assert_eq!(cell_from_data(&Data::String("  \n ".into())), Cell::Empty);
    }
}
