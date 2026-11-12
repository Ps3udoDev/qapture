//! Exportación al formato de origen y comprobación de la copia original (G4-T4).
//!
//! Permite comprobar la disponibilidad e integridad de la copia del guion (`check_copy`),
//! cargar los datos de origen y mapeos (`OriginData::load`), calcular las fórmulas en riesgo
//! (`formulas_at_risk`) y generar el `.xlsx` ejecutado reescribiendo únicamente las celdas
//! destino e injertando la hoja «Evidencias» si corresponde (`build`).

use std::collections::HashMap;

use chrono::NaiveDate;
use rusqlite::Connection;

use super::{
    cells, evidence, evidence_graft,
    parts::XlsxParts,
    values::{self, EXECUTED, OriginCase},
};
use crate::{
    error::{AppResult, validation},
    excel::{data::ExportData, evidence_sheet::BackLink},
    media::{fixtures::sha256_hex, storage::EvidenceFiles},
    repo::guion::{self, ProjectSource},
};

pub const NO_COPY: &str = "No se encontró el guion original de este proyecto. Vuelve a importarlo para exportar en su formato.";
pub const CHANGED: &str = "El guion original cambió desde que se importó.";

const STRICT_NS: &str = "http://purl.oclc.org/ooxml/spreadsheetml/main";

#[derive(Debug, Clone)]
pub struct OriginData {
    pub source: ProjectSource,
    pub cases: Vec<OriginCase>,
    pub unmapped: Vec<String>, // códigos sin fila en el guion original
    pub export: ExportData,    // solo casos para la hoja Evidencias (P7)
}

fn parse_local_date(iso: &str) -> Option<NaiveDate> {
    chrono::DateTime::parse_from_rfc3339(iso)
        .map(|dt| dt.with_timezone(&chrono::Local).date_naive())
        .ok()
        .or_else(|| {
            chrono::NaiveDateTime::parse_from_str(iso, "%Y-%m-%d %H:%M:%S")
                .map(|ndt| ndt.date())
                .ok()
        })
        .or_else(|| crate::excel::data::parse_iso(iso))
}

impl OriginData {
    pub fn load(
        conn: &Connection,
        files: &EvidenceFiles,
        project_id: &str,
    ) -> AppResult<Option<Self>> {
        let Some(source) = guion::source(conn, project_id)? else {
            return Ok(None);
        };

        let all_cases = crate::repo::cases::list(conn, project_id)?;
        let row_map: HashMap<String, u32> =
            guion::case_rows(conn, project_id)?.into_iter().collect();

        let fields = crate::repo::fields::list_for_project(conn, project_id)?;
        let id_to_key: HashMap<String, String> =
            fields.into_iter().map(|f| (f.id, f.key)).collect();

        let mut stmt = conn.prepare(
            "SELECT e.case_id, MAX(e.captured_at) FROM evidence e
             JOIN test_case c ON c.id = e.case_id
             WHERE c.project_id = ?1 GROUP BY e.case_id",
        )?;
        let last_ev_rows = stmt.query_map([project_id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })?;
        let mut last_evidence_map: HashMap<String, NaiveDate> = HashMap::new();
        for row in last_ev_rows {
            let (case_id, iso) = row?;
            if let Some(date) = parse_local_date(&iso) {
                last_evidence_map.insert(case_id, date);
            }
        }

        let mut cases = Vec::new();
        let mut unmapped = Vec::new();

        for mut case in all_cases {
            let mut mapped_values = std::collections::BTreeMap::new();
            for (k, v) in &case.values {
                if let Some(key) = id_to_key.get(k) {
                    mapped_values.insert(key.clone(), v.clone());
                }
                mapped_values.insert(k.clone(), v.clone());
            }
            case.values = mapped_values;

            if let Some(&row) = row_map.get(&case.id) {
                let last_evidence = last_evidence_map.get(&case.id).copied();
                cases.push(OriginCase {
                    row,
                    case,
                    last_evidence,
                });
            } else {
                unmapped.push(case.code);
            }
        }
        cases.sort_by_key(|c| c.row);

        let mut export = ExportData::load(conn, files, project_id)?;
        export.cases.retain(|c| {
            row_map.contains_key(&c.case.id)
                && (EXECUTED.contains(&c.case.status.as_str()) || !c.evidence.is_empty())
        });
        export.cases.sort_by_key(|c| row_map[&c.case.id]);

        Ok(Some(Self {
            source,
            cases,
            unmapped,
            export,
        }))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Availability {
    Ready(Vec<u8>),
    Unavailable(&'static str),
}

pub fn check_copy(files: &EvidenceFiles, source: &ProjectSource) -> Availability {
    let path = files.absolute(&source.stored_path);
    let Ok(bytes) = std::fs::read(&path) else {
        return Availability::Unavailable(NO_COPY);
    };
    if sha256_hex(&bytes) != source.sha256 {
        return Availability::Unavailable(CHANGED);
    }
    Availability::Ready(bytes)
}

#[derive(Debug, Clone, Copy)]
pub struct OriginRequest<'a> {
    pub tester: &'a str,
    pub observations: &'a str,
    pub include_evidence: bool,
    pub image_max_width: u32,
}

#[derive(Debug, Clone)]
pub struct Built {
    pub bytes: Vec<u8>,
    pub cases_written: usize,
    pub evidence: usize,
    pub missing_images: usize,
    pub formulas_removed: Vec<String>,
}

fn find_sheet_part(parts: &XlsxParts, sheet_name: &str) -> AppResult<String> {
    let workbook_xml = parts.text("xl/workbook.xml")?;
    if workbook_xml.contains(STRICT_NS) {
        return Err(validation(evidence_graft::STRICT));
    }

    let mut sheet_rid = None;
    for tag in evidence_graft::tags(&workbook_xml, "<sheet ") {
        if let Some(raw_name) = evidence_graft::attr(tag, "name") {
            let name = evidence_graft::unescape(raw_name);
            if name.eq_ignore_ascii_case(sheet_name)
                && let Some(rid) =
                    evidence_graft::attr(tag, "r:id").or_else(|| evidence_graft::attr(tag, "id"))
            {
                sheet_rid = Some(rid.to_string());
                break;
            }
        }
    }

    let Some(r_id) = sheet_rid else {
        return Err(validation(format!(
            "La hoja «{sheet_name}» ya no está en el guion original."
        )));
    };

    let rels_xml = parts.text("xl/_rels/workbook.xml.rels")?;
    for tag in evidence_graft::tags(&rels_xml, "<Relationship ") {
        if evidence_graft::attr(tag, "Id") == Some(&r_id)
            && let Some(target) = evidence_graft::attr(tag, "Target")
        {
            let target = target.trim_start_matches('/');
            let path = if target.starts_with("xl/") {
                target.to_string()
            } else {
                format!("xl/{target}")
            };
            return Ok(path);
        }
    }

    Err(validation(format!(
        "No se encontró la parte de la hoja «{sheet_name}» en el guion original."
    )))
}

fn remove_xml_tags(xml: &str, open: &str, predicate: impl Fn(&str) -> bool) -> String {
    let mut out = String::with_capacity(xml.len());
    let mut cursor = 0;
    while let Some(offset) = xml[cursor..].find(open) {
        let tag_start = cursor + offset;
        out.push_str(&xml[cursor..tag_start]);
        if let Some(end_offset) = xml[tag_start..].find('>') {
            let tag_end = tag_start + end_offset + 1;
            let tag = &xml[tag_start..tag_end];
            if predicate(tag) {
                cursor = tag_end;
                if xml[cursor..].starts_with("\r\n") {
                    cursor += 2;
                } else if xml[cursor..].starts_with('\n') {
                    cursor += 1;
                }
            } else {
                out.push_str(tag);
                cursor = tag_end;
            }
        } else {
            out.push_str(&xml[tag_start..]);
            cursor = xml.len();
            break;
        }
    }
    out.push_str(&xml[cursor..]);
    out
}

fn remove_calc_chain(parts: &mut XlsxParts) -> AppResult<()> {
    parts.remove("xl/calcChain.xml");

    if let Ok(rels) = parts.text("xl/_rels/workbook.xml.rels") {
        let cleaned = remove_xml_tags(&rels, "<Relationship", |tag| {
            evidence_graft::attr(tag, "Target")
                .is_some_and(|t| t == "calcChain.xml" || t.ends_with("/calcChain.xml"))
                || evidence_graft::attr(tag, "Type").is_some_and(|t| t.ends_with("/calcChain"))
        });
        parts.set("xl/_rels/workbook.xml.rels", cleaned.into_bytes());
    }

    if let Ok(content_types) = parts.text("[Content_Types].xml") {
        let cleaned = remove_xml_tags(&content_types, "<Override", |tag| {
            evidence_graft::attr(tag, "PartName")
                .is_some_and(|p| p == "/xl/calcChain.xml" || p == "xl/calcChain.xml")
        });
        parts.set("[Content_Types].xml", cleaned.into_bytes());
    }

    Ok(())
}

pub fn build(data: &OriginData, base: &[u8], req: &OriginRequest) -> AppResult<Built> {
    let mut parts = XlsxParts::read(base)?;
    let workbook_xml = parts.text("xl/workbook.xml")?;
    if workbook_xml.contains(STRICT_NS) {
        return Err(validation(evidence_graft::STRICT));
    }

    let sheet_path = find_sheet_part(&parts, &data.source.sheet)?;
    let sheet_text = parts.text(&sheet_path)?;
    let styles_text = parts.text("xl/styles.xml").unwrap_or_default();
    let is_1904 = cells::date1904(&workbook_xml);

    let mut writes = Vec::new();
    for case in &data.cases {
        writes.extend(values::row_writes(
            &data.source.definition,
            case,
            req.tester,
        ));
    }
    let cover = values::Cover {
        tester: req.tester,
        observations: req.observations,
    };
    writes.extend(values::cover_writes(
        &data.source.definition,
        &data.cases,
        &cover,
    ));

    let edit = cells::write_cells(&sheet_text, &styles_text, is_1904, &writes)?;
    parts.set(&sheet_path, edit.xml.into_bytes());

    if !edit.formulas_removed.is_empty() && parts.get("xl/calcChain.xml").is_some() {
        remove_calc_chain(&mut parts)?;
    }

    if req.include_evidence && !data.export.cases.is_empty() {
        let wb_current = parts.text("xl/workbook.xml")?;
        let sheet_name = evidence::sheet_name(&wb_current);

        let rows: Vec<u32> = data
            .export
            .cases
            .iter()
            .filter_map(|ec| {
                data.cases
                    .iter()
                    .find(|oc| oc.case.id == ec.case.id)
                    .map(|oc| oc.row)
            })
            .collect();

        let donor = evidence::donor(
            &data.export,
            BackLink::Sheet {
                name: &sheet_name,
                rows: &rows,
            },
            req.image_max_width,
        )?;

        let base_with_edits = parts.write()?;
        let grafted_bytes =
            evidence_graft::graft_sheet(&base_with_edits, &donor.bytes, &sheet_name)?;

        Ok(Built {
            bytes: grafted_bytes,
            cases_written: data.cases.len(),
            evidence: donor.evidence,
            missing_images: donor.missing_images,
            formulas_removed: edit.formulas_removed,
        })
    } else {
        let bytes = parts.write()?;
        Ok(Built {
            bytes,
            cases_written: data.cases.len(),
            evidence: 0,
            missing_images: 0,
            formulas_removed: edit.formulas_removed,
        })
    }
}

pub fn formulas_at_risk(data: &OriginData, base: &[u8], tester: &str) -> AppResult<Vec<String>> {
    let parts = XlsxParts::read(base)?;
    let sheet_path = find_sheet_part(&parts, &data.source.sheet)?;
    let sheet_text = parts.text(&sheet_path)?;

    let mut writes = Vec::new();
    for case in &data.cases {
        writes.extend(values::row_writes(&data.source.definition, case, tester));
    }
    let cover = values::Cover {
        tester,
        observations: "",
    };
    writes.extend(values::cover_writes(
        &data.source.definition,
        &data.cases,
        &cover,
    ));

    let target_cells: Vec<String> = writes.into_iter().map(|w| w.cell).collect();
    cells::formula_cells(&sheet_text, &target_cells)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use rusqlite::params;

    use super::*;
    use crate::{
        dto::CaseInput,
        excel::{
            guion_fixture,
            origin::evidence_graft::tags,
            testing::{Book, write_png},
        },
        media::storage::TempDir,
        repo::{cases, test_support},
        transfer::guion::{import, profile, read},
    };

    fn setup_imported_guion() -> (rusqlite::Connection, TempDir, Vec<u8>, String) {
        let conn = test_support::conn();
        let tmp = TempDir::new();
        let files = tmp.files();
        let bytes = guion_fixture::build();
        let book = read::read(&bytes).unwrap();
        let def = profile::tests::reference();

        let summary = import::import(
            &conn,
            &files,
            import::tests::request(&book, &bytes, &def, None),
        )
        .unwrap();
        let project_id = summary.projects[0].id.clone();
        (conn, tmp, bytes, project_id)
    }

    #[test]
    fn round_trip_changes_only_target_cells_and_adds_the_sheet() {
        let (conn, tmp, base, project_id) = setup_imported_guion();
        let files = tmp.files();

        let case_list = cases::list(&conn, &project_id).unwrap();
        let cp01 = &case_list[0];
        let cp02 = &case_list[1];
        let cp03 = &case_list[2];
        let cp04 = &case_list[3];

        // CP01: Aprobado + 1 evidencia
        let rel_img = EvidenceFiles::render_rel(&project_id, "e1");
        fs::create_dir_all(files.project_dir(&project_id)).unwrap();
        write_png(
            &files.project_dir(&project_id),
            "e1.render.png",
            100,
            100,
            1,
        );
        conn.execute(
            "INSERT INTO evidence (id, case_id, position, description, original_path, rendered_path, annotations, width, height, captured_at, updated_at) VALUES ('e1', ?1, 0, 'Captura CP01', ?2, ?2, '{}', 100, 100, '2026-11-10T12:00:00Z', '2026-11-10T12:00:00Z')",
            params![cp01.id, rel_img],
        ).unwrap();
        cases::update(
            &conn,
            &cp01.id,
            &CaseInput {
                title: cp01.title.clone(),
                status: "Aprobado".into(),
                values: cp01.values.clone(),
            },
        )
        .unwrap();

        // CP02: Fallido
        cases::update(
            &conn,
            &cp02.id,
            &CaseInput {
                title: cp02.title.clone(),
                status: "Fallido".into(),
                values: cp02.values.clone(),
            },
        )
        .unwrap();

        // CP03: Aprobado + condicionado
        let fields = crate::repo::fields::list_for_project(&conn, &project_id).unwrap();
        let cond_field = fields
            .iter()
            .find(|f| f.key == profile::CONDITIONED_KEY)
            .unwrap();
        let mut cp03_values = cp03.values.clone();
        cp03_values.insert(cond_field.id.clone(), "true".into());
        cases::update(
            &conn,
            &cp03.id,
            &CaseInput {
                title: cp03.title.clone(),
                status: "Aprobado".into(),
                values: cp03_values,
            },
        )
        .unwrap();

        // CP04: Bloqueado
        cases::update(
            &conn,
            &cp04.id,
            &CaseInput {
                title: cp04.title.clone(),
                status: "Bloqueado".into(),
                values: cp04.values.clone(),
            },
        )
        .unwrap();

        let data = OriginData::load(&conn, &files, &project_id)
            .unwrap()
            .unwrap();

        assert_eq!(
            check_copy(&files, &data.source),
            Availability::Ready(base.clone())
        );

        let req = OriginRequest {
            tester: "Ana",
            observations: "Prueba completada",
            include_evidence: true,
            image_max_width: 940,
        };

        let built = build(&data, &base, &req).unwrap();
        assert_eq!(built.cases_written, 36);
        assert_eq!(built.evidence, 1);
        assert_eq!(built.missing_images, 0);

        // Criterio de éxito 2: Todas las partes del original, salvo la hoja del guion
        // y las 3 partes índice, son idénticas byte a byte.
        let orig_parts = XlsxParts::read(&base).unwrap();
        let built_parts = XlsxParts::read(&built.bytes).unwrap();

        for name in orig_parts.names() {
            if name == "xl/worksheets/sheet1.xml"
                || name == "xl/workbook.xml"
                || name == "xl/_rels/workbook.xml.rels"
                || name == "[Content_Types].xml"
            {
                continue;
            }
            assert_eq!(
                orig_parts.get(name),
                built_parts.get(name),
                "parte modificada inesperadamente: {name}"
            );
        }

        // Las partes índice solo agregan entradas
        let orig_wb = orig_parts.text("xl/workbook.xml").unwrap();
        let built_wb = built_parts.text("xl/workbook.xml").unwrap();
        for sheet_tag in tags(&orig_wb, "<sheet ") {
            assert!(
                built_wb.contains(sheet_tag),
                "sheet tag missing: {sheet_tag}"
            );
        }

        let orig_wb_rels = orig_parts.text("xl/_rels/workbook.xml.rels").unwrap();
        let built_wb_rels = built_parts.text("xl/_rels/workbook.xml.rels").unwrap();
        for rel_tag in tags(&orig_wb_rels, "<Relationship ") {
            assert!(
                built_wb_rels.contains(rel_tag),
                "rel tag missing: {rel_tag}"
            );
        }

        let orig_ct = orig_parts.text("[Content_Types].xml").unwrap();
        let built_ct = built_parts.text("[Content_Types].xml").unwrap();
        for override_tag in tags(&orig_ct, "<Override ") {
            assert!(
                built_ct.contains(override_tag),
                "override missing: {override_tag}"
            );
        }
        for default_tag in tags(&orig_ct, "<Default ") {
            assert!(
                built_ct.contains(default_tag),
                "default missing: {default_tag}"
            );
        }

        // Calamine: Celdas del guion
        let book_orig = Book::from_bytes(&base);
        let book_built = Book::from_bytes(&built.bytes);

        // Columnas modificables:
        // I = col 8, K = col 10, L = col 11, M = col 12 (0-indexed)
        // Filas 16..=51 (0-indexed 15..=50)
        // Carátula: D5 (row 4, col 3), D6 (row 5, col 3), D8 (row 7, col 3)
        let guion_sheet = "Guión de Pruebas";
        let height = book_orig.height(guion_sheet);
        for row in 0..height {
            for col in 0..14 {
                let is_cover = col == 3 && (row == 4 || row == 5 || row == 7);
                let is_target =
                    (15..=50).contains(&row) && (col == 8 || col == 10 || col == 11 || col == 12);
                if !is_cover && !is_target {
                    assert_eq!(
                        book_orig.text(guion_sheet, row, col),
                        book_built.text(guion_sheet, row, col),
                        "celda fuera de destino modificada en fila {row}, col {col}"
                    );
                }
            }
        }

        // K16..K19: Aceptado, Rechazado, Condicionado, ""
        assert_eq!(book_built.text(guion_sheet, 15, 10), "Aceptado");
        assert_eq!(book_built.text(guion_sheet, 16, 10), "Rechazado");
        assert_eq!(book_built.text(guion_sheet, 17, 10), "Condicionado");
        assert_eq!(book_built.text(guion_sheet, 18, 10), "");

        // M19 empieza por Bloqueado
        let m19 = book_built.text(guion_sheet, 18, 12);
        assert!(
            m19.starts_with("Bloqueado"),
            "M19 no empieza por Bloqueado: '{m19}'"
        );

        // Hoja Evidencias existe con sección CP01
        assert!(book_built.sheet_names().contains(&"Evidencias".to_string()));
        let ev_text = book_built.text("Evidencias", 3, 0);
        assert!(
            ev_text.contains("CP01"),
            "La hoja Evidencias no contiene CP01: '{ev_text}'"
        );
    }

    #[test]
    fn without_evidence_sheet_only_the_guion_sheet_changes() {
        let (conn, tmp, base, project_id) = setup_imported_guion();
        let files = tmp.files();

        let data = OriginData::load(&conn, &files, &project_id)
            .unwrap()
            .unwrap();

        let req = OriginRequest {
            tester: "Ana",
            observations: "",
            include_evidence: false,
            image_max_width: 940,
        };

        let built = build(&data, &base, &req).unwrap();
        assert_eq!(built.evidence, 0);

        let orig_parts = XlsxParts::read(&base).unwrap();
        let built_parts = XlsxParts::read(&built.bytes).unwrap();

        assert_eq!(orig_parts.names(), built_parts.names());
        for name in orig_parts.names() {
            if name == "xl/worksheets/sheet1.xml" {
                assert_ne!(
                    orig_parts.get(name),
                    built_parts.get(name),
                    "la hoja del guion debería haber cambiado"
                );
            } else {
                assert_eq!(
                    orig_parts.get(name),
                    built_parts.get(name),
                    "la parte {name} no debería haber cambiado"
                );
            }
        }
    }

    #[test]
    fn writes_by_source_row_not_by_code() {
        let (conn, tmp, base, project_id) = setup_imported_guion();
        let files = tmp.files();

        let case_list = cases::list(&conn, &project_id).unwrap();
        let cp05 = &case_list[4]; // row 20
        let cp06 = &case_list[5]; // row 21

        // Renombrar CP05 a X-1 y ponerle estado Aprobado
        conn.execute(
            "UPDATE test_case SET code = 'X-1', status = 'Aprobado' WHERE id = ?1",
            [cp05.id.clone()],
        )
        .unwrap();

        // Borrar CP06
        cases::delete(&conn, &cp06.id).unwrap();

        // Crear caso nuevo CP99
        let cp99 = cases::create(
            &conn,
            &project_id,
            &CaseInput {
                title: "Caso nuevo fuera de guion".into(),
                status: "Aprobado".into(),
                values: std::collections::BTreeMap::new(),
            },
        )
        .unwrap();
        conn.execute(
            "UPDATE test_case SET code = 'CP99' WHERE id = ?1",
            [cp99.id],
        )
        .unwrap();

        let data = OriginData::load(&conn, &files, &project_id)
            .unwrap()
            .unwrap();

        assert_eq!(data.unmapped, vec!["CP99"]);

        let req = OriginRequest {
            tester: "Ana",
            observations: "",
            include_evidence: false,
            image_max_width: 940,
        };

        let built = build(&data, &base, &req).unwrap();
        let book_orig = Book::from_bytes(&base);
        let book_built = Book::from_bytes(&built.bytes);

        // Fila 20 (0-indexed 19): CP05 ahora X-1 se escribió en fila 20 -> K20 = "Aceptado"
        assert_eq!(book_built.text("Guión de Pruebas", 19, 10), "Aceptado");

        // Fila 21 (0-indexed 20): CP06 fue borrado -> queda como en el original
        for col in 0..14 {
            assert_eq!(
                book_orig.text("Guión de Pruebas", 20, col),
                book_built.text("Guión de Pruebas", 20, col),
                "la fila 21 no quedó como en el original en col {col}"
            );
        }
    }

    #[test]
    fn removing_formulas_drops_the_calc_chain() {
        let (conn, tmp, base, project_id) = setup_imported_guion();
        let files = tmp.files();

        // Modificamos base para que I16 tenga fórmula y exista calcChain.xml
        let mut parts = XlsxParts::read(&base).unwrap();
        let sheet1 = parts.text("xl/worksheets/sheet1.xml").unwrap();
        // Inyectar fórmula en I16
        assert!(
            sheet1.contains(r#"r="J16""#),
            "sheet1 debe contener celda J16"
        );
        let sheet1_mod = sheet1.replace(
            r#"<c r="J16""#,
            r#"<c r="I16" s="2"><f>TODAY()</f><v>45962</v></c><c r="J16""#,
        );
        parts.set("xl/worksheets/sheet1.xml", sheet1_mod.into_bytes());

        // Agregar calcChain.xml
        parts.set(
            "xl/calcChain.xml",
            b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<calcChain xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><c r=\"I16\" i=\"1\"/></calcChain>".to_vec(),
        );
        let rels = parts.text("xl/_rels/workbook.xml.rels").unwrap();
        let rels_mod = rels.replace(
            "</Relationships>",
            "<Relationship Id=\"rIdCalc\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/calcChain\" Target=\"calcChain.xml\"/></Relationships>",
        );
        parts.set("xl/_rels/workbook.xml.rels", rels_mod.into_bytes());

        let ct = parts.text("[Content_Types].xml").unwrap();
        let ct_mod = ct.replace(
            "</Types>",
            "<Override PartName=\"/xl/calcChain.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.calcChain+xml\"/></Types>",
        );
        parts.set("[Content_Types].xml", ct_mod.into_bytes());

        let modified_base = parts.write().unwrap();

        let data = OriginData::load(&conn, &files, &project_id)
            .unwrap()
            .unwrap();

        let at_risk = formulas_at_risk(&data, &modified_base, "Ana").unwrap();
        assert_eq!(at_risk, vec!["I16"]);

        let req = OriginRequest {
            tester: "Ana",
            observations: "",
            include_evidence: false,
            image_max_width: 940,
        };

        let built = build(&data, &modified_base, &req).unwrap();
        assert_eq!(built.formulas_removed, vec!["I16"]);

        let res_parts = XlsxParts::read(&built.bytes).unwrap();
        assert!(res_parts.get("xl/calcChain.xml").is_none());

        let res_rels = res_parts.text("xl/_rels/workbook.xml.rels").unwrap();
        assert!(!res_rels.contains("calcChain.xml"));

        let res_ct = res_parts.text("[Content_Types].xml").unwrap();
        assert!(!res_ct.contains("calcChain.xml"));
    }

    #[test]
    fn copy_missing_or_changed_is_unavailable() {
        let (conn, tmp, _, project_id) = setup_imported_guion();
        let files = tmp.files();

        let data = OriginData::load(&conn, &files, &project_id)
            .unwrap()
            .unwrap();

        let path = files.absolute(&data.source.stored_path);
        let original_bytes = fs::read(&path).unwrap();

        // 1. Borrar archivo -> Unavailable(NO_COPY)
        fs::remove_file(&path).unwrap();
        assert_eq!(
            check_copy(&files, &data.source),
            Availability::Unavailable(NO_COPY)
        );

        // 2. Modificar un byte -> Unavailable(CHANGED)
        let mut changed_bytes = original_bytes;
        changed_bytes[0] ^= 0xFF;
        fs::write(&path, changed_bytes).unwrap();
        assert_eq!(
            check_copy(&files, &data.source),
            Availability::Unavailable(CHANGED)
        );
    }

    #[test]
    fn a_renamed_guion_sheet_is_rejected() {
        let (conn, tmp, base, project_id) = setup_imported_guion();
        let files = tmp.files();

        let data = OriginData::load(&conn, &files, &project_id)
            .unwrap()
            .unwrap();

        let mut parts = XlsxParts::read(&base).unwrap();
        let wb = parts.text("xl/workbook.xml").unwrap();
        let wb_renamed = wb.replace("Guión de Pruebas", "Hoja Renombrada");
        parts.set("xl/workbook.xml", wb_renamed.into_bytes());
        let bad_base = parts.write().unwrap();

        let req = OriginRequest {
            tester: "Ana",
            observations: "",
            include_evidence: false,
            image_max_width: 940,
        };

        let err = build(&data, &bad_base, &req).unwrap_err();
        assert_eq!(
            err.to_string(),
            "La hoja «Guión de Pruebas» ya no está en el guion original."
        );
    }

    #[test]
    #[ignore = "genera archivo para abrir a mano"]
    fn write_origin_for_manual_check() {
        let (conn, tmp, base, project_id) = setup_imported_guion();
        let files = tmp.files();

        let case_list = cases::list(&conn, &project_id).unwrap();
        let cp01 = &case_list[0];
        let cp02 = &case_list[1];
        let cp03 = &case_list[2];
        let cp04 = &case_list[3];

        // CP01: Aprobado + 2 evidencias
        fs::create_dir_all(files.project_dir(&project_id)).unwrap();
        let rel_img1 = EvidenceFiles::render_rel(&project_id, "e1");
        write_png(
            &files.project_dir(&project_id),
            "e1.render.png",
            100,
            100,
            1,
        );
        conn.execute(
            "INSERT INTO evidence (id, case_id, position, description, original_path, rendered_path, annotations, width, height, captured_at, updated_at) VALUES ('e1', ?1, 0, 'Captura 1 del CP01', ?2, ?2, '{}', 100, 100, '2026-11-10T12:00:00Z', '2026-11-10T12:00:00Z')",
            params![cp01.id, rel_img1],
        ).unwrap();

        let rel_img2 = EvidenceFiles::render_rel(&project_id, "e2");
        write_png(
            &files.project_dir(&project_id),
            "e2.render.png",
            100,
            100,
            2,
        );
        conn.execute(
            "INSERT INTO evidence (id, case_id, position, description, original_path, rendered_path, annotations, width, height, captured_at, updated_at) VALUES ('e2', ?1, 1, 'Captura 2 del CP01', ?2, ?2, '{}', 100, 100, '2026-11-10T12:05:00Z', '2026-11-10T12:05:00Z')",
            params![cp01.id, rel_img2],
        ).unwrap();

        cases::update(
            &conn,
            &cp01.id,
            &CaseInput {
                title: cp01.title.clone(),
                status: "Aprobado".into(),
                values: cp01.values.clone(),
            },
        )
        .unwrap();

        // CP02: Fallido
        cases::update(
            &conn,
            &cp02.id,
            &CaseInput {
                title: cp02.title.clone(),
                status: "Fallido".into(),
                values: cp02.values.clone(),
            },
        )
        .unwrap();

        // CP03: Aprobado condicionado (condicionado = "true")
        let fields = crate::repo::fields::list_for_project(&conn, &project_id).unwrap();
        let cond_field = fields
            .iter()
            .find(|f| f.key == profile::CONDITIONED_KEY)
            .unwrap();
        let mut cp03_values = cp03.values.clone();
        cp03_values.insert(cond_field.id.clone(), "true".into());
        cases::update(
            &conn,
            &cp03.id,
            &CaseInput {
                title: cp03.title.clone(),
                status: "Aprobado".into(),
                values: cp03_values,
            },
        )
        .unwrap();

        // CP04: Bloqueado (con Condiciones "Sin ambiente")
        let conditions_field = fields
            .iter()
            .find(|f| f.label.eq_ignore_ascii_case("condiciones") || f.key == "condiciones")
            .unwrap();
        let mut cp04_values = cp04.values.clone();
        cp04_values.insert(conditions_field.id.clone(), "Sin ambiente".into());
        cases::update(
            &conn,
            &cp04.id,
            &CaseInput {
                title: cp04.title.clone(),
                status: "Bloqueado".into(),
                values: cp04_values,
            },
        )
        .unwrap();

        let data = OriginData::load(&conn, &files, &project_id)
            .unwrap()
            .unwrap();

        let req = OriginRequest {
            tester: "Ana Tester",
            observations: "Ejecución de prueba del bloque G4.",
            include_evidence: true,
            image_max_width: 940,
        };

        let built = build(&data, &base, &req).unwrap();
        assert_eq!(built.cases_written, 36);
        assert_eq!(built.evidence, 2);

        let out_dir = std::path::Path::new("target/manual-check");
        fs::create_dir_all(out_dir).unwrap();
        let dest = out_dir.join("guion_ejecutado.xlsx");
        fs::write(&dest, &built.bytes).unwrap();

        if std::path::Path::new("../package.json").exists() {
            let root_out = std::path::Path::new("../target/manual-check");
            let _ = fs::create_dir_all(root_out);
            let _ = fs::write(root_out.join("guion_ejecutado.xlsx"), &built.bytes);
        }
    }
}
