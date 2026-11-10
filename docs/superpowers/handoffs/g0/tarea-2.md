# Tarea 2 · Partes del xlsx

**Estado:** DONE_WITH_CONCERNS
**Commits:** ver `git log` de la rama (feat(g0): leer y reescribir un xlsx parte por parte sin alterarlo)

## Qué se implementó
`excel::origin::parts` con `NOT_XLSX`, `XlsxParts` (`read`, `names`, `get`, `text`, `set`, `write`). Conserva orden y método de compresión por parte; `set` reemplaza en sitio o agrega al final con `Deflated`. `read` falla con `validation(NOT_XLSX)` si no es ZIP, si una entrada no se puede leer o si faltan `xl/workbook.xml` o `[Content_Types].xml`. Los errores de escritura de `zip` y de UTF-8 terminan en `AppError::Export` con mensaje.

## Archivos
- `src-tauri/src/excel/origin/mod.rs` (solo `pub mod parts;`)
- `src-tauri/src/excel/origin/parts.rs`
- `src-tauri/src/excel/mod.rs` (`pub mod origin;` con `#[allow(dead_code)]` y comentario)

## Pruebas
- RED: `cargo test excel::origin::parts` -> error de compilación `E0425: cannot find value NOT_XLSX` y `E0433: cannot find type XlsxParts` (aún no existían), como se esperaba.
- GREEN: `cargo test excel::origin::parts` -> `test result: ok. 4 passed; 0 failed`.
- Lint: `cargo fmt --check` limpio; `cargo clippy --all-targets -- -D warnings` limpio.

## Autorrevisión
- Sin dependencias nuevas. Aserciones del plan intactas; solo `rejects_a_zip_without_workbook` añadida como pedía el plan.
- En zip 9 `ZipFile::name()` devuelve `Result`, se mapea a `NOT_XLSX`. `AppError` no tiene `From<ZipError>`, se usa `map_err` a `AppError::Export`.
- Se derivó `Debug` en `Part` y `XlsxParts` (necesario para `unwrap_err()` en las pruebas).

## Dudas o riesgos
- `clippy -D warnings` marcaba código muerto (módulo sin uso en producción); se añadió `#[allow(dead_code)]` en la declaración `pub mod origin;`. Retirarlo cuando Task 3 / G4 lo consuman.
- `get` y `set` buscan linealmente por nombre (suficiente para el tamaño de un xlsx).

## Revisión (la llena el controlador)

## Correcciones (ronda <R>)
