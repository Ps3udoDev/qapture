//! Edición del XML de una hoja de Excel como texto (G4-T1).
//!
//! Permite escribir valores (`CellValue::Text`, `CellValue::Date`, `CellValue::Empty`)
//! en celdas específicas de una hoja, preservando o heredando estilos y atributos,
//! gestionando fechas (número de serie vs texto según estilo y sistema 1904) y
//! retirando fórmulas individuales o grupos de fórmulas compartidas.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fmt::Write as _;

use chrono::{Datelike, NaiveDate};

use crate::error::{AppResult, validation};
use crate::transfer::guion::read::column_index;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CellValue {
    Text(String),
    Date(NaiveDate),
    Empty,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellWrite {
    pub cell: String,
    pub value: CellValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetEdit {
    pub xml: String,
    pub formulas_removed: Vec<String>,
}

/// Detecta si el libro usa el sistema de fechas 1904 (Macintosh).
pub fn date1904(workbook_xml: &str) -> bool {
    tags(workbook_xml, "<workbookPr")
        .any(|tag| matches!(attr(tag, "date1904"), Some("1") | Some("true")))
}

/// Identifica las celdas dadas que contienen fórmulas en la hoja, expandiendo a todos los miembros
/// de sus grupos de fórmulas compartidas (`t="shared"` con el mismo `si`).
/// Devuelve las referencias de celda ordenadas por fila y columna.
pub fn formula_cells(sheet_xml: &str, cells: &[String]) -> AppResult<Vec<String>> {
    let parsed = parse_sheet(sheet_xml)?;
    let target_set: HashSet<&str> = cells.iter().map(|s| s.as_str()).collect();

    // Identificar grupos compartidos afectados y celdas individuales
    let mut triggered_sis: HashSet<String> = HashSet::new();
    let mut directly_triggered: HashSet<String> = HashSet::new();

    for row in &parsed.rows {
        for cell in &row.cells {
            if let Some(f_info) = cell.formula.as_ref()
                && target_set.contains(cell.cell_ref.as_str())
            {
                match f_info {
                    FormulaInfo::Single => {
                        directly_triggered.insert(cell.cell_ref.clone());
                    }
                    FormulaInfo::Shared { si } => {
                        triggered_sis.insert(si.clone());
                    }
                }
            }
        }
    }

    // Expandir a todas las celdas de los grupos afectados
    let mut result_set: BTreeSet<(u32, u32, String)> = BTreeSet::new();

    for row in &parsed.rows {
        for cell in &row.cells {
            if let Some(f_info) = cell.formula.as_ref() {
                let is_affected = match f_info {
                    FormulaInfo::Single => directly_triggered.contains(&cell.cell_ref),
                    FormulaInfo::Shared { si } => triggered_sis.contains(si),
                };
                if is_affected {
                    result_set.insert((cell.row, cell.col, cell.cell_ref.clone()));
                }
            }
        }
    }

    Ok(result_set.into_iter().map(|(_, _, r)| r).collect())
}

/// Escribe los valores especificados en la hoja XML.
pub fn write_cells(
    sheet_xml: &str,
    styles_xml: &str,
    is_date1904: bool,
    writes: &[CellWrite],
) -> AppResult<SheetEdit> {
    let parsed = parse_sheet(sheet_xml)?;

    // Mapear escrituras válidas agrupadas por fila
    let mut writes_by_row: BTreeMap<u32, Vec<(u32, &CellWrite)>> = BTreeMap::new();
    let mut write_cells_set: HashSet<String> = HashSet::new();

    for write in writes {
        if let Some((_, row, col)) = parse_cell_ref(&write.cell) {
            writes_by_row.entry(row).or_default().push((col, write));
            write_cells_set.insert(write.cell.clone());
        }
    }

    // Identificar fórmulas afectadas (directas y compartidas)
    let mut triggered_sis: HashSet<String> = HashSet::new();
    let mut directly_triggered: HashSet<String> = HashSet::new();

    for row in &parsed.rows {
        for cell in &row.cells {
            if let Some(f_info) = cell.formula.as_ref()
                && write_cells_set.contains(&cell.cell_ref)
            {
                match f_info {
                    FormulaInfo::Single => {
                        directly_triggered.insert(cell.cell_ref.clone());
                    }
                    FormulaInfo::Shared { si } => {
                        triggered_sis.insert(si.clone());
                    }
                }
            }
        }
    }

    let mut formulas_removed_set: BTreeSet<(u32, u32, String)> = BTreeSet::new();
    let mut dependent_formula_cells: HashSet<String> = HashSet::new();

    for row in &parsed.rows {
        for cell in &row.cells {
            if let Some(f_info) = cell.formula.as_ref() {
                let is_affected = match f_info {
                    FormulaInfo::Single => directly_triggered.contains(&cell.cell_ref),
                    FormulaInfo::Shared { si } => triggered_sis.contains(si),
                };
                if is_affected {
                    formulas_removed_set.insert((cell.row, cell.col, cell.cell_ref.clone()));
                    if !write_cells_set.contains(&cell.cell_ref) {
                        dependent_formula_cells.insert(cell.cell_ref.clone());
                    }
                }
            }
        }
    }

    let formulas_removed: Vec<String> = formulas_removed_set
        .into_iter()
        .map(|(_, _, r)| r)
        .collect();

    // Planificar reemplazos en el XML original (edits de mayor a menor posición)
    struct TextEdit {
        start: usize,
        end: usize,
        replacement: String,
    }

    let mut edits: Vec<TextEdit> = Vec::new();

    // 1. Manejar filas existentes
    for row in &parsed.rows {
        let row_writes = writes_by_row.get(&row.row_num);
        let has_writes = row_writes.is_some();
        let has_dependent_formulas = row
            .cells
            .iter()
            .any(|c| dependent_formula_cells.contains(&c.cell_ref));

        if !has_writes && !has_dependent_formulas {
            continue;
        }

        // Reconstruir la fila combinando celdas existentes y nuevas
        let mut row_cells_by_col: BTreeMap<u32, String> = BTreeMap::new();

        // Agregar celdas existentes
        for cell in &row.cells {
            if let Some(write) = row_writes.and_then(|w| w.iter().find(|(col, _)| *col == cell.col))
            {
                // Esta celda se sobreescribe
                let style = attr(&cell.tag, "s").map(|s| s.to_string());
                let cell_xml = format_cell(
                    &cell.cell_ref,
                    style.as_deref(),
                    &write.1.value,
                    styles_xml,
                    is_date1904,
                );
                row_cells_by_col.insert(cell.col, cell_xml);
            } else if dependent_formula_cells.contains(&cell.cell_ref) {
                // Quitar fórmula de celda dependiente preservando valor y atributos
                let cleaned_inner = strip_formula(&cell.inner);
                let open_tag = &cell.tag[..cell.tag.len() - 1]; // sin '>'
                let cell_xml = if cleaned_inner.is_empty() {
                    format!("{open_tag}/>")
                } else {
                    format!("{open_tag}>{cleaned_inner}</c>")
                };
                row_cells_by_col.insert(cell.col, cell_xml);
            } else {
                // Conservar celda intacta
                let original = &sheet_xml[cell.start..cell.end];
                row_cells_by_col.insert(cell.col, original.to_string());
            }
        }

        // Agregar celdas nuevas en esta fila
        if let Some(writes_in_row) = row_writes {
            for (col, write) in writes_in_row {
                if !row_cells_by_col.contains_key(col) {
                    let style = determine_new_cell_style(Some(&row.open_tag), *col, sheet_xml);
                    let cell_xml = format_cell(
                        &write.cell,
                        style.as_deref(),
                        &write.value,
                        styles_xml,
                        is_date1904,
                    );
                    row_cells_by_col.insert(*col, cell_xml);
                }
            }
        }

        // Ensamblar la fila
        let mut new_row = String::new();
        // Asegurar que la etiqueta de apertura sea <row ...> y no self-closing
        let open_tag = if row.open_tag.ends_with("/>") {
            format!("{}>", &row.open_tag[..row.open_tag.len() - 2])
        } else {
            row.open_tag.clone()
        };
        new_row.push_str(&open_tag);
        for (_, cell_xml) in row_cells_by_col {
            new_row.push_str(&cell_xml);
        }
        new_row.push_str("</row>");

        edits.push(TextEdit {
            start: row.start,
            end: row.end,
            replacement: new_row,
        });
    }

    // 2. Manejar filas completamente nuevas
    let existing_row_nums: HashSet<u32> = parsed.rows.iter().map(|r| r.row_num).collect();
    let new_row_nums: Vec<u32> = writes_by_row
        .keys()
        .copied()
        .filter(|r| !existing_row_nums.contains(r))
        .collect();

    if !new_row_nums.is_empty() {
        if parsed.is_sheet_data_self_closing {
            // Reemplazar <sheetData/> por <sheetData>...filas...</sheetData>
            let mut all_rows_xml = String::new();
            for row_num in &new_row_nums {
                let writes_in_row = &writes_by_row[row_num];
                let mut cells_xml = String::new();
                for (col, write) in writes_in_row {
                    let style = determine_new_cell_style(None, *col, sheet_xml);
                    cells_xml.push_str(&format_cell(
                        &write.cell,
                        style.as_deref(),
                        &write.value,
                        styles_xml,
                        is_date1904,
                    ));
                }
                write!(all_rows_xml, r#"<row r="{row_num}">{cells_xml}</row>"#).ok();
            }
            edits.push(TextEdit {
                start: parsed.sheet_data_start,
                end: parsed.sheet_data_end,
                replacement: format!("<sheetData>{all_rows_xml}</sheetData>"),
            });
        } else {
            // Insertar nuevas filas en la posición correcta entre o después de filas existentes
            // Agrupamos filas nuevas por su punto de inserción
            let mut insertions: BTreeMap<usize, Vec<(u32, String)>> = BTreeMap::new();

            for row_num in new_row_nums {
                let writes_in_row = &writes_by_row[&row_num];
                let mut cells_xml = String::new();
                for (col, write) in writes_in_row {
                    let style = determine_new_cell_style(None, *col, sheet_xml);
                    cells_xml.push_str(&format_cell(
                        &write.cell,
                        style.as_deref(),
                        &write.value,
                        styles_xml,
                        is_date1904,
                    ));
                }
                let row_xml = format!(r#"<row r="{row_num}">{cells_xml}</row>"#);

                // Encontrar la primera fila existente con r > row_num
                let insert_pos = match parsed.rows.iter().find(|r| r.row_num > row_num) {
                    Some(next_row) => next_row.start,
                    None => parsed.sheet_data_close_start,
                };
                insertions
                    .entry(insert_pos)
                    .or_default()
                    .push((row_num, row_xml));
            }

            for (insert_pos, mut rows) in insertions {
                rows.sort_by_key(|(r, _)| *r);
                let mut combined = String::new();
                for (_, row_xml) in rows {
                    combined.push_str(&row_xml);
                }
                edits.push(TextEdit {
                    start: insert_pos,
                    end: insert_pos,
                    replacement: combined,
                });
            }
        }
    }

    // Aplicar ediciones de mayor posición a menor posición para no alterar desplazamientos
    // En caso de empate en start, las inserciones (start == end) deben aplicarse después
    // de reemplazos para mantener el orden natural
    edits.sort_by(|a, b| {
        b.start
            .cmp(&a.start)
            .then_with(|| (b.end - b.start).cmp(&(a.end - a.start)))
    });

    let mut result_xml = sheet_xml.to_string();
    for edit in edits {
        result_xml.replace_range(edit.start..edit.end, &edit.replacement);
    }

    Ok(SheetEdit {
        xml: result_xml,
        formulas_removed,
    })
}

// ---------------------------------------------------------------------------
// Helpers de formato de celda y estilos
// ---------------------------------------------------------------------------

fn format_cell(
    cell_ref: &str,
    style: Option<&str>,
    value: &CellValue,
    styles_xml: &str,
    is_date1904: bool,
) -> String {
    let mut style_attr = String::new();
    if let Some(s) = style {
        write!(style_attr, r#" s="{s}""#).ok();
    }

    match value {
        CellValue::Text(text) => {
            let inline = format_inline_str(text);
            format!(r#"<c r="{cell_ref}"{style_attr} t="inlineStr">{inline}</c>"#)
        }
        CellValue::Date(date) => {
            let has_date_style = style
                .and_then(|s| s.parse::<u32>().ok())
                .is_some_and(|s_idx| is_date_style(styles_xml, s_idx));

            if has_date_style {
                let serial = date_serial(*date, is_date1904);
                format!(r#"<c r="{cell_ref}"{style_attr}><v>{serial}</v></c>"#)
            } else {
                let text = format_date_text(*date);
                let inline = format_inline_str(&text);
                format!(r#"<c r="{cell_ref}"{style_attr} t="inlineStr">{inline}</c>"#)
            }
        }
        CellValue::Empty => {
            format!(r#"<c r="{cell_ref}"{style_attr}/>"#)
        }
    }
}

fn determine_new_cell_style(
    row_tag: Option<&str>,
    col_idx: u32,
    sheet_xml: &str,
) -> Option<String> {
    // Si la fila tiene customFormat="1" y s="...", hereda el estilo de la fila
    if let Some(tag) = row_tag
        && attr(tag, "customFormat") == Some("1")
        && let Some(s) = attr(tag, "s")
    {
        return Some(s.to_string());
    }

    // Si no, buscar en <cols><col min="X" max="Y" style="S"/></cols>
    find_col_style(sheet_xml, col_idx)
}

fn find_col_style(sheet_xml: &str, col_idx: u32) -> Option<String> {
    let cols_section = slice_between(sheet_xml, "<cols", "</cols>")?;
    let col_1based = col_idx + 1;
    for col_tag in tags(cols_section, "<col") {
        let min = attr(col_tag, "min").and_then(|m| m.parse::<u32>().ok());
        let max = attr(col_tag, "max").and_then(|m| m.parse::<u32>().ok());
        if let (Some(min), Some(max)) = (min, max)
            && min <= col_1based
            && col_1based <= max
            && let Some(style) = attr(col_tag, "style")
        {
            return Some(style.to_string());
        }
    }
    None
}

fn is_date_style(styles_xml: &str, s_index: u32) -> bool {
    let cell_xfs = match slice_between(styles_xml, "<cellXfs", "</cellXfs>") {
        Some(s) => s,
        None => return false,
    };
    let xf_tags: Vec<&str> = tags(cell_xfs, "<xf").collect();
    let xf = match xf_tags.get(s_index as usize) {
        Some(xf) => xf,
        None => return false,
    };
    let num_fmt_id: u32 = attr(xf, "numFmtId")
        .and_then(|id| id.parse().ok())
        .unwrap_or(0);

    // Formatos de fecha incorporados en Excel: 14..=22 y 45..=47
    if (14..=22).contains(&num_fmt_id) || (45..=47).contains(&num_fmt_id) {
        return true;
    }

    // Formato personalizado en <numFmts>
    if let Some(num_fmts) = slice_between(styles_xml, "<numFmts", "</numFmts>") {
        let id_str = num_fmt_id.to_string();
        for num_fmt in tags(num_fmts, "<numFmt") {
            if attr(num_fmt, "numFmtId") == Some(&id_str)
                && let Some(code) = attr(num_fmt, "formatCode")
            {
                return is_date_format_code(code);
            }
        }
    }

    false
}

fn is_date_format_code(format_code: &str) -> bool {
    let unescaped = unescape(format_code);
    let mut in_quotes = false;
    let mut in_brackets = false;
    for ch in unescaped.chars() {
        match ch {
            '"' if !in_brackets => in_quotes = !in_quotes,
            '[' if !in_quotes => in_brackets = true,
            ']' if !in_quotes => in_brackets = false,
            'd' | 'D' | 'm' | 'M' | 'y' | 'Y' if !in_quotes && !in_brackets => return true,
            _ => {}
        }
    }
    false
}

fn date_serial(date: NaiveDate, is_date1904: bool) -> i64 {
    let epoch = if is_date1904 {
        NaiveDate::from_ymd_opt(1904, 1, 1).unwrap()
    } else {
        NaiveDate::from_ymd_opt(1899, 12, 30).unwrap()
    };
    (date - epoch).num_days()
}

fn format_date_text(date: NaiveDate) -> String {
    format!("{:02}/{:02}/{:04}", date.day(), date.month(), date.year())
}

fn clean_xml_text(s: &str) -> String {
    s.chars()
        .filter(|&c| !matches!(c, '\u{0}'..='\u{8}' | '\u{B}' | '\u{C}' | '\u{E}'..='\u{1F}'))
        .collect()
}

fn escape_element_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(ch),
        }
    }
    out
}

fn needs_preserve(text: &str) -> bool {
    text.starts_with(' ') || text.ends_with(' ') || text.contains('\n') || text.contains('\r')
}

fn format_inline_str(text: &str) -> String {
    let cleaned = clean_xml_text(text);
    let preserve = needs_preserve(&cleaned);
    let escaped = escape_element_text(&cleaned);
    if preserve {
        format!(r#"<is><t xml:space="preserve">{escaped}</t></is>"#)
    } else {
        format!(r#"<is><t>{escaped}</t></is>"#)
    }
}

fn strip_formula(inner: &str) -> String {
    if let Some(f_pos) = inner.find("<f") {
        let rest = &inner[f_pos..];
        if let Some(f_end) = rest.find('>') {
            let f_tag = &rest[..=f_end];
            if f_tag.ends_with("/>") {
                let mut out = String::new();
                out.push_str(&inner[..f_pos]);
                out.push_str(&rest[f_end + 1..]);
                return out;
            } else if let Some(close) = rest.find("</f>") {
                let mut out = String::new();
                out.push_str(&inner[..f_pos]);
                out.push_str(&rest[close + "</f>".len()..]);
                return out;
            }
        }
    }
    inner.to_string()
}

// ---------------------------------------------------------------------------
// Helpers de parsing de XML de la hoja
// ---------------------------------------------------------------------------

#[derive(Debug)]
enum FormulaInfo {
    Single,
    Shared { si: String },
}

#[derive(Debug)]
struct ParsedCell {
    cell_ref: String,
    row: u32,
    col: u32,
    start: usize,
    end: usize,
    tag: String,
    inner: String,
    formula: Option<FormulaInfo>,
}

#[derive(Debug)]
struct ParsedRow {
    row_num: u32,
    start: usize,
    end: usize,
    open_tag: String,
    cells: Vec<ParsedCell>,
}

#[derive(Debug)]
struct ParsedSheet {
    sheet_data_start: usize,
    sheet_data_end: usize,
    sheet_data_close_start: usize,
    is_sheet_data_self_closing: bool,
    rows: Vec<ParsedRow>,
}

fn parse_sheet(sheet_xml: &str) -> AppResult<ParsedSheet> {
    let sheet_data_start = sheet_xml
        .find("<sheetData")
        .ok_or_else(|| validation("No se encontró <sheetData> en la hoja XML"))?;

    let after_start = &sheet_xml[sheet_data_start..];
    let tag_close = after_start
        .find('>')
        .ok_or_else(|| validation("XML de hoja malformado: <sheetData> incompleto"))?;

    let tag_full = &after_start[..=tag_close];
    let is_self_closing = tag_full.ends_with("/>");

    if is_self_closing {
        return Ok(ParsedSheet {
            sheet_data_start,
            sheet_data_end: sheet_data_start + tag_close + 1,
            sheet_data_close_start: sheet_data_start + tag_close + 1,
            is_sheet_data_self_closing: true,
            rows: Vec::new(),
        });
    }

    let sheet_data_content_start = sheet_data_start + tag_close + 1;
    let close_tag_offset = sheet_xml[sheet_data_content_start..]
        .find("</sheetData>")
        .ok_or_else(|| validation("XML de hoja malformado: falta </sheetData>"))?;
    let sheet_data_close_start = sheet_data_content_start + close_tag_offset;
    let sheet_data_end = sheet_data_close_start + "</sheetData>".len();

    let sheet_data_content = &sheet_xml[sheet_data_content_start..sheet_data_close_start];
    let mut rows: Vec<ParsedRow> = Vec::new();
    let mut search_from = 0;

    while let Some(rel_row_start) = sheet_data_content[search_from..].find("<row") {
        let row_start_in_content = search_from + rel_row_start;
        let rest = &sheet_data_content[row_start_in_content..];

        // Validar que sea etiqueta <row ...> o <row>
        let next_ch = rest.chars().nth(4);
        if !matches!(next_ch, Some(c) if c.is_whitespace() || c == '>' || c == '/') {
            search_from = row_start_in_content + 4;
            continue;
        }

        let row_tag_end = rest
            .find('>')
            .ok_or_else(|| validation("XML malformado: <row> sin cierre de etiqueta"))?;
        let row_open_tag = &rest[..=row_tag_end];
        let is_row_self_closing = row_open_tag.ends_with("/>");

        let row_num = attr(row_open_tag, "r")
            .and_then(|r| r.parse::<u32>().ok())
            .unwrap_or(0);

        let abs_row_start = sheet_data_content_start + row_start_in_content;

        if is_row_self_closing {
            let abs_row_end = abs_row_start + row_tag_end + 1;
            rows.push(ParsedRow {
                row_num,
                start: abs_row_start,
                end: abs_row_end,
                open_tag: row_open_tag.to_string(),
                cells: Vec::new(),
            });
            search_from = row_start_in_content + row_tag_end + 1;
        } else {
            let row_inner_start = row_tag_end + 1;
            let row_close_rel = rest[row_inner_start..]
                .find("</row>")
                .ok_or_else(|| validation("XML malformado: falta </row>"))?;
            let row_inner_end = row_inner_start + row_close_rel;
            let abs_row_end = abs_row_start + row_inner_end + "</row>".len();
            let row_inner = &rest[row_inner_start..row_inner_end];

            let mut cells: Vec<ParsedCell> = Vec::new();
            let mut c_search = 0;

            while let Some(rel_c_start) = row_inner[c_search..].find("<c") {
                let c_start_in_inner = c_search + rel_c_start;
                let c_rest = &row_inner[c_start_in_inner..];

                let next_ch = c_rest.chars().nth(2);
                if !matches!(next_ch, Some(c) if c.is_whitespace() || c == '>' || c == '/') {
                    c_search = c_start_in_inner + 2;
                    continue;
                }

                let c_tag_end = c_rest
                    .find('>')
                    .ok_or_else(|| validation("XML malformado: <c> sin cierre de etiqueta"))?;
                let c_tag = &c_rest[..=c_tag_end];
                let is_c_self_closing = c_tag.ends_with("/>");

                let abs_c_start = abs_row_start + row_inner_start + c_start_in_inner;

                let cell_ref = attr(c_tag, "r").unwrap_or("").to_string();
                let (_, r_num, col_idx) = parse_cell_ref(&cell_ref).unwrap_or(("", row_num, 0));

                if is_c_self_closing {
                    let abs_c_end = abs_c_start + c_tag_end + 1;
                    cells.push(ParsedCell {
                        cell_ref,
                        row: r_num,
                        col: col_idx,
                        start: abs_c_start,
                        end: abs_c_end,
                        tag: c_tag.to_string(),
                        inner: String::new(),
                        formula: None,
                    });
                    c_search = c_start_in_inner + c_tag_end + 1;
                } else {
                    let c_inner_start = c_tag_end + 1;
                    let c_close_rel = c_rest[c_inner_start..]
                        .find("</c>")
                        .ok_or_else(|| validation("XML malformado: falta </c>"))?;
                    let c_inner_end = c_inner_start + c_close_rel;
                    let abs_c_end = abs_c_start + c_inner_end + "</c>".len();
                    let c_inner = &c_rest[c_inner_start..c_inner_end];

                    let formula = detect_formula(c_inner);

                    cells.push(ParsedCell {
                        cell_ref,
                        row: r_num,
                        col: col_idx,
                        start: abs_c_start,
                        end: abs_c_end,
                        tag: c_tag.to_string(),
                        inner: c_inner.to_string(),
                        formula,
                    });
                    c_search = c_start_in_inner + c_inner_end + "</c>".len();
                }
            }

            rows.push(ParsedRow {
                row_num,
                start: abs_row_start,
                end: abs_row_end,
                open_tag: row_open_tag.to_string(),
                cells,
            });

            search_from = row_start_in_content + row_inner_end + "</row>".len();
        }
    }

    Ok(ParsedSheet {
        sheet_data_start,
        sheet_data_end,
        sheet_data_close_start,
        is_sheet_data_self_closing: false,
        rows,
    })
}

fn detect_formula(inner: &str) -> Option<FormulaInfo> {
    let f_start = inner.find("<f")?;
    let rest = &inner[f_start..];
    let next_ch = rest.chars().nth(2)?;
    if !next_ch.is_whitespace() && next_ch != '>' && next_ch != '/' {
        return None;
    }
    let f_tag_end = rest.find('>')?;
    let f_tag = &rest[..=f_tag_end];

    if attr(f_tag, "t") == Some("shared")
        && let Some(si) = attr(f_tag, "si")
    {
        return Some(FormulaInfo::Shared { si: si.to_string() });
    }
    Some(FormulaInfo::Single)
}

fn parse_cell_ref(cell: &str) -> Option<(&str, u32, u32)> {
    let split = cell.find(|c: char| c.is_ascii_digit())?;
    let (col_str, row_str) = cell.split_at(split);
    let col = column_index(col_str)?;
    let row = row_str.parse::<u32>().ok()?;
    if row == 0 {
        return None;
    }
    Some((col_str, row, col))
}

fn tags<'a>(xml: &'a str, open_prefix: &'a str) -> impl Iterator<Item = &'a str> {
    xml.match_indices(open_prefix).filter_map(move |(i, _)| {
        let rest = &xml[i + open_prefix.len()..];
        if let Some(ch) = rest.chars().next()
            && !ch.is_whitespace()
            && ch != '>'
            && ch != '/'
        {
            return None;
        }
        let end = rest.find('>')?;
        Some(&rest[..end])
    })
}

fn attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let target = format!("{name}=\"");
    let mut search_from = 0;
    while let Some(pos) = tag[search_from..].find(&target) {
        let abs_pos = search_from + pos;
        if abs_pos == 0 || tag.as_bytes()[abs_pos - 1].is_ascii_whitespace() {
            let start = abs_pos + target.len();
            let end = tag[start..].find('"')? + start;
            return Some(&tag[start..end]);
        }
        search_from = abs_pos + 1;
    }
    None
}

fn slice_between<'a>(xml: &'a str, open_tag: &str, close_tag: &str) -> Option<&'a str> {
    let start = xml.find(open_tag)?;
    let end = xml[start..].find(close_tag)? + start;
    Some(&xml[start..end])
}

fn unescape(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sheet_wrap(body: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>{body}</sheetData></worksheet>"#
        )
    }

    fn sample_styles() -> &'static str {
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <numFmts count="2">
    <numFmt numFmtId="164" formatCode="dd/mm/yyyy;@"/>
    <numFmt numFmtId="165" formatCode="0.00"/>
  </numFmts>
  <cellXfs count="5">
    <xf numFmtId="0"/>
    <xf numFmtId="0"/>
    <xf numFmtId="14"/>
    <xf numFmtId="164"/>
    <xf numFmtId="165"/>
  </cellXfs>
</styleSheet>"#
    }

    #[test]
    fn text_is_written_inline_and_keeps_the_style() {
        let prefix = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData><row r="16">"#;
        let cell = r#"<c r="K16" s="7" t="s"><v>3</v></c>"#;
        let suffix = r#"</row></sheetData></worksheet>"#;
        let xml = format!("{prefix}{cell}{suffix}");

        let edit = write_cells(
            &xml,
            sample_styles(),
            false,
            &[CellWrite {
                cell: "K16".into(),
                value: CellValue::Text("Aceptado".into()),
            }],
        )
        .unwrap();

        let expected_cell = r#"<c r="K16" s="7" t="inlineStr"><is><t>Aceptado</t></is></c>"#;
        assert_eq!(edit.xml, format!("{prefix}{expected_cell}{suffix}"));
        assert!(edit.xml.starts_with(prefix));
        assert!(edit.xml.ends_with(suffix));
        assert!(edit.formulas_removed.is_empty());
    }

    #[test]
    fn text_is_escaped_and_keeps_spaces_and_newlines() {
        let xml = sheet_wrap(r#"<row r="16"><c r="K16" s="7"/></row>"#);
        let input = " A & B <c>\n\u{1}x ";

        let edit = write_cells(
            &xml,
            sample_styles(),
            false,
            &[CellWrite {
                cell: "K16".into(),
                value: CellValue::Text(input.into()),
            }],
        )
        .unwrap();

        assert!(
            edit.xml
                .contains("<t xml:space=\"preserve\"> A &amp; B &lt;c&gt;\nx </t>")
        );
        assert!(!edit.xml.contains('\u{1}'));
    }

    #[test]
    fn empty_clears_the_value_but_keeps_the_cell() {
        let xml = sheet_wrap(r#"<row r="16"><c r="I16" s="4"><v>45000</v></c></row>"#);

        let edit = write_cells(
            &xml,
            sample_styles(),
            false,
            &[CellWrite {
                cell: "I16".into(),
                value: CellValue::Empty,
            }],
        )
        .unwrap();

        assert!(edit.xml.contains(r#"<c r="I16" s="4"/>"#));
        assert!(!edit.xml.contains("<v>"));
    }

    #[test]
    fn dates_use_the_existing_date_style() {
        let xml = sheet_wrap(r#"<row r="16"><c r="I16" s="2"><v>45000</v></c></row>"#);
        let date = NaiveDate::from_ymd_opt(2026, 11, 11).unwrap();

        let edit = write_cells(
            &xml,
            sample_styles(),
            false,
            &[CellWrite {
                cell: "I16".into(),
                value: CellValue::Date(date),
            }],
        )
        .unwrap();

        assert!(edit.xml.contains(r#"<c r="I16" s="2"><v>46337</v></c>"#));
    }

    #[test]
    fn dates_use_the_1904_system_when_the_workbook_does() {
        let xml = sheet_wrap(r#"<row r="16"><c r="I16" s="2"><v>45000</v></c></row>"#);
        let date = NaiveDate::from_ymd_opt(2026, 11, 11).unwrap();

        let edit = write_cells(
            &xml,
            sample_styles(),
            true,
            &[CellWrite {
                cell: "I16".into(),
                value: CellValue::Date(date),
            }],
        )
        .unwrap();

        assert!(edit.xml.contains(r#"<c r="I16" s="2"><v>44875</v></c>"#));
    }

    #[test]
    fn a_date_in_a_cell_without_date_style_is_written_as_text() {
        let date = NaiveDate::from_ymd_opt(2026, 11, 11).unwrap();

        // Sin s -> texto inline 11/11/2026
        let xml1 = sheet_wrap(r#"<row r="16"><c r="I16"/></row>"#);
        let edit1 = write_cells(
            &xml1,
            sample_styles(),
            false,
            &[CellWrite {
                cell: "I16".into(),
                value: CellValue::Date(date),
            }],
        )
        .unwrap();
        assert!(
            edit1
                .xml
                .contains(r#"<c r="I16" t="inlineStr"><is><t>11/11/2026</t></is></c>"#)
        );

        // Con formato numérico propio 0.00 (s="4" -> numFmtId="165") -> texto inline 11/11/2026
        let xml2 = sheet_wrap(r#"<row r="16"><c r="I16" s="4"/></row>"#);
        let edit2 = write_cells(
            &xml2,
            sample_styles(),
            false,
            &[CellWrite {
                cell: "I16".into(),
                value: CellValue::Date(date),
            }],
        )
        .unwrap();
        assert!(
            edit2
                .xml
                .contains(r#"<c r="I16" s="4" t="inlineStr"><is><t>11/11/2026</t></is></c>"#)
        );

        // Con formato de fecha propio dd/mm/yyyy;@ (s="3" -> numFmtId="164") -> serial 46337
        let xml3 = sheet_wrap(r#"<row r="16"><c r="I16" s="3"/></row>"#);
        let edit3 = write_cells(
            &xml3,
            sample_styles(),
            false,
            &[CellWrite {
                cell: "I16".into(),
                value: CellValue::Date(date),
            }],
        )
        .unwrap();
        assert!(edit3.xml.contains(r#"<c r="I16" s="3"><v>46337</v></c>"#));
    }

    #[test]
    fn missing_cells_and_rows_are_created_in_order() {
        // Fila 16 con A16 y M16; escribir K16 la inserta entre ellas
        let xml = sheet_wrap(r#"<row r="16"><c r="A16"><v>1</v></c><c r="M16"><v>2</v></c></row>"#);
        let edit = write_cells(
            &xml,
            sample_styles(),
            false,
            &[CellWrite {
                cell: "K16".into(),
                value: CellValue::Text("K".into()),
            }],
        )
        .unwrap();
        let expected_row = r#"<row r="16"><c r="A16"><v>1</v></c><c r="K16" t="inlineStr"><is><t>K</t></is></c><c r="M16"><v>2</v></c></row>"#;
        assert!(edit.xml.contains(expected_row));

        // Fila 52 no existe; se crea después de la 51
        let xml_rows = sheet_wrap(r#"<row r="51"><c r="A51"><v>1</v></c></row>"#);
        let edit_rows = write_cells(
            &xml_rows,
            sample_styles(),
            false,
            &[CellWrite {
                cell: "A52".into(),
                value: CellValue::Text("52".into()),
            }],
        )
        .unwrap();
        assert!(
            edit_rows
                .xml
                .contains(r#"<row r="51"><c r="A51"><v>1</v></c></row><row r="52"><c r="A52" t="inlineStr"><is><t>52</t></is></c></row>"#)
        );

        // Con sheetData vacío también funciona
        let xml_empty =
            r#"<?xml version="1.0" encoding="UTF-8"?><worksheet><sheetData/></worksheet>"#;
        let edit_empty = write_cells(
            xml_empty,
            sample_styles(),
            false,
            &[CellWrite {
                cell: "A1".into(),
                value: CellValue::Text("Uno".into()),
            }],
        )
        .unwrap();
        assert!(
            edit_empty
                .xml
                .contains(r#"<sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>Uno</t></is></c></row></sheetData>"#)
        );
    }

    #[test]
    fn new_cells_take_the_row_or_column_style() {
        // Estilo de fila customFormat="1" s="9"
        let xml_row = sheet_wrap(r#"<row r="16" s="9" customFormat="1"/>"#);
        let edit_row = write_cells(
            &xml_row,
            sample_styles(),
            false,
            &[CellWrite {
                cell: "K16".into(),
                value: CellValue::Text("val".into()),
            }],
        )
        .unwrap();
        assert!(
            edit_row
                .xml
                .contains(r#"<c r="K16" s="9" t="inlineStr"><is><t>val</t></is></c>"#)
        );

        // Sin estilo de fila y con <col min="11" max="11" style="5"/>
        let xml_col = r#"<?xml version="1.0"?><worksheet><cols><col min="11" max="11" style="5"/></cols><sheetData><row r="16"/></sheetData></worksheet>"#;
        let edit_col = write_cells(
            xml_col,
            sample_styles(),
            false,
            &[CellWrite {
                cell: "K16".into(),
                value: CellValue::Text("val".into()),
            }],
        )
        .unwrap();
        assert!(
            edit_col
                .xml
                .contains(r#"<c r="K16" s="5" t="inlineStr"><is><t>val</t></is></c>"#)
        );
    }

    #[test]
    fn a_formula_is_removed_and_reported() {
        let xml =
            sheet_wrap(r#"<row r="16"><c r="I16" s="2"><f>TODAY()</f><v>46000</v></c></row>"#);
        let date = NaiveDate::from_ymd_opt(2026, 11, 11).unwrap();

        let edit = write_cells(
            &xml,
            sample_styles(),
            false,
            &[CellWrite {
                cell: "I16".into(),
                value: CellValue::Date(date),
            }],
        )
        .unwrap();

        assert!(!edit.xml.contains("<f>"));
        assert!(edit.xml.contains(r#"<c r="I16" s="2"><v>46337</v></c>"#));
        assert_eq!(edit.formulas_removed, vec!["I16"]);
    }

    #[test]
    fn shared_formula_groups_lose_their_formulas_together() {
        let xml = sheet_wrap(
            r#"<row r="16"><c r="I16" s="2"><f t="shared" ref="I16:I18" si="0">A16+1</f><v>46000</v></c></row><row r="17"><c r="I17" s="2"><f t="shared" si="0"/><v>46001</v></c></row><row r="18"><c r="I18" s="2"><f t="shared" si="0"/><v>46002</v></c></row>"#,
        );
        let date = NaiveDate::from_ymd_opt(2026, 11, 11).unwrap();

        let edit = write_cells(
            &xml,
            sample_styles(),
            false,
            &[CellWrite {
                cell: "I16".into(),
                value: CellValue::Date(date),
            }],
        )
        .unwrap();

        assert!(!edit.xml.contains("<f"));
        assert!(edit.xml.contains(r#"<c r="I16" s="2"><v>46337</v></c>"#));
        assert!(edit.xml.contains(r#"<c r="I17" s="2"><v>46001</v></c>"#));
        assert!(edit.xml.contains(r#"<c r="I18" s="2"><v>46002</v></c>"#));
        assert_eq!(edit.formulas_removed, vec!["I16", "I17", "I18"]);

        let formulas = formula_cells(&xml, &["I16".into()]).unwrap();
        assert_eq!(formulas, vec!["I16", "I17", "I18"]);
    }

    #[test]
    fn malformed_sheets_are_rejected() {
        let bad_xml = r#"<?xml version="1.0"?><worksheet><other/></worksheet>"#;
        assert!(
            write_cells(
                bad_xml,
                sample_styles(),
                false,
                &[CellWrite {
                    cell: "A1".into(),
                    value: CellValue::Empty,
                }],
            )
            .is_err()
        );
        assert!(formula_cells(bad_xml, &["A1".into()]).is_err());
    }

    #[test]
    fn date1904_detects_flag_in_workbook_xml() {
        let wb_1904_one = r#"<workbook><workbookPr date1904="1"/></workbook>"#;
        assert!(date1904(wb_1904_one));

        let wb_1904_true = r#"<workbook><workbookPr date1904="true"/></workbook>"#;
        assert!(date1904(wb_1904_true));

        let wb_1900_zero = r#"<workbook><workbookPr date1904="0"/></workbook>"#;
        assert!(!date1904(wb_1900_zero));

        let wb_1900_none = r#"<workbook><workbookPr/></workbook>"#;
        assert!(!date1904(wb_1900_none));
    }
}
