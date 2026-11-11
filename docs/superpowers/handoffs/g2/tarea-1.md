# Tarea 1 · Restaurar respaldos anteriores a las tablas del guion

**Estado:** DONE
**Commits:** ver `git log` (`fix(g2): restaurar respaldos anteriores a las tablas del guion`).

## Qué se implementó
- `repo::evidence::has_table(conn: &Connection, name: &str) -> AppResult<bool>` (`pub(crate)`): comprueba la existencia de una tabla en la base de datos consultando `sqlite_master`.
- `repo::evidence::references`: consulta `project_source` de forma condicional, únicamente si `has_table(conn, "project_source")?` es true. Esto evita errores de SQLite (`no such table: project_source`) al restaurar o recuperar bases con esquema anterior a la migración 0007.
- `db::write_version_for_tests(path: &Path, version: usize) -> AppResult<()>` (`#[cfg(test)] pub(crate)`): crea directorios padre de ser necesario, inicializa una conexión SQLite en `path`, aplica ordenadamente `MIGRATIONS[..version]` actualizando `PRAGMA user_version` tras cada migración, y deja la base en la versión indicada. Se refactorizó `write_v1_for_tests` para reutilizarlo.

## Archivos
- `src-tauri/src/repo/evidence.rs`.
- `src-tauri/src/db/mod.rs`.
- `src-tauri/src/backup.rs`.
- `src-tauri/src/db/recovery.rs`.

## Pruebas
Todos los comandos desde `src-tauri/`.
- RED:
  - `cargo test a_backup_from_before_the_guion_tables_is_restorable` → FAIL con `panicked at src\backup.rs: called Result::unwrap() on an Err value: Db(SqliteFailure(Error { code: Unknown, extended_code: 1 }, Some("no such table: project_source")))`.
  - `cargo test references_of_tolerates_a_schema_without_guion_tables` → FAIL con `panicked at src\db\recovery.rs: called Result::unwrap() on an Err value: Db(SqliteFailure(Error { code: Unknown, extended_code: 1 }, Some("no such table: project_source")))`.
  El fallo reprodujo exactamente el comportamiento esperado: al examinar referencias para apartar archivos o restaurar un respaldo con esquema 6, fallaba por ausencia de `project_source`.
- GREEN:
  - `cargo test a_backup_from_before_the_guion_tables_is_restorable` → 1 passed; 0 failed.
  - `cargo test references_of_tolerates_a_schema_without_guion_tables` → 1 passed; 0 failed.
  - `cargo test has_table_checks_table_existence` → 1 passed; 0 failed.
- Regresión completa: `cargo test` → 546 passed, 0 failed, 4 ignored en biblioteca; 4 passed en pruebas de integración (`tests/bridge.rs`).
- Lint y formato: `cargo fmt --check` y `cargo clippy --all-targets -- -D warnings` → exit 0, sin advertencias.

## Autorrevisión
- `has_table` ejecuta una consulta parametrizada contra `sqlite_master` con `type = 'table' AND name = ?1`.
- `references` mantiene intacta la recolección de rutas de `evidence` y de `project_source` (cuando existe), resolviendo el bloqueo al restaurar respaldos antiguos (esquema < 7).
- `write_version_for_tests` corre las migraciones con transacciones individuales y fija `user_version` tal como `db::migrate`.
- Sin dependencias nuevas ni cambios en el esquema de base de datos.

## Dudas o riesgos
- Cuando la Tarea 3 introduzca `case_fixture`, sus rutas en disco también deberán añadirse a `references` de manera condicional usando `has_table(conn, "case_fixture")?` para preservar la compatibilidad con respaldos anteriores a G2.

## Revisión (la llena el controlador)
- Spec:
- Hallazgos y cómo se resolvieron
