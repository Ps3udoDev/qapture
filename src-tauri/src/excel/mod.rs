//! Excel con carátula Teams4Soft (spec §7, UC-16 y UC-17), generado con `rust_xlsxwriter`.

// Hasta la Task 10 ningún comando usa este módulo: solo lo ejercitan los tests.
#![allow(dead_code)]

pub mod cover;
pub mod data;
pub mod evidence_sheet;
pub mod matrix;
pub mod styles;
#[cfg(test)]
pub(crate) mod testing;
