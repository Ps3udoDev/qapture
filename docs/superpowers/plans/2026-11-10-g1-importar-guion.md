# G · Bloque 1 — Importar guion de pruebas (G1) · Implementation Plan

> **Seguimiento de ejecución (2026-11-11):** T1–T7 completadas y revisadas; T8 tiene código y verificación automática completos. La prueba de escritorio y el cierre del criterio G1 siguen pendientes de resultado humano. Ver [handoffs G1](../handoffs/g1/README.md) y [revisión final](../handoffs/g1/revision-final.md).

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** El tester elige el `.xlsx` del guion del cliente en «Datos › Importar guion de pruebas…», mapea sus columnas en un asistente de 3 pasos y obtiene un proyecto con un caso por fila, la copia del archivo guardada y, si quiere, un perfil reutilizable para el siguiente guion.

**Architecture:** `transfer/guion/` lee el libro con `calamine` (`read.rs`), aplica un perfil JSON versionado (`profile.rs`) y arma una vista previa con avisos y errores (`import.rs`). La importación reutiliza el camino transaccional de `transfer/bundle.rs`, que ahora también escribe `project_source` y `case_source` y guarda la copia del `.xlsx` en `evidence/<proyecto>/guion.xlsx`. El archivo elegido queda en memoria en `AppState` con un token entre los tres comandos del asistente: el webview nunca manda rutas.

**Tech Stack:** Rust 2024, `calamine` 0.36 (pasa a producción, sin features), `rusqlite`, `sha2`, `serde`; React 19, Zustand, `zod`, Vitest + Testing Library.

**Spec:** [`2026-11-10-guion-a-evidencias-design.md`](../specs/2026-11-10-guion-a-evidencias-design.md) (§3 D1–D5 y D8, §5, §6 «Importar guion», §7) · **Contrato:** [`G-guion-a-evidencias`](../contracts/G-guion-a-evidencias.md) (G1-T1 a G1-T8) · **Base:** bloque 0 cerrado con go ([pico-trasplante](../handoffs/g0/pico-trasplante.md)).

## Global Constraints

- Rama `feat/g1-importar-guion`, creada desde `main`. Un PR al terminar G1; G2 y G4 van en sus propias ramas.
- Única dependencia que cambia: `calamine = "0.36.1"` pasa de `[dev-dependencies]` a `[dependencies]`, sin features. El `[patch.crates-io]` de `zip8` se queda. Nada más en `Cargo.toml` ni en `package.json`.
- Ningún archivo real de un cliente en el repo: las pruebas usan `excel::guion_fixture`.
- Nada específico de un cliente en el código (anti-objetivo del spec §2): fila 15, nombres de columna y vocabulario de estados viven en el perfil o en las sugerencias genéricas del asistente.
- Mensajes fijos del contrato, idénticos:
  - `El archivo no es un libro de Excel (.xlsx) válido.`
  - `El archivo está protegido con contraseña. Quita la protección y vuelve a intentar.`
  - `No se encontró la fila de encabezados. Elígela a mano.`
  - `Falta asignar la columna del código del caso.`
  - `Hay códigos repetidos: <lista>.` · `Hay filas sin código: <filas>.`
  - `«<valor>» no es un estado del perfil; el caso entra como «No ejecutado».`
- Mensajes nuevos de este plan (también fijos): `Vuelve a elegir el archivo del guion.` · `El guion no tiene la hoja «<hoja>».` · `El guion no tiene casos debajo de la fila de encabezados.` · `El guion supera 10000 casos.` · `El archivo supera 50 MB.` · `Ya existe un perfil llamado «<nombre>».` · `El perfil no existe.`
- Límites: archivo ≤ 50 MB; ≤ 10 000 casos; vista previa de hojas con las primeras 200 filas y 52 columnas.
- Estados de Qapture sin cambios (`domain::status::STATUSES`); «Condicionado» es el campo booleano `condicionado` (D5).
- Verificación por task, desde `src-tauri/`: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test <filtro>`. Desde la raíz, en las tasks de TS: `npm run lint`, `npm run typecheck`, `npx vitest run <archivo>`.
- Commits `feat(g1): …`, `test(g1): …`, `docs(g1): …`, terminados en `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Handoffs en `docs/superpowers/handoffs/g1/`: el controlador copia el `README.md` de `handoffs/g0/` (mismo formato y mismas reglas) antes de la Task 1. Cada implementador escribe `tarea-<N>.md`.

### Decisiones de este plan (no están en el spec)

| # | Decisión | Por qué |
|---|---|---|
| P1 | `project_source` agrega la columna `definition` (JSON del perfil usado) | G4 necesita roles, columnas, `statusMap` y carátula aunque el perfil no se haya guardado o se borre después (`profile_id` puede ser nulo) |
| P2 | La copia va en `evidence/<proyecto>/guion.xlsx`, y `repo::evidence::references` la incluye | Hereda gratis el borrado con el proyecto, la limpieza si falla la importación (`ImportFiles`) y la protección en `sweep` y `set_aside` |
| P3 | El archivo elegido vive en `AppState` (uno a la vez) y los comandos usan un token | El webview nunca manda rutas (mismo criterio que `reveal_export`) |
| P4 | Sin función ni comando para renombrar perfiles | No hay interfaz que lo use (YAGNI). La Task 8 corrige el texto de G1-T3 en el contrato |
| P5 | Tipos de campo que el perfil admite: `text`, `longtext`, `list`, `date` | Es lo que traen los guiones. «Condicionado» (`boolean`) lo crea el import, no el perfil |
| P6 | Título vacío → el código como título, con aviso. Título de más de 300 → se recorta, con aviso | El guion se importa sin editarlo (criterio de éxito 1) |

## Review Focus

- **Status con mayúsculas, acentos o espacios** (« ACEPTADO », «Condicionádo»): se pliegan antes de comparar (D4). Test `status_matching_ignores_case_accents_and_spaces` en la Task 2.
- **Guion que ya trae fechas como fecha de Excel o como texto `dd/mm/aaaa`**: las dos terminan en `AAAA-MM-DD`; lo demás queda vacío y se avisa. Test `dates_come_from_serials_and_day_first_text` en la Task 1 y `a_bad_date_is_left_empty_with_a_warning` en la Task 4.
- **Archivo `.xls` o cifrado con contraseña** (los dos son contenedores CFB): el cifrado da el mensaje de contraseña y el `.xls`, el de libro inválido. Test `rejects_cfb_files_with_the_right_message` en la Task 1.
- **Fila de datos sin código en medio del guion y filas vacías al final**: la primera bloquea con `Hay filas sin código: <filas>.`; las vacías no cuentan como casos ni como error. Test `empty_rows_are_skipped_and_rows_without_code_block` en la Task 4.
- **Fallo después de escribir la copia del guion** (p. ej. nombre de perfil repetido): no queda proyecto, ni perfil, ni `evidence/<proyecto>/guion.xlsx`. Test `a_failed_import_leaves_no_rows_no_profile_and_no_copy` en la Task 5.

---

### Task 1: Lector de guion (G1-T1)

**Files:**
- Modify: `src-tauri/Cargo.toml` (mover `calamine`)
- Create: `src-tauri/src/transfer/guion/mod.rs`, `src-tauri/src/transfer/guion/read.rs`
- Modify: `src-tauri/src/transfer/mod.rs` (`pub mod guion;` y doc del módulo)
- Modify: `src-tauri/src/excel/guion_fixture.rs` (variante editable del fixture)

**Interfaces:**
- Consumes: `excel::guion_fixture::{build, SHEET, HIDDEN_SHEET, HEADER_ROW, FIRST_CASE_ROW, CASES, HEADERS, COVER}` (G0).
- Produces:
  - `read::NOT_XLSX`, `read::PROTECTED` (los dos mensajes fijos), `read::MAX_FILE_BYTES: u64 = 50 * 1024 * 1024`.
  - `read::Cell { Empty, Text(String), Number(f64), Date(String /* AAAA-MM-DD */) }` con `Cell::text(&self) -> String` (número entero sin `.0`; fecha en ISO).
  - `read::GuionSheet { pub name: String, pub rows: Vec<Vec<Cell>>, pub merged: Vec<(u32, u32, u32, u32)> }` (índices 0-based; `merged` = `(fila_ini, col_ini, fila_fin, col_fin)`), con `cell(&self, row: u32, col: u32) -> &Cell` (fuera de rango → `Cell::Empty`).
  - `read::GuionBook { pub sheets: Vec<GuionSheet> }` (solo hojas visibles, en orden) y `GuionBook::sheet(&self, name: &str) -> Option<&GuionSheet>`.
  - `read::read(bytes: &[u8]) -> AppResult<GuionBook>`.
  - `read::suggest_header_row(sheet: &GuionSheet) -> Option<u32>` (1-based, como en Excel).
  - `read::column_letter(index: u32) -> String` (`0 → "A"`, `26 → "AA"`) y `read::column_index(letter: &str) -> Option<u32>` (solo `A`–`XFD`, mayúsculas).
  - `guion_fixture::Put { Text(&'static str), Date(u16, u8, u8) }` y `guion_fixture::build_edited(puts: &[(u32, u16, Put)]) -> Vec<u8>` (fila y columna 0-based; escribe sobre el fixture base). `build()` pasa a ser `build_edited(&[])`.

- [x] **Step 1: Mover `calamine` a `[dependencies]`**

Quitarla de `[dev-dependencies]` y agregar `calamine = "0.36.1"` en `[dependencies]`, junto a `rust_xlsxwriter`, con el comentario `# Lectura del guion del cliente (G1).` Run: `cargo test excel::guion_fixture` → 3 passed (el resto del árbol compila igual).

- [x] **Step 2: Write the failing tests** (`read.rs`, `mod tests`)

```rust
#[test]
fn lists_only_visible_sheets_with_their_cells() {
    let book = read(&guion_fixture::build()).unwrap();
    let names: Vec<&str> = book.sheets.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, [guion_fixture::SHEET]);
    let sheet = book.sheet(guion_fixture::SHEET).unwrap();
    assert_eq!(sheet.cell(guion_fixture::HEADER_ROW, 2).text(), "ID");
    assert_eq!(sheet.cell(guion_fixture::FIRST_CASE_ROW, 2).text(), "CP01");
    assert_eq!(sheet.cell(guion_fixture::FIRST_CASE_ROW, 9).text(), "1.0");
    assert_eq!(sheet.cell(9_999, 99), &Cell::Empty);
    assert!(sheet.merged.contains(&(0, 0, 0, 12)), "A1:M1");
}

#[test]
fn suggests_the_header_row_after_the_cover() {
    let book = read(&guion_fixture::build()).unwrap();
    assert_eq!(suggest_header_row(&book.sheets[0]), Some(15));
}

#[test]
fn dates_come_from_serials_and_day_first_text() {
    use guion_fixture::Put;
    let r = guion_fixture::FIRST_CASE_ROW;
    let bytes = guion_fixture::build_edited(&[
        (r, 8, Put::Date(2026, 11, 3)),
        (r + 1, 8, Put::Text("04/11/2026")),
        (r + 2, 8, Put::Text("2026-11-05")),
    ]);
    let book = read(&bytes).unwrap();
    let sheet = &book.sheets[0];
    assert_eq!(sheet.cell(r, 8), &Cell::Date("2026-11-03".into()));
    assert_eq!(sheet.cell(r + 1, 8).text(), "04/11/2026", "el texto sigue siendo texto");
    assert_eq!(parse_date(sheet.cell(r + 1, 8)), Some("2026-11-04".into()));
    assert_eq!(parse_date(sheet.cell(r + 2, 8)), Some("2026-11-05".into()));
    assert_eq!(parse_date(&Cell::Text("31/02/2026".into())), None);
    assert_eq!(parse_date(&Cell::Number(46_329.0)), Some("2026-11-03".into()));
}

#[test]
fn rejects_cfb_files_with_the_right_message() {
    let mut cfb = vec![0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
    cfb.extend_from_slice(&[0; 512]);
    assert_eq!(read(&cfb).unwrap_err().to_string(), NOT_XLSX, ".xls");
    let name: Vec<u8> = "EncryptedPackage".encode_utf16().flat_map(u16::to_le_bytes).collect();
    cfb.extend_from_slice(&name);
    assert_eq!(read(&cfb).unwrap_err().to_string(), PROTECTED);
    assert_eq!(read(b"hola").unwrap_err().to_string(), NOT_XLSX);
}

#[test]
fn column_letters_round_trip() {
    for (i, l) in [(0, "A"), (12, "M"), (25, "Z"), (26, "AA"), (16_383, "XFD")] {
        assert_eq!(column_letter(i), l);
        assert_eq!(column_index(l), Some(i));
    }
    for bad in ["", "a", "XFE", "A1", "ÁB"] {
        assert_eq!(column_index(bad), None, "{bad}");
    }
}
```

También `read::parse_date(cell: &Cell) -> Option<String>` en las Interfaces: `Date` tal cual; `Text` con `AAAA-MM-DD` o `dd/mm/aaaa` válidos; `Number` entre 1 y 2 958 465 como número de serie de Excel (base 1899-12-30); el resto, `None`.

- [x] **Step 3: Run tests to verify they fail**

Run: `cargo test transfer::guion::read`
Expected: FAIL de compilación (`read`, `Cell`, `build_edited` no existen).

- [x] **Step 4: Implement**

- `read`: si los bytes empiezan con la firma CFB `D0 CF 11 E0 A1 B1 1A E1` → `PROTECTED` si contienen `EncryptedPackage` en UTF-16LE; si no, `NOT_XLSX`. Si no, `calamine::open_workbook_from_rs::<Xlsx<_>, _>(Cursor::new(bytes))`; cualquier error de calamine → `NOT_XLSX`. Hojas con `SheetVisible::Visible` de `sheets_metadata()`. `load_merged_regions()` antes de leer; `merged_regions_by_sheet`.
- Conversión de `calamine::Data`: `String` → `Text`; `Float`/`Int` → `Number`; `Bool` → `Text("true"|"false")`; `DateTime(d)` → `Date` con `d.to_ymd_hms_milli()` (sin feature `chrono` de calamine); `DateTimeIso(s)` → `Date` si sus 10 primeros caracteres pasan `is_iso_date`, si no `Text(s)`; `Error`/`Empty`/`DurationIso` → `Empty`. Los textos solo de espacios → `Empty`.
- `rows` cubre desde la fila 0 hasta la última usada del rango (`Range::end`), aunque el rango empiece más abajo.
- `suggest_header_row`: la primera fila (de las 200 primeras) con al menos 4 celdas `Text` seguidas y alguna celda no vacía en la fila siguiente.
- `guion_fixture::build_edited`: el contenido de `build()` y luego cada `Put` (`Date` con `ExcelDateTime::from_ymd` y formato `dd/mm/yyyy`).

- [x] **Step 5: Run tests to verify they pass**

Run: `cargo test transfer::guion::read` → 5 passed. `cargo test excel::` → todo verde (el fixture no cambió).

- [x] **Step 6: Lint y commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/transfer src-tauri/src/excel/guion_fixture.rs
git commit -m "feat(g1): leer el guion del cliente con calamine"
```

---

### Task 2: Perfil de importación (G1-T2)

**Files:**
- Create: `src-tauri/src/transfer/guion/profile.rs`
- Modify: `src-tauri/src/domain/text.rs` (`fold`)

**Interfaces:**
- Consumes: `read::{column_index, Cell}` (Task 1), `domain::fields::FieldType`, `domain::status::STATUSES`.
- Produces:
  - `domain::text::fold(value: &str) -> String`: minúsculas, sin acentos (mismo mapa que `slugify`, que pasa a usarlo), espacios internos colapsados a uno, recortado.
  - Tipos serde (`camelCase`, `deny_unknown_fields`, `Serialize + Deserialize + Clone + PartialEq + Debug`) con la forma JSON del spec §5:
    - `ProfileDefinition { version: u32, sheet: String, header_row: u32, columns: Vec<ColumnMap>, status_map: StatusMap, cover: CoverCells }`
    - `ColumnMap { column: String, header: String, target: Target }`
    - `Target` con `#[serde(tag = "kind", rename_all = "camelCase")]`: `Code`, `Title`, `Status`, `Field { key: String, label: String, #[serde(rename = "type")] field_type: FieldType, #[serde(default)] options: Vec<String>, #[serde(default)] role: Option<FieldRole> }`
    - `FieldRole { ExecutionDate, Tester, Conditions }` (`camelCase`)
    - `StatusMap { import: BTreeMap<String, String>, export: BTreeMap<String, String> }`
    - `CoverCells { execution_date: Option<String>, responsible: Option<String>, observations: Option<String> }` (`#[serde(default)]` en cada campo)
  - `profile::CONDITIONED: &str = "Aprobado+condicionado"`, `CONDITIONED_KEY: &str = "condicionado"`, `CONDITIONED_LABEL: &str = "Condicionado"`, `NO_CODE: &str = "Falta asignar la columna del código del caso."`.
  - `profile::default_status_map() -> StatusMap`: import `aceptado`/`aprobado` → `Aprobado`, `rechazado` → `Fallido`, `condicionado` → `CONDITIONED`, `""` → `No ejecutado`; export `Aprobado` → `Aceptado`, `CONDITIONED` → `Condicionado`, `Fallido` → `Rechazado` (D4, D6).
  - `profile::problems(def: &ProfileDefinition) -> Vec<String>`: problemas de estructura, vacío si es válido (la vista previa los muestra como errores).
  - `profile::map_status(def: &ProfileDefinition, raw: &str) -> Option<(String, bool)>`: `(estado de Qapture, condicionado)`, comparando con `fold`; `None` si el valor no está en `status_map.import`.
  - `profile::uses_conditioned(def: &ProfileDefinition) -> bool`.
  - `profile::match_option(options: &[String], raw: &str) -> Option<String>`: la opción cuyo `fold` coincide.

- [x] **Step 1: Write the failing tests** (`profile.rs`, en un `#[cfg(test)] pub(crate) mod tests`, con `pub(crate) fn reference() -> ProfileDefinition`. Es el perfil del spec §5 completado a las 13 columnas del fixture: C código, F título, K estado y campos A, B, D, E, G, H — lista `["Crítico", "Importante"]` —, I — fecha, `executionDate` —, J, L — `tester` — y M — `longtext`, `conditions` —, con `default_status_map()` y la carátula `D5`/`D6`/`D8`. Las Tasks 3 a 5 lo reutilizan)

```rust
#[test]
fn the_spec_json_round_trips() {
    let json = serde_json::to_value(reference()).unwrap();
    assert_eq!(json["headerRow"], 15);
    assert_eq!(json["columns"][0]["target"]["kind"], "field");
    assert_eq!(json["statusMap"]["import"]["condicionado"], CONDITIONED);
    let back: ProfileDefinition = serde_json::from_value(json.clone()).unwrap();
    assert_eq!(back, reference());
    let mut extra = json;
    extra["otro"] = serde_json::json!(1);
    assert!(serde_json::from_value::<ProfileDefinition>(extra).is_err());
}

#[test]
fn status_matching_ignores_case_accents_and_spaces() {
    let def = reference();
    assert_eq!(map_status(&def, " ACEPTADO "), Some(("Aprobado".into(), false)));
    assert_eq!(map_status(&def, "Condicionádo"), Some(("Aprobado".into(), true)));
    assert_eq!(map_status(&def, "rechazado"), Some(("Fallido".into(), false)));
    assert_eq!(map_status(&def, "   "), Some(("No ejecutado".into(), false)));
    assert_eq!(map_status(&def, "Pendiente"), None);
    assert!(uses_conditioned(&def));
}

#[test]
fn list_values_match_without_accents() {
    let options = vec!["Crítico".to_string(), "Importante".to_string()];
    assert_eq!(match_option(&options, "Crìtico"), Some("Crítico".into()));
    assert_eq!(match_option(&options, "importante "), Some("Importante".into()));
    assert_eq!(match_option(&options, "Bajo"), None);
    assert_eq!(fold("  Crìtico   Alto "), "critico alto");
}

#[test]
fn structural_problems_are_listed() {
    assert!(problems(&reference()).is_empty());
    let mut def = reference();
    def.columns.retain(|c| c.target != Target::Code);
    assert_eq!(problems(&def), [NO_CODE]);
    let cases: [(fn(&mut ProfileDefinition), &str); 7] = [
        (|d| d.version = 2, "Versión de perfil no soportada."),
        (|d| d.columns[1].column = "a1".into(), "«a1» no es una columna válida."),
        (|d| d.columns[1].column = d.columns[0].column.clone(), "La columna «A» está asignada dos veces."),
        (|d| d.columns.push(d.columns.iter().find(|c| c.target == Target::Status).unwrap().clone().with_column("Z")), "Solo una columna puede ser el estado."),
        (|d| field_mut(d, "H").options.clear(), "«Criticidad» es una lista sin opciones."),
        (|d| field_mut(d, "L").role = Some(FieldRole::ExecutionDate), "El rol «Fecha de ejecución» solo va en una columna de fecha."),
        (|d| { d.status_map.import.insert("pendiente".into(), "Cerrado".into()); }, "«Cerrado» no es un estado de Qapture."),
    ];
    for (change, expected) in cases {
        let mut def = reference();
        change(&mut def);
        assert!(problems(&def).iter().any(|p| p == expected), "falta «{expected}» en {:?}", problems(&def));
    }
}
```

`with_column` (copia de un `ColumnMap` con otra letra) y `field_mut(def, letra)` son helpers del módulo de pruebas. `field_mut` devuelve un `FieldMut { options: &mut Vec<String>, role: &mut Option<FieldRole> }` del `Target::Field` de esa columna. Los demás problemas que `problems` también revisa, con estos textos: `La fila de encabezados debe ser 1 o mayor.`, `Solo una columna puede ser el código.` / `… el título.`, `La clave «<key>» se repite.`, `«<label>» necesita un nombre.`, `«condicionado» está reservado para la casilla Condicionado.` (si un campo usa `CONDITIONED_KEY` y el mapa usa `CONDITIONED`), `El rol «<rol>» está en dos columnas.`, `El tipo «<tipo>» no se puede importar.` (P5). Etiquetas de rol: «Fecha de ejecución», «Tester», «Condiciones».

- [x] **Step 2: Run tests to verify they fail**

Run: `cargo test transfer::guion::profile` → FAIL de compilación.

- [x] **Step 3: Implement** los tipos y funciones de las Interfaces. `fold` en `domain/text.rs` con su test (`fold("  Crìtico   Alto ") == "critico alto"`); `slugify` usa el mismo mapa de acentos, y sus pruebas siguen iguales.

- [x] **Step 4: Run tests to verify they pass**

Run: `cargo test transfer::guion::profile domain::text` → verde.

- [x] **Step 5: Lint y commit** — `feat(g1): perfil de importación del guion`

---

### Task 3: Migración 0007 y repositorios (G1-T3)

**Files:**
- Create: `src-tauri/src/db/migrations/0007_guion.sql`
- Modify: `src-tauri/src/db/mod.rs` (`MIGRATIONS`)
- Create: `src-tauri/src/repo/guion.rs`; Modify: `src-tauri/src/repo/mod.rs`
- Modify: `src-tauri/src/repo/evidence.rs` (`references`)
- Modify: `src-tauri/src/dto.rs` (`ImportProfileDto`)

**Interfaces:**
- Consumes: `profile::ProfileDefinition` (Task 2), `domain::text::required_text`.
- Produces:
  - Tablas del spec §5 más `project_source.definition TEXT NOT NULL` (P1). `profile_id` con `REFERENCES import_profile(id) ON DELETE SET NULL`. Índice `idx_case_fixture_case`.
  - `dto::ImportProfileDto { id, name, definition: ProfileDefinition, created_at, updated_at }` (`Serialize`, `camelCase`).
  - `repo::guion::create_profile(conn: &Connection, name: &str, def: &ProfileDefinition) -> AppResult<ImportProfileDto>`: nombre con `required_text("Nombre del perfil", name, 100)`; repetido sin distinguir mayúsculas → `Ya existe un perfil llamado «<nombre>».`
  - `repo::guion::list_profiles(conn) -> AppResult<Vec<ImportProfileDto>>`: por nombre sin distinguir mayúsculas. Una fila cuyo JSON ya no se puede leer se omite con `eprintln!`.
  - `repo::guion::delete_profile(conn, id: &str) -> AppResult<()>`: inexistente → `AppError::NotFound("El perfil no existe.")`.
  - `repo::guion::ProjectSource { file_name, stored_path, sha256, sheet, header_row: u32, profile_id: Option<String>, definition: ProfileDefinition, imported_at }` y `repo::guion::source(conn, project_id: &str) -> AppResult<Option<ProjectSource>>`.
  - `repo::guion::case_rows(conn, project_id: &str) -> AppResult<Vec<(String, u32)>>` (`case_id`, fila), en el orden de los casos.
  - `repo::evidence::references` agrega cada `project_source.stored_path` al conjunto de rutas referenciadas.

- [x] **Step 1: Write the failing tests** (`repo/guion.rs` y `repo/evidence.rs`)

```rust
#[test]
fn profiles_are_created_listed_and_deleted() {
    let conn = test_support::conn();
    let b = create_profile(&conn, "  cliente B · v1 ", &reference()).unwrap();
    create_profile(&conn, "Cliente A · guion v3", &reference()).unwrap();
    assert_eq!(b.name, "cliente B · v1");
    let names: Vec<String> = list_profiles(&conn).unwrap().into_iter().map(|p| p.name).collect();
    assert_eq!(names, ["Cliente A · guion v3", "cliente B · v1"]);
    assert_eq!(
        create_profile(&conn, "CLIENTE B · V1", &reference()).unwrap_err().to_string(),
        "Ya existe un perfil llamado «CLIENTE B · V1»."
    );
    delete_profile(&conn, &b.id).unwrap();
    assert_eq!(delete_profile(&conn, &b.id).unwrap_err().to_string(), "El perfil no existe.");
    assert_eq!(list_profiles(&conn).unwrap().len(), 1);
}

#[test]
fn deleting_a_profile_keeps_the_project_source() { /* inserta project_source con profile_id = perfil; borra el perfil; source(...).profile_id == None y definition intacta */ }

#[test]
fn source_rows_go_away_with_their_project_and_cases() { /* borrar un caso quita su case_source; borrar el proyecto quita project_source */ }
```

En `repo/evidence.rs`: `sweep_keeps_the_guion_copy`. Con un `project_source` cuyo `stored_path` es `evidence/<p>/guion.xlsx` y el archivo en disco, `sweep_orphans` devuelve 0 y el archivo sigue ahí. En `db/mod.rs`, `migrations_are_idempotent` también comprueba `SELECT project_id, definition FROM project_source`.

- [x] **Step 2: Run tests to verify they fail** — `cargo test repo::guion repo::evidence db::` → FAIL.

- [x] **Step 3: Implement** la migración, el DTO y el repositorio. Los tests de `case_rows` y `source` insertan las filas con SQL directo; la escritura real llega en la Task 5.

- [x] **Step 4: Run tests to verify they pass** — `cargo test repo:: db::` → verde.

- [x] **Step 5: Lint y commit** — `feat(g1): tablas y repositorios del origen del guion`

---

### Task 4: Vista previa (G1-T4)

**Files:**
- Create: `src-tauri/src/transfer/guion/import.rs` (parte de la vista previa)
- Modify: `src-tauri/src/transfer/guion/read.rs` (`detect_cover`)
- Modify: `src-tauri/src/dto.rs` (DTO de vista previa)

**Interfaces:**
- Consumes: Tasks 1 y 2.
- Produces:
  - `read::detect_cover(sheet: &GuionSheet, header_row: u32) -> CoverCells`: busca, por encima de la fila de encabezados, las celdas cuyo `fold` sin `:` final es «fecha de la prueba», «responsable» u «observaciones». La celda de valor es la primera no vacía a su derecha en la misma fila; si no hay, el inicio de la primera combinación a su derecha; si tampoco hay, la celda de al lado. Se devuelve en notación `D5`.
  - `dto::GuionPreviewCaseDto { row: u32, code, title, status, conditioned: bool }`, `dto::GuionPreviewFieldDto { key, label, #[serde(rename = "type")] field_type: FieldType, options: Vec<String> }` y `dto::GuionPreviewDto { cases: Vec<GuionPreviewCaseDto>, fields: Vec<GuionPreviewFieldDto>, warnings: Vec<String>, errors: Vec<String> }` (`camelCase`).
  - `import::Plan { pub preview: GuionPreviewDto, pub cases: Vec<PlannedCase> }`, `import::PlannedCase { pub row: u32, pub code: String, pub title: String, pub status: String, pub values: BTreeMap<String, String> /* key → valor ya normalizado */ }`.
  - `import::plan(book: &GuionBook, def: &ProfileDefinition) -> Plan`. Nunca falla: todo lo que impide importar va a `preview.errors`.

Reglas de `plan`, en este orden:
1. `profile::problems(def)` a `errors`. Hoja inexistente → `El guion no tiene la hoja «<hoja>».` Si hay errores aquí, se devuelve sin casos.
2. Filas desde `header_row + 1` hasta la última usada. Una fila cuyas celdas mapeadas están todas vacías se omite; si hay alguna entre casos, se avisa `Se omitieron <n> filas vacías.`. Más de 10 000 casos → `El guion supera 10000 casos.`
3. Código vacío con otras celdas llenas → `Hay filas sin código: <filas 1-based separadas por ", ">.`. Repetidos → `Hay códigos repetidos: <códigos en orden de primera aparición>.`. Sin casos → `El guion no tiene casos debajo de la fila de encabezados.`. Código de más de 40 caracteres → `<fila>: el código supera 40 caracteres.`
4. Título (P6): vacío → el código, con aviso `<código>: sin título; se usa el código.`; más de 300 caracteres → recortado, con aviso `<código>: el título se recortó a 300 caracteres.`
5. Estado: `map_status`; si da `None` → `No ejecutado`, aviso `<código>: «<valor>» no es un estado del perfil; el caso entra como «No ejecutado».` y el valor original se agrega como línea `Estado original: <valor>` al campo con rol `Conditions`, si lo hay. Condicionado → `values["condicionado"] = "true"`.
6. Campos: `list` → `match_option`; sin coincidencia → la opción se agrega a la lista del campo (una vez) y se avisa `«<valor>» no estaba en la lista de «<label>»; se agregó como opción.`. `date` → `read::parse_date`; si da `None` con la celda llena, el valor queda vacío y se avisa `<código>: «<valor>» no es una fecha en «<label>»; se deja vacía.`. `text` con algún valor de más de 500 caracteres → el campo pasa a `longtext` (como `legacy::columns_for`). Valor de más de 16 000 → error `<código>: «<label>» supera 16000 caracteres.`
7. `fields`: los campos del perfil en el orden de sus columnas y, si `uses_conditioned`, al final `condicionado` / `Condicionado` / `boolean`.

- [x] **Step 1: Write the failing tests** (`import.rs`)

```rust
#[test]
fn the_reference_guion_gives_36_cases_and_the_13_columns() {
    let book = read::read(&guion_fixture::build()).unwrap();
    let plan = plan(&book, &profile::tests::reference());
    assert!(plan.preview.errors.is_empty(), "{:?}", plan.preview.errors);
    assert_eq!(plan.cases.len(), 36);
    assert_eq!((plan.cases[0].row, plan.cases[0].code.as_str()), (16, "CP01"));
    assert_eq!(plan.cases[35].code, "CP36");
    assert_eq!(plan.cases[0].title, "Escenario 1");
    assert_eq!(plan.cases[0].status, "No ejecutado");
    let keys: Vec<&str> = plan.preview.fields.iter().map(|f| f.key.as_str()).collect();
    assert_eq!(keys.len(), 11, "10 columnas como campos + Condicionado: {keys:?}");
    assert_eq!(keys.last(), Some(&"condicionado"));
    assert_eq!(plan.cases[2].values["criticidad"], "Crítico");
}

#[test]
fn statuses_and_unknown_values() {
    use guion_fixture::Put;
    let r = guion_fixture::FIRST_CASE_ROW;
    let bytes = guion_fixture::build_edited(&[
        (r, 10, Put::Text("Aceptado")),
        (r + 1, 10, Put::Text("Condicionado")),
        (r + 2, 10, Put::Text("Pendiente")),
    ]);
    let plan = plan(&read::read(&bytes).unwrap(), &profile::tests::reference());
    assert_eq!(plan.cases[0].status, "Aprobado");
    assert_eq!(plan.cases[1].values["condicionado"], "true");
    assert_eq!(plan.cases[2].status, "No ejecutado");
    assert!(plan.cases[2].values["condiciones"].ends_with("Estado original: Pendiente"));
    assert!(plan.preview.warnings.contains(&"CP03: «Pendiente» no es un estado del perfil; el caso entra como «No ejecutado».".to_string()));
}

#[test]
fn empty_rows_are_skipped_and_rows_without_code_block() { /* vaciar entera la fila de CP05 → sin error y con el aviso «Se omitió 1 fila vacía.»; borrar solo el código de CP07 → errors == ["Hay filas sin código: 22."]; las filas vacías después de CP36 no avisan */ }

#[test]
fn repeated_codes_and_missing_code_column_block() { /* CP02 → "CP01" da "Hay códigos repetidos: CP01."; perfil sin columna de código da [NO_CODE] y ningún caso */ }

#[test]
fn a_bad_date_is_left_empty_with_a_warning() { /* I16 = "mañana" → values sin "fecha_prueba" y el aviso exacto; I17 fecha de Excel → "2026-11-03" */ }

#[test]
fn unknown_list_values_become_options() { /* H16 = "Bajo" → options de criticidad termina en "Bajo" y un solo aviso aunque se repita */ }

#[test]
fn detects_the_cover_cells() {
    let book = read::read(&guion_fixture::build()).unwrap();
    let cover = read::detect_cover(&book.sheets[0], 15);
    assert_eq!(cover.execution_date.as_deref(), Some("D5"));
    assert_eq!(cover.responsible.as_deref(), Some("D6"));
    assert_eq!(cover.observations.as_deref(), Some("D8"));
}
```

El aviso de filas vacías tiene singular y plural: `Se omitió 1 fila vacía.` / `Se omitieron <n> filas vacías.`

- [x] **Step 2: Run tests to verify they fail** — `cargo test transfer::guion::import` → FAIL.

- [x] **Step 3: Implement** `plan` y `detect_cover`. Para vaciar celdas en las pruebas, `guion_fixture::Put` suma la variante `Blank`.

- [x] **Step 4: Run tests to verify they pass** — `cargo test transfer::guion` → verde.

- [x] **Step 5: Lint y commit** — `feat(g1): vista previa del guion con avisos y errores`

---

### Task 5: Importar todo o nada (G1-T5)

**Files:**
- Modify: `src-tauri/src/transfer/bundle.rs` (origen del guion y `insert_in`)
- Modify: `src-tauri/src/transfer/legacy.rs`, `src-tauri/src/transfer/package.rs` (`source_row: None`, `source: None`)
- Modify: `src-tauri/src/transfer/guion/import.rs` (`import`)

**Interfaces:**
- Consumes: Tasks 3 y 4, `bundle::{ImportFiles, ProjectBundle, BundleCase, BundleField, insert}`.
- Produces:
  - `BundleCase.source_row: Option<u32>` y `ProjectBundle.source: Option<BundleSource>`, con `BundleSource { file_name, stored_path, sha256, sheet, header_row: u32, profile_id: Option<String>, definition: String }`. `insert_case` escribe `case_source` cuando hay fila, e `insert_project` escribe `project_source` cuando hay origen.
  - `bundle::insert_in(conn: &Connection, bundles: &[ProjectBundle]) -> AppResult<ImportSummaryDto>`: todo lo que hoy hace `insert` dentro de la transacción, sin abrirla ni confirmarla. `insert` la usa y su comportamiento no cambia.
  - `ImportFiles::write_file(&mut self, project_id: &str, name: &str, bytes: &[u8]) -> AppResult<String>`: escribe `evidence/<proyecto>/<name>` con `write_atomic` y devuelve la ruta relativa.
  - `import::GUION_FILE: &str = "guion.xlsx"`.
  - `import::import(conn: &Connection, files: &EvidenceFiles, request: ImportRequest) -> AppResult<ImportSummaryDto>` con `ImportRequest<'a> { book: &'a GuionBook, bytes: &'a [u8], file_name: &'a str, def: &'a ProfileDefinition, project_name: &'a str, save_profile_as: Option<&'a str> }`.

Comportamiento de `import`:
- `plan`; con `errors` → `Err` con el formato de `bundle::Problems` (`IMPORT_FAILED` y viñetas).
- Proyecto: nombre `required_text("Nombre del proyecto", …, 150)` y `unique_name`; `test_type`, `client` y fechas vacíos; `next_sequence = 1`; `source_template_id = None`.
- Campos con ancho por tipo: `text` 24, `longtext` 44, `list` 14, `date` 15, `boolean` 12.
- Copia: `write_file(project_id, GUION_FILE, bytes)`; `sha256` en hex minúsculas (`sha2`).
- Una transacción: `create_profile` si `save_profile_as` (su id va a `BundleSource.profile_id`), `insert_in`, commit y `files.commit()`. Si algo falla, `ImportFiles` borra la carpeta al soltarse.

- [x] **Step 1: Write the failing tests** (`import.rs`)

```rust
#[test]
fn imports_the_reference_guion_with_its_source_and_copy() {
    let tmp = TempDir::new();
    let files = tmp.files();
    let conn = test_support::conn();
    let bytes = guion_fixture::build();
    let book = read::read(&bytes).unwrap();
    let def = profile::tests::reference();
    let summary = import(&conn, &files, request(&book, &bytes, &def, Some("Cliente A · v3"))).unwrap();
    assert_eq!(summary.cases, 36);
    let p = projects::get(&conn, &summary.projects[0].id).unwrap();
    assert_eq!(p.name, "guion_cliente");
    assert_eq!(p.fields.len(), 11);
    let list = cases::list(&conn, &p.id).unwrap();
    assert_eq!(list[0].code, "CP01");
    let rows = guion::case_rows(&conn, &p.id).unwrap();
    assert_eq!(rows[0], (list[0].id.clone(), 16));
    assert_eq!(rows[35].1, 51);
    let source = guion::source(&conn, &p.id).unwrap().unwrap();
    assert_eq!((source.sheet.as_str(), source.header_row), (guion_fixture::SHEET, 15));
    assert_eq!(source.definition, def);
    assert_eq!(fs::read(files.absolute(&source.stored_path)).unwrap(), bytes);
    assert_eq!(source.sha256.len(), 64);
    assert_eq!(guion::list_profiles(&conn).unwrap()[0].id, source.profile_id.unwrap());
    let next = cases::create(&conn, &p.id, &new_case()).unwrap();
    assert_eq!(next.code, "EJ-001", "los casos nuevos siguen la secuencia de Qapture");
}

#[test]
fn a_failed_import_leaves_no_rows_no_profile_and_no_copy() {
    // perfil «Cliente A · v3» ya existe → el import falla con «Ya existe un perfil llamado…»
    // y después: 0 proyectos, 1 perfil (el de antes), project_dirs(&files) == 0
}

#[test]
fn errors_in_the_preview_stop_the_import() {
    // CP02 → "CP01": Err cuyo texto empieza con IMPORT_FAILED y contiene "• Hay códigos repetidos: CP01."; nada en la base ni en disco
}
```

`request` arma un `ImportRequest` con `file_name = "guion_cliente.xlsx"` y `project_name = "guion_cliente"`. En `bundle.rs`, los tests existentes siguen verdes sin cambios de aserciones.

- [x] **Step 2: Run tests to verify they fail** — `cargo test transfer::guion::import transfer::bundle` → FAIL.

- [x] **Step 3: Implement** según las Interfaces.

- [x] **Step 4: Run tests to verify they pass** — `cargo test transfer::` → verde (legacy y package incluidos).

- [x] **Step 5: Lint y commit** — `feat(g1): importar el guion como proyecto, todo o nada`

---

### Task 6: Comandos y contrato (G1-T6)

**Files:**
- Create: `src-tauri/src/commands/guion.rs`; Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/commands/names.rs`, `src-tauri/src/lib.rs`, `src-tauri/capabilities/main.json`
- Modify: `src-tauri/src/state.rs` (archivo pendiente)
- Modify: `src-tauri/src/dto.rs` (DTO de inspección, input y muestras del contrato)
- Modify: `src/infrastructure/tauri/dto.ts`, `src/infrastructure/tauri/__fixtures__/contracts.json` (regenerado), `src/infrastructure/tauri/contract.test.ts`

**Interfaces:**
- Consumes: Tasks 1–5.
- Produces:
  - `state::PendingGuion { token: String, file_name: String, bytes: Vec<u8>, book: GuionBook }` y en `AppState`: `set_guion(&self, pending: PendingGuion)`, `with_guion<T>(&self, token: &str, run: impl FnOnce(&PendingGuion) -> AppResult<T>) -> AppResult<T>` (otro token o ninguno → `Vuelve a elegir el archivo del guion.`) y `clear_guion(&self)`.
  - `dto::GuionSheetDto { name, rows: Vec<Vec<String>> /* hasta 200 × 52, como texto */, suggested_header_row: Option<u32>, cover: CoverCells }`, `dto::GuionInspectDto { token, file_name, sheets: Vec<GuionSheetDto>, suggested_profile_id: Option<String> }`.
  - `dto::GuionImportInput { token, profile: ProfileDefinition, project_name: String, save_profile_as: Option<String> }` (`Deserialize`, `camelCase`, `deny_unknown_fields`).
  - Comandos `#[tauri::command(async)]`:
    - `guion_inspect(window, state) -> AppResult<Option<GuionInspectDto>>`: diálogo «Importar guion de pruebas» con el filtro «Libro de Excel» / `xlsx` (reutiliza `pick_file`, que pasa a `pub(crate)` en `commands/data.rs`); más de `MAX_FILE_BYTES` → `El archivo supera 50 MB.`; `read::read`; `set_guion`. `cover` se detecta con la fila sugerida (vacío si no hay). `suggested_profile_id`: el primer perfil de `list_profiles` cuya hoja existe y cuyos `header` coinciden con `fold` con las celdas de su `header_row`.
    - `guion_preview(state, token: String, profile: ProfileDefinition) -> AppResult<GuionPreviewDto>`.
    - `guion_import(state, input: GuionImportInput) -> AppResult<ImportSummaryDto>`: si sale bien, `clear_guion`.
    - `import_profiles_list(state) -> AppResult<Vec<ImportProfileDto>>` y `import_profile_delete(state, id: String) -> AppResult<()>`.
  - Muestras en `data_samples()`: `guionInspect`, `guionPreview`, `importProfile` e `inputs.guionImportInput`. Esquemas `zod` estrictos en `dto.ts`: `guionInspectSchema`, `guionPreviewSchema`, `importProfileSchema` y `profileDefinitionSchema` (con `target` como `z.discriminatedUnion("kind", …)`).

- [x] **Step 1: Write the failing tests**
  - Rust: `state.rs` → `a_pending_guion_is_only_reachable_with_its_token` (token correcto devuelve el nombre; otro token y tras `clear_guion` dan `Vuelve a elegir el archivo del guion.`). `dto.rs` → `inputs_deserialize_from_ts_shape` y `inputs_reject_unknown_fields` cubren `guionImportInput`, con `profile` incluido. `commands/guion.rs` → `suggests_a_profile_whose_headers_match` sobre una función pura `suggested_profile(book, profiles) -> Option<String>` (coincide con el fixture; con un encabezado cambiado, no).
  - TS: `contract.test.ts` agrega `["guionInspect", guionInspectSchema]`, `["guionPreview", guionPreviewSchema]` y `["importProfile", importProfileSchema]`. En «TS → Rust», `profileDefinitionSchema.parse(fixture.inputs.guionImportInput.profile)`, y las claves de un `GuionImportInput` de TS son exactamente las del fixture.

- [x] **Step 2: Run tests to verify they fail** — `cargo test commands::guion state dto` y `npx vitest run src/infrastructure/tauri/contract.test.ts` → FAIL.

- [x] **Step 3: Implement**, registrar los 5 comandos en `names.rs`, `generate_handler!` y `main.json` (`allow-guion-inspect`, …), y regenerar el fixture: `UPDATE_CONTRACTS=1 cargo test contract` (PowerShell: `$env:UPDATE_CONTRACTS=1; cargo test contract; Remove-Item Env:UPDATE_CONTRACTS`).

- [x] **Step 4: Run tests to verify they pass** — `cargo test` (todo, incluidos los de `names.rs`) y `npx vitest run src/infrastructure/tauri/contract.test.ts` → verde.

- [x] **Step 5: Lint y commit** — `feat(g1): comandos del asistente de guion y su contrato`

---

### Task 7: Puerto y estado (G1-T7)

**Files:**
- Create: `src/domain/guion.ts` (+ `guion.test.ts`), `src/ports/guion.ts`
- Create: `src/infrastructure/tauri/guion.ts`, `src/infrastructure/memory/guion.ts`
- Modify: `src/ports/repositories.ts`, `src/infrastructure/tauri/repositories.ts`, `src/infrastructure/memory/repositories.ts`
- Modify: `src/application/use-cases.ts`
- Create: `src/ui/store/guion-slice.ts` (+ `guion-slice.test.ts`); Modify: `src/ui/store/workspace.ts`

**Interfaces:**
- Consumes: esquemas y comandos de la Task 6.
- Produces:
  - `domain/guion.ts`: tipos `ProfileDefinition`, `ColumnMap`, `Target`, `FieldRole`, `CoverCells`, `GuionSheet`, `GuionInspection`, `GuionPreview`, `ImportProfile` y `GuionImportInput`, con la misma forma que los DTO.
    - `QAPTURE_STATUS_TARGETS`: los 6 estados más `"Aprobado+condicionado"`, etiquetado «Aprobado + Condicionado».
    - `DEFAULT_STATUS_MAP` (igual que `profile::default_status_map`).
    - `fold(value: string): string` (mismo resultado que el de Rust para «  Crìtico   Alto »).
    - `columnLetter(index: number): string`.
    - `suggestTarget(header: string, values: string[]): Target | null`. Por el `fold` del encabezado: `id`, `codigo`, `cp` → `code`; contiene `escenario`, `titulo` o `caso de prueba` → `title`; `status`, `estado` → `status`. Si no, un campo nuevo con `key = slug(header)` y tipo según los valores: `date` si el encabezado contiene `fecha`; `longtext` si algún valor tiene salto de línea o más de 500 caracteres; `list` si hay entre 2 y 6 valores distintos en al menos 10 filas; si no, `text`. Rol: `fecha` + `date` → `executionDate`; `tester` → `tester`; `condicion` → `conditions`. Encabezado vacío → `null` («No importar»).
    - `buildProfile(sheet: GuionSheet, headerRow: number, targets: (Target | null)[], statusImport: Record<string, string>, cover: CoverCells): ProfileDefinition` (`export` = `DEFAULT_STATUS_MAP.export`).
    - `statusValues(sheet, headerRow, column): string[]`: valores distintos de la columna de estado (con su texto original), para el paso 2.
  - `ports/guion.ts`: `interface GuionImporter { inspect(): Promise<GuionInspection | null>; preview(token: string, profile: ProfileDefinition): Promise<GuionPreview>; import(input: GuionImportInput): Promise<ImportSummary>; listProfiles(): Promise<ImportProfile[]>; deleteProfile(id: string): Promise<void>; }` y `Repositories.guion: GuionImporter`.
  - Memoria: `createMemoryGuion(deps: { importProject(name: string, cases: number): Promise<ImportedProject> })`. Devuelve una inspección de ejemplo con la forma del fixture (hoja «Guión de Pruebas», encabezados en la fila 15, 3 casos); la vista previa aplica las reglas de errores de código de la Task 4 (vacíos y repetidos); guarda perfiles en memoria con el nombre único. Tiene `mode: "ok" | "cancel"` y `calls: string[]`, como `MemoryData`.
  - Casos de uso: `importGuion(repos, input): Promise<Outcome<ImportSummary>>` (error de validación → `{ ok: false, errors: { _form } }`).
  - Slice `GuionSlice`: `guionWizard: { inspection: GuionInspection; preview: GuionPreview | null; profiles: ImportProfile[] } | null`, `openGuionImport(): Promise<void>` (exclusivo con `dataBusy`; cancelar el diálogo no abre nada), `previewGuion(profile): Promise<void>`, `importGuion(input): Promise<FormResult>` (si sale bien, cierra el asistente, recarga y selecciona el proyecto y muestra el toast `importSummaryMessage` y los `warnings`), `closeGuionImport(): void` y `deleteImportProfile(id): Promise<void>`.

- [x] **Step 1: Write the failing tests**
  - `guion.test.ts`: `suggestTarget` sobre los 13 `HEADERS` del fixture con 36 valores de ejemplo da `code` para «ID», `title` para «Escenario de Prueba», `status` para «Status», `list` para «Criticidad», `date` + `executionDate` para «Fecha de Prueba», `tester` para «Nombre del tester» y `conditions` para «Condiciones». `fold("  Crìtico   Alto ") === "critico alto"`. `columnLetter(26) === "AA"`. `buildProfile` arma un perfil cuyo JSON coincide con `fixture.inputs.guionImportInput.profile` en la forma (claves).
  - `guion-slice.test.ts`: abrir, previsualizar e importar con el repositorio en memoria; tras importar, `project.name` es el importado y el asistente queda cerrado. Con `mode = "cancel"`, `guionWizard` sigue en `null`. Un error de validación al importar vuelve como `{ ok: false }` y el asistente sigue abierto.

- [x] **Step 2: Run tests to verify they fail** — `npx vitest run src/domain/guion.test.ts src/ui/store/guion-slice.test.ts` → FAIL.

- [x] **Step 3: Implement.**

- [x] **Step 4: Run tests to verify they pass** — los dos archivos, más `npm run typecheck` y `npm run lint`.

- [x] **Step 5: Commit** — `feat(g1): puerto, casos de uso y estado del import de guion`

---

### Task 8: Asistente «Importar guion de pruebas…» y cierre de G1 (G1-T8)

**Files:**
- Create: `src/ui/screens/ImportGuionDialog.tsx`, `src/ui/screens/ImportGuion.test.tsx`
- Modify: `src/ui/screens/DataMenu.tsx`, `src/ui/screens/AppShell.tsx`
- Modify: `docs/superpowers/contracts/G-guion-a-evidencias.md` (estado de G1-T1 a G1-T8, criterio de G1, texto de G1-T3 sin «renombrar» por P4, columna `definition` de P1)

**Interfaces:**
- Consumes: `GuionSlice` (Task 7), `Dialog`, `DialogFooter` y `btn` de `ui/components`.
- Produces: el ítem de menú `Importar guion de pruebas…` (debajo de «Importar paquete .qapture…») y el diálogo de 3 pasos del spec §6.

Contenido de cada paso (textos fijos entre comillas):
1. **«Archivo y hoja»**: nombre del archivo, un `select` «Hoja» con las hojas visibles y un `select` «Perfil» con «Sin perfil» más los perfiles guardados (preseleccionado `suggestedProfileId`). Cada perfil tiene un botón «Borrar perfil» con confirmación. Elegir un perfil va directo al paso 3 con su definición. Botón «Siguiente».
2. **«Columnas»**: campo numérico «Fila de encabezados» (precargado con `suggestedHeaderRow`; si es `null`, vacío y el aviso `No se encontró la fila de encabezados. Elígela a mano.`). Una fila por columna con encabezado: letra, encabezado, `select` de destino («Código», «Título», «Estado», «Campo nuevo», «No importar»), tipo («Texto», «Texto largo», «Lista», «Fecha») y rol para exportar («Ninguno», «Fecha de ejecución», «Tester», «Condiciones»), precargados con `suggestTarget`. Si hay columna de estado, la tabla «Estados» lista cada valor de `statusValues` con un `select` de `QAPTURE_STATUS_TARGETS`. Si se detectó la carátula, la línea «Carátula: fecha en <celda> · responsable en <celda> · observaciones en <celda>» y la casilla «Usar estas celdas» (marcada). Botón «Vista previa».
3. **«Vista previa»**: «<n> casos» y una tabla con fila, código, título y estado. Los errores en rojo y los avisos aparte. Campo «Nombre del proyecto» (el nombre del archivo sin `.xlsx`), casilla «Guardar como perfil» con su campo «Nombre del perfil». «Importar» queda deshabilitado si hay errores o si el nombre del proyecto está vacío. Los errores de validación del import se muestran en el diálogo.

- [x] **Step 1: Write the failing tests** (`ImportGuion.test.tsx`, con `renderWithStore` de `test-utils` y repositorios en memoria)
  - `el menú Datos abre el asistente y cancelar el archivo no abre nada`.
  - `recorre los tres pasos e importa`: elige la hoja, comprueba la fila 15 precargada y que «ID» tenga «Código», pasa a la vista previa («3 casos»), escribe el nombre del proyecto e importa. El proyecto queda abierto y el diálogo, cerrado.
  - `con códigos repetidos Importar queda deshabilitado y se ve el error` (`Hay códigos repetidos: CP01.`).
  - `sin columna de código la vista previa lo dice` (`Falta asignar la columna del código del caso.`).
  - `un perfil sugerido salta al paso 3 y se puede borrar`.

- [x] **Step 2: Run tests to verify they fail** — `npx vitest run src/ui/screens/ImportGuion.test.tsx` → FAIL.

- [x] **Step 3: Implement** el diálogo y el ítem de menú; `AppShell` monta `ImportGuionDialog` cuando `guionWizard` no es `null`.

- [x] **Step 4: Run tests to verify they pass** — el archivo, más `npx vitest run` (toda la suite), `npm run typecheck`, `npm run lint` y, desde `src-tauri/`, `cargo test`, `cargo fmt --check` y `cargo clippy --all-targets -- -D warnings`.

- [ ] **Step 5: Prueba manual (la pide el implementador a una persona)**

Con `npm run tauri dev`, importar `%TEMP%\g0\guion_sintetico.xlsx` (lo generó G0). Se espera: 36 casos CP01–CP36, las 13 columnas del guion (código, título y estado como datos del caso, 10 campos y «Condicionado»), «Criticidad» como lista y menos de un minuto. Después, guardar el perfil e importar el mismo archivo otra vez: el perfil se sugiere y entra sin mapear. Anotar el resultado en `handoffs/g1/tarea-8.md`.

- [ ] **Step 6: Actualizar el contrato y commit**

En `G-guion-a-evidencias.md`: G1-T1 a G1-T8 pasan a `✅` con la fecha, se marca el criterio **G1** y se ajusta G1-T3 (P4) y el modelo (P1).

```bash
git add src/ui/screens docs/superpowers/contracts/G-guion-a-evidencias.md docs/superpowers/handoffs/g1
git commit -m "feat(g1): asistente de 3 pasos para importar el guion"
```

## Fuera de este plan (para G2 y G4)

- **Paquetes `.qapture`:** `package.rs` valida códigos `EJ-NNN` (`is_case_code`), así que un proyecto importado de un guion (códigos `CP01`…) se exporta pero no se vuelve a importar como paquete. G2-T3 (paquete formato 2) tiene que aceptar códigos de guion y llevar `project_source`, `case_source` y la copia del guion.
- **Respaldos:** la base guarda `project_source`, y la copia del guion queda en `evidence/<proyecto>/`, igual que las capturas. G2-T3 decide cómo viajan los archivos en los respaldos.
- **G4** lee `project_source.definition` (roles, `statusMap.export`, `cover`) y `case_source.row`.
