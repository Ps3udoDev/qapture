# G · Bloque 0 — Fixture de guion y pico de trasplante · Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Tener un guion de prueba sintético con la forma del guion de referencia y probar que Qapture puede agregar una hoja «Evidencias» con imágenes a un libro ajeno sin dañarlo (decisión go / no-go de D10).

**Architecture:** Un helper de pruebas genera el guion sintético con `rust_xlsxwriter`. Un módulo nuevo `excel::origin` lee el `.xlsx` como lista ordenada de partes del ZIP, injerta la hoja de un libro «donante» de una sola hoja (generado con `rust_xlsxwriter`) y vuelve a escribir el ZIP sin tocar el resto de las partes. Las cadenas del donante pasan a texto en línea y sus estilos se quitan, para no tocar `sharedStrings.xml` ni `styles.xml` del cliente.

**Tech Stack:** Rust 2024, `zip` 9 (ya en producción), `rust_xlsxwriter` (ya en producción), `calamine` 0.36 (solo pruebas), `image` 0.25.

**Spec:** [`docs/superpowers/specs/2026-11-10-guion-a-evidencias-design.md`](../specs/2026-11-10-guion-a-evidencias-design.md) (D9, D10, §8) · **Contrato:** [`G-guion-a-evidencias`](../contracts/G-guion-a-evidencias.md) (G-T1, G-T2).

## Global Constraints

- Sin dependencias nuevas en `Cargo.toml` (no `regex`, no `quick-xml`): el XML se trata con búsqueda de texto, como `excel::testing`.
- `calamine` sigue siendo solo de pruebas en este bloque.
- **Nunca** se agrega al repo un archivo real del cliente. El fixture se genera en memoria dentro de los tests.
- Mensajes al usuario en español. Mensaje fijo de archivo inválido: `El archivo no es un libro de Excel (.xlsx) válido.`
- Verificación por tarea, desde `src-tauri/`: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test <filtro>`.
- Commits con el estilo del repo (`test(g0): …`, `feat(g0): …`, `docs(g0): …`) y terminados en la línea `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Rama: `feat/g0-fixture-y-pico`, creada desde `main`.

## Review Focus

- **Nombre de hoja con `&`, `<` o comillas** (p. ej. «Evidencias & anexos»): se escapa en `workbook.xml` y `calamine` lo lee tal cual. Test en la Task 3.
- **Dos pestañas seleccionadas**: si el donante trae `tabSelected="1"`, Excel abre el libro con las hojas agrupadas. El injerto lo quita. Test en la Task 3.
- **Libro que ya tiene imágenes PNG** (el logo del guion): no se duplica `<Default Extension="png">` ni se pisa `xl/media/image1.png` ni `xl/drawings/drawing1.xml`. Test en la Task 3.
- **Partes con numeración no consecutiva** (`sheet1.xml`, `sheet7.xml`): la hoja nueva toma un número libre. Test unitario de `next_free` en la Task 3.
- **Donante con relaciones distintas de dibujos** (vínculos externos, comentarios): se rechaza con un error claro en vez de producir un libro roto. Test en la Task 3.

---

### Task 1: Guion sintético de referencia (G-T1)

**Files:**
- Create: `src-tauri/src/excel/guion_fixture.rs`
- Modify: `src-tauri/src/excel/mod.rs` (declarar `#[cfg(test)] pub(crate) mod guion_fixture;` junto a `testing`)
- Modify: `src-tauri/src/excel/testing.rs` (agregar `Book::from_bytes`)

**Interfaces:**
- Consumes: `excel::testing::Book` (`at`, `sheet_names`, `text`, `row_texts`, `part`, `sheet_xml`, `pictures`), `media::storage::TempDir`.
- Produces (todo `#[cfg(test)]`, `pub(crate)`):
  - `guion_fixture::SHEET: &str = "Guión de Pruebas"`, `HIDDEN_SHEET: &str = "Hoja1"`
  - `guion_fixture::HEADER_ROW: u32 = 14` (índice 0; fila 15 de Excel), `FIRST_CASE_ROW: u32 = 15`, `CASES: u32 = 36`
  - `guion_fixture::HEADERS: [&str; 13] = ["Módulo", "SubMódulo", "ID", "Opción", "RN", "Escenario de Prueba", "Resultado Esperado", "Criticidad", "Fecha de Prueba", "Version", "Status", "Nombre del tester", "Condiciones"]`
  - `guion_fixture::COVER: [(u32, &str, &str); 6]` = filas (índice 0) 2–7 con etiqueta en B y valor en D: `(2, "Código:", "GP-001")`, `(3, "Formato:", "Guion de pruebas v3.0")`, `(4, "Fecha de la prueba:", "")`, `(5, "Responsable:", "Pendiente de asignación")`, `(6, "Versión:", "1.0")`, `(7, "Observaciones:", "Guion propuesto; pendiente de ejecución")`
  - `guion_fixture::build() -> Vec<u8>`: el `.xlsx` en memoria.
  - `testing::Book::from_bytes(bytes: &[u8]) -> Book`: guarda los bytes en un `TempDir` propio y devuelve el libro.

- [ ] **Step 1: Write the failing tests** (en `guion_fixture.rs`, `mod tests`)

```rust
#[test]
fn has_the_visible_sheet_and_the_hidden_list() {
    let book = Book::from_bytes(&build());
    assert_eq!(book.sheet_names(), [SHEET, HIDDEN_SHEET]);
    let wb = book.part("xl/workbook.xml");
    let hidden = &wb[wb.find(r#"name="Hoja1""#).unwrap()..];
    assert!(hidden[..hidden.find("/>").unwrap()].contains(r#"state="hidden""#));
    assert_eq!(book.row_texts(HIDDEN_SHEET, 0, 1), ["Aprobado"]);
}

#[test]
fn cover_headers_and_36_cases_are_where_the_reference_puts_them() {
    let book = Book::from_bytes(&build());
    assert_eq!(book.text(SHEET, 0, 0), "Guion de pruebas");
    for (row, label, value) in COVER {
        assert_eq!(book.text(SHEET, row, 1), label);
        assert_eq!(book.text(SHEET, row, 3), value);
    }
    assert_eq!(book.row_texts(SHEET, HEADER_ROW, 13), HEADERS);
    assert_eq!(book.text(SHEET, FIRST_CASE_ROW, 2), "CP01");
    assert_eq!(book.text(SHEET, FIRST_CASE_ROW + CASES - 1, 2), "CP36");
    assert_eq!(book.text(SHEET, FIRST_CASE_ROW, 8), ""); // Fecha de Prueba vacía
    assert_eq!(book.text(SHEET, FIRST_CASE_ROW, 10), ""); // Status vacío
    assert_eq!(book.text(SHEET, FIRST_CASE_ROW, 9), "1.0");
}

#[test]
fn keeps_the_format_details_that_g4_must_not_break() {
    let book = Book::from_bytes(&build());
    let xml = book.sheet_xml(SHEET);
    assert!(xml.contains(r#"<autoFilter ref="A15:M15"/>"#));
    assert!(xml.contains(r#"<mergeCell ref="A1:M1"/>"#));
    assert!(xml.contains(r#"<mergeCell ref="D3:M3"/>"#));
    assert!(xml.contains(r#"<dataValidations count="2">"#));
    assert!(xml.contains("Crìtico"));
    assert!(xml.contains(r#"sqref="K16:K35""#));
    assert!(book.part("xl/workbook.xml").contains("$A$1:$M$51"));
    assert_eq!(book.pictures(SHEET), 1);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test excel::guion_fixture`
Expected: FAIL de compilación (`build`, `Book::from_bytes` no existen).

- [ ] **Step 3: Implement `Book::from_bytes` y `guion_fixture::build() -> Vec<u8>`**

Contenido exacto del libro:
- Hoja `SHEET`, ancho de columnas A–M de 18. `A1:M1` combinada con «Guion de pruebas» en negrita; logo: un PNG de 120×40 generado en memoria (`image::RgbImage` codificado a PNG en un `Vec<u8>`) e insertado en `A1` con `Image::new_from_buffer`.
- Carátula: por cada `(row, label, value)` de `COVER`, `label` en la columna B y `value` en `D{row+1}:M{row+1}` combinada (también si `value` está vacío: se escribe la celda en blanco con `merge_range`).
- Fila `HEADER_ROW`: `HEADERS` en A–M, en negrita con relleno gris claro.
- Filas `FIRST_CASE_ROW..FIRST_CASE_ROW + CASES` (caso `n` = 1..=36): A «Sistema», B «Herramientas / Carga», C `CP{n:02}`, D `["Acceso", "Archivo", "Estructura", "Validación"][(n - 1) % 4]`, E `RN{((n - 1) % 15) + 1:02}`, F `Escenario {n}`, G `Resultado esperado {n}`, H «Crítico» si `n % 3 == 0`, si no «Importante», I vacío, J «1.0», K vacío, L vacío, M «Condiciones generales».
- Validaciones de lista solo en las filas 16–35 de Excel (índices 15–34): K con `["Aceptado", "Rechazado", "Condicionado"]` y H con `["Crìtico", "Importante"]` (acento grave a propósito).
- `autofilter(HEADER_ROW, 0, HEADER_ROW, 12)` y `set_print_area(0, 0, FIRST_CASE_ROW + CASES - 1, 12)`.
- Hoja `HIDDEN_SHEET` oculta con «Aprobado», «Rechazado» y «Condicionado» en A1:A3.
- Devolver `workbook.save_to_buffer()`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test excel::guion_fixture`
Expected: 3 passed.

- [ ] **Step 5: Lint y commit**

Run: `cargo fmt --check && cargo clippy --all-targets -- -D warnings`
```bash
git add src-tauri/src/excel/guion_fixture.rs src-tauri/src/excel/mod.rs src-tauri/src/excel/testing.rs
git commit -m "test(g0): guion sintético con la forma del guion de referencia"
```

---

### Task 2: Partes de un `.xlsx` en orden

**Files:**
- Create: `src-tauri/src/excel/origin/mod.rs` (`pub mod parts; pub mod evidence_graft;` — `evidence_graft` llega en la Task 3; en esta task solo `parts`)
- Create: `src-tauri/src/excel/origin/parts.rs`
- Modify: `src-tauri/src/excel/mod.rs` (`pub mod origin;`)

**Interfaces:**
- Consumes: `guion_fixture::build` (Task 1), `error::{AppError, AppResult, validation}`.
- Produces:
  - `pub const NOT_XLSX: &str = "El archivo no es un libro de Excel (.xlsx) válido."`
  - `pub struct XlsxParts` (orden original de las partes y método de compresión de cada una).
  - `XlsxParts::read(bytes: &[u8]) -> AppResult<XlsxParts>`: falla con `validation(NOT_XLSX)` si no es ZIP o si falta `xl/workbook.xml` o `[Content_Types].xml`.
  - `fn names(&self) -> Vec<&str>` en orden; `fn get(&self, name: &str) -> Option<&[u8]>`; `fn text(&self, name: &str) -> AppResult<String>` (UTF-8; `AppError::Export` si no existe).
  - `fn set(&mut self, name: &str, data: Vec<u8>)`: reemplaza conservando posición y método; si no existe, agrega al final con `Deflated`.
  - `fn write(&self) -> AppResult<Vec<u8>>`: ZIP en memoria con el mismo orden y método por parte.

- [ ] **Step 1: Write the failing tests** (en `parts.rs`)

```rust
#[test]
fn read_then_write_keeps_every_part_and_its_order() {
    let original = guion_fixture::build();
    let parts = XlsxParts::read(&original).unwrap();
    let again = XlsxParts::read(&parts.write().unwrap()).unwrap();
    assert_eq!(again.names(), parts.names());
    for name in parts.names() {
        assert_eq!(again.get(name), parts.get(name), "{name}");
    }
}

#[test]
fn set_replaces_in_place_or_appends() {
    let mut parts = XlsxParts::read(&guion_fixture::build()).unwrap();
    let before = parts.names().iter().map(|n| n.to_string()).collect::<Vec<_>>();
    parts.set("xl/workbook.xml", b"<x/>".to_vec());
    parts.set("xl/nueva.xml", b"<y/>".to_vec());
    let after = parts.names();
    let before: Vec<&str> = before.iter().map(String::as_str).collect();
    assert_eq!(&after[..before.len()], before.as_slice());
    assert_eq!(after.last(), Some(&"xl/nueva.xml"));
    assert_eq!(parts.text("xl/workbook.xml").unwrap(), "<x/>");
}

#[test]
fn rejects_files_that_are_not_xlsx() {
    let err = XlsxParts::read(b"no es un zip").unwrap_err();
    assert_eq!(err.to_string(), NOT_XLSX);
}
```

Agregar además `rejects_a_zip_without_workbook`: un ZIP creado con `zip::ZipWriter` que solo contiene `a.txt` → `NOT_XLSX`.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test excel::origin::parts`
Expected: FAIL de compilación (`XlsxParts` no existe).

- [ ] **Step 3: Implement `XlsxParts`**

Con `zip::ZipArchive` sobre `std::io::Cursor`; se leen todas las entradas en orden de índice guardando `compression()`; `write` usa `ZipWriter` sobre `Cursor<Vec<u8>>` con `SimpleFileOptions::default().compression_method(method)`. Los errores de escritura de `zip` se convierten en `AppError::Export`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test excel::origin::parts`
Expected: 4 passed.

- [ ] **Step 5: Lint y commit**

```bash
git add src-tauri/src/excel/origin src-tauri/src/excel/mod.rs
git commit -m "feat(g0): leer y reescribir un xlsx parte por parte sin alterarlo"
```

---

### Task 3: Trasplante de una hoja con imágenes (G-T2)

**Files:**
- Create: `src-tauri/src/excel/origin/evidence_graft.rs`
- Modify: `src-tauri/src/excel/origin/mod.rs` (`pub mod evidence_graft;`)

**Interfaces:**
- Consumes: `XlsxParts` (Task 2), `guion_fixture` (Task 1), `excel::testing::{Book, write_png}`.
- Produces:
  - `pub fn graft_sheet(base: &[u8], donor: &[u8], name: &str) -> AppResult<Vec<u8>>`: agrega como **última** hoja de `base` la única hoja de `donor`, con el nombre `name`.
  - `pub(crate) fn next_free(names: &[&str], prefix: &str, suffix: &str) -> u32`: el menor `n` mayor que todos los `prefix{n}suffix` existentes (1 si no hay).
  - Errores (`validation`): `Ya existe una hoja llamada «<name>».`; `El nombre de hoja no es válido.` (vacío, más de 31 caracteres o con `[ ] : * ? / \`); `El libro de evidencias debe tener una sola hoja.`; `El libro de evidencias tiene vínculos que no se pueden trasladar.`

Algoritmo (no lo determinan las firmas):

1. Leer `base` y `donor` con `XlsxParts::read`. Validar `name` y que no esté entre los `name="…"` de `<sheets>` de `base` (comparando el texto ya desescapado).
2. En el donante, la hoja es la única `xl/worksheets/sheet*.xml`; si hay más de una, error. Sus relaciones (`xl/worksheets/_rels/sheetK.xml.rels`, si existen) solo pueden ser de tipo `…/relationships/drawing`; si hay otra, error.
3. Números libres en `base`: `N = next_free(nombres, "xl/worksheets/sheet", ".xml")`, `D = next_free(nombres, "xl/drawings/drawing", ".xml")` y `M = next_free(nombres de xl/media/ sin su extensión, "xl/media/image", "")`. Cada imagen del donante se renombra a `xl/media/image{M}.{ext}`, `image{M+1}.{ext}`…
4. XML de la hoja del donante:
   a. Quitar los atributos ` s="<n>"`, ` style="<n>"`, ` customFormat="1"` y ` tabSelected="1"`.
   b. Cada celda `<c r="X" t="s"><v>i</v></c>` pasa a `<c r="X" t="inlineStr"><is>{contenido interno del i-ésimo <si> de xl/sharedStrings.xml del donante}</is></c>` (un `<si/>` vacío da `<is><t></t></is>`).
5. Copiar al `base`: la hoja como `xl/worksheets/sheet{N}.xml`; sus relaciones como `xl/worksheets/_rels/sheet{N}.xml.rels` con el destino `../drawings/drawing{D}.xml`; el dibujo como `xl/drawings/drawing{D}.xml`; sus relaciones como `xl/drawings/_rels/drawing{D}.xml.rels` con los destinos `../media/…` renombrados; las imágenes con sus nombres nuevos.
6. `xl/workbook.xml`: insertar `<sheet name="{name escapado}" sheetId="{máximo sheetId + 1}" r:id="{rid}"/>` justo antes de `</sheets>`. `rid` = `rIdQ{k}` con el menor `k` que no exista en `xl/_rels/workbook.xml.rels`.
7. `xl/_rels/workbook.xml.rels`: agregar `<Relationship Id="{rid}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet{N}.xml"/>` antes de `</Relationships>`.
8. `[Content_Types].xml`: agregar `Override` para `/xl/worksheets/sheet{N}.xml` (`application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml`) y `/xl/drawings/drawing{D}.xml` (`application/vnd.openxmlformats-officedocument.drawing+xml`), y un `Default` por cada extensión de imagen nueva que no esté ya (`png` → `image/png`, `jpeg`/`jpg` → `image/jpeg`).
9. No se toca ninguna otra parte (`docProps/app.xml` incluido). Devolver `parts.write()`.

- [ ] **Step 1: Write the failing tests** (en `evidence_graft.rs`)

Helper de test: `fn donor(images: usize) -> Vec<u8>`: libro de `rust_xlsxwriter` con una hoja «Donante»: `A1` = «Evidencias de pruebas» en negrita, `A2` = «CP05 — Adjuntar archivo no Excel», y `images` PNG de 300×200 (con `write_png` y `seed` distinto) insertados desde la fila 3, separados 12 filas. Devuelve `save_to_buffer()`.

```rust
#[test]
fn adds_the_sheet_last_with_its_text_and_images() {
    let out = graft_sheet(&guion_fixture::build(), &donor(2), "Evidencias").unwrap();
    let book = Book::from_bytes(&out);
    assert_eq!(book.sheet_names(), [guion_fixture::SHEET, guion_fixture::HIDDEN_SHEET, "Evidencias"]);
    assert_eq!(book.text("Evidencias", 0, 0), "Evidencias de pruebas");
    assert_eq!(book.text("Evidencias", 1, 0), "CP05 — Adjuntar archivo no Excel");
    assert_eq!(book.pictures("Evidencias"), 2);
    assert_eq!(book.pictures(guion_fixture::SHEET), 1); // el logo sigue
    assert_eq!(book.text(guion_fixture::SHEET, guion_fixture::FIRST_CASE_ROW, 2), "CP01");
}

#[test]
fn every_original_part_is_untouched_except_the_three_index_parts() {
    let base = guion_fixture::build();
    let before = XlsxParts::read(&base).unwrap();
    let after = XlsxParts::read(&graft_sheet(&base, &donor(2), "Evidencias").unwrap()).unwrap();
    let index = ["xl/workbook.xml", "xl/_rels/workbook.xml.rels", "[Content_Types].xml"];
    for name in before.names() {
        if !index.contains(&name) {
            assert_eq!(after.get(name), before.get(name), "{name}");
        }
    }
    let wb = after.text("xl/workbook.xml").unwrap();
    assert_eq!(wb.replace(&wb[wb.find("<sheet name=\"Evidencias\"").unwrap()..wb.find("</sheets>").unwrap()], ""),
               before.text("xl/workbook.xml").unwrap());
}

#[test]
fn new_parts_do_not_collide_and_are_registered_once() {
    let out = graft_sheet(&guion_fixture::build(), &donor(2), "Evidencias").unwrap();
    let parts = XlsxParts::read(&out).unwrap();
    assert!(parts.get("xl/worksheets/sheet3.xml").is_some());
    assert!(parts.get("xl/drawings/drawing2.xml").is_some());
    assert!(parts.get("xl/media/image2.png").is_some());
    assert!(parts.get("xl/media/image3.png").is_some());
    let ct = parts.text("[Content_Types].xml").unwrap();
    assert!(ct.contains(r#"PartName="/xl/worksheets/sheet3.xml""#));
    assert!(ct.contains(r#"PartName="/xl/drawings/drawing2.xml""#));
    assert_eq!(ct.matches(r#"Extension="png""#).count(), 1);
}

#[test]
fn the_grafted_sheet_has_no_shared_strings_styles_or_selected_tab() {
    let out = graft_sheet(&guion_fixture::build(), &donor(1), "Evidencias").unwrap();
    let xml = XlsxParts::read(&out).unwrap().text("xl/worksheets/sheet3.xml").unwrap();
    assert!(!xml.contains(r#"t="s""#));
    assert!(!xml.contains(" s=\""));
    assert!(!xml.contains("tabSelected"));
    assert!(xml.contains(r#"t="inlineStr""#));
}

#[test]
fn names_are_escaped_and_validated() {
    let base = guion_fixture::build();
    let out = graft_sheet(&base, &donor(1), "Evidencias & anexos").unwrap();
    assert!(XlsxParts::read(&out).unwrap().text("xl/workbook.xml").unwrap().contains(r#"name="Evidencias &amp; anexos""#));
    assert_eq!(Book::from_bytes(&out).sheet_names()[2], "Evidencias & anexos");
    assert_eq!(graft_sheet(&base, &donor(1), "Hoja1").unwrap_err().to_string(), "Ya existe una hoja llamada «Hoja1».");
    for bad in ["", "a/b", "[x]", &"x".repeat(32)] {
        assert_eq!(graft_sheet(&base, &donor(1), bad).unwrap_err().to_string(), "El nombre de hoja no es válido.");
    }
}

#[test]
fn rejects_donors_that_cannot_be_moved() {
    let base = guion_fixture::build();
    // dos hojas
    let mut two = rust_xlsxwriter::Workbook::new();
    two.add_worksheet();
    two.add_worksheet();
    assert_eq!(graft_sheet(&base, &two.save_to_buffer().unwrap(), "E").unwrap_err().to_string(),
               "El libro de evidencias debe tener una sola hoja.");
    // vínculo externo
    let mut linked = rust_xlsxwriter::Workbook::new();
    linked.add_worksheet().write_url(0, 0, "https://example.com").unwrap();
    assert_eq!(graft_sheet(&base, &linked.save_to_buffer().unwrap(), "E").unwrap_err().to_string(),
               "El libro de evidencias tiene vínculos que no se pueden trasladar.");
}

#[test]
fn next_free_skips_gaps_and_other_parts() {
    let names = ["xl/worksheets/sheet1.xml", "xl/worksheets/sheet7.xml", "xl/worksheets/_rels/sheet9.xml.rels"];
    assert_eq!(next_free(&names, "xl/worksheets/sheet", ".xml"), 8);
    assert_eq!(next_free(&[], "xl/drawings/drawing", ".xml"), 1);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test excel::origin::evidence_graft`
Expected: FAIL de compilación (`graft_sheet`, `next_free` no existen).

- [ ] **Step 3: Implement `graft_sheet` y `next_free` siguiendo el algoritmo de arriba**

El escape del nombre usa `&amp;`, `&lt;`, `&gt;`, `&quot;`, `&apos;`. Para leer los nombres existentes de `<sheets>` se desescapan las mismas cinco entidades.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test excel::origin`
Expected: 11 passed (4 de `parts` + 7 de `evidence_graft`).

- [ ] **Step 5: Lint y commit**

```bash
git add src-tauri/src/excel/origin
git commit -m "feat(g0): trasplantar una hoja con imágenes a un libro ajeno"
```

---

### Task 4: Verificación manual y decisión go / no-go

**Files:**
- Modify: `src-tauri/src/excel/origin/evidence_graft.rs` (test ignorado que escribe los archivos)
- Create: `docs/superpowers/handoffs/g0/pico-trasplante.md`
- Modify: `docs/superpowers/contracts/G-guion-a-evidencias.md` (estado de G-T1 y G-T2)

**Interfaces:**
- Consumes: `guion_fixture::build`, `graft_sheet`, helper `donor` (Task 3).
- Produces: el veredicto de D10 que necesita el plan de G4.

- [ ] **Step 1: Agregar el test ignorado `write_files_for_manual_check`**

Marcado `#[ignore = "genera archivos para abrir a mano"]`. Lee la carpeta de `QAPTURE_SPIKE_OUT` (si falta, `panic!` con el mensaje `Define QAPTURE_SPIKE_OUT con una carpeta existente.`) y escribe allí `guion_sintetico.xlsx` (`build()`) y `guion_con_evidencias.xlsx` (`graft_sheet(&build(), &donor(3), "Evidencias")`).

- [ ] **Step 2: Generar los archivos**

Run (PowerShell, desde `src-tauri/`): `$env:QAPTURE_SPIKE_OUT="$env:TEMP\g0"; New-Item -ItemType Directory -Force $env:QAPTURE_SPIKE_OUT; cargo test write_files_for_manual_check -- --ignored`
Expected: 1 passed y los dos `.xlsx` en `%TEMP%\g0`.

- [ ] **Step 3: Verificación manual (la hace una persona; el subagente se detiene aquí y la pide)**

Abrir `guion_con_evidencias.xlsx` en **Excel de escritorio** y en **LibreOffice Calc** y anotar en el handoff, por programa:

| Comprobación | Esperado |
|---|---|
| Aviso de reparación al abrir | Ninguno |
| Pestañas | «Guión de Pruebas», «Evidencias» (Hoja1 sigue oculta) |
| Hoja «Evidencias» | Los dos textos y las 3 imágenes visibles |
| Hoja del guion | Logo, celdas combinadas, autofiltro en la fila 15, listas en K16:K35 y H16:H35 |
| Área de impresión | `A1:M51` en la vista preliminar |
| Guardar desde Excel y volver a abrir | Sin aviso |

- [ ] **Step 4: Escribir el handoff**

`docs/superpowers/handoffs/g0/pico-trasplante.md` con: qué se construyó (Tasks 1–3), la tabla del Step 3 llena, el veredicto (**go**: G4-T3 parte de `graft_sheet`; **no-go**: qué falló y las alternativas de la propuesta 03, G4) y las limitaciones conocidas: la hoja injertada no conserva estilos (negritas, rellenos) y `docProps/app.xml` no lista la hoja nueva.

- [ ] **Step 5: Actualizar el contrato y commit**

En `G-guion-a-evidencias.md`, G-T1 y G-T2 pasan a `✅` con la fecha (o G-T2 a `❌ no-go` con el enlace al handoff).

```bash
git add src-tauri/src/excel/origin/evidence_graft.rs docs/superpowers/handoffs/g0/pico-trasplante.md docs/superpowers/contracts/G-guion-a-evidencias.md
git commit -m "docs(g0): resultado del pico de trasplante de la hoja Evidencias"
```
