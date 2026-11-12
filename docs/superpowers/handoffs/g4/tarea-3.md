# Tarea 3 · Hoja «Evidencias» injertada (G4-T3)

**Estado:** DONE
**Commits:** `53b3ccb` (`feat(g4): hoja Evidencias para el formato de origen`)

## Qué se implementó

1. `src-tauri/src/excel/evidence_sheet.rs`:
   - `pub enum BackLink<'a>` con variantes `Matrix` y `Sheet { name: &'a str, rows: &'a [u32] }`.
   - Modificación de `evidence_sheet::write` para recibir `back: BackLink`.
     - `BackLink::Matrix`: texto «Volver a la matriz» y destino `internal:'Matriz'!A<fila>`.
     - `BackLink::Sheet { name, rows }`: texto «Volver al guion» y destino `internal:'<nombre_escapado>'!A<fila>` (con comillas simples duplicadas).
   - Test unitario: `back_link_points_to_the_guion_row`.

2. `src-tauri/src/excel/mod.rs`:
   - Actualización de la llamada existente en `build(...)` pasando `evidence_sheet::BackLink::Matrix`.

3. `src-tauri/src/excel/origin/evidence.rs`:
   - `pub fn sheet_name(workbook_xml: &str) -> String`: genera el nombre de hoja no colisionante según P8 (comprobación case-insensitive: `Evidencias` → `Evidencias (Qapture)` → `Evidencias (Qapture 2)`...).
   - `pub struct Donor`: bytes del archivo, conteo de imágenes faltantes y conteo de evidencias.
   - `pub fn donor(data: &ExportData, back: BackLink, image_max_width: u32) -> AppResult<Donor>`: arma en memoria un libro donante con una única hoja creada mediante `evidence_sheet::write`, con las imágenes preparadas.
   - Tests: `the_sheet_name_never_collides` y `donor_has_one_sheet_with_the_images`.

4. `src-tauri/src/excel/origin/evidence_graft.rs` (los 6 Minors del pico):
   - **Minor 1:** Parsing robusto con `parse_sheets`. Falla cerrado con `Err(validation(...))` ante cualquier etiqueta `<sheet>` sin `sheetId`, sin `name`, con comillas simples o malformada. Test: `sheets_without_id_or_name_fail_closed`.
   - **Minor 2:** `next_free` y su extracción numérica ahora comparan prefijos y sufijos ignorando mayúsculas/minúsculas de acuerdo con la especificación OPC. Test: `next_free_ignores_case`.
   - **Minor 3:** Validación del nombre de hoja contando unidades de código UTF-16 (`encode_utf16().count() <= 31`), rechazando nombres que comiencen o terminen con apóstrofo `'` y nombres con caracteres de control Unicode. Test: `names_count_utf16_and_reject_quotes_at_the_ends`.
   - **Minor 4:** `pub const STRICT: &str` y rechazo inmediato de libros con espacio de nombres Strict OOXML (`http://purl.oclc.org/ooxml/spreadsheetml/main`). Test: `strict_workbooks_are_rejected`.
   - **Minor 5:** Se añade `" dxfId=\""` a `DROPPED` para eliminar referencias de formato condicional del donante. Test: `donor_dxf_ids_are_dropped`.
   - **Minor 6:** Verificación de preservación íntegra de entradas previas en partes índice (`[Content_Types].xml`, `workbook.xml`, `workbook.xml.rels`), rechazo ante relaciones de dibujo no reconocidas y nombres de hoja duplicados sin distinguir mayúsculas. Test: `index_parts_only_gain_entries`.
   - Visibilidad: `tags`, `attr`, `unescape` expuestos como `pub(crate)` para reutilización en `origin::evidence`.

5. `src-tauri/src/excel/origin/mod.rs`:
   - Exporta `pub mod evidence;`.

## Archivos
- `src-tauri/src/excel/evidence_sheet.rs` (modificado).
- `src-tauri/src/excel/mod.rs` (modificado).
- `src-tauri/src/excel/origin/evidence_graft.rs` (modificado).
- `src-tauri/src/excel/origin/evidence.rs` (nuevo).
- `src-tauri/src/excel/origin/mod.rs` (modificado).

## Pruebas
Todos los comandos ejecutados desde `src-tauri/`.
- RED:
  - `cargo test excel::evidence_sheet` / `cargo test excel::origin`
  - Fallos iniciales por tipo incompatible y funciones aún no definidas en el nuevo módulo.
- GREEN:
  - `cargo test excel::evidence_sheet` → 6 passed; 0 failed.
  - `cargo test excel::origin` → 38 passed; 0 failed; 1 ignored.
  - `cargo test excel` → 102 passed; 0 failed; 2 ignored.
- Lint:
  - `cargo fmt --check` → limpio (código 0).
  - `cargo clippy --all-targets -- -D warnings` → limpio (código 0).

## Autorrevisión
- Cumple con especificaciones P7, P8, P10 y los 6 Minors documentados en el pico.
- Los tests existentes del exportador habitual continúan pasando sin ninguna regresión gracias a `BackLink::Matrix`.
- `Donor` e injerto de hojas con imágenes verificado con lectura mediante `calamine` (`Book::from_bytes`).

## Dudas o riesgos
- Ninguno. Las piezas para el ensamble de la exportación en la Task 4 están completas y probadas.

## Revisión (la llena el controlador)
- Spec: Pendiente · Calidad: Pendiente
- Hallazgos y cómo se resolvieron
