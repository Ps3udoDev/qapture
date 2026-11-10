//! Guion de pruebas sintético con la forma del guion de referencia de un cliente.
//! Se genera en memoria; nunca se guarda un archivo real de un cliente en el repo.

use rust_xlsxwriter::{Color, DataValidation, Format, Image, Workbook};

pub(crate) const SHEET: &str = "Guión de Pruebas";
pub(crate) const HIDDEN_SHEET: &str = "Hoja1";
/// Índice (base 0) de la fila de encabezados; es la fila 15 de Excel.
pub(crate) const HEADER_ROW: u32 = 14;
pub(crate) const FIRST_CASE_ROW: u32 = 15;
pub(crate) const CASES: u32 = 36;
pub(crate) const HEADERS: [&str; 13] = [
    "Módulo",
    "SubMódulo",
    "ID",
    "Opción",
    "RN",
    "Escenario de Prueba",
    "Resultado Esperado",
    "Criticidad",
    "Fecha de Prueba",
    "Version",
    "Status",
    "Nombre del tester",
    "Condiciones",
];
/// Carátula: `(fila base 0, etiqueta en B, valor en D)`.
pub(crate) const COVER: [(u32, &str, &str); 6] = [
    (2, "Código:", "GP-001"),
    (3, "Formato:", "Guion de pruebas v3.0"),
    (4, "Fecha de la prueba:", ""),
    (5, "Responsable:", "Pendiente de asignación"),
    (6, "Versión:", "1.0"),
    (
        7,
        "Observaciones:",
        "Guion propuesto; pendiente de ejecución",
    ),
];

fn logo_png() -> Vec<u8> {
    let img = image::RgbImage::from_fn(120, 40, |x, y| {
        image::Rgb([(x % 256) as u8, (y % 256) as u8, 90])
    });
    let mut out = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
        .expect("png del logo");
    out
}

pub(crate) fn build() -> Vec<u8> {
    let mut workbook = Workbook::new();
    let bold = Format::new().set_bold();
    let header = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0xD9D9D9));
    let last_row = FIRST_CASE_ROW + CASES - 1;

    let ws = workbook.add_worksheet();
    ws.set_name(SHEET).expect("nombre de hoja");
    ws.set_column_range_width(0, 12, 18).expect("anchos");
    ws.merge_range(0, 0, 0, 12, "Guion de pruebas", &bold)
        .expect("título");
    let logo = Image::new_from_buffer(&logo_png()).expect("logo");
    ws.insert_image(0, 0, &logo).expect("insertar logo");

    let plain = Format::new();
    for (row, label, value) in COVER {
        ws.write_string(row, 1, label).expect("etiqueta");
        ws.merge_range(row, 3, row, 12, value, &plain)
            .expect("valor");
    }

    for (col, text) in HEADERS.iter().enumerate() {
        ws.write_string_with_format(HEADER_ROW, col as u16, *text, &header)
            .expect("encabezado");
    }

    let scenarios = ["Acceso", "Archivo", "Estructura", "Validación"];
    for n in 1..=CASES {
        let row = FIRST_CASE_ROW + n - 1;
        let texts: [(u16, String); 10] = [
            (0, "Sistema".into()),
            (1, "Herramientas / Carga".into()),
            (2, format!("CP{n:02}")),
            (3, scenarios[(n as usize - 1) % 4].into()),
            (4, format!("RN{:02}", ((n - 1) % 15) + 1)),
            (5, format!("Escenario {n}")),
            (6, format!("Resultado esperado {n}")),
            (7, if n % 3 == 0 { "Crítico" } else { "Importante" }.into()),
            (9, "1.0".into()),
            (12, "Condiciones generales".into()),
        ];
        for (col, text) in texts {
            ws.write_string(row, col, text).expect("celda de caso");
        }
    }

    // Validaciones solo en las filas 16 a 35 de Excel (índices 15 a 34).
    let status = DataValidation::new()
        .allow_list_strings(&["Aceptado", "Rechazado", "Condicionado"])
        .expect("lista de status");
    ws.add_data_validation(15, 10, 34, 10, &status)
        .expect("validación de status");
    let criticidad = DataValidation::new()
        .allow_list_strings(&["Crìtico", "Importante"])
        .expect("lista de criticidad");
    ws.add_data_validation(15, 7, 34, 7, &criticidad)
        .expect("validación de criticidad");

    ws.autofilter(HEADER_ROW, 0, HEADER_ROW, 12)
        .expect("autofiltro");
    ws.set_print_area(0, 0, last_row, 12)
        .expect("área de impresión");

    let hidden = workbook.add_worksheet();
    hidden.set_name(HIDDEN_SHEET).expect("nombre de hoja");
    for (row, text) in ["Aprobado", "Rechazado", "Condicionado"].iter().enumerate() {
        hidden.write_string(row as u32, 0, *text).expect("lista");
    }
    hidden.set_hidden(true);

    workbook.save_to_buffer().expect("guardar xlsx en memoria")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::excel::testing::Book;

    #[test]
    fn has_the_visible_sheet_and_the_hidden_list() {
        let book = Book::from_bytes(&build());
        assert_eq!(book.sheet_names(), [SHEET, HIDDEN_SHEET]);
        let wb = book.part("xl/workbook.xml");
        let hidden = &wb[wb.find(r#"name="Hoja1""#).unwrap()..];
        assert!(hidden[..hidden.find("/>").unwrap()].contains(r#"state="hidden""#));
        assert_eq!(book.row_texts(HIDDEN_SHEET, 0, 1), ["Aprobado"]);
    }

    #[test]
    fn cover_headers_and_36_cases_are_where_the_reference_puts_them() {
        let book = Book::from_bytes(&build());
        assert_eq!(book.text(SHEET, 0, 0), "Guion de pruebas");
        for (row, label, value) in COVER {
            assert_eq!(book.text(SHEET, row, 1), label);
            assert_eq!(book.text(SHEET, row, 3), value);
        }
        assert_eq!(book.row_texts(SHEET, HEADER_ROW, 13), HEADERS);
        assert_eq!(book.text(SHEET, FIRST_CASE_ROW, 2), "CP01");
        assert_eq!(book.text(SHEET, FIRST_CASE_ROW + CASES - 1, 2), "CP36");
        assert_eq!(book.text(SHEET, FIRST_CASE_ROW, 8), ""); // Fecha de Prueba vacía
        assert_eq!(book.text(SHEET, FIRST_CASE_ROW, 10), ""); // Status vacío
        assert_eq!(book.text(SHEET, FIRST_CASE_ROW, 9), "1.0");
    }

    #[test]
    fn keeps_the_format_details_that_g4_must_not_break() {
        let book = Book::from_bytes(&build());
        let xml = book.sheet_xml(SHEET);
        assert!(xml.contains(r#"<autoFilter ref="A15:M15"/>"#));
        assert!(xml.contains(r#"<mergeCell ref="A1:M1"/>"#));
        assert!(xml.contains(r#"<mergeCell ref="D3:M3"/>"#));
        assert!(xml.contains(r#"<dataValidations count="2">"#));
        assert!(xml.contains("Crìtico"));
        assert!(xml.contains(r#"sqref="K16:K35""#));
        assert!(book.part("xl/workbook.xml").contains("$A$1:$M$51"));
        assert_eq!(book.pictures(SHEET), 1);
    }
}
