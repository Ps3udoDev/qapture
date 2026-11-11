# Tarea 3 · Migración 0007 y repositorios (G1-T3)

**Estado:** DONE_WITH_CONCERNS
**Commits:** ver `git log` (`feat(g1): tablas y repositorios del origen del guion`).

## Qué se implementó
- Migración 0007: `import_profile`, `project_source` con `definition`, `case_source` y `case_fixture`, índice por caso y cascadas. Borrar un perfil pone `profile_id` en NULL sin perder la definición usada.
- DTO de perfil y repositorio para crear, listar y borrar perfiles; lectura de origen y filas en el orden de los casos.
- La copia del guion forma parte de las referencias de evidencia y queda protegida frente a `sweep` y `set_aside`.
- Los perfiles con JSON ilegible se omiten al listar y se registra un aviso sin volcar su contenido.

## Archivos
- `src-tauri/src/db/migrations/0007_guion.sql`.
- `src-tauri/src/db/mod.rs`.
- `src-tauri/src/repo/guion.rs`, `repo/mod.rs` y `repo/evidence.rs`.
- `src-tauri/src/dto.rs`.

## Pruebas
Comandos desde `src-tauri/`, salvo `git diff --check` desde la raíz.
- RED: `cargo test repo::guion` → 17 errores de compilación por funciones y tipos del repositorio inexistentes; las tres pruebas del plan ya estaban escritas.
- GREEN inicial: `cargo test repo::guion` → 3 passed, 0 failed.
- GREEN final: `cargo test repo::` → 62 passed, 0 failed (incluye seis pruebas del repositorio de guion y la prueba de conservación de su copia).
- Migraciones y recuperación: `cargo test db::` → 20 passed, 0 failed, incluida la consulta de `project_source.definition` en la prueba de idempotencia.
- Lint: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` y `git diff --check` → exit 0, sin errores ni advertencias de clippy.

## Autorrevisión
- Se mantienen los mensajes fijos del plan y la validación de nombres con límite de 100 caracteres.
- Los tests insertan origen y filas con SQL directo; la escritura transaccional real pertenece a T5.
- Se comprueba el orden de filas aunque `case_source` se inserte en orden inverso, la conservación de `definition`, las cascadas de origen y archivos de prueba y la conservación de la copia en disco.
- Sin dependencias nuevas ni API para renombrar perfiles.

## Dudas o riesgos
- `allow(dead_code)` temporal en `repo::guion` hasta su consumo desde T5/T6; retirarlo al integrar los consumidores.
- La comparación de nombres de perfiles se hace con `to_lowercase`, sin eliminar acentos; la conexión serial de `AppState` evita carreras entre consultas e inserción. El índice SQLite NOCASE es una guarda adicional para nombres ASCII.
- El enlazador MSVC mantiene su aviso previo `linker_messages`, ajeno a esta tarea.

## Revisión (la llena el controlador)
- Spec: ✅ · Calidad: Aprobada. Revisión independiente del diff `ed09680..248d731`.
- Important corregido en ronda 1: `COLLATE NOCASE` solo reconoce mayúsculas ASCII; se añadió comparación y orden Unicode con `to_lowercase`, conservando la distinción entre letras con y sin acento.
- La prueba de regresión reprodujo el duplicado antes del cambio y pasó después. Conservación de `definition`, cascadas, orden de casos y protección de la copia verificados; sin hallazgos pendientes.

## Correcciones (ronda 1)
- Hallazgo Important: SQLite NOCASE solo compara ASCII y no detectaba perfiles duplicados con mayúsculas españolas (`GUION ÁREA` frente a `guion área`). Se añadió una prueba que también conserva la distinción entre nombres con y sin acento y verifica su orden.
- RED: `cargo test repo::guion::tests::profile_names_ignore_unicode_case_but_keep_accents` → 1 failed: `unwrap_err()` recibió un perfil duplicado creado correctamente, confirmando el fallo de NOCASE con Á/á.
- Corrección: comparación de todos los nombres mediante `to_lowercase` y ordenación de perfiles con la misma regla. GREEN: la prueba pasa dentro de `cargo test repo::` (62 passed).
