# Tarea 1 · Guion sintético de referencia (G-T1)

**Estado:** DONE
**Commits:** ver `git log` de la rama (`test(g0): guion sintético con la forma del guion de referencia`)

## Qué se implementó
- `guion_fixture::build() -> Vec<u8>`: libro en memoria con la hoja `Guión de Pruebas` (título combinado A1:M1 con logo PNG de 120x40 generado con `image`, carátula en B/D con D:M combinadas, encabezados en la fila 15, 36 casos, dos validaciones de lista en filas 16-35, autofiltro y área de impresión) y la hoja oculta `Hoja1` con la lista.
- Constantes `SHEET`, `HIDDEN_SHEET`, `HEADER_ROW`, `FIRST_CASE_ROW`, `CASES`, `HEADERS`, `COVER` tal como el brief (todas `#[cfg(test)]`, `pub(crate)`).
- `Book::from_bytes(&[u8])` en `testing.rs`: guarda los bytes en un `TempDir` propio.

## Archivos
- `src-tauri/src/excel/guion_fixture.rs` (nuevo)
- `src-tauri/src/excel/mod.rs` (declaración `#[cfg(test)] pub(crate) mod guion_fixture;`; rustfmt la ubicó en orden alfabético, antes de `images`)
- `src-tauri/src/excel/testing.rs` (`Book::from_bytes`)

## Pruebas
- RED: `cargo test excel::guion_fixture` falló al compilar con `E0425: cannot find function 'build'` / `cannot find value 'SHEET'` / `'HIDDEN_SHEET'` (y `Book::from_bytes` inexistente), como se esperaba.
- GREEN: `cargo test excel::guion_fixture` -> `3 passed; 0 failed`. Las aserciones del brief se usaron tal cual, sin ajustes.
- Lint: `cargo fmt --check` sin diferencias; `cargo clippy --all-targets -- -D warnings` sin avisos. (El aviso `linker_messages` de la DLL aparece al compilar tests; es previo y ajeno a esta tarea.)

## Autorrevisión
- Sin dependencias nuevas; `calamine` sigue solo en dev. Ningún archivo de cliente: todo se genera en memoria.
- La lista de criticidad lleva «Crìtico» (acento grave) a propósito, y «Crítico» normal en las celdas de caso, según el brief.
- Los `expect` solo existen en código `#[cfg(test)]`.

## Dudas o riesgos
- Ninguna.

## Revisión (la llena el controlador)
