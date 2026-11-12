//! Valores por fila y carátula para la exportación al formato de origen (G4-T2).
//!
//! Traduce el estado, fecha de ejecución, tester y condiciones de cada caso a
//! las celdas mapeadas en el perfil de importación (`ProfileDefinition`), y calcula
//! los valores agregados de la carátula (fecha máxima, responsable y observaciones).

use chrono::NaiveDate;

use super::cells::{CellValue, CellWrite};
use crate::domain::text::fold;
use crate::dto::CaseDto;
use crate::excel::data::parse_iso;
use crate::transfer::guion::profile::{
    CONDITIONED, CONDITIONED_KEY, FieldRole, ProfileDefinition, Target,
};

#[derive(Debug, Clone, PartialEq)]
pub struct OriginCase {
    pub row: u32,
    pub case: CaseDto,
    pub last_evidence: Option<NaiveDate>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cover<'a> {
    pub tester: &'a str,
    pub observations: &'a str,
}

pub const EXECUTED: [&str; 3] = ["Aprobado", "Fallido", "Bloqueado"];

/// Obtiene la fecha de ejecución para un caso según el perfil y evidencias (D7).
///
/// Si el perfil mapea una columna con rol `ExecutionDate`, intenta parsear el valor ISO del caso.
/// Si está vacío o no es una fecha válida, recurre a `case.last_evidence`.
/// Si ninguna está presente, devuelve `None`.
pub fn execution_date(def: &ProfileDefinition, case: &OriginCase) -> Option<NaiveDate> {
    def.columns
        .iter()
        .find_map(|col| match &col.target {
            Target::Field {
                key,
                role: Some(FieldRole::ExecutionDate),
                ..
            } => case
                .case
                .values
                .get(key)
                .and_then(|raw| parse_iso(raw.trim())),
            _ => None,
        })
        .or(case.last_evidence)
}

/// Genera las escrituras de celda para un caso según el perfil de importación (D6, D7, P4, P5).
///
/// Solo emite escrituras para columnas con `Target::Status` o `Target::Field` con `FieldRole`.
/// Las columnas sin rol, código o título no se modifican.
pub fn row_writes(def: &ProfileDefinition, case: &OriginCase, tester: &str) -> Vec<CellWrite> {
    let mut writes = Vec::new();
    for col in &def.columns {
        let cell = format!("{}{}", col.column, case.row);
        match &col.target {
            Target::Status => {
                let status_val = match case.case.status.as_str() {
                    "Aprobado" => {
                        let is_conditioned = case
                            .case
                            .values
                            .get(CONDITIONED_KEY)
                            .is_some_and(|v| v == "true");
                        let key = if is_conditioned {
                            CONDITIONED
                        } else {
                            "Aprobado"
                        };
                        def.status_map
                            .export
                            .get(key)
                            .filter(|s| !s.is_empty())
                            .cloned()
                            .map(CellValue::Text)
                            .unwrap_or(CellValue::Empty)
                    }
                    "Fallido" => def
                        .status_map
                        .export
                        .get("Fallido")
                        .filter(|s| !s.is_empty())
                        .cloned()
                        .map(CellValue::Text)
                        .unwrap_or(CellValue::Empty),
                    _ => CellValue::Empty,
                };
                writes.push(CellWrite {
                    cell,
                    value: status_val,
                });
            }
            Target::Field {
                role: Some(FieldRole::ExecutionDate),
                ..
            } => {
                let value = match execution_date(def, case) {
                    Some(date) => CellValue::Date(date),
                    None => CellValue::Empty,
                };
                writes.push(CellWrite { cell, value });
            }
            Target::Field {
                key,
                role: Some(FieldRole::Tester),
                ..
            } => {
                let current = case.case.values.get(key).map(|s| s.trim()).unwrap_or("");
                let value = if !current.is_empty() {
                    CellValue::Text(current.to_string())
                } else if EXECUTED.contains(&case.case.status.as_str()) {
                    let t = tester.trim();
                    if !t.is_empty() {
                        CellValue::Text(t.to_string())
                    } else {
                        CellValue::Empty
                    }
                } else {
                    CellValue::Empty
                };
                writes.push(CellWrite { cell, value });
            }
            Target::Field {
                key,
                role: Some(FieldRole::Conditions),
                ..
            } => {
                let current = case.case.values.get(key).map(|s| s.trim()).unwrap_or("");
                let value = if case.case.status == "Bloqueado" {
                    if current.is_empty() {
                        CellValue::Text("Bloqueado".to_string())
                    } else if fold(current).starts_with("bloqueado") {
                        CellValue::Text(current.to_string())
                    } else {
                        CellValue::Text(format!("Bloqueado: {current}"))
                    }
                } else if !current.is_empty() {
                    CellValue::Text(current.to_string())
                } else {
                    CellValue::Empty
                };
                writes.push(CellWrite { cell, value });
            }
            Target::Code | Target::Title | Target::Field { role: None, .. } => {}
        }
    }
    writes
}

/// Genera las escrituras de la carátula según el perfil y los casos (D8, P6).
///
/// - Fecha de ejecución: fecha máxima de ejecución entre todos los casos (si existe alguna).
/// - Responsable: tester del diálogo de exportar (si no está vacío).
/// - Observaciones: observaciones del diálogo de exportar (si no están vacías).
///
/// Las celdas sin valor o no configuradas en el perfil no generan escrituras.
pub fn cover_writes(
    def: &ProfileDefinition,
    cases: &[OriginCase],
    cover: &Cover,
) -> Vec<CellWrite> {
    let mut writes = Vec::new();
    if let Some(cell) = &def.cover.execution_date {
        let max_date = cases.iter().filter_map(|c| execution_date(def, c)).max();
        if let Some(date) = max_date {
            writes.push(CellWrite {
                cell: cell.clone(),
                value: CellValue::Date(date),
            });
        }
    }
    if let Some(cell) = &def.cover.responsible {
        let tester = cover.tester.trim();
        if !tester.is_empty() {
            writes.push(CellWrite {
                cell: cell.clone(),
                value: CellValue::Text(cover.tester.to_string()),
            });
        }
    }
    if let Some(cell) = &def.cover.observations {
        let obs = cover.observations.trim();
        if !obs.is_empty() {
            writes.push(CellWrite {
                cell: cell.clone(),
                value: CellValue::Text(cover.observations.to_string()),
            });
        }
    }
    writes
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use chrono::NaiveDate;

    use super::*;
    use crate::excel::origin::cells::CellValue;
    use crate::transfer::guion::profile::{CONDITIONED_KEY, CoverCells, tests::reference};

    fn origin_case(
        row: u32,
        code: &str,
        status: &str,
        values: &[(&str, &str)],
        last_evidence: Option<NaiveDate>,
    ) -> OriginCase {
        let mut map = BTreeMap::new();
        for (k, v) in values {
            map.insert(k.to_string(), v.to_string());
        }
        OriginCase {
            row,
            case: CaseDto {
                id: format!("case-{row}"),
                project_id: "prj-1".into(),
                code: code.into(),
                title: format!("Escenario {code}"),
                status: status.into(),
                values: map,
                evidence_count: 0,
                created_at: String::new(),
                updated_at: String::new(),
            },
            last_evidence,
        }
    }

    fn find_write<'a>(writes: &'a [CellWrite], cell: &str) -> Option<&'a CellValue> {
        writes.iter().find(|w| w.cell == cell).map(|w| &w.value)
    }

    #[test]
    fn status_uses_the_profile_vocabulary() {
        let def = reference();

        // Aprobado → K16 = Text("Aceptado")
        let c_aprobado = origin_case(16, "CP01", "Aprobado", &[], None);
        let writes = row_writes(&def, &c_aprobado, "");
        assert_eq!(
            find_write(&writes, "K16"),
            Some(&CellValue::Text("Aceptado".into()))
        );

        // Aprobado con condicionado = "true" → "Condicionado"
        let c_condicionado =
            origin_case(16, "CP01", "Aprobado", &[(CONDITIONED_KEY, "true")], None);
        let writes = row_writes(&def, &c_condicionado, "");
        assert_eq!(
            find_write(&writes, "K16"),
            Some(&CellValue::Text("Condicionado".into()))
        );

        // Fallido → "Rechazado"
        let c_fallido = origin_case(16, "CP01", "Fallido", &[], None);
        let writes = row_writes(&def, &c_fallido, "");
        assert_eq!(
            find_write(&writes, "K16"),
            Some(&CellValue::Text("Rechazado".into()))
        );

        // No ejecutado, En ejecución y Por documentar → Empty (D6)
        for st in ["No ejecutado", "En ejecución", "Por documentar"] {
            let c = origin_case(16, "CP01", st, &[], None);
            let writes = row_writes(&def, &c, "");
            assert_eq!(
                find_write(&writes, "K16"),
                Some(&CellValue::Empty),
                "Estado {st} debería escribir Empty en K16"
            );
        }
    }

    #[test]
    fn blocked_cases_write_the_reason_in_conditions() {
        let def = reference();

        // Bloqueado con Condiciones "Sin ambiente" → K = Empty, M = "Bloqueado: Sin ambiente"
        let c_motivo = origin_case(
            16,
            "CP01",
            "Bloqueado",
            &[("condiciones", "Sin ambiente")],
            None,
        );
        let writes = row_writes(&def, &c_motivo, "");
        assert_eq!(find_write(&writes, "K16"), Some(&CellValue::Empty));
        assert_eq!(
            find_write(&writes, "M16"),
            Some(&CellValue::Text("Bloqueado: Sin ambiente".into()))
        );

        // con Condiciones vacío → "Bloqueado"
        let c_vacio = origin_case(16, "CP01", "Bloqueado", &[("condiciones", "")], None);
        let writes = row_writes(&def, &c_vacio, "");
        assert_eq!(find_write(&writes, "K16"), Some(&CellValue::Empty));
        assert_eq!(
            find_write(&writes, "M16"),
            Some(&CellValue::Text("Bloqueado".into()))
        );

        // con "bloqueado: ya escrito" queda igual (P4)
        let c_ya_escrito = origin_case(
            16,
            "CP01",
            "Bloqueado",
            &[("condiciones", "bloqueado: ya escrito")],
            None,
        );
        let writes = row_writes(&def, &c_ya_escrito, "");
        assert_eq!(find_write(&writes, "K16"), Some(&CellValue::Empty));
        assert_eq!(
            find_write(&writes, "M16"),
            Some(&CellValue::Text("bloqueado: ya escrito".into()))
        );

        // Variante con acento y mayúsculas: "BLÓQUEADO - ambiente caído"
        let c_acento = origin_case(
            16,
            "CP01",
            "Bloqueado",
            &[("condiciones", "BLÓQUEADO - ambiente caído")],
            None,
        );
        let writes = row_writes(&def, &c_acento, "");
        assert_eq!(find_write(&writes, "K16"), Some(&CellValue::Empty));
        assert_eq!(
            find_write(&writes, "M16"),
            Some(&CellValue::Text("BLÓQUEADO - ambiente caído".into()))
        );
    }

    #[test]
    fn date_falls_back_to_the_last_evidence() {
        let def = reference();
        let date_field = NaiveDate::from_ymd_opt(2026, 11, 5).unwrap();
        let date_evidence = NaiveDate::from_ymd_opt(2026, 11, 9).unwrap();

        // campo fecha_prueba = "2026-11-05" → I = Date(2026-11-05)
        let c1 = origin_case(
            16,
            "CP01",
            "Aprobado",
            &[("fecha_prueba", "2026-11-05")],
            Some(date_evidence),
        );
        assert_eq!(execution_date(&def, &c1), Some(date_field));
        let writes = row_writes(&def, &c1, "");
        assert_eq!(
            find_write(&writes, "I16"),
            Some(&CellValue::Date(date_field))
        );

        // vacío con last_evidence = 2026-11-09 → esa fecha
        let c2 = origin_case(16, "CP01", "Aprobado", &[], Some(date_evidence));
        assert_eq!(execution_date(&def, &c2), Some(date_evidence));
        let writes = row_writes(&def, &c2, "");
        assert_eq!(
            find_write(&writes, "I16"),
            Some(&CellValue::Date(date_evidence))
        );

        // fecha_prueba inválida con last_evidence → last_evidence
        let c2_invalid = origin_case(
            16,
            "CP01",
            "Aprobado",
            &[("fecha_prueba", "invalida")],
            Some(date_evidence),
        );
        assert_eq!(execution_date(&def, &c2_invalid), Some(date_evidence));

        // sin ninguna → Empty (D7)
        let c3 = origin_case(16, "CP01", "Aprobado", &[], None);
        assert_eq!(execution_date(&def, &c3), None);
        let writes = row_writes(&def, &c3, "");
        assert_eq!(find_write(&writes, "I16"), Some(&CellValue::Empty));
    }

    #[test]
    fn tester_falls_back_only_for_executed_cases() {
        let def = reference();

        // campo vacío, Aprobado, tester "Ana" → L = "Ana"
        let c1 = origin_case(16, "CP01", "Aprobado", &[], None);
        let writes = row_writes(&def, &c1, "Ana");
        assert_eq!(
            find_write(&writes, "L16"),
            Some(&CellValue::Text("Ana".into()))
        );

        // No ejecutado → Empty
        let c2 = origin_case(16, "CP01", "No ejecutado", &[], None);
        let writes = row_writes(&def, &c2, "Ana");
        assert_eq!(find_write(&writes, "L16"), Some(&CellValue::Empty));

        // campo "Luis" → "Luis" siempre (P5), incluso en No ejecutado
        let c3 = origin_case(16, "CP01", "No ejecutado", &[("tester", "Luis")], None);
        let writes = row_writes(&def, &c3, "Ana");
        assert_eq!(
            find_write(&writes, "L16"),
            Some(&CellValue::Text("Luis".into()))
        );

        // Fallido y Bloqueado también son ejecutados → toman "Ana" si están vacíos
        let c4 = origin_case(16, "CP01", "Fallido", &[], None);
        let writes = row_writes(&def, &c4, "Ana");
        assert_eq!(
            find_write(&writes, "L16"),
            Some(&CellValue::Text("Ana".into()))
        );

        let c5 = origin_case(16, "CP01", "Bloqueado", &[], None);
        let writes = row_writes(&def, &c5, "Ana");
        assert_eq!(
            find_write(&writes, "L16"),
            Some(&CellValue::Text("Ana".into()))
        );
    }

    #[test]
    fn columns_without_a_role_are_not_written() {
        let def = reference();
        let c = origin_case(
            16,
            "CP01",
            "Aprobado",
            &[
                ("modulo", "Sistema"),
                ("opcion", "Acceso"),
                ("rn", "RN01"),
                ("criticidad", "Crítico"),
            ],
            None,
        );
        let writes = row_writes(&def, &c, "Ana");
        let cells: Vec<&str> = writes.iter().map(|w| w.cell.as_str()).collect();

        // las columnas C (código), F (título), H (criticidad) y E (RN) del perfil no aparecen en las escrituras
        assert!(!cells.contains(&"C16"), "C16 (Código) no debe escribirse");
        assert!(!cells.contains(&"F16"), "F16 (Título) no debe escribirse");
        assert!(
            !cells.contains(&"H16"),
            "H16 (Criticidad) no debe escribirse"
        );
        assert!(!cells.contains(&"E16"), "E16 (RN) no debe escribirse");

        // Solo las columnas con rol o estado deben estar presentes
        assert_eq!(cells, vec!["I16", "K16", "L16", "M16"]);
    }

    #[test]
    fn cover_writes_last_date_tester_and_observations() {
        let def = reference();
        let _d05 = NaiveDate::from_ymd_opt(2026, 11, 5).unwrap();
        let d09 = NaiveDate::from_ymd_opt(2026, 11, 9).unwrap();

        let c1 = origin_case(
            16,
            "CP01",
            "Aprobado",
            &[("fecha_prueba", "2026-11-05")],
            None,
        );
        let c2 = origin_case(
            17,
            "CP02",
            "Aprobado",
            &[("fecha_prueba", "2026-11-09")],
            None,
        );

        // dos casos con fechas 05 y 09 → D5 = Date(09), D6 = "Ana", D8 = "Texto"
        let cover = Cover {
            tester: "Ana",
            observations: "Texto",
        };
        let writes = cover_writes(&def, &[c1.clone(), c2.clone()], &cover);
        assert_eq!(find_write(&writes, "D5"), Some(&CellValue::Date(d09)));
        assert_eq!(
            find_write(&writes, "D6"),
            Some(&CellValue::Text("Ana".into()))
        );
        assert_eq!(
            find_write(&writes, "D8"),
            Some(&CellValue::Text("Texto".into()))
        );

        // con observaciones "" no hay escritura en D8
        let cover_sin_obs = Cover {
            tester: "Ana",
            observations: "",
        };
        let writes = cover_writes(&def, &[c1.clone(), c2.clone()], &cover_sin_obs);
        assert_eq!(find_write(&writes, "D5"), Some(&CellValue::Date(d09)));
        assert_eq!(
            find_write(&writes, "D6"),
            Some(&CellValue::Text("Ana".into()))
        );
        assert_eq!(find_write(&writes, "D8"), None);

        // sin fechas no hay escritura en D5
        let c_sin_fecha = origin_case(16, "CP01", "Aprobado", &[], None);
        let writes = cover_writes(&def, &[c_sin_fecha], &cover);
        assert_eq!(find_write(&writes, "D5"), None);
        assert_eq!(
            find_write(&writes, "D6"),
            Some(&CellValue::Text("Ana".into()))
        );
        assert_eq!(
            find_write(&writes, "D8"),
            Some(&CellValue::Text("Texto".into()))
        );

        // un perfil con cover vacío no produce nada (P6)
        let mut def_sin_cover = def.clone();
        def_sin_cover.cover = CoverCells::default();
        let writes = cover_writes(&def_sin_cover, &[c1, c2], &cover);
        assert!(writes.is_empty(), "cover vacío no produce escrituras");
    }
}
