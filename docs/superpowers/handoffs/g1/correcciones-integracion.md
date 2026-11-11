# Correcciones de integración · G1

**Estado:** DONE.
**Commits:** ver `git log` (`fix(g1): conservar editabilidad de listas y evitar colisiones de casos`).

## Qué se implementó

La revisión integrada detectó dos hallazgos Important al usar los datos importados con los editores existentes:

1. Una lista podía incorporar opciones de más de 100 caracteres o superar 50 opciones. La importación directa admitía esos valores, pero después `cases::update` o `fields::save` los rechazaban. Por decisión del controlador, esas listas se convierten en `longtext` al finalizar la normalización de todas las filas, con un aviso en español y las opciones vacías. También se revisan las listas predefinidas aunque el guion no les dé valores. El perfil original de `project_source.definition` no cambia.
2. Un guion puede importar `EJ-001`/`EJ-002` manteniendo `next_sequence = 1`, como pide el plan. Antes, crear o duplicar intentaba reutilizar esos códigos y fallaba por la restricción única. `repo::cases::insert_case` avanza la secuencia hasta encontrar un código libre dentro de la transacción existente. El primer `UPDATE` toma el bloqueo de escritura antes de consultar la disponibilidad. El fixture con códigos `CP01` continúa generando `EJ-001`.

## Archivos

- `src-tauri/src/transfer/guion/import.rs`: promoción final de listas y regresiones de importación/edición.
- `src-tauri/src/repo/cases.rs`: asignación de códigos libres para creación y duplicado.
- Referencias en los handoffs de las tareas 4 y 5.

## Pruebas

Todos los comandos desde `src-tauri/`.

- RED: `cargo test integration_` → 4 failed. Tres fallos `List != Longtext` y uno `UNIQUE constraint failed: test_case.project_id, test_case.code`, antes de escribir las correcciones.
- GREEN focalizado: `cargo test integration_` → 4 passed, 0 failed. Las pruebas importan el guion y pasan por `cases::update` y `fields::save`, comparando que los valores y el perfil de origen se conservan. Cubren valores de 200 caracteres, 51 opciones y opciones predefinidas inválidas sin valores. La regresión de códigos verifica creación y duplicado, secuencia inicial 1 y filas de origen solo en los 36 casos importados.
- Se añadió una quinta regresión de límites: 50 opciones de hasta 100 caracteres siguen siendo una lista editable, sin aviso de promoción.
- Suite completa: `cargo test` → 543 passed, 0 failed, 4 ignored en la biblioteca; 4 passed en `tests/bridge.rs`; binarios y doc-tests sin fallos. Incluye las cinco nuevas regresiones de integración.
- Lint: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` y `git diff --check` → exit 0.
- El aviso previo `linker_messages` de MSVC aparece al enlazar la DLL durante los tests; clippy termina limpio.

## Autorrevisión

- La promoción se realiza después de procesar todas las filas, para conservar el plegado y la normalización de opciones antes de convertir el campo a texto libre.
- Los avisos explican si se superaron 50 opciones o si una opción superó 100 caracteres. No se altera la definición del perfil ni las celdas del archivo fuente.
- La asignación de secuencia se comparte entre crear y duplicar; las importaciones conservan sus códigos originales y no cambian `next_sequence` al importarse.
- No se agregan dependencias ni archivos de cliente.

## Dudas o riesgos

- Los límites siguen siendo los de los editores existentes (100 caracteres por opción y 50 opciones). La promoción evita imponer al tester una limpieza manual de su guion.

## Revisión (la llena el controlador)
- Spec: ✅ · Calidad: Aprobada. Revisión independiente de `1609889`.
- Ambos Important resueltos: los casos y columnas importados siguen siendo editables al superar los límites de listas; crear y duplicar saltan códigos ocupados bajo la misma transacción.
- Las cinco regresiones comprueban conservación de valores, definición original, límites válidos y asignación de códigos. Sin hallazgos pendientes.
