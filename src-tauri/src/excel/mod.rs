//! Excel con carátula Teams4Soft (spec §7, UC-16 y UC-17), generado con `rust_xlsxwriter`.

// Hasta la Task 10 ningún comando usa este módulo: solo lo ejercitan los tests.
#![allow(dead_code)]

pub mod cover;
pub mod data;
pub mod evidence_sheet;
pub mod images;
pub mod matrix;
pub mod save;
pub mod styles;
pub mod summary;
#[cfg(test)]
pub(crate) mod testing;

use std::path::Path;

use rust_xlsxwriter::{DocProperties, Workbook};

use self::{
    data::{ExportData, ExportSettings},
    styles::Styles,
};
use crate::{
    dto::{ExportProgressDto, ExportResultDto, ExportStage},
    error::AppResult,
};

/// Recibe el avance; se llama desde varios hilos mientras se preparan las imágenes.
pub type Progress<'a> = &'a (dyn Fn(ExportProgressDto) + Sync);

fn report(progress: Progress, stage: ExportStage, current: usize, total: usize) {
    progress(ExportProgressDto {
        stage,
        current: current as i64,
        total: total as i64,
    });
}

pub struct Built {
    pub workbook: Workbook,
    pub missing_images: usize,
}

const COVER: usize = 0;
const MATRIX: usize = 1;
const EVIDENCE: usize = 2;

/// Arma el libro: Carátula, Matriz y, según las opciones, Evidencias y Resumen.
/// Las hojas se crean primero (en su orden final y con el formato por defecto Arial) y la
/// hoja Evidencias se llena antes que la Matriz, que necesita saber dónde empieza cada caso.
pub fn build(data: &ExportData, settings: &ExportSettings, progress: Progress) -> AppResult<Built> {
    let styles = Styles::new();
    report(progress, ExportStage::Preparing, 0, 0);
    let mut workbook = styles::workbook()?;
    let title = if settings.template {
        "Plantilla de pruebas"
    } else {
        "Matriz de pruebas y evidencias"
    };
    workbook.set_properties(
        &DocProperties::new()
            .set_title(title)
            .set_author(&settings.tester)
            .set_company("Teams4Soft"),
    );
    let sheets = 2 + usize::from(settings.include_evidence) + usize::from(settings.include_summary);
    for _ in 0..sheets {
        workbook.add_worksheet();
    }

    let mut missing_images = 0;
    let mut anchors = None;
    if settings.include_evidence {
        let paths: Vec<_> = data
            .cases
            .iter()
            .flat_map(|c| c.evidence.iter().map(|e| e.render_path.clone()))
            .collect();
        let total = paths.len();
        let prepared = images::prepare_all(&paths, settings.image_max_width, &|done| {
            if done == total || done % 10 == 0 {
                report(progress, ExportStage::Images, done, total);
            }
        });
        missing_images = prepared.iter().filter(|i| i.is_none()).count();
        report(progress, ExportStage::Writing, 0, 0);
        let ws = workbook.worksheet_from_index(EVIDENCE)?;
        anchors = Some(evidence_sheet::write(ws, data, prepared, &styles)?);
    } else {
        report(progress, ExportStage::Writing, 0, 0);
    }
    cover::write(
        workbook.worksheet_from_index(COVER)?,
        data,
        settings,
        &styles,
    )?;
    let ws = workbook.worksheet_from_index(MATRIX)?;
    matrix::write(ws, data, anchors.as_deref(), settings.template, &styles)?;
    if settings.include_summary {
        summary::write(workbook.worksheet_from_index(sheets - 1)?, data, &styles)?;
    }
    Ok(Built {
        workbook,
        missing_images,
    })
}

/// Genera el libro y lo guarda en `dest` de forma atómica.
pub fn export(
    data: &ExportData,
    settings: &ExportSettings,
    dest: &Path,
    progress: Progress,
) -> AppResult<ExportResultDto> {
    // Si Excel tiene abierto el archivo, se avisa antes de procesar cientos de imágenes.
    save::ensure_writable(dest)?;
    let mut built = build(data, settings, progress)?;
    report(progress, ExportStage::Saving, 0, 0);
    save::save_atomic(&mut built.workbook, dest)?;
    Ok(ExportResultDto {
        path: dest.to_string_lossy().into_owned(),
        file_name: dest
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        cases: if settings.template {
            0
        } else {
            data.cases.len() as i64
        },
        evidence: if settings.include_evidence {
            data.evidence_total() as i64
        } else {
            0
        },
        missing_images: built.missing_images as i64,
    })
}

#[cfg(test)]
mod tests {
    use std::{sync::Mutex, time::Instant};

    use chrono::NaiveDate;

    use super::*;
    use crate::{
        excel::testing::{Book, case, evidence, fixture, version, write_png},
        media::storage::TempDir,
    };

    fn settings() -> ExportSettings {
        ExportSettings {
            tester: "Ana Pérez".into(),
            version: Some(version("ver-1", "1.0", "2026-10-01")),
            include_evidence: true,
            include_summary: true,
            image_max_width: 940,
            today: NaiveDate::from_ymd_opt(2026, 10, 16).unwrap(),
            template: false,
        }
    }

    /// El fixture con imágenes reales en disco.
    fn with_images(tmp: &TempDir) -> ExportData {
        let mut data = fixture();
        let mut seed = 0;
        for c in &mut data.cases {
            for e in &mut c.evidence {
                seed += 1;
                e.render_path = write_png(tmp.path(), &format!("{seed}.png"), 1200, 700, seed);
            }
        }
        data
    }

    #[test]
    fn exports_four_sheets_that_calamine_reads_back() {
        let tmp = TempDir::new();
        let data = with_images(&tmp);
        let dest = tmp.path().join("T4S_ACME_Recibos.xlsx");
        let stages = Mutex::new(Vec::new());
        let result = export(&data, &settings(), &dest, &|p| {
            stages.lock().unwrap().push(p.stage)
        })
        .unwrap();
        assert_eq!(result.file_name, "T4S_ACME_Recibos.xlsx");
        assert_eq!(
            (result.cases, result.evidence, result.missing_images),
            (4, 3, 0)
        );

        let book = Book::at(&dest);
        assert_eq!(
            book.sheet_names(),
            ["Carátula", "Matriz", "Evidencias", "Resumen"]
        );
        let headers = book.row_texts(matrix::SHEET, matrix::HEADER_ROW, 12);
        let mut expected = vec!["ID".to_string(), "Caso de prueba".to_string()];
        expected.extend(data.project.fields.iter().map(|f| f.label.clone()));
        expected.extend(["Estado".to_string(), "Evidencias".to_string()]);
        assert_eq!(headers, expected);
        assert_eq!(
            book.height(matrix::SHEET),
            matrix::FIRST_ROW + data.cases.len() as u32
        );
        assert_eq!(book.pictures(evidence_sheet::SHEET), 3);
        assert_eq!(book.pictures(cover::SHEET), 2);
        assert_eq!(book.charts(summary::SHEET), 1);

        let stages = stages.into_inner().unwrap();
        assert_eq!(stages.first(), Some(&ExportStage::Preparing));
        assert!(stages.contains(&ExportStage::Images));
        assert_eq!(stages.last(), Some(&ExportStage::Saving));
    }

    #[test]
    fn links_go_both_ways_between_matrix_and_evidence() {
        let tmp = TempDir::new();
        let dest = tmp.path().join("libro.xlsx");
        export(&with_images(&tmp), &settings(), &dest, &|_| {}).unwrap();
        let book = Book::at(&dest);
        let target_row = |location: &str, sheet: &str| -> u32 {
            let prefix = format!("'{sheet}'!A");
            location
                .strip_prefix(&prefix)
                .unwrap()
                .parse::<u32>()
                .unwrap()
                - 1
        };
        for (i, (cell, to)) in book.hyperlinks(matrix::SHEET).iter().enumerate() {
            assert_eq!(cell, &format!("L{}", matrix::row_of(i) + 1));
            let block = target_row(to, evidence_sheet::SHEET);
            let code = format!("EJ-00{}", i + 1);
            assert!(
                book.text(evidence_sheet::SHEET, block, 0)
                    .starts_with(&code),
                "{to}"
            );
            let (_, back) = &book.hyperlinks(evidence_sheet::SHEET)[i];
            let row = target_row(back, matrix::SHEET);
            assert_eq!(book.text(matrix::SHEET, row, 0), code);
        }
    }

    #[test]
    fn sheets_can_be_left_out_and_missing_images_are_counted() {
        let tmp = TempDir::new();
        let mut data = with_images(&tmp);
        data.cases[1].evidence[0].render_path = tmp.path().join("borrada.png");
        let dest = tmp.path().join("libro.xlsx");
        let result = export(&data, &settings(), &dest, &|_| {}).unwrap();
        assert_eq!(result.missing_images, 1);

        let mut only_matrix = settings();
        only_matrix.include_evidence = false;
        only_matrix.include_summary = false;
        let result = export(&data, &only_matrix, &dest, &|_| {}).unwrap();
        assert_eq!(result.evidence, 0);
        assert_eq!(Book::at(&dest).sheet_names(), ["Carátula", "Matriz"]);
    }

    #[test]
    fn template_has_cover_and_empty_matrix() {
        let tmp = TempDir::new();
        let dest = tmp.path().join("plantilla.xlsx");
        let mut template = settings();
        template.template = true;
        template.include_evidence = false;
        template.include_summary = false;
        let result = export(&fixture(), &template, &dest, &|_| {}).unwrap();
        assert_eq!(result.cases, 0);
        let book = Book::at(&dest);
        assert_eq!(book.sheet_names(), ["Carátula", "Matriz"]);
        assert_eq!(
            book.text(cover::SHEET, cover::TITLE_ROW, 0),
            "Plantilla de pruebas"
        );
        assert_eq!(book.text(matrix::SHEET, matrix::FIRST_ROW, 0), "");
    }

    #[test]
    fn workbook_properties_name_the_tester() {
        let tmp = TempDir::new();
        let dest = tmp.path().join("libro.xlsx");
        export(&fixture(), &settings(), &dest, &|_| {}).unwrap();
        let core = Book::at(&dest).part("docProps/core.xml");
        assert!(core.contains("<dc:creator>Ana Pérez</dc:creator>"));
        assert!(core.contains("<dc:title>Matriz de pruebas y evidencias</dc:title>"));
    }

    /// Criterio de aceptación F4: 300 evidencias en menos de 20 s y con memoria acotada.
    /// Se mide en release: `cargo test --release excel::tests::exports_300_evidences -- --ignored --nocapture`
    #[test]
    #[ignore = "medición de rendimiento: correr en release"]
    fn exports_300_evidences_in_under_20_seconds() {
        let tmp = TempDir::new();
        let mut data = fixture();
        data.cases.clear();
        for i in 0..30 {
            let mut c = case(
                &data.project,
                &format!("EJ-{i:03}"),
                "Aprobado",
                &[("entorno", "QA")],
            );
            for j in 0..10 {
                let path = write_png(
                    tmp.path(),
                    &format!("{i}-{j}.png"),
                    1920,
                    1080,
                    (i * 10 + j) as u8,
                );
                c.evidence.push(evidence(&format!("Captura {j}"), &path));
            }
            data.cases.push(c);
        }
        let dest = tmp.path().join("grande.xlsx");
        let started = Instant::now();
        let result = export(&data, &settings(), &dest, &|_| {}).unwrap();
        let elapsed = started.elapsed();
        let size = std::fs::metadata(&dest).unwrap().len();
        println!(
            "300 evidencias: {elapsed:?}, {:.1} MB",
            size as f64 / 1_048_576.0
        );
        assert_eq!(result.evidence, 300);
        assert_eq!(Book::at(&dest).pictures(evidence_sheet::SHEET), 300);
        assert!(elapsed.as_secs_f64() < 20.0, "tardó {elapsed:?}");
        // Cada captura entra reducida a 940 px: el libro (y lo que se tiene en memoria) queda acotado.
        assert!(size < 300 * 1024 * 1024, "{size} bytes");
    }
}
