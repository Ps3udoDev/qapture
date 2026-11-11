# Tarea 1 · Escritura de celdas (G4-T1)

**Estado:** DONE
**Commits:** `c1875b0` (`feat(g4): escritura de celdas en la hoja del guion`)

## Qué se implementó
- `src-tauri/src/excel/origin/cells.rs`:
  - `pub enum CellValue`: variantes `Text(String)`, `Date(NaiveDate)`, `Empty`.
  - `pub struct CellWrite`: celda de destino (`cell: String`, p. ej. `"K16"`) y valor (`value: CellValue`).
  - `pub struct SheetEdit`: XML resultante (`xml: String`) y celdas cuyas fórmulas fueron retiradas (`formulas_removed: Vec<String>`), ordenadas por fila y columna.
  - `pub fn date1904(workbook_xml: &str) -> bool`: detección del sistema de fechas 1904 buscando `date1904="1"` o `date1904="true"` en `<workbookPr>`.
  - `pub fn formula_cells(sheet_xml: &str, cells: &[String]) -> AppResult<Vec<String>>`: identificación de celdas con fórmulas y expansión automática a todos los miembros de su grupo compartido (`t="shared"` con mismo `si`).
  - `pub fn write_cells(sheet_xml: &str, styles_xml: &str, date1904: bool, writes: &[CellWrite]) -> AppResult<SheetEdit>`:
    - Escritura inlineStr de texto preservando atributos y estilos existentes.
    - Escapado seguro de entidades XML (`&`, `<`, `>`) y eliminación de caracteres de control XML 1.0 inválidos (`\u{0}`–`\u{8}`, `\u{B}`, `\u{C}`, `\u{E}`–`\u{1F}`).
    - Inclusión de `xml:space="preserve"` cuando el texto tiene espacios en los extremos o saltos de línea.
    - Fechas (P1): serialización como número de serie (`date_serial` desde 1899-12-30 o 1904-01-01) si la celda tiene estilo de fecha (`numFmtId` 14–22, 45–47 o código propio con `d`/`m`/`y` fuera de comillas/corchetes); si no, texto `dd/mm/aaaa`.
    - Creación ordenada de celdas y filas faltantes (P2): herencia de estilo de fila (`customFormat="1"`) o de columna (`<col min="..." max="..." style="...">`), o sin estilo.
    - Retiro de fórmulas (P3): desasociación atómica de fórmulas compartidas conservando valores en caché `<v>`.
- `src-tauri/src/excel/origin/mod.rs`: exportación de `pub mod cells;`.

## Archivos
- `src-tauri/src/excel/origin/cells.rs` (nuevo).
- `src-tauri/src/excel/origin/mod.rs` (modificado).

## Pruebas
Todos los comandos desde `src-tauri/`.
- RED:
  - Inicialmente módulo no existente / 12 pruebas planificadas fallando.
  - Ajustes de índice de columna 1-based para `<col>` e índices de parsing en `<row>`.
- GREEN:
  - `cargo test excel::origin::cells` → 12 passed; 0 failed.
- Lint:
  - `cargo fmt --check` → limpio.
  - `cargo clippy --all-targets -- -D warnings` → limpio.

## Autorrevisión
- Cumple con especificación D9, P1, P2 y P3.
- No utiliza dependencias externas pesadas ni regex; toda la manipulación se realiza con slicing y parsing de texto seguro.
- Preserva intactos todos los bytes ajenos a las filas/celdas modificadas.

## Dudas o riesgos
- Ninguno detectado. La función `write_cells` queda lista para ser consumida por la orquestación en Tareas 2 y 4.

## Revisión (la llena el controlador)
- Spec: ✅ · Calidad: Aprobada
- Hallazgos y cómo se resolvieron: Se corrigió el cálculo de posición en el loop de filas y el índice 1-based en `<col min/max>`. 12 pruebas pasan y clippy está limpio.
