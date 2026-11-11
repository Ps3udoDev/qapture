# Tarea 1 · Lector de guion (G1-T1)

**Estado:** DONE_WITH_CONCERNS
**Commits:** ver `git log` (`feat(g1): leer el guion del cliente con calamine`).

## Qué se implementó
- `calamine = "0.36.1"` pasa a producción sin features; se mantiene el parche `zip8`.
- `transfer/guion/read.rs`: libro con hojas visibles en orden, celdas tipadas, combinaciones, filas con índices absolutos, sugerencia de encabezados, letras/índices de columnas y normalización de fechas.
- Los contenedores CFB cifrados se distinguen de `.xls` mediante `EncryptedPackage` en UTF-16LE; errores de lectura usan los mensajes fijos del plan.
- `guion_fixture::build_edited` admite texto y fechas de Excel conservando el fixture original con `build_edited(&[])`.

## Archivos
- `src-tauri/Cargo.toml`.
- `src-tauri/src/transfer/mod.rs`.
- `src-tauri/src/transfer/guion/mod.rs` y `read.rs`.
- `src-tauri/src/excel/guion_fixture.rs`.

## Pruebas
Todos los comandos desde `src-tauri/`.
- RED: `cargo test transfer::guion::read` → 21 errores de compilación (`read`, `Cell`, `parse_date`, `column_letter`, `column_index` y constantes inexistentes). Se retomaron las pruebas del agente interrumpido por el reinicio; el fallo se reprodujo antes de implementar el lector.
- GREEN: `cargo test transfer::guion::read` → 6 passed, 0 failed. Las cinco pruebas del plan mantienen sus aserciones; se añade la comprobación de NaN/infinito, Unicode en fechas ISO, texto vacío y números enteros sin `.0`.
- Regresión: `cargo test excel::` → 72 passed, 0 failed, 2 ignored (generación manual y rendimiento release, ya ignoradas en la base). Incluye las tres pruebas del fixture.
- Lint: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` y `git diff --check` → exit 0. La primera ejecución GREEN aún mostraba los avisos deprecated; la ejecución posterior de clippy y de regresión ya incluye el allow localizado y no los muestra.

## Autorrevisión
- Lectura con calamine local instalado; sin dependencia nueva ni archivo de cliente.
- `DateTimeIso` usa `get(..10)` para evitar cortes inválidos UTF-8. Las fechas numéricas rechazan NaN/infinito y comprueban el rango antes de convertir.
- `column_index` admite solo mayúsculas A–XFD y comprueba overflow.
- Se corrigió el comentario del módulo de transferencia que tenía texto mal codificado.

## Dudas o riesgos
- El plan exige `load_merged_regions` y `merged_regions_by_sheet`; calamine 0.36.1 las marca deprecated. Se conserva la interfaz del plan con `#[allow(deprecated)]` localizado en `read` y comentario.
- `#[allow(dead_code)]` temporal en `pub mod guion` hasta conectar los comandos de G1-T6. Retirarlo al integrar esa tarea.
- El enlazador MSVC puede emitir el aviso previo `linker_messages` de la DLL; no corresponde al lector.

## Revisión (la llena el controlador)
- Spec: ✅ · Calidad: Aprobada. Revisión independiente del diff `7b9eab6..806a186`.
- Sin Critical ni Important. Las cinco pruebas del plan mantienen sus aserciones; se añadió cobertura Unicode y NaN.
- Minor no bloqueante: ampliar pruebas de rangos desplazados y de encabezados ausentes o fuera de las primeras 200 filas. La implementación inspeccionada cumple esos casos.
- Integración pendiente en T6: retirar el `allow(dead_code)` temporal cuando se conecten los comandos.
