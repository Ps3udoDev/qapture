# Tarea 4 · Exportación completa y prueba de ida y vuelta (G4-T4)

**Estado:** DONE
**Commits:** `6d18041` (`feat(g4): exportación al formato de origen con prueba de ida y vuelta`)

## Qué se implementó

1. `src-tauri/src/excel/origin/export.rs`:
   - Mensajes fijos: `NO_COPY` y `CHANGED`.
   - `OriginData`: estructura con `source: ProjectSource`, `cases: Vec<OriginCase>`, `unmapped: Vec<String>` y `export: ExportData`.
   - `OriginData::load(conn, files, project_id)`:
     - Consulta `repo::guion::source` (devuelve `Ok(None)` si el proyecto no fue importado).
     - Consulta casos con `repo::cases::list` y filas asociadas con `repo::guion::case_rows`.
     - Traduce las claves de `case.values` desde `field_id` a `field.key` preservando ambos para que `values::row_writes` y `values::cover_writes` encuentren los campos por su clave (`condicionado`, `fecha_prueba`, etc.) y `evidence_sheet` por su `field_id`.
     - Consulta `last_evidence` por caso (`SELECT e.case_id, MAX(e.captured_at) ...`) convirtiendo a fecha local `NaiveDate`.
     - Clasifica casos en mapeados (`OriginCase { row, case, last_evidence }`, ordenados por `row`) y no mapeados (`unmapped`).
     - Carga `ExportData` y filtra según P7: conserva solo los casos con fila en `case_source` y ejecutados (`EXECUTED`) o con evidencias registradas, ordenados por `row`.
   - `Availability` y `check_copy(files, source)`:
     - Resuelve la ruta con `files.absolute(&source.stored_path)`.
     - Devuelve `Unavailable(NO_COPY)` si el archivo no existe o no se puede leer.
     - Comprueba el hash `sha256_hex(&bytes) == source.sha256`: si difiere, devuelve `Unavailable(CHANGED)`.
     - Si coincide, devuelve `Ready(bytes)`.
   - `OriginRequest` y `Built`.
   - `build(data, base, req)`:
     - Lee el libro con `XlsxParts::read(base)`.
     - Rechaza libros Strict OOXML (`http://purl.oclc.org/ooxml/spreadsheetml/main`) con `evidence_graft::STRICT`.
     - Localiza la hoja del guion (`source.sheet`) en `workbook.xml` y resuelve su archivo en el ZIP a través de `xl/_rels/workbook.xml.rels`. Si la hoja no existe, devuelve `Err(validation("La hoja «<sheet>» ya no está en el guion original."))`.
     - Genera escrituras de fila (`values::row_writes`) y carátula (`values::cover_writes`), detecta sistema de fecha con `cells::date1904` y escribe con `cells::write_cells`.
     - Si se eliminaron fórmulas (`!formulas_removed.is_empty()`) y existe `xl/calcChain.xml` (P3): elimina la parte del ZIP, su relación en `workbook.xml.rels` y su `<Override>` en `[Content_Types].xml`.
     - Si `req.include_evidence` y hay casos para evidencias en `data.export`:
       - Determina el nombre libre con `evidence::sheet_name(&wb)` (P8).
       - Obtiene las filas de origen para `BackLink::Sheet { name: &sheet_name, rows: &rows }`.
       - Genera el donante con `evidence::donor` e injerta la hoja con `evidence_graft::graft_sheet`.
     - Si no se solicitan evidencias o no hay casos para la hoja, reempaqueta el ZIP preservando todas las partes originales.
   - `formulas_at_risk(data, base, tester)`: simula las celdas a escribir y reporta las fórmulas en riesgo con `cells::formula_cells`.

2. `src-tauri/src/excel/origin/mod.rs`:
   - Exporta `pub mod export;`.

3. `src-tauri/src/excel/origin/parts.rs`:
   - Añade `pub fn remove(&mut self, name: &str) -> bool` a `XlsxParts`.

4. `src-tauri/src/repo/guion.rs`:
   - Elimina `#[allow(dead_code)]` de `ProjectSource`, `source` y `case_rows`.

5. `src-tauri/src/excel/save.rs`:
   - Añade `pub fn save_bytes_atomic(bytes: &[u8], dest: &Path) -> AppResult<()>`.
   - Test unitario: `save_bytes_atomic_leaves_no_temporary_files`.

6. `src-tauri/src/transfer/guion/import.rs`:
   - Expone `pub(crate) mod tests` y `pub(crate) fn request<'a>(...)` para reuso en los tests de exportación.

## Archivos
- `src-tauri/src/excel/origin/export.rs` (nuevo).
- `src-tauri/src/excel/origin/mod.rs` (modificado).
- `src-tauri/src/excel/origin/parts.rs` (modificado).
- `src-tauri/src/repo/guion.rs` (modificado).
- `src-tauri/src/excel/save.rs` (modificado).
- `src-tauri/src/transfer/guion/import.rs` (modificado).

## Pruebas
Todos los comandos ejecutados desde `src-tauri/`.
- RED:
  - `cargo test excel::origin::export`
  - Fallo inicial por tipos y llamadas no resueltas antes de completar la implementación.
- GREEN:
  - `cargo test excel::origin::export` → 6 passed; 0 failed.
  - `cargo test excel::save` → 7 passed; 0 failed.
  - `cargo test excel::origin` → 44 passed; 0 failed; 1 ignored.
- Lint:
  - `cargo fmt --check` → limpio (código 0).
  - `cargo clippy --all-targets -- -D warnings` → limpio (código 0).

## Autorrevisión
- Cumple los criterios de éxito del spec (criterio 2: byte-a-byte idéntico en partes no tocadas, las partes índice solo agregan entradas, `calcChain.xml` eliminado si se eliminan fórmulas).
- Valida la integridad de la copia (`check_copy`) contra `sha256`.
- El vínculo de retorno de las evidencias enlaza exactamente a la fila de origen del guion del cliente (`'Hoja'!A<fila>`).

## Dudas o riesgos
- Ninguno. Las interfaces y el pipeline completo de exportación al formato de origen están listos para ser consumidos por los comandos de Tauri en la Task 5.

## Revisión (la llena el controlador)
- Spec: Pendiente · Calidad: Pendiente
- Hallazgos y cómo se resolvieron
