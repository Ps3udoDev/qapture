# Tarea 2 · Almacén en disco de los archivos de prueba (G2-T1, parte 1)

**Estado:** DONE
**Commits:** `39bd023` (`feat(g2): almacén en disco de archivos de prueba`).

## Qué se implementó
- Módulo `media::fixtures`:
  - Constantes `MAX_FIXTURE_BYTES = 25 MB`, `EXTENSIONS = ["xlsx", "xls", "csv", "txt", "pdf", "json", "xml", "zip"]`, y mensajes de error `TOO_BIG`, `NOT_ALLOWED`, `BAD_NAME`.
  - `clean_name(raw: &str) -> AppResult<String>`: recorta espacios; valida longitud (1 a 200 caracteres), ausencia de caracteres prohibidos (`<>:"/\|?*`) o de control, terminación sin punto ni espacio, y que el nombre base antes de puntos no coincida sin distinguir mayúsculas con nombres de dispositivo reservados DOS (`CON`, `PRN`, `AUX`, `NUL`, `COM1`..`COM9`, `LPT1`..`LPT9`); verifica que la extensión figure en `EXTENSIONS` (en minúsculas), devolviendo el nombre recortado.
  - `check_size(len: u64) -> AppResult<()>`: valida que el archivo no supere `MAX_FIXTURE_BYTES`.
  - `fixture_rel(project_id: &str, fixture_id: &str, name: &str) -> String`: construye la ruta relativa `evidence/{project_id}/fixtures/{fixture_id}/{name}` con `/`.
  - `write(files: &EvidenceFiles, rel: &str, bytes: &[u8]) -> AppResult<()>`: crea carpetas intermedias si faltan, escribe atómicamente con `storage::write_atomic`, y fija permisos de solo lectura (`permissions.set_readonly(true)`).
  - `sha256_hex(bytes: &[u8]) -> String`: genera el resumen SHA-256 en hexadecimal en minúsculas con `sha2::Sha256`.
- Módulo `media::storage`:
  - `pub(crate) fn remove_file(path: &Path) -> std::io::Result<()>`: si el archivo existe, retira la marca de solo lectura (`set_readonly(false)`) y lo borra con `fs::remove_file`. Utilizado por `remove_files`, `sweep` y el repositorio de fixtures.
  - `remove_project`: si `fs::remove_dir_all` falla por archivos de solo lectura, limpia recursivamente el atributo de solo lectura de todos los contenidos y reintenta el borrado del directorio.
  - `sweep` y `set_aside`: recorren recursivamente las carpetas de los proyectos (`collect_files_recursive`) para abarcar archivos en subdirectorios anidados como `fixtures/<id>/<nombre>`. Las rutas relativas normalizan separadores a `/`. `sweep` limpia subdirectorios vacíos al concluir. `set_aside` preserva la estructura anidada bajo `evidence-sin-uso/<stamp>/<proyecto>/...`, y `undo` devuelve cada archivo a su ruta original eliminando los directorios vacíos creados en `evidence-sin-uso/`.
- `transfer::guion::import`: reutiliza `media::fixtures::sha256_hex` para el cálculo del hash del archivo de guion.

## Archivos
- `src-tauri/src/media/fixtures.rs` (creado)
- `src-tauri/src/media/mod.rs` (modificado)
- `src-tauri/src/media/storage.rs` (modificado)
- `src-tauri/src/transfer/guion/import.rs` (modificado)

## Pruebas
Todos los comandos desde `src-tauri/`.
- RED:
  - `cargo test media::` → FAILED (4 fallos esperados):
    - `media::fixtures::tests::names_and_extensions_follow_d12`
    - `media::fixtures::tests::written_fixtures_are_read_only`
    - `media::storage::tests::sweep_handles_nested_files`
    - `media::storage::tests::set_aside_handles_nested_files`
- GREEN:
  - `cargo test media::` → 21 passed; 0 failed.
- Regresión completa:
  - `cargo test` → 551 passed, 0 failed, 4 ignored en biblioteca; 4 passed en pruebas de integración (`tests/bridge.rs`).
- Lint y formato:
  - `cargo fmt --check` → exit 0.
  - `cargo clippy --all-targets -- -D warnings` → exit 0, sin advertencias.

## Autorrevisión
- Los nombres DOS reservados se comprueban separando por el primer punto para cubrir casos como `con.txt` o `LPT1.csv` sin importar mayúsculas.
- `write` deja el archivo en disco con el bit readonly activado, garantizando inmutabilidad.
- `remove_file` y `remove_project` eliminan de forma segura archivos readonly en Windows sin errores de permiso (`Access is denied`).
- La recursión en `storage` maneja estructuras de profundidad arbitraria para fixtures y guiones sin dejar carpetas vacías tras `sweep` o `undo`.

## Dudas o riesgos
- Ninguna para la Tarea 3. La interfaz `media::fixtures` y las primitivas de `storage` quedan listas para ser consumidas por el repositorio de fixtures.

## Revisión (la llena el controlador)
- Spec:
- Hallazgos y cómo se resolvieron
