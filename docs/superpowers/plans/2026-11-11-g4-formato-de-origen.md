# G · Bloque 3 — Formato de origen (G4) · Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Desde «Exportar › Formato de origen», el tester obtiene `<guion>_ejecutado.xlsx`: el mismo archivo del cliente con estado, fecha, tester y condiciones escritos en sus celdas, la carátula llena y una hoja «Evidencias» añadida. El archivo abre sin aviso de reparación.

**Architecture:** Todo vive en `src-tauri/src/excel/origin/`, sobre las piezas del bloque 0: `parts.rs` (leer y reescribir el ZIP parte por parte) y `evidence_graft.rs` (`graft_sheet`). `cells.rs` edita el XML de una hoja como texto: solo cambia las celdas destino (D9). `values.rs` decide **qué** se escribe en cada celda a partir del perfil guardado en `project_source.definition` (D6, D7, D8), con funciones puras. `evidence.rs` genera la hoja «Evidencias» con `excel::evidence_sheet` en un libro donante y la injerta (D10). `mod.rs` carga los datos, comprueba la copia del guion (`sha256`) y orquesta. Dos comandos: `export_origin_info`, para el diálogo, y `export_origin`, que abre «Guardar como» en Rust. En TS, el puerto `Exporter` gana dos métodos y `ExportDialog` una tercera opción, visible solo en proyectos importados.

**Tech Stack:** Rust 2024, `zip` 9, `rust_xlsxwriter`, `calamine` (lectura en tests), `sha2`, `chrono`, `rusqlite`; React 19, Zustand, `zod`, Vitest + Testing Library.

**Spec:** [`2026-11-10-guion-a-evidencias-design.md`](../specs/2026-11-10-guion-a-evidencias-design.md) (§1 criterios de éxito, §3 D6–D11, §4, §6 «Exportar › Formato de origen», §7, §8) · **Contrato:** [`G-guion-a-evidencias`](../contracts/G-guion-a-evidencias.md) (G4-T1 a G4-T6) · **Base:** G1 y G2 fusionados · **Pico:** [pico-trasplante](../handoffs/g0/pico-trasplante.md) y los 6 Minor de [g0/tarea-3](../handoffs/g0/tarea-3.md).

## Global Constraints

- Rama `feat/g4-formato-de-origen`, creada desde `main` (G2 fusionado, `e8341ec`). Un PR al terminar G4.
- Sin dependencias nuevas, ni en `Cargo.toml` ni en `package.json`. Sin migraciones: `project_source` y `case_source` ya existen (0007). Se quitan los `#[allow(dead_code)]` de `repo::guion::{ProjectSource, source, case_rows}`.
- **Nunca se escribe sobre la copia guardada** (`project_source.stored_path`) ni sobre el original del tester: se lee la copia y se guarda un archivo nuevo.
- Criterio de éxito 2 del spec: toda parte del ZIP que no sea la hoja del guion queda **byte a byte igual**, salvo `xl/workbook.xml`, `xl/_rels/workbook.xml.rels` y `[Content_Types].xml`, que solo **agregan** la hoja nueva. Única excepción permitida (P3): si se quitó alguna fórmula, también se borra `xl/calcChain.xml` con su relación y su `Override`.
- Mensajes fijos del contrato, idénticos:
  - `No se encontró el guion original de este proyecto. Vuelve a importarlo para exportar en su formato.`
  - `El guion original cambió desde que se importó.`
- Mensajes nuevos de este plan (también fijos):
  - `La hoja «<hoja>» ya no está en el guion original.`
  - `El libro usa el formato «Strict» de Excel. Ábrelo en Excel, guárdalo como «Libro de Excel (.xlsx)» y vuelve a importarlo.`
  - `<n> casos no tienen fila en el guion y no se escriben: <códigos separados por «, »>.` (con `n = 1`: `1 caso no tiene fila en el guion y no se escribe: <código>.`)
  - `Las celdas <lista> tenían fórmula: se escribe el valor y se quita la fórmula.`
  - `Revisa que las capturas no muestren datos personales o del cliente antes de entregar el archivo.`
  - `La observación supera 500 caracteres.`
  - `Guion ejecutado guardado: <archivo>` (aviso de éxito)
- Nombre sugerido (D11): `<file_name sin .xlsx>_ejecutado.xlsx`. «Guardar como» lo abre Rust. El diálogo nativo pide confirmación antes de sobrescribir, y `save::ensure_writable` avisa si Excel tiene abierto el destino.
- Sin herramientas MCP nuevas (spec A5).
- Verificación por task, desde `src-tauri/`: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test <filtro>`. Desde la raíz, en las tasks de TS: `npm run lint`, `npm run typecheck`, `npx vitest run <archivo>`.
- Commits `feat(g4): …`, `fix(g4): …`, `test(g4): …`, `docs(g4): …`, terminados en `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Handoffs en `docs/superpowers/handoffs/g4/`: antes de la Task 1, el controlador copia el `README.md` de `handoffs/g2/` (mismo formato y reglas) y vacía su registro. Cada implementador escribe `tarea-<N>.md`.

### Decisiones de este plan (no están en el spec)

| # | Decisión | Por qué |
|---|---|---|
| P1 | Una fecha se escribe como número de serie solo si el estilo de la celda ya es de fecha (`numFmtId` 14–22 o 45–47, o un formato propio con `d`, `m` o `y` fuera de comillas y corchetes). Si no, se escribe como texto `dd/mm/aaaa` | Un número de serie en una celda «General» se ve como `45972`. D9 pide conservar el estilo de la celda, y así se conserva |
| P2 | Una celda que no existe se crea con el estilo de su fila (`s` con `customFormat="1"`) o, si no, con el de su columna (`<col style>`). Si tampoco hay, va sin `s` | Así hereda bordes y fuente de la tabla del cliente sin tocar `styles.xml` |
| P3 | Quitar una fórmula compartida (`t="shared"`) quita la fórmula de **todo su grupo** (mismo `si`), y cada celda conserva su valor en caché. Si se quitó alguna fórmula, se borra `xl/calcChain.xml` | Si la celda principal pierde su fórmula, las dependientes quedan huérfanas y una `calcChain` con celdas sin fórmula hace que Excel ofrezca reparar el libro. Excel vuelve a crear la cadena al calcular |
| P4 | Motivo del bloqueo (D6): `Bloqueado: <Condiciones>`, o solo `Bloqueado` si Condiciones está vacío. No se duplica el prefijo si el texto ya empieza por «Bloqueado» (sin distinguir acentos ni mayúsculas) | No hay campo «motivo». El documento 05 del cliente pide el motivo en Condiciones |
| P5 | El tester de Ajustes (D7) solo se usa en casos **ejecutados**: Aprobado, Fallido o Bloqueado. En los demás se escribe el valor del campo, aunque esté vacío | Un caso «No ejecutado» con nombre de tester haría pensar que alguien lo ejecutó |
| P6 | El tester sale del diálogo, que se llena con el de Ajustes como hoy. La fecha de la carátula es la fecha máxima de D7 entre los casos escritos; si no hay ninguna, no se toca. Observaciones vacío deja la celda del cliente como está | Mismo comportamiento que el diálogo actual. No se borra lo que el cliente escribió si el tester no pone nada |
| P7 | La hoja «Evidencias» lleva una sección por caso **escrito con estado exportado** (Aprobado, Fallido, Bloqueado) o con alguna evidencia, en el orden de las filas. El vínculo de cada sección dice «Volver al guion» y apunta a `'<hoja>'!A<fila>` | El libro del cliente no tiene hoja «Matriz». El contrato pide una sección por caso ejecutado |
| P8 | Nombre de la hoja: `Evidencias`; si ya existe (sin distinguir mayúsculas), `Evidencias (Qapture)`; si también existe, `Evidencias (Qapture 2)`, `3`… | Spec §7, más el caso en que se exporta el mismo libro dos veces |
| P9 | Comando adicional `export_origin_info`: el diálogo lo usa para mostrar el archivo de origen y los conteos, y para deshabilitar la opción con su motivo. No se envían rutas al webview | Spec §6 pide mostrar el archivo, los conteos y el motivo antes de exportar. Calcularlo en TS exigiría exponer el perfil y las filas |
| P10 | Libro Strict OOXML (`http://purl.oclc.org/ooxml/...` en `workbook.xml`): se rechaza con su mensaje fijo, antes de escribir nada | Minor 4 del pico. Los prefijos y tipos de relación de `graft_sheet` son transicionales |

## Review Focus

- **Fórmula compartida en una columna destino** (por ejemplo, `I16` principal de un grupo `I16:I51`): el libro abre sin reparación; las dependientes no quedan huérfanas; desaparece `calcChain.xml`. Test `shared_formula_groups_lose_their_formulas_together` (Task 1) y `removing_formulas_drops_the_calc_chain` (Task 4).
- **El tester cambió códigos, agregó o borró casos después del import**: se escribe por `case_source.row`. Un caso nuevo se lista como «no se escribe». La fila de un caso borrado queda como estaba. Test `writes_by_source_row_not_by_code` en la Task 4.
- **Condiciones con `&`, `<`, saltos de línea, espacios en los extremos o caracteres de control**: llegan iguales a Excel, sin romper el XML (los caracteres de control no válidos en XML 1.0 se quitan). Test `text_is_escaped_and_keeps_spaces_and_newlines` en la Task 1.
- **Fechas en un libro con sistema 1904 o en una celda sin estilo de fecha**: la fecha que Excel muestra es la del caso. Tests `dates_use_the_1904_system_when_the_workbook_does` y `a_date_in_a_cell_without_date_style_is_written_as_text` en la Task 1.
- **Exportar dos veces el mismo resultado o un guion que ya trae «Evidencias»**: el nombre de la hoja nueva no choca con ninguna (sin distinguir mayúsculas) y tiene como mucho 31 caracteres UTF-16. Test `the_sheet_name_never_collides` en la Task 3.

---

### Task 1: Escritura de celdas (G4-T1)

**Files:**
- Create: `src-tauri/src/excel/origin/cells.rs`
- Modify: `src-tauri/src/excel/origin/mod.rs` (`pub mod cells;`)

**Interfaces:**
- Consumes: nada de otras tasks.
- Produces:
  ```rust
  pub enum CellValue { Text(String), Date(chrono::NaiveDate), Empty }
  pub struct CellWrite { pub cell: String /* "K16" */, pub value: CellValue }
  pub struct SheetEdit { pub xml: String, pub formulas_removed: Vec<String> /* "I16", ordenadas por fila y columna */ }
  pub fn write_cells(sheet_xml: &str, styles_xml: &str, date1904: bool, writes: &[CellWrite]) -> AppResult<SheetEdit>
  pub fn formula_cells(sheet_xml: &str, cells: &[String]) -> AppResult<Vec<String>> // incluye las del grupo compartido (P3)
  pub fn date1904(workbook_xml: &str) -> bool
  ```

- [ ] **Step 1: Write the failing tests** en `cells.rs` (`#[cfg(test)]`), con hojas XML mínimas escritas a mano:
  - `text_is_written_inline_and_keeps_the_style`: `<c r="K16" s="7" t="s"><v>3</v></c>` + `Text("Aceptado")` → `<c r="K16" s="7" t="inlineStr"><is><t>Aceptado</t></is></c>`; el resto del XML queda igual (comparar el prefijo y el sufijo).
  - `text_is_escaped_and_keeps_spaces_and_newlines`: `" A & B <c>\n\u{1}x "` → `<t xml:space="preserve"> A &amp; B &lt;c&gt;\nx </t>`.
  - `empty_clears_the_value_but_keeps_the_cell`: `<c r="I16" s="4"><v>45000</v></c>` → `<c r="I16" s="4"/>`.
  - `dates_use_the_existing_date_style`: estilo `s="2"` con `numFmtId="14"` en `cellXfs`; `Date(2026-11-11)` → `<c r="I16" s="2"><v>46337</v></c>`.
  - `dates_use_the_1904_system_when_the_workbook_does`: mismo caso con `date1904 = true` → `<v>44875</v>`.
  - `a_date_in_a_cell_without_date_style_is_written_as_text`: sin `s` → inline `11/11/2026` (P1); también con un `numFmt` propio `"0.00"`. Con un formato propio `dd/mm/yyyy;@` se escribe como número.
  - `missing_cells_and_rows_are_created_in_order`: fila 16 con `A16`, `M16`; escribir `K16` la mete entre ellas. Escribir en la fila 52, que no existe, crea `<row r="52">` después de la 51. Con `<sheetData/>` vacío también funciona.
  - `new_cells_take_the_row_or_column_style`: `<row r="16" s="9" customFormat="1">` → la celda nueva lleva `s="9"`; sin estilo de fila y con `<col min="11" max="11" style="5"/>` → `s="5"` (P2).
  - `a_formula_is_removed_and_reported`: `<c r="I16" s="2"><f>TODAY()</f><v>46000</v></c>` + `Date` → sin `<f>`; `formulas_removed == ["I16"]`.
  - `shared_formula_groups_lose_their_formulas_together`: principal `I16` con `<f t="shared" ref="I16:I18" si="0">A16+1</f>` y dependientes `I17`, `I18` con `<f t="shared" si="0"/>`; escribir solo `I16` → ninguna de las tres tiene `<f>`, `I17` e `I18` conservan su `<v>`; `formulas_removed == ["I16","I17","I18"]`; `formula_cells(xml, ["I16"])` devuelve lo mismo.
  - `malformed_sheets_are_rejected`: sin `<sheetData` → `Err` de validación.

- [ ] **Step 2: Run** `cargo test excel::origin::cells` → FAIL (el módulo no existe).

- [ ] **Step 3: Implement** las funciones de **Interfaces** en `cells.rs`. Búsqueda de texto, como en `evidence_graft.rs` (los atributos se leen con su helper `attr`; muévelo a `pub(super)` si hace falta). Columnas con `transfer::guion::read::column_index`. Número de serie: días desde `1899-12-30` (o `1904-01-01` si `date1904`). Las escrituras se aplican de mayor a menor posición en el texto para no recalcular desplazamientos. Los caracteres no permitidos en XML 1.0 (`\u{0}`–`\u{8}`, `\u{B}`, `\u{C}`, `\u{E}`–`\u{1F}`) se quitan. Se usa `xml:space="preserve"` solo si hay espacios en los extremos o saltos de línea.

- [ ] **Step 4: Run** `cargo test excel::origin::cells` → PASS; `cargo fmt --check`; `cargo clippy --all-targets -- -D warnings`.

- [ ] **Step 5: Commit** `feat(g4): escritura de celdas en la hoja del guion`

### Task 2: Valores por fila y carátula (G4-T2)

**Files:**
- Create: `src-tauri/src/excel/origin/values.rs`
- Modify: `src-tauri/src/excel/origin/mod.rs` (`pub mod values;`)

**Interfaces:**
- Consumes: `cells::{CellValue, CellWrite}` (Task 1); `transfer::guion::profile::{ProfileDefinition, Target, FieldRole, CONDITIONED, CONDITIONED_KEY}`; `domain::text::fold`; `excel::data::parse_iso`.
- Produces:
  ```rust
  pub struct OriginCase { pub row: u32, pub case: CaseDto, pub last_evidence: Option<NaiveDate> }
  pub struct Cover<'a> { pub tester: &'a str, pub observations: &'a str }
  pub const EXECUTED: [&str; 3] = ["Aprobado", "Fallido", "Bloqueado"];
  pub fn row_writes(def: &ProfileDefinition, case: &OriginCase, tester: &str) -> Vec<CellWrite>
  pub fn execution_date(def: &ProfileDefinition, case: &OriginCase) -> Option<NaiveDate>
  pub fn cover_writes(def: &ProfileDefinition, cases: &[OriginCase], cover: &Cover) -> Vec<CellWrite>
  ```

- [ ] **Step 1: Write the failing tests** con `profile::tests::reference()` (columna K = estado; I, L y M con roles `executionDate`, `tester` y `conditions`; cover `D5`, `D6`, `D8`):
  - `status_uses_the_profile_vocabulary`: Aprobado → `K16 = Text("Aceptado")`; Aprobado con `condicionado = "true"` → `"Condicionado"`; Fallido → `"Rechazado"`; No ejecutado, En ejecución y Por documentar → `Empty` (D6).
  - `blocked_cases_write_the_reason_in_conditions`: Bloqueado con Condiciones `"Sin ambiente"` → `K = Empty`, `M = "Bloqueado: Sin ambiente"`; con Condiciones vacío → `"Bloqueado"`; con `"bloqueado: ya escrito"` queda igual (P4).
  - `date_falls_back_to_the_last_evidence`: campo `fecha_prueba = "2026-11-05"` → `I = Date(2026-11-05)`; vacío con `last_evidence = 2026-11-09` → esa fecha; sin ninguna → `Empty` (D7).
  - `tester_falls_back_only_for_executed_cases`: campo vacío, Aprobado, tester `"Ana"` → `L = "Ana"`; No ejecutado → `Empty`; campo `"Luis"` → `"Luis"` siempre (P5).
  - `columns_without_a_role_are_not_written`: las columnas C, F, H y E del perfil no aparecen en las escrituras.
  - `cover_writes_last_date_tester_and_observations`: dos casos con fechas 05 y 09 → `D5 = Date(09)`, `D6 = "Ana"`, `D8 = "Texto"`; con observaciones `""` no hay escritura en `D8`; sin fechas no hay escritura en `D5`; un perfil con `cover` vacío no produce nada (P6).

- [ ] **Step 2: Run** `cargo test excel::origin::values` → FAIL.

- [ ] **Step 3: Implement** las funciones de **Interfaces**. La celda es `<column><row>`. El estado se busca en `status_map.export` con la clave `CONDITIONED` cuando el caso es Aprobado y tiene la casilla en `"true"`; si no hay clave, `Empty`. Si el perfil no tiene columna de estado o de un rol, esa escritura no existe.

- [ ] **Step 4: Run** `cargo test excel::origin::values` → PASS; fmt; clippy.

- [ ] **Step 5: Commit** `feat(g4): valores del guion por fila y carátula`

### Task 3: Hoja «Evidencias» injertada (G4-T3)

**Files:**
- Create: `src-tauri/src/excel/origin/evidence.rs`
- Modify: `src-tauri/src/excel/origin/evidence_graft.rs` (los 6 Minor del pico), `src-tauri/src/excel/evidence_sheet.rs` (parámetro del vínculo de regreso), `src-tauri/src/excel/mod.rs` (llamada actual con `BackLink::Matrix`), `src-tauri/src/excel/origin/mod.rs`

**Interfaces:**
- Consumes: `evidence_sheet::write`, `images::prepare_all`, `styles::{workbook, Styles}`, `data::ExportData`.
- Produces:
  ```rust
  // excel/evidence_sheet.rs
  pub enum BackLink<'a> { Matrix, Sheet { name: &'a str, rows: &'a [u32] /* fila 1-based por caso de data.cases */ } }
  pub fn write(ws, data, images, styles, back: BackLink) -> AppResult<Vec<u32>>
  // excel/origin/evidence.rs
  pub fn sheet_name(workbook_xml: &str) -> String                 // P8
  pub struct Donor { pub bytes: Vec<u8>, pub missing_images: usize, pub evidence: usize }
  pub fn donor(data: &ExportData, back: BackLink, image_max_width: u32) -> AppResult<Donor>
  // evidence_graft.rs: graft_sheet sin cambios de firma; nuevo mensaje
  pub const STRICT: &str = "El libro usa el formato «Strict» de Excel. Ábrelo en Excel, guárdalo como «Libro de Excel (.xlsx)» y vuelve a importarlo.";
  ```

- [ ] **Step 1: Write the failing tests**
  - En `evidence_sheet.rs`: `back_link_points_to_the_guion_row`: con `BackLink::Sheet { name: "Guión de Pruebas", rows: &[16] }` la hoja trae un vínculo interno `'Guión de Pruebas'!A16` con texto «Volver al guion». Los tests actuales de la matriz siguen pasando con `BackLink::Matrix`.
  - En `evidence.rs`:
    - `the_sheet_name_never_collides`: hojas `["Guión de Pruebas"]` → `Evidencias`; `["evidencias"]` → `Evidencias (Qapture)`; con las dos → `Evidencias (Qapture 2)`.
    - `donor_has_one_sheet_with_the_images`: `excel::testing::fixture()` con 2 PNG (`testing::write_png`) → `Donor { evidence: 2, missing_images: 0 }` y `graft_sheet(&guion_fixture::build(), &donor.bytes, "Evidencias")` es `Ok`; `calamine` lee las dos hojas.
  - En `evidence_graft.rs` (Minor del pico):
    - `sheets_without_id_or_name_fail_closed` (1): un `<sheet>` sin `sheetId` o con `name='x'` → `Err`, no se adivina.
    - `next_free_ignores_case` (2): `["xl/media/Image1"]` → `2`.
    - `names_count_utf16_and_reject_quotes_at_the_ends` (3): 31 emojis → `Err`; `'Ev'` y nombres con control → `Err`; 31 caracteres BMP → `Ok`.
    - `strict_workbooks_are_rejected` (4): `workbook.xml` con `xmlns="http://purl.oclc.org/ooxml/spreadsheetml/main"` → `Err(STRICT)`.
    - `donor_dxf_ids_are_dropped` (5): un `dxfId="0"` en el XML del donante no llega a la hoja injertada.
    - `index_parts_only_gain_entries` (6): cada `<sheet …/>`, `<Relationship …/>` y `<Override …/>`/`<Default …/>` del original sigue igual en el resultado; también con un donante cuya relación de dibujo no apunta a una imagen y con un nombre duplicado que solo cambia en mayúsculas.

- [ ] **Step 2: Run** `cargo test excel::origin` y `cargo test excel::evidence_sheet` → FAIL.

- [ ] **Step 3: Implement**
  - `evidence_sheet::write`: el vínculo depende de `back`. Con `Sheet`, el texto es «Volver al guion» y apunta a `internal:'<name>'!A<rows[i]>`; las comillas simples del nombre se duplican.
  - `evidence::donor`: libro con `styles::workbook()`, una hoja, `images::prepare_all` y `evidence_sheet::write`; `save_to_buffer`.
  - `evidence_graft`: los 6 Minor, cada uno con su test.

- [ ] **Step 4: Run** `cargo test excel` → PASS (incluidos los tests del export actual); fmt; clippy.

- [ ] **Step 5: Commit** `feat(g4): hoja Evidencias para el formato de origen`

### Task 4: Exportación completa y prueba de ida y vuelta (G4-T4)

**Files:**
- Create: `src-tauri/src/excel/origin/export.rs`
- Modify: `src-tauri/src/excel/origin/mod.rs`, `src-tauri/src/repo/guion.rs` (quitar `allow(dead_code)`), `src-tauri/src/excel/save.rs` (`save_bytes_atomic`), `src-tauri/src/transfer/guion/import.rs` (hacer `pub(crate)` el helper de tests `request` dentro de `pub(crate) mod tests`)

**Interfaces:**
- Consumes: Tasks 1–3; `repo::guion::{source, case_rows, ProjectSource}`; `ExportData::load`; `media::fixtures::sha256_hex`; `EvidenceFiles::absolute`; `parts::XlsxParts`.
- Produces:
  ```rust
  pub const NO_COPY: &str = "No se encontró el guion original de este proyecto. Vuelve a importarlo para exportar en su formato.";
  pub const CHANGED: &str = "El guion original cambió desde que se importó.";
  pub struct OriginData { pub source: ProjectSource, pub cases: Vec<OriginCase>, pub unmapped: Vec<String> /* códigos sin fila */, pub export: ExportData /* solo casos de la hoja Evidencias, P7 */ }
  impl OriginData { pub fn load(conn: &Connection, files: &EvidenceFiles, project_id: &str) -> AppResult<Option<Self>> } // None: el proyecto no se importó
  pub enum Availability { Ready(Vec<u8>), Unavailable(&'static str) }
  pub fn check_copy(files: &EvidenceFiles, source: &ProjectSource) -> Availability // NO_COPY o CHANGED
  pub struct OriginRequest<'a> { pub tester: &'a str, pub observations: &'a str, pub include_evidence: bool, pub image_max_width: u32 }
  pub struct Built { pub bytes: Vec<u8>, pub cases_written: usize, pub evidence: usize, pub missing_images: usize, pub formulas_removed: Vec<String> }
  pub fn build(data: &OriginData, base: &[u8], req: &OriginRequest) -> AppResult<Built>
  pub fn formulas_at_risk(data: &OriginData, base: &[u8], tester: &str) -> AppResult<Vec<String>> // para el diálogo
  // excel/save.rs
  pub fn save_bytes_atomic(bytes: &[u8], dest: &Path) -> AppResult<()> // mismo temporal + rename + errores que save_atomic
  ```

- [ ] **Step 1: Write the failing tests** en `export.rs`, con `test_support::conn()`, `TempDir`, `guion_fixture::build()` importado por `transfer::guion::import::import(…, tests::request(…))` y `profile::tests::reference()`. Ponles estado a CP01 (Aprobado + 1 evidencia), CP02 (Fallido), CP03 (Aprobado + `condicionado`) y CP04 (Bloqueado):
  - `round_trip_changes_only_target_cells_and_adds_the_sheet` (criterio de éxito 2): todas las partes del original, salvo la hoja del guion y las tres partes índice, son iguales byte a byte en el resultado. Las partes índice solo agregan entradas (mismo helper que la Task 3). Con `calamine`, toda celda de la hoja del guion fuera de I, K, L y M de las filas 16–51 y de `D5`, `D6`, `D8` vale lo mismo que en el original. `K16..K19 = Aceptado, Rechazado, Condicionado, ""`, `M19` empieza por `Bloqueado`. Existe la hoja «Evidencias» con la sección de CP01.
  - `without_evidence_sheet_only_the_guion_sheet_changes`: `include_evidence = false` → solo cambia la parte de la hoja del guion.
  - `writes_by_source_row_not_by_code`: renombrar CP05 a `X-1` y borrar CP06; crear un caso nuevo `CP99`. `X-1` se escribe en la fila 20; la fila 21 queda como en el original; `unmapped == ["CP99"]`.
  - `removing_formulas_drops_the_calc_chain`: base editada con `guion_fixture::build_edited` para que `I16` tenga fórmula (agrega `xl/calcChain.xml` a mano con `XlsxParts::set` si `rust_xlsxwriter` no la crea). El resultado no tiene `calcChain.xml`, ni su relación ni su `Override`, y `formulas_removed == ["I16"]`.
  - `copy_missing_or_changed_is_unavailable`: borrar el archivo → `Unavailable(NO_COPY)`; cambiar un byte → `Unavailable(CHANGED)`.
  - `a_renamed_guion_sheet_is_rejected`: base cuya hoja ya no se llama como `source.sheet` → `Err` con `La hoja «Guión de Pruebas» ya no está en el guion original.`
  - En `save.rs`: `save_bytes_atomic_leaves_no_temporary_files`.

- [ ] **Step 2: Run** `cargo test excel::origin::export` → FAIL.

- [ ] **Step 3: Implement**
  - La hoja del guion se busca por nombre en `workbook.xml`, y su parte por `xl/_rels/workbook.xml.rels`.
  - `date1904`, `styles.xml` y `write_cells` sobre esa parte; luego `XlsxParts::set`.
  - Si hay fórmulas quitadas y existe `xl/calcChain.xml`: quitar la parte, su `<Relationship>` y su `<Override>` (P3).
  - Si `include_evidence`, `graft_sheet` con `evidence::donor` y `sheet_name` (P7, P8).
  - `OriginData::load` lee `last_evidence` como la fecha local de `MAX(captured_at)` por caso, igual que lo muestra la interfaz.

- [ ] **Step 4: Run** `cargo test excel::origin` y `cargo test excel::save` → PASS; fmt; clippy.

- [ ] **Step 5: Commit** `feat(g4): exportación al formato de origen con prueba de ida y vuelta`

### Task 5: Comandos y contrato (G4-T5)

**Files:**
- Create: `src-tauri/src/commands/origin.rs`
- Modify: `src-tauri/src/commands/mod.rs`, `src-tauri/src/commands/names.rs`, `src-tauri/src/lib.rs` (`generate_handler!`), `src-tauri/capabilities/main.json`, `src-tauri/src/dto.rs` (DTO + `sample()`), `src/infrastructure/tauri/dto.ts`, `src/infrastructure/tauri/__fixtures__/contracts.json` (regenerado), `src/infrastructure/tauri/contract.test.ts`

**Interfaces:**
- Consumes: Task 4; `commands::export::EXPORT_PROGRESS_EVENT` (mismas etapas `preparing → images → writing → saving`); `state.settings().profile.tester`.
- Produces (Rust `camelCase` ↔ TS):
  ```rust
  pub struct OriginInfoDto { pub file_name: String, pub sheet: String, pub cases_written: i64, pub unmapped: Vec<String>, pub evidence: i64, pub formulas: Vec<String>, pub unavailable: Option<String>, pub suggested_name: String }
  pub struct OriginExportInput { pub tester: String, pub observations: String, pub include_evidence: bool, pub image_max_width: u32 } // deny_unknown_fields
  // reutiliza ExportResultDto { path, file_name, cases, evidence, missing_images }
  #[tauri::command(async)] pub fn export_origin_info(state, project_id: String) -> AppResult<Option<OriginInfoDto>> // None: no importado
  #[tauri::command(async)] pub fn export_origin(app, window, state, project_id: String, input: OriginExportInput) -> AppResult<Option<ExportResultDto>> // None: canceló «Guardar como»
  ```

- [ ] **Step 1: Write the failing tests**
  - `dto.rs`: agrega `OriginInfoDto` y `OriginExportInput` a `sample()`/`inputs_json()`; `origin_input_rejects_unknown_fields`. Los tests de `names.rs` (comando registrado en las tres listas) fallan hasta registrarlos.
  - `commands/origin.rs`, con una función interna `info(conn, files, project_id, tester)` probada sin Tauri: `info_is_none_for_projects_not_imported`; `info_reports_unavailable_copy` (`unavailable == Some(CHANGED)`, conteos en 0); `info_counts_cases_evidence_formulas_and_unmapped`; `suggested_name_adds_ejecutado` (`guion_cliente.xlsx` → `guion_cliente_ejecutado.xlsx`).
  - Validación de `OriginExportInput`: tester con `required_text("Tester", …, 100)`, observaciones de hasta 500 caracteres (`La observación supera 500 caracteres.`) y ancho dentro de `IMAGE_WIDTH_RANGE`, con los mismos mensajes que `resolve_options`.

- [ ] **Step 2: Run** `cargo test commands::origin` y `cargo test dto` → FAIL.

- [ ] **Step 3: Implement**
  - `export_origin` lee los datos y la copia, y suelta la base antes de abrir el diálogo, como `commands::export::run`. Si `check_copy` no da `Ready`, devuelve `validation(<motivo>)`.
  - Abre «Guardar como» con `suggested_name` y el filtro `xlsx`, y aplica `with_xlsx_extension`.
  - Llama a `save::ensure_writable` antes de `build` y a `save_bytes_atomic` al final; luego `state.exports().remember(&dest)`, para que funcionen «Abrir archivo» y «Mostrar en carpeta».
  - Regenera el contrato con `UPDATE_CONTRACTS=1 cargo test contract` y agrega los esquemas `zod` en `dto.ts`.

- [ ] **Step 4: Run** `cargo test` (todo), `npx vitest run src/infrastructure/tauri/contract.test.ts` → PASS; fmt; clippy; `git diff --exit-code -- src/infrastructure/tauri/__fixtures__/contracts.json` limpio después de commitear.

- [ ] **Step 5: Commit** `feat(g4): comandos export_origin y export_origin_info con su contrato`

### Task 6: Opción «Formato de origen» en el diálogo de exportar (G4-T6)

**Files:**
- Modify: `src/domain/export.ts`, `src/domain/export.test.ts`, `src/ports/export.ts`, `src/infrastructure/tauri/export.ts`, `src/infrastructure/memory/export.ts`, `src/infrastructure/memory/export.test.ts`, `src/application/use-cases.ts`, `src/ui/store/export-slice.ts`, `src/ui/store/export-slice.test.ts`, `src/ui/screens/ExportDialog.tsx`, `src/ui/screens/Export.test.tsx`

**Interfaces:**
- Consumes: comandos y DTO de la Task 5.
- Produces:
  ```ts
  export type ExportKind = "project" | "template" | "origin";
  export interface OriginInfo { fileName: string; sheet: string; casesWritten: number; unmapped: string[]; evidence: number; formulas: string[]; unavailable: string | null; suggestedName: string }
  export interface OriginExportOptions { tester: string; observations: string; includeEvidence: boolean; imageMaxWidth: number }
  export function unmappedMessage(codes: string[]): string | null   // mensaje fijo, singular/plural
  export function formulasMessage(cells: string[]): string | null
  // Exporter
  originInfo(projectId: string): Promise<OriginInfo | null>;
  exportOrigin(projectId: string, options: OriginExportOptions): Promise<ExportResult | null>;
  // ExportSlice
  originInfo: OriginInfo | null; loadOriginInfo(): Promise<void>; exportOrigin(options: OriginExportOptions): Promise<FormResult>;
  ```

- [ ] **Step 1: Write the failing tests**
  - `domain/export.test.ts`: `unmappedMessage(["CP99"])` y `(["CP99","X"])` dan los textos fijos de singular y plural, y `[]` → `null`; `formulasMessage(["I16","I17"])` → `Las celdas I16, I17 tenían fórmula: se escribe el valor y se quita la fórmula.`
  - `Export.test.tsx` (con el adaptador en memoria):
    - `origin_option_is_hidden_for_projects_not_imported`.
    - `origin_option_shows_file_counts_and_warnings`: muestra el nombre del archivo, «36 casos se escriben», el número de evidencias, el aviso de casos sin fila, el de fórmulas y el aviso fijo de datos personales; trae la casilla «Agregar hoja Evidencias» marcada y el campo «Observaciones de la carátula».
    - `origin_option_is_disabled_with_the_reason`: con `unavailable` = mensaje fijo, el radio está deshabilitado y el motivo se ve.
    - `exporting_origin_sends_the_options_and_shows_the_toast`: «Exportar guion» llama a `exportOrigin` con `{tester, observations, includeEvidence, imageMaxWidth}` y muestra `Guion ejecutado guardado: guion_cliente_ejecutado.xlsx`, con «Abrir archivo» y «Mostrar en carpeta».
  - `export-slice.test.ts`: `loadOriginInfo` se vuelve a pedir al abrir el diálogo; un error de validación de Rust aparece en `errors._form`.

- [ ] **Step 2: Run** `npx vitest run src/domain/export.test.ts src/ui/screens/Export.test.tsx src/ui/store/export-slice.test.ts` → FAIL.

- [ ] **Step 3: Implement**
  - Adaptadores Tauri (`export_origin_info`, `export_origin`) y en memoria. El de memoria guarda un `OriginInfo` configurable para los tests.
  - Caso de uso `exportOrigin` en `use-cases.ts`.
  - Slice y diálogo: `KINDS` gana `{ kind: "origin", label: "Formato de origen", hint: "El guion del cliente con resultados y evidencias." }`, solo si `originInfo !== null`. Con `origin` se ocultan versión y Resumen. El texto del botón es «Exportar guion».

- [ ] **Step 4: Run** `npm run lint`, `npm run typecheck`, `npx vitest run` → PASS.

- [ ] **Step 5: Commit** `feat(g4): opción Formato de origen en el diálogo de exportar`

### Task 7: Cierre del bloque

**Files:**
- Modify: `src-tauri/src/excel/origin/export.rs` (test ignorado), `docs/superpowers/contracts/G-guion-a-evidencias.md`, `docs/superpowers/handoffs/g4/README.md`

- [ ] **Step 1: Add** el test `#[ignore]` `write_origin_for_manual_check`, que escribe `guion_ejecutado.xlsx` en `target/manual-check/` con el escenario de la Task 4: 3 casos (Aprobado con 2 capturas, Fallido, Aprobado condicionado), uno Bloqueado y la carátula.
- [ ] **Step 2: Run** `cargo test write_origin_for_manual_check -- --ignored` y anota en `handoffs/g4/README.md` la tabla de verificación manual, con las comprobaciones de `pico-trasplante.md` más I, K, L y M y la carátula. La persona la llena al abrir el archivo en Excel y LibreOffice.
- [ ] **Step 3: Update** el contrato: G4-T1 a G4-T6 en ✅ con fecha, el plan enlazado en «Planes», la sección «Interfaces producidas · G4» (comandos, DTO y puerto) y el criterio de aceptación G4 marcado solo después de la verificación manual. El estado pasa a «Sigue: Bloque 4 (cierre)».
- [ ] **Step 4: Run** la verificación completa: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `npm run lint`, `npm run typecheck`, `npx vitest run`.
- [ ] **Step 5: Commit** `docs(g4): cierre del bloque y verificación manual`
