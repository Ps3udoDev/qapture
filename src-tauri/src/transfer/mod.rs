//! Importación y exportación de datos: Bitácora v1, paquetes `.qapture` y guiones Excel.
pub mod bundle;
pub mod github_json;
// G1 conecta estos módulos progresivamente hasta registrar los comandos del asistente.
#[allow(dead_code)]
pub mod guion;
pub mod legacy;
pub mod package;
