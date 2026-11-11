# Tarea 4 · Vista previa del guion (G1-T4)

**Estado:** DONE_WITH_CONCERNS
**Commits:** ver `git log` (`feat(g1): vista previa del guion con avisos y errores`).

## Qué se implementó
- `import::plan`, `Plan` y `PlannedCase`: vista previa pura, sin escritura en base ni disco. Valida perfil/hoja, detecta filas sin código o repetidas, aplica límites, normaliza estados, listas y fechas y conserva el texto original del estado desconocido en Condiciones.
- Los títulos vacíos usan el código; los largos se recortan por caracteres Unicode. Los campos de texto pasan a texto largo cuando lo necesitan.
- Las filas vacías entre casos generan avisos en singular o plural; las finales y las anteriores al primer caso no cuentan como casos ni avisos.
- `read::detect_cover`: etiquetas normalizadas, primera celda con valor a la derecha, combinación a la derecha o celda contigua; devuelve el inicio real de una combinación, incluso vertical.
- DTO de vista previa y fixture editable con `Put::Blank` que sobrescribe una celda existente con `write_blank`.

## Archivos
- `src-tauri/src/transfer/guion/import.rs` (nuevo).
- `src-tauri/src/transfer/guion/mod.rs`.
- `src-tauri/src/transfer/guion/read.rs`.
- `src-tauri/src/dto.rs`.
- `src-tauri/src/excel/guion_fixture.rs`.

## Pruebas
Todos los comandos desde `src-tauri/`, salvo `git diff --check` desde la raíz.
- RED: `cargo test transfer::guion::import` → 14 errores de compilación por `plan`, `detect_cover` y `Put::Blank` inexistentes. Se escribieron primero las siete pruebas exigidas y dos de límites adicionales.
- Primer pase: ocho pruebas correctas y una fallida (`empty_rows_are_skipped_and_rows_without_code_block`, 36 casos frente a 35). Detectó que `write_string("")` no reemplaza la celda previa; se corrigió el fixture a `write_blank` sin debilitar aserciones.
- GREEN previo a revisión: `cargo test transfer::guion` → 24 passed, 0 failed.
- GREEN final: `cargo test transfer::guion` → 25 passed, 0 failed (13 de vista previa, seis de perfil y seis de lector).
- Lint: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` y `git diff --check` → exit 0. El mensaje `linker_messages` de MSVC aparece al enlazar tests, como antes de esta tarea; clippy termina sin avisos.

## Autorrevisión
- Las siete pruebas del plan están completas. Se añade cobertura de títulos Unicode, promoción de texto largo, límites de valores/casos, fila de encabezados extrema, orden de códigos repetidos, plural de filas vacías, carátula y crecimiento de Condiciones por el estado original.
- El límite de 16000 se comprueba tanto en el valor de origen como después de anexar el estado original. Los saltos de línea se conservan.
- No hay datos de cliente ni dependencias nuevas. El perfil original se consulta sin modificarlo; las opciones añadidas pertenecen a la vista previa.
- `header_row = u32::MAX` produce guion sin casos, sin sumar uno ni provocar overflow.

## Dudas o riesgos
- Se conserva el `allow(dead_code)` transitorio del módulo hasta conectar G1-T6.
- Esta tarea solo construye la vista previa. La importación transaccional y la copia del archivo corresponden a G1-T5.

## Revisión (la llena el controlador)

## Correcciones (ronda 1)
- Hallazgo Important: el perfil admite el rol Conditions sobre lista o fecha. Anexar `Estado original: …` produciría un valor incompatible con ese tipo de campo.
- Decisión del controlador: cuando se anexa ese texto, el campo Condiciones de la vista previa pasa a `longtext` y sus opciones se vacían. La promoción se hace al terminar todas las filas para conservar la normalización original de las fechas y listas siguientes. El perfil guardado conserva su definición.
- Prueba `conditions_promote_lists_and_dates_after_normalizing_all_rows`: cubre lista y fecha, contenido original más línea de auditoría, opciones vacías, segunda fila normalizada y perfil original intacto.
- RED de regresión: `cargo test conditions_promote_lists_and_dates_after_normalizing_all_rows`, desactivando temporalmente la promoción, falló `left: List / right: Longtext`. La mutación se revirtió antes del GREEN final.
