# Tarea 2 · Valores por fila y carátula (G4-T2)

**Estado:** DONE
**Commits:** `6b6c4fc` (`feat(g4): valores del guion por fila y carátula`)

## Qué se implementó
- `src-tauri/src/excel/origin/values.rs`:
  - `pub struct OriginCase`: fila en el guion original (`row: u32`), DTO del caso (`case: CaseDto`) y fecha de última evidencia (`last_evidence: Option<NaiveDate>`).
  - `pub struct Cover<'a>`: responsable / tester (`tester: &'a str`) y observaciones (`observations: &'a str`).
  - `pub const EXECUTED: [&str; 3] = ["Aprobado", "Fallido", "Bloqueado"];`
  - `pub fn execution_date(def: &ProfileDefinition, case: &OriginCase) -> Option<NaiveDate>`: resolución de fecha por columna con rol `ExecutionDate` (parseada con `excel::data::parse_iso`), con fallback a `case.last_evidence` (D7).
  - `pub fn row_writes(def: &ProfileDefinition, case: &OriginCase, tester: &str) -> Vec<CellWrite>`:
    - Estado traducido mediante `status_map.export` del perfil; soporte de `CONDITIONED` si `condicionado == "true"`; Bloqueado, No ejecutado, En ejecución y Por documentar escriben `CellValue::Empty` (D6).
    - Rol `ExecutionDate`: escribe `Date` o `Empty` (D7).
    - Rol `Tester`: escribe el tester existente del caso; si está vacío y el caso es ejecutado (Aprobado, Fallido, Bloqueado) recurre al tester de Ajustes/diálogo; en casos no ejecutados escribe `Empty` (P5).
    - Rol `Conditions`: escribe el texto de condiciones existente, o en casos Bloqueados antepone el prefijo `Bloqueado:` salvo que ya empiece por bloqueado (usando `domain::text::fold`), o `Bloqueado` si está vacío (P4).
    - Columnas sin rol, código o título no generan escrituras.
  - `pub fn cover_writes(def: &ProfileDefinition, cases: &[OriginCase], cover: &Cover) -> Vec<CellWrite>`:
    - Fecha máxima de ejecución entre todos los casos en la celda configurada en `def.cover.execution_date` (si hay al menos una fecha).
    - Responsable en `def.cover.responsible` (si el tester no está vacío).
    - Observaciones en `def.cover.observations` (si no están vacías; no se borra la celda si el tester no ingresa observaciones, P6).
- `src-tauri/src/excel/origin/mod.rs`: exportación de `pub mod values;`.

## Archivos
- `src-tauri/src/excel/origin/values.rs` (nuevo).
- `src-tauri/src/excel/origin/mod.rs` (modificado).

## Pruebas
Todos los comandos desde `src-tauri/`.
- RED:
  - `cargo test excel::origin::values`
  - 6 fallos esperados por stubs (`todo!("implement row_writes")`, `todo!("implement execution_date")`, `todo!("implement cover_writes")`).
- GREEN:
  - `cargo test excel::origin::values` → 6 passed; 0 failed.
  - `cargo test excel::origin` → 30 passed; 0 failed; 1 ignored.
- Lint:
  - `cargo fmt --check` → limpio.
  - `cargo clippy --all-targets -- -D warnings` → limpio (código 0).

## Autorrevisión
- Cumple con especificación D6, D7, D8, P4, P5 y P6.
- Se reutiliza `parse_iso` de `excel::data` y `fold` de `domain::text` para normalización sin dependencias externas innecesarias.
- Solo emite `CellWrite` para celdas con roles específicos o estado, preservando intactas las demás columnas.

## Dudas o riesgos
- Ninguno. Las interfaces quedan listas para su consumo en la Task 4 (`OriginData` y generación de la exportación completa).

## Revisión (la llena el controlador)
- Spec: Pendiente · Calidad: Pendiente
- Hallazgos y cómo se resolvieron
