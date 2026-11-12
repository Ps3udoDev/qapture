//! Trasplante de una hoja con imágenes (G-T2): agrega a un libro ajeno la única hoja de un libro
//! hecho con `rust_xlsxwriter`. Del libro base solo cambian las tres partes índice; el XML se
//! trata con búsqueda de texto (las comillas dentro de un valor siempre van escapadas, así que
//! ` nombre="` solo puede ser el comienzo de un atributo).

use std::fmt::Write;

use super::parts::XlsxParts;
use crate::error::{AppError, AppResult, validation};

pub const STRICT: &str = "El libro usa el formato «Strict» de Excel. Ábrelo en Excel, guárdalo como «Libro de Excel (.xlsx)» y vuelve a importarlo.";
const STRICT_NS: &str = "http://purl.oclc.org/ooxml/spreadsheetml/main";
const WORKBOOK: &str = "xl/workbook.xml";
const WORKBOOK_RELS: &str = "xl/_rels/workbook.xml.rels";
const CONTENT_TYPES: &str = "[Content_Types].xml";
const BAD_NAME: &str = "El nombre de hoja no es válido.";
const ONE_SHEET: &str = "El libro de evidencias debe tener una sola hoja.";
const LINKS: &str = "El libro de evidencias tiene vínculos que no se pueden trasladar.";
/// Atributos de estilo y selección que no tienen sentido fuera del libro donante.
const DROPPED: [&str; 5] = [
    " s=\"",
    " style=\"",
    " customFormat=\"",
    " tabSelected=\"",
    " dxfId=\"",
];

/// Agrega como última hoja de `base` la única hoja de `donor`, con el nombre `name`.
pub fn graft_sheet(base: &[u8], donor: &[u8], name: &str) -> AppResult<Vec<u8>> {
    let mut parts = XlsxParts::read(base)?;
    let donor = XlsxParts::read(donor)?;
    let workbook = parts.text(WORKBOOK)?;
    if workbook.contains(STRICT_NS) {
        return Err(validation(STRICT));
    }
    let base_sheets = parse_sheets(&workbook)?;
    check_name(&base_sheets, name)?;

    let sheets: Vec<&str> = donor
        .names()
        .into_iter()
        .filter(|n| number(n, "xl/worksheets/sheet", ".xml").is_some())
        .collect();
    let [sheet] = sheets[..] else {
        return Err(validation(ONE_SHEET));
    };
    let sheet_rels = rels_path(sheet);
    let drawing = match donor.get(&sheet_rels) {
        None => None,
        Some(_) => match &targets(&donor.text(&sheet_rels)?, "drawing")?[..] {
            [] => None,
            [target] => Some(resolve("xl/worksheets", target)),
            _ => return Err(validation(LINKS)),
        },
    };

    let (n, d, mut m) = {
        let names = parts.names();
        let media: Vec<&str> = names
            .iter()
            .filter(|n| n.len() >= 9 && n[..9].eq_ignore_ascii_case("xl/media/"))
            .map(|n| n.rsplit_once('.').map_or(*n, |(stem, _)| stem))
            .collect();
        (
            next_free(&names, "xl/worksheets/sheet", ".xml"),
            next_free(&names, "xl/drawings/drawing", ".xml"),
            next_free(&media, "xl/media/image", ""),
        )
    };

    let xml = inline_strings(&strip_attrs(&donor.text(sheet)?), &shared_strings(&donor)?)?;
    let mut added = vec![(format!("xl/worksheets/sheet{n}.xml"), xml.into_bytes())];
    let mut types = format!(
        r#"<Override PartName="/xl/worksheets/sheet{n}.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>"#
    );
    if let Some(drawing) = drawing {
        let rels = retarget(&donor.text(&sheet_rels)?, |_| {
            format!("../drawings/drawing{d}.xml")
        });
        added.push((
            rels_path(&format!("xl/worksheets/sheet{n}.xml")),
            rels.into_bytes(),
        ));
        let data = donor.get(&drawing).ok_or_else(|| validation(LINKS))?;
        added.push((format!("xl/drawings/drawing{d}.xml"), data.to_vec()));
        types.push_str(&format!(
            r#"<Override PartName="/xl/drawings/drawing{d}.xml" ContentType="application/vnd.openxmlformats-officedocument.drawing+xml"/>"#
        ));
        let drawing_rels = rels_path(&drawing);
        if donor.get(&drawing_rels).is_some() {
            let rels = donor.text(&drawing_rels)?;
            // Destino original -> nombre nuevo; varias relaciones pueden compartir una imagen.
            let mut renamed: Vec<(String, String)> = Vec::new();
            let mut content_types = parts.text(CONTENT_TYPES)?.to_lowercase();
            for target in targets(&rels, "image")? {
                if renamed.iter().any(|(old, _)| *old == target) {
                    continue;
                }
                let path = resolve("xl/drawings", &target);
                let data = donor.get(&path).ok_or_else(|| validation(LINKS))?;
                let ext = path.rsplit_once('.').map_or("", |(_, ext)| ext);
                let file = format!("image{m}.{ext}");
                m += 1;
                added.push((format!("xl/media/{file}"), data.to_vec()));
                let key = format!(r#"extension="{}""#, ext.to_lowercase());
                if !content_types.contains(&key) {
                    let kind = image_type(ext, &donor)?;
                    write!(
                        types,
                        r#"<Default Extension="{ext}" ContentType="{kind}"/>"#
                    )
                    .ok();
                    content_types.push_str(&key);
                }
                renamed.push((target, file));
            }
            let rels = retarget(&rels, |old| {
                renamed
                    .iter()
                    .find(|(o, _)| o == old)
                    .map_or_else(|| old.to_string(), |(_, new)| format!("../media/{new}"))
            });
            added.push((
                format!("xl/drawings/_rels/drawing{d}.xml.rels"),
                rels.into_bytes(),
            ));
        }
    }

    let sheet_id = base_sheets.iter().map(|s| s.sheet_id).max().unwrap_or(0) + 1;
    let workbook_rels = parts.text(WORKBOOK_RELS)?;
    let ids: Vec<&str> = tags(&workbook_rels, "<Relationship ")
        .filter_map(|t| attr(t, "Id"))
        .collect();
    let rid = (1..)
        .map(|k| format!("rIdQ{k}"))
        .find(|id| !ids.contains(&id.as_str()))
        .expect("siempre hay un id libre");
    let sheet_tag = format!(
        r#"<sheet name="{}" sheetId="{sheet_id}" r:id="{rid}"/>"#,
        escape(name)
    );
    let relationship = format!(
        r#"<Relationship Id="{rid}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet{n}.xml"/>"#
    );
    let content_types = insert_before(&parts.text(CONTENT_TYPES)?, "</Types>", &types)?;
    parts.set(
        WORKBOOK,
        insert_before(&workbook, "</sheets>", &sheet_tag)?.into_bytes(),
    );
    parts.set(
        WORKBOOK_RELS,
        insert_before(&workbook_rels, "</Relationships>", &relationship)?.into_bytes(),
    );
    parts.set(CONTENT_TYPES, content_types.into_bytes());
    for (path, data) in added {
        parts.set(&path, data);
    }
    parts.write()
}

/// El menor `n` mayor que todos los `prefix{n}suffix` de `names` (1 si no hay ninguno).
/// Compara ignorando mayúsculas según la convención de Open Packaging Conventions (OPC).
pub(crate) fn next_free(names: &[&str], prefix: &str, suffix: &str) -> u32 {
    names
        .iter()
        .filter_map(|name| number(name, prefix, suffix))
        .max()
        .map_or(1, |n| n + 1)
}

fn strip_prefix_ignore_case<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    if s.len() >= prefix.len() && s[..prefix.len()].eq_ignore_ascii_case(prefix) {
        Some(&s[prefix.len()..])
    } else {
        None
    }
}

fn strip_suffix_ignore_case<'a>(s: &'a str, suffix: &str) -> Option<&'a str> {
    if suffix.is_empty() {
        return Some(s);
    }
    if s.len() >= suffix.len() && s[s.len() - suffix.len()..].eq_ignore_ascii_case(suffix) {
        Some(&s[..s.len() - suffix.len()])
    } else {
        None
    }
}

fn number(name: &str, prefix: &str, suffix: &str) -> Option<u32> {
    let digits = strip_suffix_ignore_case(strip_prefix_ignore_case(name, prefix)?, suffix)?;
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

pub(crate) struct SheetInfo<'a> {
    pub name: &'a str,
    pub sheet_id: u32,
}

pub(crate) fn parse_sheets<'a>(workbook: &'a str) -> AppResult<Vec<SheetInfo<'a>>> {
    let mut sheets = Vec::new();
    for tag in tags(workbook, "<sheet ") {
        let name = attr(tag, "name")
            .ok_or_else(|| validation("La etiqueta de hoja del libro no es válida."))?;
        let sheet_id = attr(tag, "sheetId")
            .and_then(|id| id.parse::<u32>().ok())
            .ok_or_else(|| validation("La etiqueta de hoja del libro no es válida."))?;
        sheets.push(SheetInfo { name, sheet_id });
    }
    if sheets.is_empty() {
        return Err(validation("El libro no contiene hojas válidas."));
    }
    Ok(sheets)
}

fn check_name(sheets: &[SheetInfo], name: &str) -> AppResult<()> {
    let length = name.encode_utf16().count();
    if length == 0
        || length > 31
        || name.starts_with('\'')
        || name.ends_with('\'')
        || name.chars().any(|c| c.is_control())
        || name.contains(['[', ']', ':', '*', '?', '/', '\\'])
    {
        return Err(validation(BAD_NAME));
    }
    // Excel no distingue mayúsculas al comparar nombres de hoja.
    let wanted = name.to_lowercase();
    if sheets
        .iter()
        .any(|s| unescape(s.name).to_lowercase() == wanted)
    {
        return Err(validation(format!("Ya existe una hoja llamada «{name}».")));
    }
    Ok(())
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

pub(crate) fn unescape(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

fn malformed() -> AppError {
    AppError::Export("el libro de evidencias tiene XML inesperado".into())
}

/// Cada etiqueta que empieza con `open` (que termina en espacio), desde ese espacio hasta antes
/// de `>`; así `attr` puede buscar siempre ` nombre="`.
pub(crate) fn tags<'a>(xml: &'a str, open: &'a str) -> impl Iterator<Item = &'a str> {
    xml.match_indices(open).map(move |(i, _)| {
        let tag = &xml[i + open.len() - 1..];
        &tag[..tag.find('>').unwrap_or(tag.len())]
    })
}

pub(crate) fn attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let key = format!(" {name}=\"");
    let start = tag.find(&key)? + key.len();
    let end = tag[start..].find('"')? + start;
    Some(&tag[start..end])
}

fn rels_path(part: &str) -> String {
    let (dir, file) = part.rsplit_once('/').unwrap_or(("", part));
    format!("{dir}/_rels/{file}.rels")
}

/// Ruta de la parte a la que apunta `target`, relativo a la carpeta `dir`.
fn resolve(dir: &str, target: &str) -> String {
    if let Some(absolute) = target.strip_prefix('/') {
        return absolute.to_string();
    }
    let mut segments: Vec<&str> = dir.split('/').collect();
    for segment in target.split('/') {
        match segment {
            ".." => {
                segments.pop();
            }
            "." => {}
            other => segments.push(other),
        }
    }
    segments.join("/")
}

/// Destinos de las relaciones; todas deben ser internas y del tipo `kind`.
fn targets(rels: &str, kind: &str) -> AppResult<Vec<String>> {
    let wanted = format!("/relationships/{kind}");
    tags(rels, "<Relationship ")
        .map(|tag| {
            let internal = attr(tag, "TargetMode").is_none_or(|mode| mode == "Internal");
            match (attr(tag, "Type"), attr(tag, "Target")) {
                (Some(t), Some(target)) if internal && t.ends_with(&wanted) => Ok(target.into()),
                _ => Err(validation(LINKS)),
            }
        })
        .collect()
}

/// Cambia cada `Target="…"` por `new(valor)` en una sola pasada.
fn retarget(rels: &str, new: impl Fn(&str) -> String) -> String {
    const KEY: &str = " Target=\"";
    let mut out = String::with_capacity(rels.len());
    let mut rest = rels;
    while let Some(i) = rest.find(KEY) {
        let value = i + KEY.len();
        let end = rest[value..].find('"').map_or(rest.len(), |e| value + e);
        out.push_str(&rest[..value]);
        out.push_str(&new(&rest[value..end]));
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

fn image_type(ext: &str, donor: &XlsxParts) -> AppResult<String> {
    match ext.to_lowercase().as_str() {
        "png" => return Ok("image/png".into()),
        "jpeg" | "jpg" => return Ok("image/jpeg".into()),
        _ => {}
    }
    // Otro formato: el tipo que declara el propio libro donante.
    let declared = donor.text(CONTENT_TYPES)?;
    tags(&declared, "<Default ")
        .find(|t| attr(t, "Extension").is_some_and(|e| e.eq_ignore_ascii_case(ext)))
        .and_then(|t| attr(t, "ContentType"))
        .map(str::to_string)
        .ok_or_else(malformed)
}

fn insert_before(xml: &str, close: &str, text: &str) -> AppResult<String> {
    let at = xml
        .rfind(close)
        .ok_or_else(|| AppError::Export(format!("falta {close} en el libro")))?;
    Ok(format!("{}{text}{}", &xml[..at], &xml[at..]))
}

/// Quita los atributos de `DROPPED`, solo dentro de las etiquetas (nunca del texto de las celdas).
fn strip_attrs(xml: &str) -> String {
    let mut out = String::with_capacity(xml.len());
    let mut rest = xml;
    while let Some(lt) = rest.find('<') {
        out.push_str(&rest[..lt]);
        let len = rest[lt..].find('>').map_or(rest.len() - lt, |i| i + 1);
        let mut tag = rest[lt..lt + len].to_string();
        for key in DROPPED {
            while let Some(start) = tag.find(key) {
                let value = start + key.len();
                let Some(end) = tag[value..].find('"') else {
                    break;
                };
                tag.replace_range(start..value + end + 1, "");
            }
        }
        out.push_str(&tag);
        rest = &rest[lt + len..];
    }
    out.push_str(rest);
    out
}

/// Contenido interno de cada `<si>` de `xl/sharedStrings.xml` (`<t></t>` si está vacío).
fn shared_strings(donor: &XlsxParts) -> AppResult<Vec<String>> {
    const PART: &str = "xl/sharedStrings.xml";
    if donor.get(PART).is_none() {
        return Ok(Vec::new());
    }
    let xml = donor.text(PART)?;
    let mut strings = Vec::new();
    let mut rest = xml.as_str();
    while let Some(start) = rest.find("<si") {
        let after = &rest[start + 3..];
        if !after.starts_with(['>', '/', ' ']) {
            rest = after;
            continue;
        }
        let open = after.find('>').ok_or_else(malformed)?;
        let body = &after[open + 1..];
        if after[..open].ends_with('/') {
            strings.push("<t></t>".into());
            rest = body;
            continue;
        }
        let close = body.find("</si>").ok_or_else(malformed)?;
        strings.push(body[..close].to_string());
        rest = &body[close + "</si>".len()..];
    }
    Ok(strings)
}

/// `<c … t="s"><v>i</v></c>` → `<c … t="inlineStr"><is>{i-ésima cadena}</is></c>`.
fn inline_strings(xml: &str, strings: &[String]) -> AppResult<String> {
    const SHARED: &str = r#" t="s""#;
    let mut out = String::with_capacity(xml.len());
    let mut rest = xml;
    while let Some(start) = rest.find("<c ") {
        out.push_str(&rest[..start]);
        let cell = &rest[start..];
        let open = cell.find('>').ok_or_else(malformed)? + 1;
        let tag = &cell[..open];
        if tag.ends_with("/>") || !tag.contains(SHARED) {
            out.push_str(tag);
            rest = &cell[open..];
            continue;
        }
        let close = cell.find("</c>").ok_or_else(malformed)?;
        let text = cell[open..close]
            .strip_prefix("<v>")
            .and_then(|v| v.strip_suffix("</v>"))
            .and_then(|v| v.trim().parse::<usize>().ok())
            .and_then(|i| strings.get(i))
            .ok_or_else(malformed)?;
        out.push_str(&tag.replace(SHARED, r#" t="inlineStr""#));
        write!(out, "<is>{text}</is></c>").ok();
        rest = &cell[close + "</c>".len()..];
    }
    out.push_str(rest);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::excel::{
        guion_fixture,
        testing::{Book, write_png},
    };
    use crate::media::storage::TempDir;

    /// Libro con una hoja «Donante»: dos textos (uno en negrita) e `images` PNG desde la fila 3.
    fn donor(images: usize) -> Vec<u8> {
        let tmp = TempDir::new();
        let mut workbook = rust_xlsxwriter::Workbook::new();
        let ws = workbook.add_worksheet();
        ws.set_name("Donante").unwrap();
        let bold = rust_xlsxwriter::Format::new().set_bold();
        ws.write_string_with_format(0, 0, "Evidencias de pruebas", &bold)
            .unwrap();
        ws.write_string(1, 0, "CP05 — Adjuntar archivo no Excel")
            .unwrap();
        for i in 0..images {
            let path = write_png(tmp.path(), &format!("{i}.png"), 300, 200, i as u8);
            let image = rust_xlsxwriter::Image::new(&path).unwrap();
            ws.insert_image(2 + 12 * i as u32, 0, &image).unwrap();
        }
        workbook.save_to_buffer().unwrap()
    }

    #[test]
    fn adds_the_sheet_last_with_its_text_and_images() {
        let out = graft_sheet(&guion_fixture::build(), &donor(2), "Evidencias").unwrap();
        let book = Book::from_bytes(&out);
        assert_eq!(
            book.sheet_names(),
            [
                guion_fixture::SHEET,
                guion_fixture::HIDDEN_SHEET,
                "Evidencias"
            ]
        );
        assert_eq!(book.text("Evidencias", 0, 0), "Evidencias de pruebas");
        assert_eq!(
            book.text("Evidencias", 1, 0),
            "CP05 — Adjuntar archivo no Excel"
        );
        assert_eq!(book.pictures("Evidencias"), 2);
        assert_eq!(book.pictures(guion_fixture::SHEET), 1); // el logo sigue
        assert_eq!(
            book.text(guion_fixture::SHEET, guion_fixture::FIRST_CASE_ROW, 2),
            "CP01"
        );
    }

    #[test]
    fn every_original_part_is_untouched_except_the_three_index_parts() {
        let base = guion_fixture::build();
        let before = XlsxParts::read(&base).unwrap();
        let after = XlsxParts::read(&graft_sheet(&base, &donor(2), "Evidencias").unwrap()).unwrap();
        let index = [
            "xl/workbook.xml",
            "xl/_rels/workbook.xml.rels",
            "[Content_Types].xml",
        ];
        for name in before.names() {
            if !index.contains(&name) {
                assert_eq!(after.get(name), before.get(name), "{name}");
            }
        }
        let wb = after.text("xl/workbook.xml").unwrap();
        assert_eq!(
            wb.replace(
                &wb[wb.find("<sheet name=\"Evidencias\"").unwrap()..wb.find("</sheets>").unwrap()],
                ""
            ),
            before.text("xl/workbook.xml").unwrap()
        );
    }

    #[test]
    fn new_parts_do_not_collide_and_are_registered_once() {
        let out = graft_sheet(&guion_fixture::build(), &donor(2), "Evidencias").unwrap();
        let parts = XlsxParts::read(&out).unwrap();
        assert!(parts.get("xl/worksheets/sheet3.xml").is_some());
        assert!(parts.get("xl/drawings/drawing2.xml").is_some());
        assert!(parts.get("xl/media/image2.png").is_some());
        assert!(parts.get("xl/media/image3.png").is_some());
        let ct = parts.text("[Content_Types].xml").unwrap();
        assert!(ct.contains(r#"PartName="/xl/worksheets/sheet3.xml""#));
        assert!(ct.contains(r#"PartName="/xl/drawings/drawing2.xml""#));
        assert_eq!(ct.matches(r#"Extension="png""#).count(), 1);
    }

    #[test]
    fn the_grafted_sheet_has_no_shared_strings_styles_or_selected_tab() {
        let out = graft_sheet(&guion_fixture::build(), &donor(1), "Evidencias").unwrap();
        let xml = XlsxParts::read(&out)
            .unwrap()
            .text("xl/worksheets/sheet3.xml")
            .unwrap();
        assert!(!xml.contains(r#"t="s""#));
        assert!(!xml.contains(" s=\""));
        assert!(!xml.contains("tabSelected"));
        assert!(xml.contains(r#"t="inlineStr""#));
    }

    #[test]
    fn names_are_escaped_and_validated() {
        let base = guion_fixture::build();
        let out = graft_sheet(&base, &donor(1), "Evidencias & anexos").unwrap();
        assert!(
            XlsxParts::read(&out)
                .unwrap()
                .text("xl/workbook.xml")
                .unwrap()
                .contains(r#"name="Evidencias &amp; anexos""#)
        );
        assert_eq!(
            Book::from_bytes(&out).sheet_names()[2],
            "Evidencias & anexos"
        );
        assert_eq!(
            graft_sheet(&base, &donor(1), "Hoja1")
                .unwrap_err()
                .to_string(),
            "Ya existe una hoja llamada «Hoja1»."
        );
        for bad in ["", "a/b", "[x]", &"x".repeat(32)] {
            assert_eq!(
                graft_sheet(&base, &donor(1), bad).unwrap_err().to_string(),
                "El nombre de hoja no es válido."
            );
        }
    }

    #[test]
    fn rejects_donors_that_cannot_be_moved() {
        let base = guion_fixture::build();
        // dos hojas
        let mut two = rust_xlsxwriter::Workbook::new();
        two.add_worksheet();
        two.add_worksheet();
        assert_eq!(
            graft_sheet(&base, &two.save_to_buffer().unwrap(), "E")
                .unwrap_err()
                .to_string(),
            "El libro de evidencias debe tener una sola hoja."
        );
        // vínculo externo
        let mut linked = rust_xlsxwriter::Workbook::new();
        linked
            .add_worksheet()
            .write_url(0, 0, "https://example.com")
            .unwrap();
        assert_eq!(
            graft_sheet(&base, &linked.save_to_buffer().unwrap(), "E")
                .unwrap_err()
                .to_string(),
            "El libro de evidencias tiene vínculos que no se pueden trasladar."
        );
    }

    #[test]
    fn next_free_skips_gaps_and_other_parts() {
        let names = [
            "xl/worksheets/sheet1.xml",
            "xl/worksheets/sheet7.xml",
            "xl/worksheets/_rels/sheet9.xml.rels",
        ];
        assert_eq!(next_free(&names, "xl/worksheets/sheet", ".xml"), 8);
        assert_eq!(next_free(&[], "xl/drawings/drawing", ".xml"), 1);
    }

    #[test]
    fn sheets_without_id_or_name_fail_closed() {
        let base = guion_fixture::build();
        let mut parts = XlsxParts::read(&base).unwrap();
        // sheet without sheetId
        let bad_xml = r#"<workbook><sheets><sheet name="Hoja1"/></sheets></workbook>"#;
        parts.set("xl/workbook.xml", bad_xml.as_bytes().to_vec());
        assert!(graft_sheet(&parts.write().unwrap(), &donor(1), "Evidencias").is_err());

        // sheet without name
        let bad_xml2 = r#"<workbook><sheets><sheet sheetId="1"/></sheets></workbook>"#;
        parts.set("xl/workbook.xml", bad_xml2.as_bytes().to_vec());
        assert!(graft_sheet(&parts.write().unwrap(), &donor(1), "Evidencias").is_err());

        // sheet with single quotes (name='x')
        let bad_xml3 = r#"<workbook><sheets><sheet name='x' sheetId='1'/></sheets></workbook>"#;
        parts.set("xl/workbook.xml", bad_xml3.as_bytes().to_vec());
        assert!(graft_sheet(&parts.write().unwrap(), &donor(1), "Evidencias").is_err());
    }

    #[test]
    fn next_free_ignores_case() {
        let names = ["xl/media/Image1"];
        assert_eq!(next_free(&names, "xl/media/image", ""), 2);
    }

    #[test]
    fn names_count_utf16_and_reject_quotes_at_the_ends() {
        let base = guion_fixture::build();
        // 31 emojis (4 bytes UTF-8 each = 2 UTF-16 code units each -> 62 UTF-16 units)
        let emojis = "😀".repeat(31);
        assert_eq!(
            graft_sheet(&base, &donor(1), &emojis)
                .unwrap_err()
                .to_string(),
            "El nombre de hoja no es válido."
        );
        // Quotes at the ends
        assert_eq!(
            graft_sheet(&base, &donor(1), "'Ev'")
                .unwrap_err()
                .to_string(),
            "El nombre de hoja no es válido."
        );
        assert_eq!(
            graft_sheet(&base, &donor(1), "'Ev")
                .unwrap_err()
                .to_string(),
            "El nombre de hoja no es válido."
        );
        assert_eq!(
            graft_sheet(&base, &donor(1), "Ev'")
                .unwrap_err()
                .to_string(),
            "El nombre de hoja no es válido."
        );
        // Control characters
        assert_eq!(
            graft_sheet(&base, &donor(1), "Ev\u{0007}idencias")
                .unwrap_err()
                .to_string(),
            "El nombre de hoja no es válido."
        );
        // 31 BMP characters is Ok
        let bmp31 = "a".repeat(31);
        assert!(graft_sheet(&base, &donor(1), &bmp31).is_ok());
    }

    #[test]
    fn strict_workbooks_are_rejected() {
        let base = guion_fixture::build();
        let mut parts = XlsxParts::read(&base).unwrap();
        let wb = parts.text("xl/workbook.xml").unwrap();
        let strict_wb = wb.replace(
            "http://schemas.openxmlformats.org/spreadsheetml/2006/main",
            "http://purl.oclc.org/ooxml/spreadsheetml/main",
        );
        parts.set("xl/workbook.xml", strict_wb.into_bytes());
        let err = graft_sheet(&parts.write().unwrap(), &donor(1), "Evidencias").unwrap_err();
        assert_eq!(err.to_string(), STRICT);
    }

    #[test]
    fn donor_dxf_ids_are_dropped() {
        let base = guion_fixture::build();
        let donor_bytes = donor(1);
        let mut parts = XlsxParts::read(&donor_bytes).unwrap();
        let sheet_xml = parts.text("xl/worksheets/sheet1.xml").unwrap();
        let with_dxf = sheet_xml.replace("<c ", r#"<c dxfId="0" "#);
        parts.set("xl/worksheets/sheet1.xml", with_dxf.into_bytes());

        let grafted = graft_sheet(&base, &parts.write().unwrap(), "Evidencias").unwrap();
        let grafted_parts = XlsxParts::read(&grafted).unwrap();
        let grafted_sheet_xml = grafted_parts.text("xl/worksheets/sheet3.xml").unwrap();
        assert!(!grafted_sheet_xml.contains("dxfId"));
    }

    #[test]
    fn index_parts_only_gain_entries() {
        let base = guion_fixture::build();
        let before = XlsxParts::read(&base).unwrap();
        let grafted = graft_sheet(&base, &donor(1), "Evidencias").unwrap();
        let after = XlsxParts::read(&grafted).unwrap();

        // 1. Cada <sheet .../>, <Relationship .../> y <Override .../>/<Default .../> del base sigue igual
        let before_wb = before.text("xl/workbook.xml").unwrap();
        let after_wb = after.text("xl/workbook.xml").unwrap();
        for sheet_tag in tags(&before_wb, "<sheet ") {
            assert!(
                after_wb.contains(sheet_tag),
                "sheet tag missing: {sheet_tag}"
            );
        }

        let before_wb_rels = before.text("xl/_rels/workbook.xml.rels").unwrap();
        let after_wb_rels = after.text("xl/_rels/workbook.xml.rels").unwrap();
        for rel_tag in tags(&before_wb_rels, "<Relationship ") {
            assert!(
                after_wb_rels.contains(rel_tag),
                "rel tag missing: {rel_tag}"
            );
        }

        let before_ct = before.text("[Content_Types].xml").unwrap();
        let after_ct = after.text("[Content_Types].xml").unwrap();
        for override_tag in tags(&before_ct, "<Override ") {
            assert!(
                after_ct.contains(override_tag),
                "override missing: {override_tag}"
            );
        }
        for default_tag in tags(&before_ct, "<Default ") {
            assert!(
                after_ct.contains(default_tag),
                "default missing: {default_tag}"
            );
        }

        // 2. Donante cuya relación de dibujo no apunta a una imagen -> Err(LINKS)
        let donor_bytes = donor(1);
        let mut d_parts = XlsxParts::read(&donor_bytes).unwrap();
        let drawing_rels = d_parts.text("xl/drawings/_rels/drawing1.xml.rels").unwrap();
        let non_image_rels = drawing_rels.replace("relationships/image", "relationships/chart");
        d_parts.set(
            "xl/drawings/_rels/drawing1.xml.rels",
            non_image_rels.into_bytes(),
        );
        let err = graft_sheet(&base, &d_parts.write().unwrap(), "Evidencias").unwrap_err();
        assert_eq!(err.to_string(), LINKS);

        // 3. Nombre duplicado que solo cambia en mayúsculas -> Err
        let err2 = graft_sheet(&base, &donor(1), "guión de pruebas").unwrap_err();
        assert_eq!(
            err2.to_string(),
            "Ya existe una hoja llamada «guión de pruebas»."
        );
    }

    #[test]
    #[ignore = "genera archivos para abrir a mano"]
    fn write_files_for_manual_check() {
        let dir = std::env::var("QAPTURE_SPIKE_OUT")
            .expect("Define QAPTURE_SPIKE_OUT con una carpeta existente.");
        let dir = std::path::Path::new(&dir);
        assert!(
            dir.is_dir(),
            "Define QAPTURE_SPIKE_OUT con una carpeta existente."
        );
        std::fs::write(dir.join("guion_sintetico.xlsx"), guion_fixture::build()).unwrap();
        let with = graft_sheet(&guion_fixture::build(), &donor(3), "Evidencias").unwrap();
        std::fs::write(dir.join("guion_con_evidencias.xlsx"), with).unwrap();
    }
}
