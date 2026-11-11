# Tarea 4 · Paquete .qapture formato 2 (G2-T3)

**Estado:** DONE
**Commits:** `3e5403b` (`feat(g2): paquete formato 2 con origen del guion y archivos de prueba`).

## Qué se implementó
- Constantes de versión en `transfer::package`:
  - `FORMAT_VERSION = 2` (el más nuevo que se lee y escribe para proyectos con guion o fixtures).
  - `FORMAT_V1 = 1` (versión histórica para proyectos sin origen de guion ni archivos de prueba).
- Estructuras del paquete:
  - `PackageProject`: agregada propiedad opcional `source: Option<PackageSource>`, omitida en JSON si es `None`.
  - `PackageSource`: define `file_name`, `sha256`, `sheet`, `header_row: u32`, `definition: ProfileDefinition`, `imported_at`, y `file: Option<String>` (con `camelCase` y `deny_unknown_fields`).
  - `PackageCase`: agregadas propiedades opcionales `source_row: Option<u32>` y `fixtures: Vec<PackageFixture>` (con `skip_serializing_if = "Vec::is_empty"` y `default`).
  - `PackageFixture`: define `file_name`, `note`, `size: i64`, `sha256`, `added_at`, y `file: String` (con `camelCase` y `deny_unknown_fields`).
- Modelo intermedio en `transfer::bundle`:
  - `BundleFixture`: estructura con `id`, `file_name`, `stored_path`, `size`, `sha256`, `note`, `added_at`.
  - `BundleCase`: agregado campo `fixtures: Vec<BundleFixture>`.
  - `BundleSource`: agregado campo `imported_at: Option<String>` para preservar la fecha original de importación del guion en round-trips.
  - `ImportFiles::write_fixture`: escribe un archivo de prueba en `evidence/<p>/fixtures/<id>/<name>` mediante `media::fixtures::write`.
  - `insert_case`: inserta fixtures en `case_fixture` con `position = 0..n`, ruta almacenada, tamaño y hash.
  - `insert_project`: inserta `project_source` respetando `source.imported_at` si está presente.
- Exportación (`package::collect` y `write_zip`):
  - Regla P5: Si el proyecto tiene `project_source` o fixtures, se exporta con `formatVersion: 2`; si no, con `formatVersion: 1` omitiendo las claves nuevas en `project.json`.
  - Copia de guion: si existe `source` y el archivo físico existe en disco, se agrega entrada `"source/guion.xlsx"` con compresión `Deflated` y `source.file = Some("source/guion.xlsx")`. Si no existe en disco, `source.file = None` y suma 1 a `missing_files`.
  - Fixtures: se enumeran globalmente `fixtures/1`, `fixtures/2`... en orden de casos y posición. Si el archivo físico existe en disco, se comprime con `Deflated` y se agrega a `fixtures`. Si no existe en disco, se omite de `PackageDoc` y suma 1 a `missing_files`.
  - `PackageExport`: incluye `files: Vec<(String, PathBuf)>` y `missing_files: i64`.
- Validación y lectura (`read_doc`, `validate`, `to_bundle`):
  - Formatos aceptados: 1 y 2. Si `format_version > 2`, devuelve `AppError::validation(NEWER)`.
  - En formato 1: si contiene `source`, `sourceRow` o `fixtures`, devuelve `AppError::validation(NOT_PACKAGE)`.
  - En formato 2: los códigos de caso validan según P6 (1-40 caracteres, sin espacios en blanco alrededor, únicos). No se exige secuencia ni prefijo `EJ-`.
  - Validación de fixtures: máximo 50 por caso, `clean_name`, nombres únicos sin distinguir mayúsculas por caso, `check_size`, nota de hasta 500 caracteres, sha256 de 64 caracteres hex en minúsculas, y `file` de la forma `"fixtures/<dígitos>"` único en todo el paquete.
  - Validación de origen de guion: definición sin problemas según `profile::problems`, `sha256` de 64 caracteres hex en minúsculas, y `file` igual a `None` o `Some("source/guion.xlsx")`.
  - `to_bundle`: lee cada entrada con su límite (`MAX_FIXTURE_BYTES` de 25 MB, `MAX_GUION_BYTES` de 50 MB), valida tamaño y sha256 (`El paquete está dañado: «<entrada>» no coincide con su registro.`), escribe los archivos atómicamente y genera IDs nuevos. Si algo falla, el drop de `ImportFiles` limpia todos los archivos creados sin dejar restos.

## Archivos
- `src-tauri/src/transfer/package.rs` (modificado)
- `src-tauri/src/transfer/bundle.rs` (modificado)
- `src-tauri/src/transfer/legacy.rs` (modificado)
- `src-tauri/src/transfer/guion/import.rs` (modificado)

## Pruebas
Todos los comandos ejecutados desde `src-tauri/`:
- RED:
  - Error de compilación por `BundleCase::fixtures` faltante en inicializadores de paquetes y validaciones previas de formato 2.
- GREEN:
  - `cargo test transfer::` → 77 passed; 0 failed.
  - Pruebas clave de la tarea:
    - `a_plain_project_still_writes_format_1`
    - `a_guion_project_round_trips_with_its_files`
    - `guion_codes_and_low_sequences_are_accepted_in_format_2`
    - `format_1_with_format_2_fields_is_not_a_package`
    - `tampered_fixture_entries_are_rejected_without_leftovers`
    - `missing_files_are_skipped_on_export_and_counted`
- Regresión completa:
  - `cargo test` → 568 passed (564 en lib, 4 en bridge.rs); 0 failed; 4 ignored (pruebas de captura que requieren monitor real).
- Lint y formato:
  - `cargo fmt --check` → exit 0.
  - `cargo clippy --all-targets -- -D warnings` → exit 0, sin advertencias.

## Autorrevisión
- Proyectos sencillos sin guion ni fixtures siguen serializándose con `formatVersion: 1` sin campos nuevos en `project.json`, preservando compatibilidad regresiva con Qapture 1.2.
- Un round-trip completo (importar guion + adjuntar fixtures -> exportar -> importar en entorno limpio -> exportar de nuevo) produce un `project.json` idéntico byte a byte.
- Paquetes manipulados (hash alterado, tamaño alterado, path traversal en nombre o entrada zip, extensiones prohibidas) se rechazan sin persistir proyectos en la BD ni archivos residuales en la carpeta de evidencias.
- Archivos faltantes en disco al momento de exportar se omiten del paquete y se contabilizan correctamente en `missing_files`.

## Dudas o riesgos
- Ninguna. Las estructuras y funciones están listas para conectarse con los comandos Tauri en la Tarea 5.

## Revisión (la llena el controlador)
- Spec:
- Hallazgos y cómo se resolvieron
