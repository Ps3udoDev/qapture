//! Hoja «Evidencias» para el formato de origen (G4-T3 / D10): genera el libro donante
//! con `evidence_sheet` y resuelve nombres de hoja que no colisionen con las del cliente.

use std::path::PathBuf;

use super::evidence_graft::{attr, tags, unescape};
use crate::{
    error::AppResult,
    excel::{
        data::ExportData,
        evidence_sheet::{self, BackLink},
        images, styles,
    },
};

/// P8: Calcula un nombre libre para la hoja de evidencias:
/// «Evidencias»; si ya existe (sin distinguir mayúsculas), «Evidencias (Qapture)»;
/// si también existe, «Evidencias (Qapture 2)», «3»...
pub fn sheet_name(workbook_xml: &str) -> String {
    let existing: Vec<String> = tags(workbook_xml, "<sheet ")
        .filter_map(|t| attr(t, "name"))
        .map(|n| unescape(n).to_lowercase())
        .collect();

    if !existing.contains(&"evidencias".to_string()) {
        return "Evidencias".into();
    }
    if !existing.contains(&"evidencias (qapture)".to_string()) {
        return "Evidencias (Qapture)".into();
    }
    let mut k = 2;
    loop {
        let candidate = format!("Evidencias (Qapture {k})");
        if !existing.contains(&candidate.to_lowercase()) {
            return candidate;
        }
        k += 1;
    }
}

pub struct Donor {
    pub bytes: Vec<u8>,
    pub missing_images: usize,
    pub evidence: usize,
}

/// Genera el libro donante que contiene únicamente la hoja «Evidencias» con sus capturas.
pub fn donor(data: &ExportData, back: BackLink, image_max_width: u32) -> AppResult<Donor> {
    let paths: Vec<PathBuf> = data
        .cases
        .iter()
        .flat_map(|c| c.evidence.iter().map(|e| e.render_path.clone()))
        .collect();
    let evidence = paths.len();
    let prepared = images::prepare_all(&paths, image_max_width, &|_| {});
    let missing_images = prepared.iter().filter(|i| i.is_none()).count();

    let mut workbook = styles::workbook()?;
    let styles = styles::Styles::new();
    let ws = workbook.add_worksheet();
    evidence_sheet::write(ws, data, prepared, &styles, back)?;
    let bytes = workbook.save_to_buffer()?;

    Ok(Donor {
        bytes,
        missing_images,
        evidence,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::excel::{
        evidence_sheet::BackLink,
        guion_fixture,
        origin::evidence_graft::graft_sheet,
        testing::{Book, fixture, write_png},
    };
    use crate::media::storage::TempDir;

    #[test]
    fn the_sheet_name_never_collides() {
        let wb1 = r#"<sheets><sheet name="Guión de Pruebas" sheetId="1" r:id="rId1"/></sheets>"#;
        assert_eq!(sheet_name(wb1), "Evidencias");

        let wb2 = r#"<sheets><sheet name="evidencias" sheetId="1" r:id="rId1"/></sheets>"#;
        assert_eq!(sheet_name(wb2), "Evidencias (Qapture)");

        let wb3 = r#"<sheets>
            <sheet name="Evidencias" sheetId="1" r:id="rId1"/>
            <sheet name="EVIDENCIAS (Qapture)" sheetId="2" r:id="rId2"/>
        </sheets>"#;
        assert_eq!(sheet_name(wb3), "Evidencias (Qapture 2)");

        let wb4 = r#"<sheets>
            <sheet name="Evidencias" sheetId="1" r:id="rId1"/>
            <sheet name="Evidencias (Qapture)" sheetId="2" r:id="rId2"/>
            <sheet name="Evidencias (Qapture 2)" sheetId="3" r:id="rId3"/>
        </sheets>"#;
        assert_eq!(sheet_name(wb4), "Evidencias (Qapture 3)");
    }

    #[test]
    fn donor_has_one_sheet_with_the_images() {
        let tmp = TempDir::new();
        let mut data = fixture();
        let img1 = write_png(tmp.path(), "img1.png", 300, 200, 1);
        let img2 = write_png(tmp.path(), "img2.png", 400, 250, 2);
        data.cases[0].evidence[0].render_path = img1;
        data.cases[0].evidence[1].render_path = img2;
        data.cases.truncate(1);

        let d = donor(
            &data,
            BackLink::Sheet {
                name: "Guión de Pruebas",
                rows: &[16],
            },
            940,
        )
        .unwrap();

        assert_eq!(d.evidence, 2);
        assert_eq!(d.missing_images, 0);

        let grafted = graft_sheet(&guion_fixture::build(), &d.bytes, "Evidencias").unwrap();
        let book = Book::from_bytes(&grafted);
        let sheets = book.sheet_names();
        assert!(sheets.iter().any(|s| s == "Guión de Pruebas"));
        assert!(sheets.iter().any(|s| s == "Evidencias"));
        assert_eq!(book.pictures("Evidencias"), 2);
    }
}
