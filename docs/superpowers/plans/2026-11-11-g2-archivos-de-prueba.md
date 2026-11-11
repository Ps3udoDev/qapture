# G · Bloque 2 — Archivos de prueba (G2) · Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** El tester adjunta a cada caso sus archivos de prueba (por ejemplo `F13_valor_distinto.xlsx`). Esos archivos y la copia del guion viajan en el paquete `.qapture` y sobreviven a restaurar un respaldo. Además, un proyecto importado de un guion (códigos `CP01`…) se puede volver a importar como paquete.

**Architecture:** Los archivos se guardan en `evidence/<proyecto>/fixtures/<id>/<nombre original>`, en solo lectura. Así heredan el borrado con el proyecto, la limpieza de huérfanos (`sweep`) y la protección al restaurar (`set_aside`); para eso, `media/storage.rs` pasa a recorrer subcarpetas. La tabla `case_fixture` (creada por la migración 0007 en G1) se maneja en `repo/fixtures.rs`. El paquete pasa al formato 2 solo cuando el proyecto tiene origen de guion o archivos de prueba; si no, se sigue escribiendo el formato 1, idéntico al de hoy. El webview manda los bytes del archivo (elegido con `<input type="file">` o soltado en la sección) y nunca recibe rutas. «Mostrar en carpeta» y «Copiar ruta» se resuelven en Rust.

**Tech Stack:** Rust 2024, `rusqlite`, `zip` 9, `sha2`, `tauri-plugin-opener` y `tauri-plugin-clipboard-manager` (ya instalados); React 19, Zustand, `zod`, Vitest + Testing Library.

**Spec:** [`2026-11-10-guion-a-evidencias-design.md`](../specs/2026-11-10-guion-a-evidencias-design.md) (§3 D12 y D13, §4, §5 `case_fixture`, §6 «Archivos de prueba») · **Contrato:** [`G-guion-a-evidencias`](../contracts/G-guion-a-evidencias.md) (G2-T1 a G2-T4) · **Base:** G1 fusionado ([handoffs G1](../handoffs/g1/README.md)).

## Global Constraints

- Rama `feat/g2-archivos-de-prueba`, creada desde `main` con G1 fusionado. Un PR al terminar G2.
- Sin dependencias nuevas, ni en `Cargo.toml` ni en `package.json`. No hay migraciones: `case_fixture` ya existe (0007).
- Mensajes fijos del contrato, idénticos: `El archivo de prueba supera 25 MB.` · `Ese tipo de archivo no se acepta como archivo de prueba.`
- Mensajes nuevos de este plan (también fijos):
  - `El nombre del archivo de prueba no es válido.`
  - `El caso ya tiene un archivo llamado «<nombre>».`
  - `El caso ya tiene 50 archivos de prueba.`
  - `La nota supera 500 caracteres.`
  - `El archivo de prueba no existe.`
  - `El archivo de prueba ya no está en disco.`
  - `El paquete está dañado: «<entrada>» no coincide con su registro.`
  - `Ruta copiada.`
- Extensiones aceptadas (D12), sin distinguir mayúsculas: `xlsx`, `xls`, `csv`, `txt`, `pdf`, `json`, `xml`, `zip`. Hasta 25 MB (`25 * 1024 * 1024` bytes) por archivo; se admiten archivos vacíos.
- Los archivos de prueba **nunca se abren ni se ejecutan** desde Qapture (D12): no hay comando que use `open_path` sobre ellos.
- El webview no recibe rutas absolutas: `FixtureDto` no lleva `stored_path`.
- Sin herramientas MCP nuevas (spec A5).
- Verificación por task, desde `src-tauri/`: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test <filtro>`. Desde la raíz, en las tasks de TS: `npm run lint`, `npm run typecheck`, `npx vitest run <archivo>`.
- Commits `fix(g2): …`, `feat(g2): …`, `test(g2): …`, `docs(g2): …`, terminados en `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Handoffs en `docs/superpowers/handoffs/g2/`: el controlador copia el `README.md` de `handoffs/g1/` (mismo formato y reglas) antes de la Task 1. Cada implementador escribe `tarea-<N>.md`.

### Decisiones de este plan (no están en el spec)

| # | Decisión | Por qué |
|---|---|---|
| P1 | Ruta `evidence/<proyecto>/fixtures/<id>/<nombre original>`; `sweep` y `set_aside` recorren subcarpetas | El tester sube el archivo con su nombre real al sistema que prueba. Dentro de `evidence/` se hereda el borrado, la limpieza y la protección al restaurar |
| P2 | Archivos en solo lectura. Todo borrado pasa por `storage::remove_file`, que quita esa marca antes | En Windows, `remove_file` falla con un archivo de solo lectura y hoy ese error se ignora (`let _ =`): el archivo quedaría para siempre |
| P3 | `media/fixtures.rs` valida y escribe en disco; `repo/fixtures.rs` maneja la tabla | Mismo reparto que `media/storage.rs` y `repo/evidence.rs`. El contrato pedía todo en `media/fixtures.rs`; la Task 7 ajusta su texto |
| P4 | «Agregar…» usa `<input type="file">` del webview y el comando recibe los bytes en un `frame` (como `add_evidence`) | Sin permisos nuevos de diálogo y sin rutas desde el webview. Soltar archivos usa el mismo camino |
| P5 | El paquete escribe `formatVersion: 2` solo si el proyecto tiene origen de guion o algún archivo de prueba | Los paquetes de proyectos normales siguen abriéndose en Qapture 1.2, y su `project.json` no cambia |
| P6 | En el formato 2 los códigos de caso son libres (sin espacios alrededor, 1–40 caracteres, únicos) y no se compara la secuencia con los `EJ-NNN` | G1 importa con `next_sequence = 1` aunque el guion traiga `EJ-001`. Desde G1, `insert_case` salta los códigos ocupados |
| P7 | Duplicar un caso no copia sus archivos de prueba | Los casos llegan del guion, no de duplicar. Copiar 25 MB por cada duplicado sorprendería al tester |
| P8 | La nota se guarda y viaja en el paquete, pero en G2 no se edita (la interfaz manda `""`) | La columna existe desde 0007. Agregar una edición no la pide ninguna tarea del contrato |

## Review Focus

- **Restaurar un respaldo hecho antes de G1** (esquema 6, sin `project_source`): hoy `recovery::references_of` falla con `no such table: project_source`. Debe restaurarse como cualquier otro. Comprobado en un respaldo diario real del 10/11. Test `a_backup_from_before_the_guion_tables_is_restorable` en la Task 1.
- **Borrar en Windows un archivo de solo lectura**: al quitarlo, al borrar su caso o al borrar su proyecto, el archivo desaparece del disco. Tests `readonly_files_are_removed` (Task 2) y `deleting_the_case_or_project_removes_its_fixture_files` (Task 3).
- **Restaurar un respaldo con archivos de prueba en subcarpetas**: los archivos que el respaldo conoce se quedan en su sitio y no se mandan a `evidence-sin-uso/`; los que no conoce se apartan y `undo` los devuelve. Tests `set_aside_handles_nested_files` (Task 2) y `restore_keeps_fixtures_and_the_guion_copy` (Task 3).
- **Paquete manipulado**: un archivo de prueba cuyo tamaño o `sha256` no coincide, un nombre como `..\x.txt`, o una entrada fuera de `fixtures/<n>`. Se rechaza todo y no se escribe nada. Test `tampered_fixture_entries_are_rejected_without_leftovers` en la Task 4.
- **Soltar un `.xlsx` sobre la sección «Archivos de prueba»**: entra como archivo de prueba, no como captura; la zona de capturas del detalle no lo recibe. Test `dropping_on_the_section_adds_a_fixture_not_an_evidence` en la Task 7.

---

### Task 1: Restaurar respaldos anteriores a las tablas del guion

**Files:**
- Modify: `src-tauri/src/repo/evidence.rs` (`references`), `src-tauri/src/db/mod.rs` (ayuda de prueba)
- Test: `src-tauri/src/backup.rs`, `src-tauri/src/db/recovery.rs`

**Interfaces:**
- Produces:
  - `repo::evidence::has_table(conn: &Connection, name: &str) -> AppResult<bool>` (`pub(crate)`).
  - `references` solo consulta `project_source` (y, desde la Task 3, `case_fixture`) si la tabla existe.
  - `db::write_version_for_tests(path: &Path, version: usize)` (`#[cfg(test)] pub(crate)`): aplica `MIGRATIONS[..version]` y deja `user_version = version`.

- [ ] **Step 1: Write the failing tests**

```rust
// backup.rs
#[test]
fn a_backup_from_before_the_guion_tables_is_restorable() {
    // Respaldo con esquema 6: tabla evidence, sin project_source ni case_fixture.
    // Se escribe con db::write_version_for_tests(.., 6) dentro de backups/ con nombre
    // «qapture-manual-20261110-120000.db» y un proyecto «Antiguo».
    // restore(...) termina bien; la base queda en latest_version() y lista «Antiguo».
}
```

En `recovery.rs`: `references_of_tolerates_a_schema_without_guion_tables`. Con la misma base de esquema 6 y una fila de `evidence`, `references_of` devuelve sus dos rutas y el id del proyecto.

- [ ] **Step 2: Run tests to verify they fail** — `cargo test backup:: recovery::` → FAIL con `no such table: project_source`.

- [ ] **Step 3: Implement** `has_table` (consulta `sqlite_master`) y su uso en `references`; también `write_version_for_tests`.

- [ ] **Step 4: Run tests to verify they pass** — `cargo test` (todo) → verde.

- [ ] **Step 5: Lint y commit** — `fix(g2): restaurar respaldos anteriores a las tablas del guion`

---

### Task 2: Almacén en disco de los archivos de prueba (G2-T1, parte 1)

**Files:**
- Create: `src-tauri/src/media/fixtures.rs`
- Modify: `src-tauri/src/media/mod.rs`, `src-tauri/src/media/storage.rs`, `src-tauri/src/transfer/guion/import.rs` (usar `sha256_hex`)

**Interfaces:**
- Produces (`media::fixtures`):
  - Constantes `MAX_FIXTURE_BYTES: u64 = 25 * 1024 * 1024`, `EXTENSIONS: [&str; 8]` (en el orden de D12), `TOO_BIG`, `NOT_ALLOWED` y `BAD_NAME` con los textos fijos.
  - `clean_name(raw: &str) -> AppResult<String>`. Recorta los espacios y valida en este orden:
    1. Nombre válido, si no `BAD_NAME`: de 1 a 200 caracteres; sin `<>:"/\|?*` ni caracteres de control; sin terminar en `.` ni en espacio; con base distinta de `CON`, `PRN`, `AUX`, `NUL`, `COM1`–`COM9` y `LPT1`–`LPT9`, sin distinguir mayúsculas.
    2. Extensión (lo que sigue al último `.`) en `EXTENSIONS`, si no `NOT_ALLOWED`.
  - `check_size(len: u64) -> AppResult<()>`: devuelve `TOO_BIG` si `len > MAX_FIXTURE_BYTES`.
  - `fixture_rel(project_id: &str, fixture_id: &str, name: &str) -> String` = `evidence/<p>/fixtures/<id>/<name>`.
  - `write(files: &EvidenceFiles, rel: &str, bytes: &[u8]) -> AppResult<()>`: crea las carpetas, usa `write_atomic` y marca el archivo como de solo lectura.
  - `sha256_hex(bytes: &[u8]) -> String` (hex en minúsculas). `guion::import` pasa a usarla.
- Produces (`media::storage`):
  - `pub(crate) fn remove_file(path: &Path) -> io::Result<()>`: quita la marca de solo lectura y borra. La usan `remove_files`, `sweep` y los fixtures.
  - `remove_project` borra la carpeta aunque tenga archivos de solo lectura.
  - `sweep` y `set_aside` recorren cada carpeta de proyecto de forma recursiva. La ruta relativa usa `/` (`evidence/<p>/fixtures/<id>/<nombre>`). Las subcarpetas vacías se borran al terminar. `set_aside` conserva la estructura bajo `evidence-sin-uso/<stamp>/<p>/…`, y `undo` la devuelve y quita las carpetas vacías que creó.

- [ ] **Step 1: Write the failing tests**

```rust
// media/fixtures.rs
#[test]
fn names_and_extensions_follow_d12() {
    assert_eq!(clean_name("  F13_valor_distinto.XLSX ").unwrap(), "F13_valor_distinto.XLSX");
    for ok in ["a.csv", "b.txt", "c.pdf", "d.json", "e.xml", "f.zip", "g.xls"] {
        assert!(clean_name(ok).is_ok(), "{ok}");
    }
    for bad in ["", "..", "a/b.txt", "a\\b.txt", "x:y.txt", "con.txt", "LPT1.csv", "a.txt.", "a\u{7}.txt"] {
        assert_eq!(clean_name(bad).unwrap_err().to_string(), BAD_NAME, "{bad:?}");
    }
    for bad in ["setup.exe", "script.ps1", "sin_extension"] {
        assert_eq!(clean_name(bad).unwrap_err().to_string(), NOT_ALLOWED, "{bad:?}");
    }
    assert!(check_size(MAX_FIXTURE_BYTES).is_ok());
    assert_eq!(check_size(MAX_FIXTURE_BYTES + 1).unwrap_err().to_string(), TOO_BIG);
}

#[test]
fn written_fixtures_are_read_only() { /* write → metadata().permissions().readonly() */ }
```

En `storage.rs`:
- `readonly_files_are_removed`: `remove_files` y `remove_project` borran un archivo de solo lectura que está en una subcarpeta.
- `sweep_handles_nested_files`: una subcarpeta referenciada se queda y otra sin referencia se borra, sin dejar carpetas vacías.
- `set_aside_handles_nested_files`: el archivo anidado sin referencia va a `evidence-sin-uso/<stamp>/<p>/fixtures/<id>/<nombre>`, y `undo` lo devuelve a su ruta y deja `evidence-sin-uso/` sin carpetas vacías.

Las pruebas actuales de `sweep` y `set_aside` siguen iguales.

- [ ] **Step 2: Run tests to verify they fail** — `cargo test media::` → FAIL.

- [ ] **Step 3: Implement** según las Interfaces. Para quitar la marca de solo lectura: `let mut p = meta.permissions(); p.set_readonly(false); fs::set_permissions(path, p)`. En `remove_project`, si `remove_dir_all` falla, recorrer la carpeta, quitar la marca de solo lectura a todos los archivos y reintentar una vez.

- [ ] **Step 4: Run tests to verify they pass** — `cargo test` (todo) → verde.

- [ ] **Step 5: Lint y commit** — `feat(g2): almacén en disco de archivos de prueba`

---

### Task 3: Repositorio de archivos de prueba (G2-T1, parte 2)

**Files:**
- Create: `src-tauri/src/repo/fixtures.rs`
- Modify: `src-tauri/src/repo/mod.rs`, `src-tauri/src/repo/evidence.rs` (`references`, `case_files`), `src-tauri/src/dto.rs` (`FixtureDto`)
- Test: `src-tauri/src/backup.rs`

**Interfaces:**
- Consumes: `media::fixtures` (Task 2), `has_table` (Task 1).
- Produces:
  - `dto::FixtureDto { id, case_id, file_name, size: i64, sha256, note, position: i64, added_at }` (camelCase, sin ruta).
  - `repo::fixtures::add(conn, files, case_id: &str, file_name: &str, note: &str, bytes: &[u8]) -> AppResult<FixtureDto>`. Orden:
    1. Valida: `clean_name`, `check_size`, nota recortada de hasta 500 caracteres (`La nota supera 500 caracteres.`), que el caso exista (`NotFound("El caso no existe.")`), menos de 50 archivos en el caso (`El caso ya tiene 50 archivos de prueba.`) y que el nombre no se repita en el caso sin distinguir mayúsculas (`El caso ya tiene un archivo llamado «<nombre>».`).
    2. Escribe con `fixtures::write`.
    3. Inserta con `position = max + 1` y `added_at = db::now()`, y toca el proyecto, todo en una transacción. Si la base falla, borra el archivo escrito.
  - `repo::fixtures::list(conn, case_id) -> AppResult<Vec<FixtureDto>>`, por `position`.
  - `repo::fixtures::remove(conn, files, id) -> AppResult<()>`: devuelve `NotFound("El archivo de prueba no existe.")` si no está. Borra la fila, compacta las posiciones, toca el proyecto y después borra el archivo y su carpeta `fixtures/<id>/`.
  - `repo::fixtures::absolute(conn, files, id) -> AppResult<PathBuf>`: devuelve `El archivo de prueba ya no está en disco.` si falta el archivo.
  - `repo::fixtures::missing_files(conn, files) -> AppResult<i64>`: cuenta las filas de `case_fixture` y `project_source` cuyo archivo no está en disco.
  - `repo::fixtures::stored_by_case(conn, project_id) -> AppResult<HashMap<String, Vec<(FixtureDto, String)>>>`: para cada `case_id`, sus archivos por `position` con su `stored_path` relativo. Lo usa la Task 4 para exportar.
  - `references` incluye `case_fixture.stored_path` si la tabla existe. `evidence::delete_case` también borra los archivos de prueba del caso.

- [ ] **Step 1: Write the failing tests** (`repo/fixtures.rs`)

```rust
#[test]
fn adds_lists_and_removes_in_order() { /* dos archivos → posiciones 0 y 1, sha256 y size; quitar el primero compacta y borra su carpeta */ }
#[test]
fn rejects_duplicates_limits_and_unknown_cases() { /* «A.txt» y «a.TXT»; 51.º archivo; caso inexistente; nota de 501 */ }
#[test]
fn a_failed_insert_leaves_no_file() { /* trigger RAISE(FAIL) sobre case_fixture → error y fixtures/ vacío */ }
#[test]
fn deleting_the_case_or_project_removes_its_fixture_files() { /* evidence::delete_case y evidence::delete_project */ }
#[test]
fn missing_files_counts_fixtures_and_the_guion_copy() { /* borrar un fixture y la copia en disco → 2 */ }
```

En `backup.rs`, `restore_keeps_fixtures_and_the_guion_copy`:
- Prepara un proyecto con `project_source` y su copia en disco, más un archivo de prueba, y crea un respaldo manual.
- Agrega un segundo archivo de prueba y restaura el respaldo.
- Comprueba que el primero y la copia siguen en su ruta, que el segundo está en `evidence-sin-uso/` y que `missing_files` es 0.

En `repo/evidence.rs`, `sweep_keeps_fixture_files`.

- [ ] **Step 2: Run tests to verify they fail** — `cargo test repo:: backup::` → FAIL.

- [ ] **Step 3: Implement** según las Interfaces.

- [ ] **Step 4: Run tests to verify they pass** — `cargo test` (todo) → verde.

- [ ] **Step 5: Lint y commit** — `feat(g2): repositorio de archivos de prueba`

---

### Task 4: Paquete `.qapture` formato 2 (G2-T3)

**Files:**
- Modify: `src-tauri/src/transfer/package.rs`, `src-tauri/src/transfer/bundle.rs`, `src-tauri/src/transfer/legacy.rs`, `src-tauri/src/transfer/guion/import.rs` (campo nuevo de `BundleCase`)

**Interfaces:**
- Consumes: `media::fixtures` (Task 2), `repo::fixtures::stored_by_case` (Task 3), `repo::guion::source`, `guion::import::GUION_FILE`, `transfer::guion::profile::{ProfileDefinition, problems}`, `guion::read::MAX_FILE_BYTES`.
- Produces:
  - `package::FORMAT_VERSION: i64 = 2` (el más nuevo que se lee) y `package::FORMAT_V1: i64 = 1`.
  - `PackageProject.source: Option<PackageSource>`, `PackageCase.source_row: Option<u32>` y `PackageCase.fixtures: Vec<PackageFixture>`, todos con `#[serde(default, skip_serializing_if = …)]`. Así un `project.json` de formato 1 se escribe y se lee igual que hoy.
  - `PackageSource { file_name, sha256, sheet, header_row: u32, definition: ProfileDefinition, imported_at, file: Option<String> }` y `PackageFixture { file_name, note, size: i64, sha256, added_at, file: String }`, con `camelCase` y `deny_unknown_fields`.
  - Entradas del ZIP: `source/guion.xlsx` y `fixtures/<n>` (n desde 1, en orden de casos y posición), escritas con `Deflated`.
  - `PackageExport.files: Vec<(String, PathBuf)>` y `PackageExport.missing_files: i64`. Si falta un archivo de prueba, se omite del documento. Si falta la copia, queda `source.file = None`. Los dos casos suman a `missing_files`.
  - `bundle::BundleFixture { id, file_name, stored_path, size, sha256, note, added_at }`, `BundleCase.fixtures: Vec<BundleFixture>` (insertados en orden, con `position` 0..n) e `ImportFiles::write_fixture(&mut self, project_id: &str, fixture_id: &str, name: &str, bytes: &[u8]) -> AppResult<String>` (usa `fixtures::write`).

Reglas:
- Escritura: `formatVersion = 2` si el proyecto tiene `project_source` o algún archivo de prueba; si no, `1` (P5).
- Lectura:
  - Se aceptan los formatos 1 y 2; uno mayor que 2 da `NEWER`.
  - En el formato 1, la presencia de `source`, `sourceRow` o `fixtures` da `NOT_PACKAGE`; el resto de sus reglas no cambia.
  - En el formato 2, los códigos siguen P6, con el mensaje de siempre (`ID de ejecución repetido o inválido.`) y sin la regla de secuencia.
  - Cada archivo de prueba pasa por `clean_name` y tiene un tamaño ≤ 25 MB, un `sha256` de 64 caracteres hex en minúsculas y una entrada `fixtures/<dígitos>` única. Cada caso tiene como máximo 50, con nombres únicos sin distinguir mayúsculas y notas de hasta 500.
  - `source.definition` sin `profile::problems`; `source.file` vacío o igual a `source/guion.xlsx`.
- Importación:
  - Cada entrada se lee con su tope: 25 MB para un archivo de prueba y `MAX_FILE_BYTES` para la copia.
  - Su largo y su `sha256` deben coincidir con el registro, si no da `El paquete está dañado: «<entrada>» no coincide con su registro.`
  - `BundleSource.profile_id = None`. `stored_path = evidence/<id>/guion.xlsx`, aunque la copia no venga en el paquete.

- [ ] **Step 1: Write the failing tests** (`package.rs`)

```rust
#[test]
fn a_plain_project_still_writes_format_1() { /* seeded() → formatVersion 1 y sin las claves nuevas en project.json */ }
#[test]
fn a_guion_project_round_trips_with_its_files() {
    // Importa el guion_fixture con G1, adjunta 2 archivos de prueba a CP01 y 1 a CP05.
    // Exporta: formatVersion 2, entradas source/guion.xlsx, fixtures/1..3.
    // Importa: 36 casos CP01–CP36, case_source con sus filas, project_source con la misma
    // definition y sha256, copia en disco idéntica, archivos de prueba con mismo nombre, bytes y orden.
    // Exportar de nuevo da el mismo project.json.
}
#[test]
fn guion_codes_and_low_sequences_are_accepted_in_format_2() { /* códigos «CP01» y «EJ-001» con nextSequence 1 */ }
#[test]
fn format_1_with_format_2_fields_is_not_a_package() {}
#[test]
fn tampered_fixture_entries_are_rejected_without_leftovers() {
    // Por separado: sha256 cambiado, size cambiado, fileName «..\\x.txt», file «fixtures/../a», extensión .exe.
    // Cada uno: error, ningún proyecto nuevo y la carpeta evidence/ sin proyectos nuevos.
}
#[test]
fn missing_files_are_skipped_on_export_and_counted() { /* borrar un archivo de prueba y la copia → missing_files 2, source.file None */ }
```

Las pruebas actuales de paquetes de formato 1 siguen verdes, sin cambiar sus aserciones.

- [ ] **Step 2: Run tests to verify they fail** — `cargo test transfer::` → FAIL.

- [ ] **Step 3: Implement** según las Interfaces.

- [ ] **Step 4: Run tests to verify they pass** — `cargo test` (todo) → verde.

- [ ] **Step 5: Lint y commit** — `feat(g2): paquete formato 2 con origen del guion y archivos de prueba`

---

### Task 5: Comandos y contrato (G2-T2)

**Files:**
- Create: `src-tauri/src/commands/fixtures.rs`
- Modify: `src-tauri/src/commands/{mod.rs,names.rs,data.rs}`, `src-tauri/src/lib.rs`, `src-tauri/capabilities/main.json`, `src-tauri/src/dto.rs`, `src-tauri/src/backup.rs` (`missing_files` en el resultado)
- Modify (TS): `src/infrastructure/tauri/dto.ts`, `src/infrastructure/tauri/contract.test.ts`, `src/infrastructure/tauri/__fixtures__/contracts.json` (regenerado), `src/domain/data.ts`, `src/domain/data.test.ts`, `src/ui/store/data-slice.ts`, `src/infrastructure/memory/data.ts`

**Interfaces:**
- Consumes: `repo::fixtures` (Task 3), `PackageExport.missing_files` (Task 4).
- Produces:
  - Comandos `#[tauri::command(async)]`, todos con `state.ensure_ready()?`:
    - `fixture_add(state, request: Request) -> AppResult<FixtureDto>`: `frame::split::<NewFixtureMeta>` y los bytes.
    - `fixture_list(state, case_id: String) -> AppResult<Vec<FixtureDto>>`.
    - `fixture_remove(state, id: String) -> AppResult<()>`.
    - `fixture_reveal(app, state, id: String) -> AppResult<()>`: `opener().reveal_item_in_dir`.
    - `fixture_copy_path(app, state, id: String) -> AppResult<()>`: `clipboard().write_text` con la ruta absoluta.
  - `dto::NewFixtureMeta { case_id, file_name, note }` (camelCase, `deny_unknown_fields`).
  - `PackageExportResultDto.missing_files: i64` y `RestoreResultDto.missing_files: i64`. La restauración lo calcula con `repo::fixtures::missing_files` después de instalar la base.
  - TS:
    - Esquema `fixtureSchema` y `missingFiles` en `packageExportResultSchema` y `restoreResultSchema`.
    - Tipos `Fixture` (en `domain/fixtures.ts`, que crea la Task 6; aquí basta con el esquema) y `missingFiles` en `PackageExportResult` y `RestoreResult`.
    - `missingFilesMessage(n)` en `domain/data.ts`. `restoreMessage` agrega su frase cuando `missingFiles > 0`.
    - `data-slice.exportPackage` muestra `missingFilesMessage` si `missingFiles > 0`.

Textos (con el `count` de `domain/data.ts`):
- `missingFilesMessage(n)` = `` `${count(n, "archivo de prueba o del guion", "archivos de prueba o del guion")} no ${n === 1 ? "está" : "están"} en disco y no se ${n === 1 ? "incluyó" : "incluyeron"}.` ``
- `restoreMessage` agrega `` ` ${count(n, "archivo de prueba o del guion", "archivos de prueba o del guion")} del respaldo ya no ${n === 1 ? "está" : "están"} en disco.` ``

- [ ] **Step 1: Write the failing tests**
  - Rust:
    - `dto.rs`: `inputs_deserialize_from_ts_shape` e `inputs_reject_unknown_fields` cubren `newFixtureMeta`.
    - `data_samples()` agrega `fixture`.
    - `backup.rs` ajusta `restore_keeps_fixtures_and_the_guion_copy` para comprobar `result.missing_files == 0`.
  - TS:
    - `contract.test.ts` agrega `["fixture", fixtureSchema]`.
    - `data.test.ts` prueba `missingFilesMessage(1)`, `missingFilesMessage(3)` y `restoreMessage` con `missingImages: 0, missingFiles: 2`.
    - `data-slice.test.ts`: un paquete con `missingFiles: 1` muestra el aviso.

- [ ] **Step 2: Run tests to verify they fail** — `cargo test dto commands backup` y `npx vitest run src/infrastructure/tauri/contract.test.ts src/domain/data.test.ts src/ui/store/data-slice.test.ts` → FAIL.

- [ ] **Step 3: Implement.** Registrar los 5 comandos en `names.rs`, `generate_handler!` y `main.json` (`allow-fixture-add`, …). Regenerar el fixture con `$env:UPDATE_CONTRACTS=1; cargo test contract; Remove-Item Env:UPDATE_CONTRACTS`.

- [ ] **Step 4: Run tests to verify they pass** — `cargo test` (todo) y `npx vitest run` (toda la suite), `npm run typecheck`, `npm run lint`.

- [ ] **Step 5: Lint y commit** — `feat(g2): comandos de archivos de prueba y su contrato`

---

### Task 6: Puerto, adaptadores y estado (G2-T4, parte 1)

**Files:**
- Create: `src/domain/fixtures.ts`, `src/domain/fixtures.test.ts`, `src/infrastructure/memory/fixtures.ts`, `src/infrastructure/tauri/fixtures.ts`, `src/ui/store/fixture-slice.ts`, `src/ui/store/fixture-slice.test.ts`
- Modify: `src/ports/repositories.ts`, `src/infrastructure/memory/repositories.ts`, `src/infrastructure/tauri/repositories.ts`, `src/ui/store/workspace.ts`

**Interfaces:**
- Consumes: comandos y `fixtureSchema` (Task 5); `encodeFrame` de los adaptadores Tauri.
- Produces:
  - `domain/fixtures.ts`:
    - `Fixture { id; caseId; fileName; size; sha256; note; position; addedAt }`.
    - Constantes `FIXTURE_EXTENSIONS` (las 8 de D12) y `MAX_FIXTURE_BYTES`.
    - `fixtureProblem(name: string, size: number): string | null`, que devuelve el mensaje fijo de extensión o de tamaño y se usa antes de leer los bytes.
    - `FIXTURE_ACCEPT = ".xlsx,.xls,.csv,.txt,.pdf,.json,.xml,.zip"`.
  - Puerto `FixtureStore { list(caseId): Promise<Fixture[]>; add(caseId: string, fileName: string, bytes: Uint8Array): Promise<Fixture>; remove(id): Promise<void>; reveal(id): Promise<void>; copyPath(id): Promise<void> }` y `Repositories.fixtures`.
  - `MemoryFixtureStore`: aplica las mismas reglas que Rust (extensión, 25 MB, nombre repetido, 50 por caso) y registra `revealed: string[]` y `copied: string[]`.
  - `GuionSlice` no cambia. `FixtureSlice`:
    - Estado `fixtures: Fixture[]` y `fixturesCaseId: string | null`.
    - `loadFixtures(caseId): Promise<void>`. Descarta la respuesta si en el intermedio cambió el caso.
    - `addFixtures(caseId, files: File[]): Promise<void>`. Por cada archivo: si `fixtureProblem` da un error, lo muestra como toast y sigue con el siguiente; si no, lee los bytes y llama a `add`. Al final recarga.
    - `removeFixture(id)`, `revealFixture(id)` y `copyFixturePath(id)`. Este último muestra el toast de éxito `Ruta copiada.`.
    - Los errores de Rust van a `h.report`.

- [ ] **Step 1: Write the failing tests**
  - `fixtures.test.ts`: `fixtureProblem("a.exe", 1)` da el mensaje de extensión, `fixtureProblem("a.csv", 25 * 1024 * 1024 + 1)` el de tamaño y `fixtureProblem("A.XLSX", 0)` da `null`.
  - `fixture-slice.test.ts`, con los repositorios en memoria:
    - Agregar dos archivos los lista en orden. Un `.exe` en el mismo lote da un toast y no detiene los demás.
    - Quitar uno lo saca de la lista.
    - `copyFixturePath` registra el id y muestra `Ruta copiada.`.
    - Si se cambia de caso mientras se carga, no se mezclan las listas.

- [ ] **Step 2: Run tests to verify they fail** — `npx vitest run src/domain/fixtures.test.ts src/ui/store/fixture-slice.test.ts` → FAIL.

- [ ] **Step 3: Implement.**

- [ ] **Step 4: Run tests to verify they pass** — los dos archivos, más `npm run typecheck` y `npm run lint`.

- [ ] **Step 5: Commit** — `feat(g2): puerto y estado de archivos de prueba`

---

### Task 7: Sección «Archivos de prueba» y cierre de G2 (G2-T4, parte 2)

**Files:**
- Create: `src/ui/screens/FixturesSection.tsx`, `src/ui/screens/Fixtures.test.tsx`
- Modify: `src/ui/screens/CaseDetail.tsx`, `docs/superpowers/contracts/G-guion-a-evidencias.md`

La sección va en `CaseDetail`, entre los campos y `EvidenceGallery`. Textos fijos:
- Título «Archivos de prueba».
- Botón «Agregar…»: abre un `<input type="file" multiple accept={FIXTURE_ACCEPT}>` oculto.
- Sin archivos: «Sin archivos de prueba. Arrastra aquí o usa «Agregar…».».
- Cada fila: nombre, `formatSize(size)`, nota (si no está vacía) y los botones «Mostrar en carpeta», «Copiar ruta» y «Quitar».
- «Quitar» abre un `ConfirmDialog` con título «Quitar archivo de prueba», mensaje `Se quitará «<nombre>» de este caso.` y botón «Quitar».
- Al cambiar de caso, la sección llama a `loadFixtures`.
- La sección es su propia zona de soltar. En `onDragOver` y `onDrop` llama a `e.stopPropagation()`, así la zona de capturas del detalle no recibe esos archivos.

- [ ] **Step 1: Write the failing tests** (`Fixtures.test.tsx`, con `renderApp` de `ui/test-utils` y repositorios en memoria)
  - `agrega con «Agregar…» y muestra nombre y tamaño`.
  - `dropping_on_the_section_adds_a_fixture_not_an_evidence`: soltar un `.xlsx` sobre la sección deja 1 archivo de prueba y 0 capturas, sin el toast de capturas.
  - `un tipo no aceptado muestra el mensaje fijo`.
  - `«Copiar ruta» y «Mostrar en carpeta» llaman al puerto`.
  - `«Quitar» pide confirmación y lo saca de la lista`.

- [ ] **Step 2: Run tests to verify they fail** — `npx vitest run src/ui/screens/Fixtures.test.tsx` → FAIL.

- [ ] **Step 3: Implement** la sección y su lugar en `CaseDetail`. Debe mantener la legibilidad del resto de las pantallas (líneas de 150 caracteres como máximo).

- [ ] **Step 4: Run tests to verify they pass** — el archivo, más `npx vitest run` (toda la suite), `npm run typecheck`, `npm run lint` y, desde `src-tauri/`, `cargo test`, `cargo fmt --check` y `cargo clippy --all-targets -- -D warnings`.

- [ ] **Step 5: Prueba manual (la pide el implementador a una persona)** con `npm run tauri dev` sobre un proyecto importado del guion sintético:

| # | Comprobación | Esperado |
|---|---|---|
| 1 | Agregar un `.xlsx`, un `.csv` y un `.txt` a CP01 («Agregar…» y arrastrar) | Aparecen con nombre y tamaño; un `.exe` muestra el mensaje fijo |
| 2 | «Mostrar en carpeta» y «Copiar ruta» | Se abre el Explorador con el archivo marcado; la ruta pegada termina en el nombre original |
| 3 | Exportar el paquete e importarlo | Proyecto «(importado)» con CP01–CP36 y los archivos en CP01 |
| 4 | Respaldo manual, quitar un archivo, restaurar | El archivo vuelve a CP01 y la copia del guion sigue presente |

- [ ] **Step 6: Actualizar el contrato y commit.**
  - Marca G2-T1 a G2-T4 y el criterio G2 cuando la prueba manual pase.
  - Completa «Interfaces producidas» con G2.
  - Ajusta el texto de G2-T1 según P3.
  - Commit `feat(g2): sección de archivos de prueba en el detalle del caso`.

---

## Fuera de este plan

- Exportar al formato de origen y la hoja «Evidencias» (G4). G4 recibe la copia del guion intacta y `missing_files` para avisar si falta.
- Editar la nota de un archivo de prueba (P8), reordenarlo o moverlo a otro caso.
- Herramientas MCP para archivos de prueba (spec A5).
- Guardar una sola vez un archivo repetido en varios casos (deduplicar por `sha256`).
