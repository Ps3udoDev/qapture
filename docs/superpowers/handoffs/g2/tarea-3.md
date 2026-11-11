# Tarea 3 · Repositorio de archivos de prueba (G2-T1, parte 2)

**Estado:** DONE
**Commits:** `89287b7` (`feat(g2): repositorio de archivos de prueba`).

## Qué se implementó
- `dto::FixtureDto` en `src-tauri/src/dto.rs`:
  - Estructura serializable y deserializable con `camelCase`: `id`, `case_id`, `file_name`, `size: i64`, `sha256`, `note`, `position: i64`, `added_at` (sin `stored_path`).
- Repositorio `repo::fixtures` en `src-tauri/src/repo/fixtures.rs`:
  - `add`: validaciones en orden estricto (`clean_name`, `check_size`, longitud de nota ≤ 500 caracteres, existencia del caso de prueba con error `NotFound`, máximo de 50 archivos por caso, unicidad de nombre de archivo sin distinguir mayúsculas `NOCASE`). Escribe en disco con `media::fixtures::write`, inserta registro en `case_fixture` con `position = MAX(position) + 1` y actualiza `project.updated_at` con `projects::touch` dentro de una transacción; si la transacción falla, limpia el archivo escrito en disco y remueve la carpeta `fixtures/<id>` si queda vacía.
  - `list`: devuelve fixtures del caso ordenados por `position ASC`.
  - `remove`: busca el archivo por ID (devolviendo `NotFound` si no existe), elimina la fila en transacción, compacta posiciones del caso a `0..n` en orden previo, toca el proyecto y posteriormente borra el archivo físico y su carpeta `fixtures/<id>/` si quedó vacía.
  - `absolute`: obtiene `stored_path` de `case_fixture` y devuelve su ruta absoluta con `files.absolute`, validando que el archivo exista en disco (`El archivo de prueba ya no está en disco.`).
  - `missing_files`: cuenta archivos faltantes en disco de las tablas `case_fixture` y `project_source` (tolerando si alguna tabla aún no existe).
  - `stored_by_case`: agrupa fixtures por `case_id` ordenados por `position ASC` con su tupla `(FixtureDto, stored_path)` para ser consumidos en la exportación de paquetes (Task 4).
- Integración en `repo::evidence`:
  - `references`: incluye todos los `stored_path` de `case_fixture` en el conjunto `referenced` cuando la tabla existe.
  - `case_files`: incluye los `stored_path` de los fixtures pertenecientes al caso a borrar.
  - `delete_case`: además de borrar las capturas y el caso, elimina los archivos físicos de los fixtures y limpia las carpetas `fixtures/<id>`.
  - Prueba `sweep_keeps_fixture_files`: verifica que `sweep_orphans` mantenga archivos de fixtures referenciados y limpie archivos huérfanos.
- Integración en `backup`:
  - Prueba `restore_keeps_fixtures_and_the_guion_copy`: verifica que tras restaurar un respaldo manual, los fixtures y la copia del guion originales permanezcan en su ruta, los fixtures creados con posterioridad se aparten en `evidence-sin-uso/<stamp>/<proyecto>/fixtures/<id>/...` y `missing_files` retorne 0.

## Archivos
- `src-tauri/src/repo/fixtures.rs` (creado)
- `src-tauri/src/repo/mod.rs` (modificado)
- `src-tauri/src/repo/evidence.rs` (modificado)
- `src-tauri/src/dto.rs` (modificado)
- `src-tauri/src/backup.rs` (modificado)

## Pruebas
Todos los comandos desde `src-tauri/`.
- RED:
  - `cargo test repo::fixtures` → FAILED con `not yet implemented: add` en las 5 pruebas del módulo.
- GREEN:
  - `cargo test repo::fixtures` → 5 passed; 0 failed.
  - `cargo test sweep_keeps_fixture_files` → 1 passed; 0 failed.
  - `cargo test restore_keeps_fixtures_and_the_guion_copy` → 1 passed; 0 failed.
- Regresión completa:
  - `cargo test` → 558 passed, 0 failed, 4 ignored en biblioteca; 4 passed en pruebas de integración (`tests/bridge.rs`).
- Lint y formato:
  - `cargo fmt --check` → exit 0.
  - `cargo clippy --all-targets -- -D warnings` → exit 0, sin advertencias.

## Autorrevisión
- Se verificó que `clean_name` y `check_size` operen con los mensajes estándar de D12.
- Si falla la inserción en BD, el archivo escrito en disco se elimina y se limpia la carpeta `fixtures/<id>` huérfana.
- `missing_files` chequea la existencia de las tablas `case_fixture` y `project_source` con `has_table` para garantizar compatibilidad con esquemas anteriores.
- `restore` de respaldos aparta correctamente las subcarpetas de fixtures en `evidence-sin-uso/` y conserva los archivos vigentes en el respaldo.

## Dudas o riesgos
- Ninguna. Las funciones y DTOs producidos están listos para Task 4 (paquete formato 2) y Task 5 (comandos tauri).

## Revisión (la llena el controlador)
- Spec:
- Hallazgos y cómo se resolvieron
