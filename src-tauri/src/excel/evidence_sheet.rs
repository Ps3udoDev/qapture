use rust_xlsxwriter::{Image, ObjectMovement, Url, Worksheet};

use super::{
    data::ExportData,
    images::SheetImage,
    matrix,
    styles::{ROW_PX, Styles, wrapped_height},
};
use crate::error::AppResult;

pub const SHEET: &str = "Evidencias";
pub const MISSING_IMAGE: &str = "No se pudo leer la imagen de esta evidencia.";
const LAST_COL: u16 = 9;
/// Columnas que se repiten junto a las capturas cuando el proyecto las tiene.
const KEY_FIELDS: [&str; 4] = [
    "resultado_esperado",
    "resultado_obtenido",
    "response_esperado",
    "response_obtenido",
];

/// Un bloque por caso: banda con código y título, «Volver a la matriz», entorno y estado,
/// campos clave y cada captura (descripción e imagen). `images` trae una entrada por
/// evidencia en el orden de `data.cases[..].evidence`; cada una se suelta al insertarla.
/// Devuelve la fila donde empieza cada caso, para los vínculos de la matriz.
pub fn write(
    ws: &mut Worksheet,
    data: &ExportData,
    images: Vec<Option<SheetImage>>,
    styles: &Styles,
) -> AppResult<Vec<u32>> {
    ws.set_name(SHEET)?;
    ws.set_screen_gridlines(false);
    for col in 0..=LAST_COL {
        ws.set_column_width(col, 14)?;
    }
    ws.set_row_height(0, 26)?;
    ws.merge_range(0, 0, 0, LAST_COL, "Evidencias de pruebas", &styles.title)?;
    ws.merge_range(
        1,
        0,
        1,
        LAST_COL,
        "Cada sección identifica la ejecución, su entorno y sus capturas.",
        &styles.subtitle,
    )?;
    let environment = data.field("entorno");
    let key_fields: Vec<_> = KEY_FIELDS.iter().filter_map(|k| data.field(k)).collect();
    let mut images = images.into_iter();
    let mut anchors = Vec::with_capacity(data.cases.len());
    let mut row = 3;
    if data.cases.is_empty() {
        ws.merge_range(
            row,
            0,
            row,
            LAST_COL,
            "No hay casos registrados.",
            &styles.text,
        )?;
    }
    for (index, c) in data.cases.iter().enumerate() {
        anchors.push(row);
        band(
            ws,
            row,
            &format!("{} — {}", c.case.code, c.case.title),
            &styles.band,
            24.0,
        )?;
        row += 1;
        let back = Url::new(format!(
            "internal:'{}'!A{}",
            matrix::SHEET,
            matrix::row_of(index) + 1
        ))
        .set_text("Volver a la matriz");
        ws.write_url_with_format(row, 0, back, &styles.link)?;
        row += 1;
        let mut status = format!("Estado: {}", c.case.status);
        if let Some(value) = environment
            .map(|f| ExportData::value(&c.case, f))
            .filter(|v| !v.is_empty())
        {
            status = format!("Entorno: {value}   ·   {status}");
        }
        ws.merge_range(row, 0, row, LAST_COL, &status, &styles.text)?;
        row += 1;
        for f in &key_fields {
            let text = format!("{}: {}", f.label, ExportData::value(&c.case, f));
            ws.set_row_height(row, wrapped_height([(text.as_str(), 140.0)], 18.0))?;
            ws.merge_range(row, 0, row, LAST_COL, &text, &styles.text)?;
            row += 1;
        }
        if c.evidence.is_empty() {
            band(
                ws,
                row,
                "No se han registrado capturas.",
                &styles.band_soft,
                22.0,
            )?;
            row += 1;
        }
        for (j, e) in c.evidence.iter().enumerate() {
            band(
                ws,
                row,
                &format!("Captura {}", j + 1),
                &styles.band_soft,
                22.0,
            )?;
            row += 1;
            ws.set_row_height(row, wrapped_height([(e.description.as_str(), 140.0)], 18.0))?;
            ws.merge_range(row, 0, row, LAST_COL, &e.description, &styles.text)?;
            row += 1;
            match images.next().flatten() {
                Some(img) => {
                    let alt: String = e.description.chars().take(200).collect();
                    let image = Image::new_from_buffer(&img.png)?
                        .set_alt_text(alt)
                        .set_object_movement(ObjectMovement::MoveButDontSizeWithCells);
                    ws.insert_image_with_offset(row, 0, &image, 4, 4)?;
                    row += (img.height + 8).div_ceil(ROW_PX) + 1;
                }
                None => {
                    ws.merge_range(row, 0, row, LAST_COL, MISSING_IMAGE, &styles.subtitle)?;
                    row += 2;
                }
            }
        }
        row += 2;
    }
    ws.set_landscape()
        .set_paper_size(9)
        .set_print_fit_to_pages(1, 0);
    Ok(anchors)
}

fn band(
    ws: &mut Worksheet,
    row: u32,
    text: &str,
    format: &rust_xlsxwriter::Format,
    height: f64,
) -> AppResult<()> {
    ws.set_row_height(row, height)?;
    ws.merge_range(row, 0, row, LAST_COL, text, format)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        excel::{
            images::prepare,
            styles,
            testing::{Book, fixture, write_png},
        },
        media::storage::TempDir,
    };

    fn render(data: &ExportData, images: Vec<Option<SheetImage>>) -> (Book, Vec<u32>) {
        let mut workbook = styles::workbook().unwrap();
        let anchors = write(workbook.add_worksheet(), data, images, &Styles::new()).unwrap();
        (Book::save(&mut workbook), anchors)
    }

    fn images(tmp: &TempDir, sizes: &[(u32, u32)]) -> Vec<Option<SheetImage>> {
        sizes
            .iter()
            .enumerate()
            .map(|(i, (w, h))| {
                let path = write_png(tmp.path(), &format!("{i}.png"), *w, *h, i as u8);
                Some(prepare(&path, 940).unwrap())
            })
            .collect()
    }

    #[test]
    fn one_block_per_case_with_back_links_and_key_fields() {
        let tmp = TempDir::new();
        let data = fixture();
        let (book, anchors) = render(&data, images(&tmp, &[(400, 200), (400, 300), (300, 100)]));
        assert_eq!(anchors.len(), 4);
        assert!(anchors.windows(2).all(|w| w[0] < w[1]));
        assert_eq!(book.text(SHEET, anchors[0], 0), "EJ-001 — Caso EJ-001");
        assert_eq!(book.text(SHEET, anchors[0] + 1, 0), "Volver a la matriz");
        assert_eq!(
            book.text(SHEET, anchors[0] + 2, 0),
            "Entorno: QA   ·   Estado: Aprobado"
        );
        assert_eq!(
            book.text(SHEET, anchors[0] + 3, 0),
            "Resultado esperado: Se aplica el recibo"
        );
        assert_eq!(
            book.text(SHEET, anchors[0] + 4, 0),
            "Resultado obtenido: Se aplicó el recibo"
        );
        assert_eq!(book.text(SHEET, anchors[0] + 5, 0), "Captura 1");
        assert_eq!(book.text(SHEET, anchors[0] + 6, 0), "Listado de recibos");
        let links = book.hyperlinks(SHEET);
        let targets: Vec<_> = links.iter().map(|(_, to)| to.as_str()).collect();
        assert_eq!(
            targets,
            ["'Matriz'!A5", "'Matriz'!A6", "'Matriz'!A7", "'Matriz'!A8"]
        );
        assert_eq!(links[0].0, format!("A{}", anchors[0] + 2));
    }

    #[test]
    fn inserts_every_image_and_leaves_room_below_it() {
        let tmp = TempDir::new();
        let data = fixture();
        let (book, anchors) = render(&data, images(&tmp, &[(400, 200), (400, 300), (300, 100)]));
        assert_eq!(book.pictures(SHEET), 3);
        // La imagen 1 empieza en anchor+7 y mide 200 px (+8 de margen): 13 filas de 17 px y 1 libre.
        assert_eq!(book.text(SHEET, anchors[0] + 7 + 14, 0), "Captura 2");
    }

    #[test]
    fn missing_images_and_cases_without_evidence_are_explicit() {
        let tmp = TempDir::new();
        let data = fixture();
        let mut imgs = images(&tmp, &[(400, 200), (400, 300), (300, 100)]);
        imgs[1] = None;
        let (book, anchors) = render(&data, imgs);
        assert_eq!(book.pictures(SHEET), 2);
        assert!(book.find_row(SHEET, MISSING_IMAGE).is_some());
        assert_eq!(
            book.text(SHEET, anchors[2] + 5, 0),
            "No se han registrado capturas."
        );
    }

    #[test]
    fn projects_without_the_key_columns_skip_them() {
        let mut data = fixture();
        data.project
            .fields
            .retain(|f| f.key != "entorno" && !f.key.starts_with("resultado_"));
        let (book, anchors) = render(&data, vec![None, None, None]);
        assert_eq!(book.text(SHEET, anchors[0] + 2, 0), "Estado: Aprobado");
        assert_eq!(book.text(SHEET, anchors[0] + 3, 0), "Captura 1");
    }

    #[test]
    fn an_empty_project_still_has_the_sheet() {
        let mut data = fixture();
        data.cases.clear();
        let (book, anchors) = render(&data, Vec::new());
        assert!(anchors.is_empty());
        assert_eq!(book.text(SHEET, 3, 0), "No hay casos registrados.");
    }
}
