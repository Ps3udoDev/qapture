# F4 — Excel con carátula Teams4Soft · Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Que el tester exporte el proyecto a un `.xlsx` con las hojas **Carátula**, **Matriz**, **Evidencias** y **Resumen** (branding Teams4Soft, generado en Rust), o una **plantilla vacía** con las columnas del proyecto, eligiendo dónde guardarlo con el «Guardar como» nativo, y que registre el **control de versiones** del documento que aparece en la carátula.

**Architecture:** Rust lee de una vez el proyecto, las columnas visibles, los casos, las evidencias y las versiones (`excel::data::ExportData`) y suelta la base de datos antes de abrir «Guardar como» (`tauri-plugin-dialog`, llamado desde Rust). Después arma el libro con `rust_xlsxwriter`: una hoja por módulo (`cover.rs`, `matrix.rs`, `evidence_sheet.rs`, `summary.rs`) sobre una paleta común (`styles.rs`). Las capturas se reducen en paralelo al ancho máximo (`images.rs`, 940 px por defecto) y la hoja Evidencias se escribe antes que la Matriz, porque los vínculos «Ver evidencias» necesitan saber en qué fila empieza cada caso. El archivo se escribe en un temporal y se renombra (`save.rs`). Si Excel lo tiene abierto, se avisa antes de procesar nada. El avance llega al front por el evento `export:progress`. En React, un diálogo junta las opciones (tester recordado, versión del documento, hojas) y al terminar muestra un toast con «Abrir archivo» y «Abrir carpeta». Esos dos botones usan comandos propios que solo aceptan archivos exportados en la sesión.

**Tech Stack:** Tauri 2.12, Rust 2024, rusqlite 0.40, `rust_xlsxwriter` (git `ade0a01`, zip 9), `tauri-plugin-dialog` 2.8.3, `tauri-plugin-opener` 2.7.2, image 0.25 · tests: `calamine` 0.36.1, `zip` 9.0.4 · React 19, TypeScript 6, Zustand 5, zod 4, lucide-react · Vitest 5, Testing Library.

**Spec:** [`docs/superpowers/specs/2026-10-15-qapture-design.md`](../specs/2026-10-15-qapture-design.md) (§5 `DOC_VERSION`, §6.5 UC-16, §7 diseño del Excel, §8 «archivo abierto», §9 capabilities) · **Contrato:** [`docs/superpowers/contracts/F4-excel-caratula.md`](../contracts/F4-excel-caratula.md)

## Global Constraints

- Rama `feat/f4-excel-caratula` creada desde `main` actualizado (PR #3 ya está en `main`). Un commit por task. Todos los mensajes terminan con una línea en blanco y `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>` (o la línea de la herramienta que haga el commit). **No hacer push ni merge sin pedirlo.**
- Plataforma Windows. Los comandos van en Git Bash desde la raíz `qapture/`; los de Rust, desde `qapture/src-tauri/`.
- Textos visibles **en español**. Mensajes fijos de esta fase (se usan tal cual en código y tests):
  - Versiones: `«Versión» es obligatorio.` · `«Fecha» debe ser una fecha válida (AAAA-MM-DD).` · `«Elaborado por» es obligatorio.` · `La versión «<v>» ya existe en este proyecto.` · `La versión del documento no existe.`
  - Opciones: `«Tester» es obligatorio.` · `Elige la versión del documento.` · `El ancho máximo de imagen debe estar entre 400 y 1600 px.`
  - Archivo: `Cierra el archivo en Excel y vuelve a intentar.` (nuevo `AppError::FileLocked`, código `file_locked`) · `No se pudo generar el Excel. Inténtalo nuevamente.` (nuevo `AppError::Export`, código `export`) · `No se puede sobrescribir el archivo: es de solo lectura o no tienes permisos.` · `Solo se pueden abrir archivos exportados en esta sesión.` · `No se encontró el archivo exportado. Puede que se haya movido o eliminado.`
  - Libro: `No hay casos registrados.` · `Sin versiones registradas.` · `No se han registrado capturas.` · `No se pudo leer la imagen de esta evidencia.` · pie `Teams4Soft · Inteligencia Híbrida · Confidencial`
  - UI: `Excel guardado: <archivo>` con las acciones `Abrir archivo` y `Abrir carpeta` · `Ya hay una exportación en curso.`
- Hojas, en este orden: `Carátula`, `Matriz`, `Evidencias`, `Resumen`. Carátula y Matriz siempre; Evidencias y Resumen, según el diálogo. La plantilla vacía lleva solo Carátula y Matriz.
- Branding (spec §1): Arial en **todo** el libro (también es el formato por defecto), `#2B629E`, `#BAC82D`, `#292B2D`, `#F6F4F1`, `#596069`. El logo y el banner se incluyen con `include_bytes!` desde `src/assets/brand/`.
- Nombre sugerido: `T4S_<Cliente>_<Proyecto>_Evidencias_<AAAAMMDD>_v<versión>.xlsx` y `T4S_<Cliente>_<Proyecto>_Plantilla_<AAAAMMDD>.xlsx`. El cliente se omite si está vacío.
- Ancho máximo de imagen: **940 px** por defecto (el de la Bitácora v1), rango 400–1600. Las capturas más anchas se reducen al exportar; las demás se insertan tal cual.
- Evento `export:progress` con payload `{ stage: "preparing" | "images" | "writing" | "saving", current, total }`, solo hacia la ventana `main`.
- **ACL (F3):** cada comando nuevo se agrega en `src-tauri/src/commands/names.rs`, en `generate_handler!` (`lib.rs`) y en `capabilities/main.json` (`allow-*`). El test de `names.rs` verifica las tres cosas. La ventana no recibe permisos JS de `dialog` ni de `opener`: los dos plugins se usan solo desde Rust.
- Dependencias nuevas, solo estas:
  - Rust: `rust_xlsxwriter` (git, rev `ade0a017439d169b2a9c4848fe1cb66919c0ca3d`), `tauri-plugin-dialog = "2.8.3"`, `tauri-plugin-opener = "2.7.2"`.
  - Rust, solo de desarrollo: `calamine = "0.36.1"` y `zip = "9.0.4"`, más el `[patch]` de `zip` 8.6 para calamine (ver «Decisiones»).
  - JS: ninguna.
- Fixtures y tests con datos ficticios ("ACME", "Recibos", "Pólizas", "Ana Pérez"). Nunca datos ni capturas de clientes.
- El fixture `src/infrastructure/tauri/__fixtures__/contracts.json` **cambia en esta fase**, y solo se regenera con `UPDATE_CONTRACTS=1 cargo test contract`.
- Cero warnings: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `pnpm typecheck`, `pnpm test` y `pnpm build` en verde al cerrar cada task.

## Review Focus

1. **El archivo destino está abierto en Excel (o es de solo lectura):** se avisa con `Cierra el archivo en Excel y vuelve a intentar.` **antes** de procesar imágenes, el archivo anterior queda intacto, no quedan `.tmp` y el diálogo sigue abierto para reintentar → tests en Task 9 (`save.rs`, `share_mode(0)` simula a Excel) y Task 12 (UI).
2. **El render de una evidencia falta o está dañado** (borrado a mano, disco con errores): el export no falla. La hoja muestra `No se pudo leer la imagen de esta evidencia.` en su lugar y un toast dice cuántas faltaron → tests en Task 7 (`images.rs`, `evidence_sheet.rs`), Task 9 (`missing_images`) y Task 12.
3. **Datos con forma inesperada que harían que Excel «repare» el libro o que Windows rechace el nombre:** cliente o proyecto con `/ : * ?`, listas de más de 255 caracteres o con comas (Excel no las acepta como validación), valores viejos que ya no cumplen su tipo (fecha «ayer», número «NaN») y textos muy largos (alto de fila ≤ 409) → tests en Task 3 (`file_segment`), Task 4 (alturas) y Task 6.
4. **Proyecto sin casos, sin versiones, sin veredictos o sin las columnas clave** (Entorno, Módulo, Resultado esperado/obtenido): el libro sigue saliendo completo con `No hay casos registrados.`, `Sin versiones registradas.`, `—` en el % de éxito y sin tablas de grupo vacías → tests en Tasks 5, 6, 7 y 8.
5. **Cancelar «Guardar como» después de pedir una versión nueva:** la versión queda registrada una sola vez, el diálogo sigue abierto y la reutiliza al reintentar (no aparece «ya existe»). Un segundo export mientras corre el primero se rechaza → tests en Task 11 (store) y Task 12 (UI).

## Decisiones respecto al contrato (se anotan en el contrato en la Task 1)

- **Dependencias por el *yank* de `zip`:** crates.io retiró todas las versiones de `zip` anteriores a la 9.0.3 (octubre de 2026), y tanto `rust_xlsxwriter` 0.99.1 como `calamine` 0.36.1 dependen de ellas, así que no resuelven desde cero. Por eso:
  - `rust_xlsxwriter` se toma del commit `ade0a01` de su repositorio, que ya usa zip 9 (misma API que 0.99.1). Cuando salga una versión publicada con zip 9, se vuelve a crates.io.
  - `calamine` solo se usa en tests (lee el xlsx que generamos nosotros). Para que resuelva, `zip` 8.6.0 se toma del tag del repositorio con un `[patch.crates-io]`.
- **Plantilla vacía (UC-17):** lleva Carátula (sin KPI) y Matriz con encabezados y 100 filas con formato y listas desplegables. No lleva Evidencias ni Resumen, y su nombre no lleva versión.
- **«Ver evidencias»** vincula a la primera fila del bloque del caso. Un caso sin capturas muestra «Ver sección». Sin la hoja Evidencias, la columna muestra el número de capturas.
- **Memoria estable con 300 evidencias:** cada captura se reduce al ancho máximo antes de insertarla (como mucho 8 en paralelo) y se suelta al insertarla. El test de rendimiento (`--ignored`, en release) verifica < 20 s y el número de imágenes. La memoria se observa en el checklist local (prueba 9, nueva).
- **Tester recordado** en `localStorage` (`qapture.tester`) hasta que F6-T9 lo pase a Settings.
- «Abrir archivo» y «Abrir carpeta» son comandos propios (`open_export`, `reveal_export`) que solo aceptan rutas exportadas en la sesión (spec §9: la ventana no tiene acceso genérico al disco).

## File Structure

```
qapture/
├─ package.json                                        (T13 versión)
├─ src/
│  ├─ application/use-cases.ts                         (T2 saveDocVersion, T11 exportWorkbook)
│  ├─ domain/
│  │  ├─ doc-versions.ts  doc-versions.test.ts         (T2) validación, sugerencia de versión, fechas
│  │  └─ export.ts  export.test.ts                     (T11) opciones, avance
│  ├─ ports/
│  │  ├─ export.ts                                     (T2 DocVersionRepository, T11 Exporter)
│  │  ├─ errors.ts                                     (T11) códigos file_locked y export
│  │  └─ repositories.ts                               (T2 docVersions, T11 exporter)
│  ├─ infrastructure/
│  │  ├─ tauri/export.ts  dto.ts  repositories.ts  contract.test.ts   (T2, T11)
│  │  └─ memory/export.ts  export.test.ts  repositories.ts            (T2, T11)
│  └─ ui/
│     ├─ components/Toasts.tsx                         (T11) acciones en el toast
│     ├─ store/export-slice.ts  preferences.ts  workspace.ts           (T2, T11)
│     ├─ screens/DocVersionsPanel.tsx  DocVersions.test.tsx  ProjectSettingsDialog.tsx  (T2)
│     ├─ screens/ExportDialog.tsx  Export.test.tsx  TopBar.tsx  AppShell.tsx           (T12)
│     └─ test-utils.tsx                                (T2) tester en renderApp
└─ src-tauri/
   ├─ Cargo.toml  Cargo.lock  tauri.conf.json          (T3, T10, T13)
   ├─ capabilities/main.json                           (T1, T10)
   └─ src/
      ├─ db/migrations/0003_doc_version.sql  db/mod.rs (T1)
      ├─ repo/doc_versions.rs  repo/mod.rs             (T1)
      ├─ commands/doc_versions.rs  commands/export.rs  commands/mod.rs  commands/names.rs  (T1, T10)
      ├─ dto.rs  error.rs  state.rs  lib.rs            (T1, T3, T9, T10)
      └─ excel/
         ├─ mod.rs                                     (T3…T9) build + export
         ├─ data.rs                                    (T3) ExportData, KPI, opciones, nombre de archivo
         ├─ testing.rs                                 (T3) datos ficticios y lectura del xlsx (calamine + zip)
         ├─ styles.rs                                  (T4) paleta T3S, Arial por defecto, alturas
         ├─ cover.rs                                   (T5) carátula
         ├─ matrix.rs                                  (T6) matriz
         ├─ images.rs  evidence_sheet.rs               (T7) capturas reducidas en paralelo + hoja
         ├─ summary.rs                                 (T8) resumen + gráfico
         └─ save.rs                                    (T9) escritura atómica, archivo abierto, RecentExports
```

Correspondencia con el contrato: F4-T1 → Tasks 1 y 2 · F4-T2 → Task 4 · F4-T3 → Task 5 · F4-T4 → Task 6 · F4-T5 → Task 7 · F4-T6 → Task 8 · F4-T7 → Tasks 9 y 10 · F4-T8 → Tasks 11 y 12 · F4-T9 → las pruebas con `calamine` de las Tasks 4–9 (en `excel/testing.rs`, creado en la Task 3).

---

### Task 1: Rama, documentos y control de versiones del documento en Rust (migración `0003`, repositorio y comandos)

**Files:**
- Create: `src-tauri/src/db/migrations/0003_doc_version.sql`, `src-tauri/src/repo/doc_versions.rs`, `src-tauri/src/commands/doc_versions.rs`
- Modify: `src-tauri/src/db/mod.rs`, `src-tauri/src/repo/mod.rs`, `src-tauri/src/dto.rs`, `src-tauri/src/commands/mod.rs`, `src-tauri/src/commands/names.rs`, `src-tauri/src/lib.rs`, `src-tauri/capabilities/main.json`, `src/infrastructure/tauri/__fixtures__/contracts.json` (regenerado)
- Modify: `docs/superpowers/contracts/README.md`, `docs/superpowers/contracts/F4-excel-caratula.md`; Add: `docs/superpowers/plans/2026-10-16-f4-excel-caratula.md` (este plan)

**Interfaces:**
- Produces:
  - Tabla `doc_version (id, project_id → project ON DELETE CASCADE, version, date, author, reviewer, approver, changes, created_at)`, con `UNIQUE (project_id, version COLLATE NOCASE)`.
  - `dto::DocVersionDto { id, project_id, version, date, author, reviewer, approver, changes, created_at }` (JSON camelCase) y `dto::DocVersionInput { version, date, author, reviewer, approver, changes }` (`deny_unknown_fields`).
  - `repo::doc_versions::{list(conn, project_id) -> AppResult<Vec<DocVersionDto>>, get(conn, id), create(conn, project_id, &DocVersionInput), update(conn, id, &DocVersionInput), delete(conn, id) -> AppResult<()>}`. `list` ordena por fecha y luego por orden de registro.
  - `repo::doc_versions::tests::input(version, date) -> DocVersionInput` (`pub(crate)`, la usan los tests de `excel`).
  - Comandos `list_doc_versions(projectId)`, `create_doc_version(projectId, input)`, `update_doc_version(id, input)`, `delete_doc_version(id)`.

- [ ] **Step 1: Rama y documentos**

```bash
git checkout main && git pull
git checkout -b feat/f4-excel-caratula
```

Este plan ya está en `docs/superpowers/plans/2026-10-16-f4-excel-caratula.md` (se escribió en `main` sin commitear, así que pasa a la rama).

En `docs/superpowers/contracts/README.md`, tabla **Estado**: cambia la fila F3 a `✅ Cerrada` y la fila F4 a:

```markdown
| F4 | [Excel con carátula](F4-excel-caratula.md) | [plan](../plans/2026-10-16-f4-excel-caratula.md) | 🟡 En curso | `v0.4.0` |
```

En `docs/superpowers/contracts/F4-excel-caratula.md`, cambia la línea de cabecera por:

```markdown
**Depende de:** F1 (y F2 para incluir imágenes) · **Plan:** [2026-10-16-f4-excel-caratula.md](../plans/2026-10-16-f4-excel-caratula.md)
```

Agrega, antes de `## Interfaces producidas`:

```markdown
## Notas de implementación (acordadas en el plan)
- crates.io retiró (*yank*) toda la serie `zip` < 9.0.3. Por eso `rust_xlsxwriter` se toma de su repositorio (commit `ade0a01`, ya con zip 9) hasta que publique una versión con zip 9. `calamine` (solo tests) usa `zip` 8.6.0 desde el tag del repositorio mediante `[patch.crates-io]`.
- Plantilla vacía: Carátula (sin KPI) + Matriz con 100 filas con formato y listas. Sin Evidencias ni Resumen, y sin versión en el nombre.
- Ancho máximo de imagen: 940 px por defecto (rango 400–1600). Las capturas más anchas se reducen al exportar. F6 lo lleva a Settings.
- «Abrir archivo» y «Abrir carpeta» solo aceptan archivos exportados en la sesión.
- El tester se recuerda en el equipo (`localStorage`) hasta F6-T9.
```

Y agrega al final del **Checklist de pruebas locales** la fila:

```markdown
| 9 | Exportar un proyecto con ~300 evidencias con el Administrador de tareas abierto | Termina en menos de 20 s; la memoria de Qapture vuelve a su nivel al terminar | ☐ |
```

- [ ] **Step 2: DTO de versiones**

En `src-tauri/src/dto.rs`, agrega antes de `#[cfg(test)]\nmod contract {`:

```rust
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocVersionDto {
    pub id: String,
    pub project_id: String,
    pub version: String,
    pub date: String,
    pub author: String,
    pub reviewer: String,
    pub approver: String,
    pub changes: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DocVersionInput {
    pub version: String,
    pub date: String,
    pub author: String,
    pub reviewer: String,
    pub approver: String,
    pub changes: String,
}
```

- [ ] **Step 3: Tests que fallan — repositorio**

En `src-tauri/src/repo/mod.rs`, agrega `pub mod doc_versions;` después de `pub mod cases;`.

Crea `src-tauri/src/repo/doc_versions.rs` solo con los tests:

```rust
#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::repo::test_support::{conn, other_project, project};

    pub fn input(version: &str, date: &str) -> DocVersionInput {
        DocVersionInput {
            version: version.into(),
            date: date.into(),
            author: "Ana Pérez".into(),
            reviewer: String::new(),
            approver: String::new(),
            changes: "Versión inicial".into(),
        }
    }

    #[test]
    fn create_trims_and_lists_by_date() {
        let conn = conn();
        let p = project(&conn);
        let mut late = input(" 1.1 ", "2026-10-20");
        late.reviewer = "  Luis Gómez ".into();
        create(&conn, &p.id, &late).unwrap();
        create(&conn, &p.id, &input("1.0", "2026-10-01")).unwrap();
        let versions = list(&conn, &p.id).unwrap();
        let names: Vec<_> = versions.iter().map(|v| v.version.as_str()).collect();
        assert_eq!(names, ["1.0", "1.1"]);
        assert_eq!(versions[1].reviewer, "Luis Gómez");
        assert_eq!(versions[1].project_id, p.id);
    }

    #[test]
    fn validates_required_fields_and_dates() {
        let conn = conn();
        let p = project(&conn);
        let no_author = DocVersionInput {
            author: " ".into(),
            ..input("1.0", "2026-10-01")
        };
        let cases = [
            (input(" ", "2026-10-01"), "«Versión» es obligatorio."),
            (
                input("1.0", "2026-02-30"),
                "«Fecha» debe ser una fecha válida (AAAA-MM-DD).",
            ),
            (input("1.0", ""), "«Fecha» es obligatorio."),
            (no_author, "«Elaborado por» es obligatorio."),
            (
                input(&"9".repeat(21), "2026-10-01"),
                "«Versión» supera 20 caracteres.",
            ),
        ];
        for (bad, message) in cases {
            assert_eq!(create(&conn, &p.id, &bad).unwrap_err().to_string(), message);
        }
    }

    #[test]
    fn versions_are_unique_per_project_ignoring_case() {
        let conn = conn();
        let p = project(&conn);
        let other = other_project(&conn);
        create(&conn, &p.id, &input("v1.0", "2026-10-01")).unwrap();
        assert_eq!(
            create(&conn, &p.id, &input("V1.0", "2026-10-02"))
                .unwrap_err()
                .to_string(),
            "La versión «V1.0» ya existe en este proyecto."
        );
        create(&conn, &other.id, &input("v1.0", "2026-10-01")).unwrap();
    }

    #[test]
    fn update_keeps_its_own_version_and_rejects_another() {
        let conn = conn();
        let p = project(&conn);
        let first = create(&conn, &p.id, &input("1.0", "2026-10-01")).unwrap();
        create(&conn, &p.id, &input("1.1", "2026-10-02")).unwrap();
        let mut edit = input("1.0", "2026-10-03");
        edit.changes = "Se agregan casos de Pólizas".into();
        let updated = update(&conn, &first.id, &edit).unwrap();
        assert_eq!(updated.date, "2026-10-03");
        assert_eq!(updated.changes, "Se agregan casos de Pólizas");
        assert!(update(&conn, &first.id, &input("1.1", "2026-10-03")).is_err());
    }

    #[test]
    fn delete_and_cascade_with_the_project() {
        let conn = conn();
        let p = project(&conn);
        let v = create(&conn, &p.id, &input("1.0", "2026-10-01")).unwrap();
        delete(&conn, &v.id).unwrap();
        assert_eq!(delete(&conn, &v.id).unwrap_err().code(), "not_found");
        create(&conn, &p.id, &input("1.0", "2026-10-01")).unwrap();
        projects::delete(&conn, &p.id).unwrap();
        let left: i64 = conn
            .query_row("SELECT COUNT(*) FROM doc_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(left, 0);
        assert_eq!(list(&conn, &p.id).unwrap_err().code(), "not_found");
    }

    #[test]
    fn changes_touch_the_project() {
        let conn = conn();
        let p = project(&conn);
        std::thread::sleep(std::time::Duration::from_millis(5));
        create(&conn, &p.id, &input("1.0", "2026-10-01")).unwrap();
        assert!(projects::get(&conn, &p.id).unwrap().updated_at > p.updated_at);
    }
}
```

- [ ] **Step 4: Verificar que fallan**

Run: `cd src-tauri && cargo test repo::doc_versions`
Expected: FAIL de compilación (`cannot find function create`, `list`…).

- [ ] **Step 5: Migración e implementación**

Crea `src-tauri/src/db/migrations/0003_doc_version.sql`:

```sql
CREATE TABLE doc_version (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES project(id) ON DELETE CASCADE,
  version TEXT NOT NULL,
  date TEXT NOT NULL,
  author TEXT NOT NULL,
  reviewer TEXT NOT NULL DEFAULT '',
  approver TEXT NOT NULL DEFAULT '',
  changes TEXT NOT NULL DEFAULT '',
  created_at TEXT NOT NULL,
  UNIQUE (project_id, version COLLATE NOCASE)
);

CREATE INDEX idx_doc_version_project ON doc_version(project_id, date);
```

En `src-tauri/src/db/mod.rs`, agrega a `MIGRATIONS` (después de `0002_evidence.sql`):

```rust
    include_str!("migrations/0003_doc_version.sql"),
```

Agrega al **inicio** de `src-tauri/src/repo/doc_versions.rs` (antes de los tests):

```rust
use rusqlite::{Connection, OptionalExtension, Row, params};

use super::projects;
use crate::{
    db::{new_id, now},
    domain::{
        fields::is_iso_date,
        text::{optional_text, required_text},
    },
    dto::{DocVersionDto, DocVersionInput},
    error::{AppError, AppResult, validation},
};

fn not_found() -> AppError {
    AppError::NotFound("La versión del documento no existe.".into())
}

fn from_row(r: &Row) -> rusqlite::Result<DocVersionDto> {
    Ok(DocVersionDto {
        id: r.get("id")?,
        project_id: r.get("project_id")?,
        version: r.get("version")?,
        date: r.get("date")?,
        author: r.get("author")?,
        reviewer: r.get("reviewer")?,
        approver: r.get("approver")?,
        changes: r.get("changes")?,
        created_at: r.get("created_at")?,
    })
}

fn validate(input: &DocVersionInput) -> AppResult<DocVersionInput> {
    let version = required_text("Versión", &input.version, 20)?;
    let date = required_text("Fecha", &input.date, 10)?;
    if !is_iso_date(&date) {
        return Err(validation(
            "«Fecha» debe ser una fecha válida (AAAA-MM-DD).",
        ));
    }
    Ok(DocVersionInput {
        version,
        date,
        author: required_text("Elaborado por", &input.author, 100)?,
        reviewer: optional_text("Revisado por", &input.reviewer, 100)?,
        approver: optional_text("Aprobado por", &input.approver, 100)?,
        changes: optional_text("Cambios", &input.changes, 1000)?,
    })
}

fn ensure_unique(
    conn: &Connection,
    project_id: &str,
    version: &str,
    except: Option<&str>,
) -> AppResult<()> {
    let taken: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM doc_version
                       WHERE project_id = ?1 AND version = ?2 COLLATE NOCASE AND id IS NOT ?3)",
        params![project_id, version, except],
        |r| r.get(0),
    )?;
    if taken {
        Err(validation(format!(
            "La versión «{version}» ya existe en este proyecto."
        )))
    } else {
        Ok(())
    }
}

/// Orden del historial: fecha y, a igual fecha, orden de registro.
pub fn list(conn: &Connection, project_id: &str) -> AppResult<Vec<DocVersionDto>> {
    projects::ensure_exists(conn, project_id)?;
    let mut stmt = conn.prepare(
        "SELECT * FROM doc_version WHERE project_id = ?1 ORDER BY date, created_at, rowid",
    )?;
    let rows = stmt
        .query_map([project_id], from_row)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn get(conn: &Connection, id: &str) -> AppResult<DocVersionDto> {
    conn.query_row("SELECT * FROM doc_version WHERE id = ?1", [id], from_row)
        .optional()?
        .ok_or_else(not_found)
}

pub fn create(
    conn: &Connection,
    project_id: &str,
    input: &DocVersionInput,
) -> AppResult<DocVersionDto> {
    projects::ensure_exists(conn, project_id)?;
    let v = validate(input)?;
    ensure_unique(conn, project_id, &v.version, None)?;
    let id = new_id();
    conn.execute(
        "INSERT INTO doc_version (id, project_id, version, date, author, reviewer, approver, changes, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            id,
            project_id,
            v.version,
            v.date,
            v.author,
            v.reviewer,
            v.approver,
            v.changes,
            now()
        ],
    )?;
    projects::touch(conn, project_id)?;
    get(conn, &id)
}

pub fn update(conn: &Connection, id: &str, input: &DocVersionInput) -> AppResult<DocVersionDto> {
    let current = get(conn, id)?;
    let v = validate(input)?;
    ensure_unique(conn, &current.project_id, &v.version, Some(id))?;
    conn.execute(
        "UPDATE doc_version SET version = ?1, date = ?2, author = ?3, reviewer = ?4, approver = ?5,
                                changes = ?6
         WHERE id = ?7",
        params![
            v.version, v.date, v.author, v.reviewer, v.approver, v.changes, id
        ],
    )?;
    projects::touch(conn, &current.project_id)?;
    get(conn, id)
}

pub fn delete(conn: &Connection, id: &str) -> AppResult<()> {
    let current = get(conn, id)?;
    conn.execute("DELETE FROM doc_version WHERE id = ?1", [id])?;
    projects::touch(conn, &current.project_id)
}
```

- [ ] **Step 6: Verificar que pasan**

Run: `cd src-tauri && cargo test repo::doc_versions && cargo test db::`
Expected: PASS (6 tests de versiones; los de `db` siguen verdes porque usan `MIGRATIONS.len()`).

- [ ] **Step 7: Comandos y ACL**

Crea `src-tauri/src/commands/doc_versions.rs`:

```rust
use tauri::State;

use crate::{
    dto::{DocVersionDto, DocVersionInput},
    error::AppResult,
    repo::doc_versions,
    state::AppState,
};

#[tauri::command(async)]
pub fn list_doc_versions(
    state: State<'_, AppState>,
    project_id: String,
) -> AppResult<Vec<DocVersionDto>> {
    doc_versions::list(&state.db(), &project_id)
}

#[tauri::command(async)]
pub fn create_doc_version(
    state: State<'_, AppState>,
    project_id: String,
    input: DocVersionInput,
) -> AppResult<DocVersionDto> {
    doc_versions::create(&state.db(), &project_id, &input)
}

#[tauri::command(async)]
pub fn update_doc_version(
    state: State<'_, AppState>,
    id: String,
    input: DocVersionInput,
) -> AppResult<DocVersionDto> {
    doc_versions::update(&state.db(), &id, &input)
}

#[tauri::command(async)]
pub fn delete_doc_version(state: State<'_, AppState>, id: String) -> AppResult<()> {
    doc_versions::delete(&state.db(), &id)
}
```

En `src-tauri/src/commands/mod.rs`, agrega `pub mod doc_versions;` después de `pub mod cases;`.

En `src-tauri/src/commands/names.rs`, agrega al final de `APP_COMMANDS` (después de su último elemento, `"cancel_capture",`; **no** en `OVERLAY_COMMANDS`, que también termina en `"cancel_capture",`):

```rust
    "list_doc_versions",
    "create_doc_version",
    "update_doc_version",
    "delete_doc_version",
```

En `src-tauri/src/lib.rs`, agrega al final de `generate_handler!` (después de `commands::capture::cancel_capture,`):

```rust
            commands::doc_versions::list_doc_versions,
            commands::doc_versions::create_doc_version,
            commands::doc_versions::update_doc_version,
            commands::doc_versions::delete_doc_version,
```

En `src-tauri/capabilities/main.json`, cambia `"allow-start-capture"` por:

```json
    "allow-start-capture",
    "allow-list-doc-versions",
    "allow-create-doc-version",
    "allow-update-doc-version",
    "allow-delete-doc-version"
```

- [ ] **Step 8: Contrato**

En `src-tauri/src/dto.rs`, módulo `contract`:

1. En `inputs_json()`, agrega después de la entrada `"selection"` (pon una coma al final de `"selection": { … }`):

```rust
            "docVersionInput": { "version": "1.0", "date": "2026-10-16", "author": "Ana Pérez",
                                 "reviewer": "Luis Gómez", "approver": "", "changes": "Versión inicial" }
```

2. Renombra `fn sample() -> Value` a `fn base_sample() -> Value` y agrega encima:

```rust
    /// F4: en un objeto aparte para no superar el límite de recursión de `json!`.
    fn export_samples() -> Value {
        json!({
            "docVersion": DocVersionDto { id: "ver-1".into(), project_id: "prj-1".into(), version: "1.0".into(),
                                          date: "2026-10-16".into(), author: "Ana Pérez".into(),
                                          reviewer: "Luis Gómez".into(), approver: String::new(),
                                          changes: "Versión inicial".into(), created_at: "2026-10-16T12:00:00.000Z".into() },
        })
    }

    fn sample() -> Value {
        let mut sample = base_sample();
        if let (Some(target), Value::Object(extra)) = (sample.as_object_mut(), export_samples()) {
            target.extend(extra);
        }
        sample
    }
```

3. En `inputs_deserialize_from_ts_shape`, agrega al final:

```rust
        serde_json::from_value::<DocVersionInput>(i["docVersionInput"].clone()).unwrap();
```

Regenera el fixture y verifica:

```bash
cd src-tauri && UPDATE_CONTRACTS=1 cargo test contract && cargo test
```

Expected: PASS. `git diff ../src/infrastructure/tauri/__fixtures__/contracts.json` muestra solo `docVersion` y `inputs.docVersionInput` nuevos.

- [ ] **Step 9: Verificación**

```bash
cd src-tauri && cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test && cd ..
pnpm typecheck && pnpm test
```

Expected: todo en verde. El test `every_registered_handler_is_listed_and_vice_versa` confirma `names.rs` ↔ `lib.rs`, y `main_window_gets_core_and_every_non_overlay_command` confirma `main.json`.

- [ ] **Step 10: Commit**

```bash
git add docs src-tauri/src src-tauri/capabilities src/infrastructure/tauri/__fixtures__/contracts.json
git commit -m "feat(f4): control de versiones del documento en Rust (migración 0003, repositorio y comandos)

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---


### Task 2: Versiones del documento en el front — dominio, puerto, adaptadores, store y pestaña «Versiones»

**Files:**
- Create: `src/domain/doc-versions.ts`, `src/domain/doc-versions.test.ts`, `src/ports/export.ts`, `src/infrastructure/tauri/export.ts`, `src/infrastructure/memory/export.ts`, `src/infrastructure/memory/export.test.ts`, `src/ui/store/export-slice.ts`, `src/ui/store/export-slice.test.ts`, `src/ui/screens/DocVersionsPanel.tsx`, `src/ui/screens/DocVersions.test.tsx`
- Modify: `src/ports/repositories.ts`, `src/infrastructure/tauri/dto.ts`, `src/infrastructure/tauri/repositories.ts`, `src/infrastructure/tauri/contract.test.ts`, `src/infrastructure/memory/repositories.ts`, `src/application/use-cases.ts`, `src/ui/store/preferences.ts`, `src/ui/store/workspace.ts`, `src/ui/screens/ProjectSettingsDialog.tsx`, `src/ui/test-utils.tsx`

**Interfaces:**
- Consumes: comandos de la Task 1 (`list_doc_versions`, `create_doc_version`, `update_doc_version`, `delete_doc_version`) y el fixture `docVersion` / `inputs.docVersionInput`.
- Produces:
  - `domain/doc-versions.ts`: `DocVersionInput`, `DocVersion`, `todayIso(now?)`, `emptyDocVersion(date, author?)`, `validateDocVersion(input): FormErrors`, `trimDocVersion(input)`, `formatIsoDate(value)`, `latestVersion(versions)`, `suggestNextVersion(versions)`.
  - `ports/export.ts`: `DocVersionRepository { list, create, update, remove }`. `Repositories.docVersions`.
  - `application/use-cases.ts`: `saveDocVersion(repos, projectId, input, id?) → Outcome<DocVersion>`.
  - `Preferences.getTester(): string` / `setTester(name)` (clave `qapture.tester`). `memoryPreferences(lastProject?, tester = "")`.
  - Store (`export-slice.ts`): `docVersions`, `loadDocVersions()`, `saveDocVersion(input, id?)`, `deleteDocVersion(id)`, `rememberedTester()`. Las versiones se recargan cada vez que cambia el proyecto abierto.
  - `renderApp({ tester })` en `test-utils.tsx`.

- [ ] **Step 1: Test que falla — dominio**

Crea `src/domain/doc-versions.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { emptyDocVersion, formatIsoDate, suggestNextVersion, todayIso, trimDocVersion, validateDocVersion, type DocVersion } from "./doc-versions";

const valid = { ...emptyDocVersion("2026-10-16", "Ana Pérez"), version: "1.0" };
const saved = (version: string): DocVersion => ({ ...valid, version, id: version, projectId: "p", createdAt: "" });

describe("versiones del documento", () => {
  it("valida obligatorios, fecha y largos como Rust", () => {
    expect(validateDocVersion(valid)).toEqual({});
    expect(validateDocVersion({ ...valid, version: " ", author: "" })).toEqual({
      version: "«Versión» es obligatorio.",
      author: "«Elaborado por» es obligatorio.",
    });
    expect(validateDocVersion({ ...valid, date: "2026-02-30" }).date).toBe("«Fecha» debe ser una fecha válida (AAAA-MM-DD).");
    expect(validateDocVersion({ ...valid, date: "" }).date).toBe("«Fecha» es obligatorio.");
    expect(validateDocVersion({ ...valid, version: "9".repeat(21) }).version).toBe("«Versión» supera 20 caracteres.");
    expect(validateDocVersion({ ...valid, changes: "🚀".repeat(1001) }).changes).toBe("«Cambios» supera 1000 caracteres.");
  });

  it("sugiere la siguiente versión menor", () => {
    expect(suggestNextVersion([])).toBe("1.0");
    expect(suggestNextVersion([saved("1.0"), saved("1.9")])).toBe("1.10");
    expect(suggestNextVersion([saved("v2.3")])).toBe("v2.4");
    expect(suggestNextVersion([saved("Final")])).toBe("");
  });

  it("usa la fecha local y recorta los textos", () => {
    expect(todayIso(new Date(2026, 9, 16, 23, 30))).toBe("2026-10-16");
    expect(formatIsoDate("2026-10-01")).toBe("01/10/2026");
    expect(formatIsoDate("ayer")).toBe("ayer");
    expect(trimDocVersion({ ...valid, version: " 1.1 ", changes: " Casos nuevos " })).toMatchObject({ version: "1.1", changes: "Casos nuevos" });
  });
});
```

Run: `pnpm vitest run src/domain/doc-versions.test.ts`
Expected: FAIL (`Failed to resolve import "./doc-versions"`).

- [ ] **Step 2: Dominio**

Crea `src/domain/doc-versions.ts`:

```ts
import type { FormErrors } from "./cases";
import { charLength, isIsoDate } from "./fields";

/** Una fila del control de versiones del documento (UC-05). */
export interface DocVersionInput {
  version: string;
  date: string;
  author: string;
  reviewer: string;
  approver: string;
  changes: string;
}

export interface DocVersion extends DocVersionInput {
  id: string;
  projectId: string;
  createdAt: string;
}

/** Fecha local de hoy en AAAA-MM-DD (no UTC: a las 21:00 en México sigue siendo hoy). */
export function todayIso(now = new Date()): string {
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`;
}

export const emptyDocVersion = (date: string, author = ""): DocVersionInput => ({
  version: "",
  date,
  author,
  reviewer: "",
  approver: "",
  changes: "",
});

const LIMITS: [keyof DocVersionInput, string, number][] = [
  ["version", "Versión", 20],
  ["author", "Elaborado por", 100],
  ["reviewer", "Revisado por", 100],
  ["approver", "Aprobado por", 100],
  ["changes", "Cambios", 1000],
];

/** Espejo de `repo/doc_versions.rs::validate` (la unicidad la revisa Rust). */
export function validateDocVersion(input: DocVersionInput): FormErrors {
  const errors: FormErrors = {};
  if (!input.version.trim()) errors.version = "«Versión» es obligatorio.";
  if (!input.date.trim()) errors.date = "«Fecha» es obligatorio.";
  else if (!isIsoDate(input.date.trim())) errors.date = "«Fecha» debe ser una fecha válida (AAAA-MM-DD).";
  if (!input.author.trim()) errors.author = "«Elaborado por» es obligatorio.";
  for (const [key, label, max] of LIMITS) {
    if (!errors[key] && charLength(input[key].trim()) > max) errors[key] = `«${label}» supera ${max} caracteres.`;
  }
  return errors;
}

export const trimDocVersion = (input: DocVersionInput): DocVersionInput => ({
  version: input.version.trim(),
  date: input.date.trim(),
  author: input.author.trim(),
  reviewer: input.reviewer.trim(),
  approver: input.approver.trim(),
  changes: input.changes.trim(),
});

/** AAAA-MM-DD → DD/MM/AAAA (como en el Excel); cualquier otro texto se muestra tal cual. */
export function formatIsoDate(value: string): string {
  if (!isIsoDate(value)) return value;
  const [y, m, d] = value.split("-");
  return `${d}/${m}/${y}`;
}

/** La lista llega ordenada por fecha (como la devuelve Rust): la última es la vigente. */
export const latestVersion = (versions: DocVersion[]): DocVersion | null => versions.at(-1) ?? null;

/** «1.0» si no hay versiones; si la última es `X.Y` (con o sin «v»), sugiere `X.(Y+1)`. */
export function suggestNextVersion(versions: DocVersion[]): string {
  const last = latestVersion(versions);
  if (!last) return "1.0";
  const match = /^(v?)(\d+)\.(\d+)$/i.exec(last.version.trim());
  return match ? `${match[1]}${match[2]}.${Number(match[3]) + 1}` : "";
}
```

Run: `pnpm vitest run src/domain/doc-versions.test.ts`
Expected: PASS (3 tests).

- [ ] **Step 3: Test que falla — adaptador en memoria**

Crea `src/infrastructure/memory/export.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { emptyDocVersion } from "../../domain/doc-versions";
import { emptyMeta } from "../../domain/project";
import { createMemoryRepositories } from "./repositories";

async function setup() {
  const repos = createMemoryRepositories();
  const p = await repos.projects.create({ meta: { ...emptyMeta(), name: "Recibos", client: "ACME" }, templateId: "tpl-funcional" });
  return { repos, p };
}

const input = (version: string, date = "2026-10-16") => ({ ...emptyDocVersion(date, "Ana Pérez"), version });

describe("versiones del documento en memoria", () => {
  it("lista por fecha, valida y no repite versiones", async () => {
    const { repos, p } = await setup();
    await repos.docVersions.create(p.id, input(" 1.1 ", "2026-10-20"));
    const first = await repos.docVersions.create(p.id, input("1.0", "2026-10-01"));
    expect((await repos.docVersions.list(p.id)).map((v) => v.version)).toEqual(["1.0", "1.1"]);
    await expect(repos.docVersions.create(p.id, input("V1.1"))).resolves.toBeTruthy();
    await expect(repos.docVersions.create(p.id, input("1.0"))).rejects.toMatchObject({ message: "La versión «1.0» ya existe en este proyecto." });
    await expect(repos.docVersions.create(p.id, input(""))).rejects.toMatchObject({ message: "«Versión» es obligatorio." });
    const updated = await repos.docVersions.update(first.id, { ...input("1.0"), changes: "Casos de Pólizas" });
    expect(updated.changes).toBe("Casos de Pólizas");
    await repos.docVersions.remove(first.id);
    await expect(repos.docVersions.remove(first.id)).rejects.toMatchObject({ code: "not_found" });
  });

  it("se borran con el proyecto", async () => {
    const { repos, p } = await setup();
    await repos.docVersions.create(p.id, input("1.0"));
    await repos.projects.remove(p.id);
    await expect(repos.docVersions.list(p.id)).rejects.toMatchObject({ code: "not_found" });
  });
});
```

Run: `pnpm vitest run src/infrastructure/memory/export.test.ts`
Expected: FAIL (`repos.docVersions` no existe).

- [ ] **Step 4: Puerto y adaptadores**

Crea `src/ports/export.ts`:

```ts
import type { DocVersion, DocVersionInput } from "../domain/doc-versions";

/** Control de versiones del documento (UC-05). */
export interface DocVersionRepository {
  list(projectId: string): Promise<DocVersion[]>;
  create(projectId: string, input: DocVersionInput): Promise<DocVersion>;
  update(id: string, input: DocVersionInput): Promise<DocVersion>;
  remove(id: string): Promise<void>;
}
```

Crea `src/infrastructure/memory/export.ts`:

```ts
import { trimDocVersion, validateDocVersion, type DocVersion, type DocVersionInput } from "../../domain/doc-versions";
import { AppError } from "../../ports/errors";
import type { DocVersionRepository } from "../../ports/export";

export interface MemoryDocVersionDeps {
  ensureProject(projectId: string): void;
  touch(projectId: string): void;
  id(prefix: string): string;
  now(): string;
}

export interface MemoryDocVersions extends DocVersionRepository {
  removeForProject(projectId: string): void;
}

/** Replica en memoria las reglas de `repo/doc_versions.rs`. */
export function createMemoryDocVersions(deps: MemoryDocVersionDeps): MemoryDocVersions {
  const items = new Map<string, DocVersion>();
  const fail = (message: string): never => {
    throw new AppError("validation", message);
  };
  const get = (id: string) => {
    const found = items.get(id);
    if (!found) throw new AppError("not_found", "La versión del documento no existe.");
    return found;
  };
  const check = (projectId: string, raw: DocVersionInput, except?: string) => {
    const first = Object.values(validateDocVersion(raw))[0];
    if (first) fail(first);
    const input = trimDocVersion(raw);
    const taken = [...items.values()].some(
      (v) => v.projectId === projectId && v.id !== except && v.version.toLowerCase() === input.version.toLowerCase(),
    );
    if (taken) fail(`La versión «${input.version}» ya existe en este proyecto.`);
    return input;
  };
  const sorted = (projectId: string) =>
    [...items.values()]
      .filter((v) => v.projectId === projectId)
      .sort((a, b) => a.date.localeCompare(b.date) || a.createdAt.localeCompare(b.createdAt));

  return {
    async list(projectId) {
      deps.ensureProject(projectId);
      return structuredClone(sorted(projectId));
    },
    async create(projectId, raw) {
      deps.ensureProject(projectId);
      const input = check(projectId, raw);
      const version: DocVersion = { ...input, id: deps.id("ver"), projectId, createdAt: deps.now() };
      items.set(version.id, version);
      deps.touch(projectId);
      return structuredClone(version);
    },
    async update(id, raw) {
      const current = get(id);
      Object.assign(current, check(current.projectId, raw, id));
      deps.touch(current.projectId);
      return structuredClone(current);
    },
    async remove(id) {
      const current = get(id);
      items.delete(id);
      deps.touch(current.projectId);
    },
    removeForProject(projectId) {
      for (const v of sorted(projectId)) items.delete(v.id);
    },
  };
}
```

Crea `src/infrastructure/tauri/export.ts`:

```ts
import { z } from "zod";
import type { DocVersionRepository } from "../../ports/export";
import { call } from "./client";
import { docVersionSchema } from "./dto";

const nothing = z.null();

export function tauriDocVersions(): DocVersionRepository {
  return {
    list: (projectId) => call("list_doc_versions", { projectId }, z.array(docVersionSchema)),
    create: (projectId, input) => call("create_doc_version", { projectId, input }, docVersionSchema),
    update: (id, input) => call("update_doc_version", { id, input }, docVersionSchema),
    remove: async (id) => {
      await call("delete_doc_version", { id }, nothing);
    },
  };
}
```

Agrega al final de `src/infrastructure/tauri/dto.ts`:

```ts
export const docVersionSchema = z.strictObject({
  id: z.string(),
  projectId: z.string(),
  version: z.string(),
  date: z.string(),
  author: z.string(),
  reviewer: z.string(),
  approver: z.string(),
  changes: z.string(),
  createdAt: z.string(),
});
```

En `src/ports/repositories.ts`, después de:

```ts
import type { ScreenCapture } from "./capture";
```

agrega:

```ts
import type { DocVersionRepository } from "./export";
```

En `src/ports/repositories.ts`, reemplaza:

```ts
  capture: ScreenCapture;
}
```

por:

```ts
  capture: ScreenCapture;
  docVersions: DocVersionRepository;
}
```

En `src/infrastructure/memory/repositories.ts`, después de:

```ts
import { createMemoryEvidence, type MemoryEvidence } from "./evidence";
```

agrega:

```ts
import { createMemoryDocVersions, type MemoryDocVersions } from "./export";
```

En `src/infrastructure/memory/repositories.ts`, reemplaza:

```ts
  capture: MemoryCapture;
}
```

por:

```ts
  capture: MemoryCapture;
  docVersions: MemoryDocVersions;
}
```

En `src/infrastructure/memory/repositories.ts`, antes de:

```ts
  const withCount = 
```

agrega:

```ts
  const docVersions = createMemoryDocVersions({
    ensureProject: (projectId) => void getProject(projectId),
    touch: (projectId) => touch(getProject(projectId)),
    id,
    now,
  });
```

En `src/infrastructure/memory/repositories.ts`, después de:

```ts
        projects.delete(projectId);
```

agrega:

```ts
        docVersions.removeForProject(projectId);
```

En `src/infrastructure/memory/repositories.ts`, reemplaza:

```ts
    capture: createMemoryCapture(evidence),
  };
```

por:

```ts
    capture: createMemoryCapture(evidence),
    docVersions,
  };
```

En `src/infrastructure/tauri/repositories.ts`, antes de:

```ts
import { encodeFrame } from "./frame";
```

agrega:

```ts
import { tauriDocVersions } from "./export";
```

En `src/infrastructure/tauri/repositories.ts`, reemplaza:

```ts
    capture: tauriScreenCapture(),
  };
```

por:

```ts
    capture: tauriScreenCapture(),
    docVersions: tauriDocVersions(),
  };
```

En `src/infrastructure/tauri/contract.test.ts`, después de:

```ts
import type { CaseInput } from "../../domain/cases";
```

agrega:

```ts
import { emptyDocVersion } from "../../domain/doc-versions";
```

En `src/infrastructure/tauri/contract.test.ts`, reemplaza:

```ts
import { appErrorSchema, capturedEventSchema, caseSchema, evidenceSchema, fieldSchema,
```

por:

```ts
import { appErrorSchema, capturedEventSchema, caseSchema, docVersionSchema, evidenceSchema, fieldSchema,
```

En `src/infrastructure/tauri/contract.test.ts`, después de:

```ts
    ["captureError", appErrorSchema],
```

agrega:

```ts
    ["docVersion", docVersionSchema],
```

En `src/infrastructure/tauri/contract.test.ts`, después de:

```ts
    expect(sortedKeys(selection)).toEqual(sortedKeys(fixture.inputs.selection));
```

agrega:

```ts
    expect(sortedKeys(emptyDocVersion(""))).toEqual(sortedKeys(fixture.inputs.docVersionInput));
```

Run: `pnpm vitest run src/infrastructure`
Expected: PASS (incluye el contrato: `docVersion` cumple el esquema estricto y `DocVersionInput` tiene las claves que Rust acepta).

- [ ] **Step 5: Test que falla — store**

Crea `src/ui/store/export-slice.test.ts` (la Task 11 lo amplía):

```ts
import { describe, expect, it } from "vitest";
import { emptyDocVersion } from "../../domain/doc-versions";
import { createMemoryRepositories } from "../../infrastructure/memory/repositories";
import { seedProject } from "../test-utils";
import { memoryPreferences } from "./preferences";
import { createWorkspaceStore } from "./workspace";

async function setup() {
  const repos = createMemoryRepositories();
  const p = await seedProject(repos);
  const prefs = memoryPreferences(p.id, "Ana Pérez");
  const store = createWorkspaceStore(repos, prefs);
  await store.getState().init();
  return { repos, store, prefs, p };
}

const newVersion = { ...emptyDocVersion("2026-10-16", "Ana Pérez"), version: "1.0", changes: "Versión inicial" };

describe("slice de exportación", () => {
  it("carga las versiones del proyecto abierto y las guarda con toast", async () => {
    const { store } = await setup();
    expect(store.getState().docVersions).toEqual([]);
    expect(await store.getState().saveDocVersion(newVersion)).toEqual({ ok: true });
    expect(store.getState().docVersions.map((v) => v.version)).toEqual(["1.0"]);
    expect(store.getState().toasts.at(-1)?.message).toBe("Versión 1.0 registrada.");
    expect(await store.getState().saveDocVersion({ ...newVersion, author: " " })).toEqual({
      ok: false,
      errors: { author: "«Elaborado por» es obligatorio." },
    });
  });

});
```

Run: `pnpm vitest run src/ui/store/export-slice.test.ts`
Expected: FAIL (`memoryPreferences` no acepta el tester y `saveDocVersion` no existe).

- [ ] **Step 6: Caso de uso, preferencias y slice**

Agrega al final de `src/application/use-cases.ts`:

```ts
export async function saveDocVersion(repos: Repositories, projectId: string, input: DocVersionInput, id?: string): Promise<Outcome<DocVersion>> {
  const errors = validateDocVersion(input);
  if (invalid(errors)) return { ok: false, errors };
  const clean = trimDocVersion(input);
  return guard(() => (id ? repos.docVersions.update(id, clean) : repos.docVersions.create(projectId, clean)));
}
```

En `src/application/use-cases.ts`, antes de:

```ts
import { validateDescription, type Evidence, type EvidenceUpdateInput, type NewEvidenceInput } from "../domain/evidence";
```

agrega:

```ts
import { trimDocVersion, validateDocVersion, type DocVersion, type DocVersionInput } from "../domain/doc-versions";
```

Reemplaza `src/ui/store/preferences.ts` por:

```ts
export interface Preferences {
  getLastProject(): string | null;
  setLastProject(id: string | null): void;
  /** Nombre del tester para la carátula. F6 lo reemplaza por el perfil de Settings. */
  getTester(): string;
  setTester(name: string): void;
}

const KEY = "qapture.lastProject";
const TESTER_KEY = "qapture.tester";

export function browserPreferences(): Preferences {
  return {
    getLastProject() {
      try {
        return localStorage.getItem(KEY);
      } catch {
        return null;
      }
    },
    setLastProject(id) {
      try {
        if (id) localStorage.setItem(KEY, id);
        else localStorage.removeItem(KEY);
      } catch {
        // Sin almacenamiento local: la app sigue funcionando, solo no recuerda el proyecto.
      }
    },
    getTester() {
      try {
        return localStorage.getItem(TESTER_KEY) ?? "";
      } catch {
        return "";
      }
    },
    setTester(name) {
      try {
        localStorage.setItem(TESTER_KEY, name);
      } catch {
        // Igual que arriba: sin almacenamiento solo se pierde la sugerencia.
      }
    },
  };
}

export function memoryPreferences(initial: string | null = null, initialTester = ""): Preferences {
  let last = initial;
  let tester = initialTester;
  return {
    getLastProject: () => last,
    setLastProject: (id) => {
      last = id;
    },
    getTester: () => tester,
    setTester: (name) => {
      tester = name;
    },
  };
}
```

Crea `src/ui/store/export-slice.ts` (la Task 11 le agrega la exportación):

```ts
import type { StoreApi } from "zustand/vanilla";
import * as useCases from "../../application/use-cases";
import type { DocVersion, DocVersionInput } from "../../domain/doc-versions";
import type { Repositories } from "../../ports/repositories";
import type { SliceHelpers } from "./evidence-slice";
import type { Preferences } from "./preferences";
import type { FormResult, WorkspaceState } from "./workspace";

export interface ExportSlice {
  docVersions: DocVersion[];
  loadDocVersions(): Promise<void>;
  saveDocVersion(input: DocVersionInput, id?: string): Promise<FormResult>;
  deleteDocVersion(id: string): Promise<void>;
  rememberedTester(): string;
}

type Set = StoreApi<WorkspaceState>["setState"];
type Get = StoreApi<WorkspaceState>["getState"];

const NO_PROJECT: FormResult = { ok: false, errors: { _form: "No hay un proyecto abierto." } };

export function createExportSlice(repos: Repositories, prefs: Preferences, set: Set, get: Get, h: SliceHelpers): ExportSlice {
  return {
    docVersions: [],

    async loadDocVersions() {
      const projectId = get().project?.id;
      if (!projectId) {
        set({ docVersions: [] });
        return;
      }
      try {
        const docVersions = await repos.docVersions.list(projectId);
        if (get().project?.id === projectId) set({ docVersions });
      } catch (error) {
        h.report(error);
      }
    },

    async saveDocVersion(input, id) {
      const current = get().project;
      if (!current) return NO_PROJECT;
      return h.form(
        () => useCases.saveDocVersion(repos, current.id, input, id),
        async (saved) => {
          await get().loadDocVersions();
          get().pushToast("success", id ? `Versión ${saved.version} actualizada.` : `Versión ${saved.version} registrada.`);
        },
      );
    },

    async deleteDocVersion(id) {
      const removed = get().docVersions.find((v) => v.id === id);
      try {
        await repos.docVersions.remove(id);
        await get().loadDocVersions();
        if (removed) get().pushToast("success", `Versión ${removed.version} eliminada.`);
      } catch (error) {
        h.report(error);
      }
    },

    rememberedTester: () => prefs.getTester(),
  };
}
```

En `src/ui/store/workspace.ts`, después de:

```ts
import { createEvidenceSlice, type EvidenceSlice } from "./evidence-slice";
```

agrega:

```ts
import { createExportSlice, type ExportSlice } from "./export-slice";
```

En `src/ui/store/workspace.ts`, reemplaza:

```ts
export interface WorkspaceState extends EvidenceSlice {
```

por:

```ts
export interface WorkspaceState extends EvidenceSlice, ExportSlice {
```

En `src/ui/store/workspace.ts`, después de:

```ts
      ...createEvidenceSlice(repos, set, get, { report, refreshCases, form }),
```

agrega:

```ts
      ...createExportSlice(repos, prefs, set, get, { report, refreshCases, form }),
```

En `src/ui/store/workspace.ts`, después de:

```ts
    if (state.selectedCaseId !== previous.selectedCaseId) void state.loadEvidence();
```

agrega:

```ts
    // Y las versiones del documento siguen al proyecto abierto.
    if (state.project?.id !== previous.project?.id) void state.loadDocVersions();
```

Run: `pnpm vitest run src/ui/store`
Expected: PASS.

- [ ] **Step 7: Test que falla — pestaña «Versiones»**

En `src/ui/test-utils.tsx`, reemplaza:

```tsx
lastProject?: string | null } = {}) {
```

por:

```tsx
lastProject?: string | null; tester?: string } = {}) {
```

En `src/ui/test-utils.tsx`, reemplaza:

```tsx
  const store = createWorkspaceStore(repos, memoryPreferences(options.lastProject ?? null));
```

por:

```tsx
  const store = createWorkspaceStore(repos, memoryPreferences(options.lastProject ?? null, options.tester ?? ""));
```

Crea `src/ui/screens/DocVersions.test.tsx`:

```tsx
import { screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { renderApp, seedProject } from "../test-utils";

async function openVersions(tester = "Ana Pérez") {
  const ctx = await renderApp({ tester, setup: async (repos) => void (await seedProject(repos)) });
  await ctx.user.click(screen.getByRole("button", { name: /Configurar/ }));
  const dialog = screen.getByRole("dialog", { name: "Configurar proyecto" });
  await ctx.user.click(within(dialog).getByRole("tab", { name: "Versiones" }));
  return { ...ctx, dialog };
}

describe("control de versiones del documento", () => {
  it("registra, edita y elimina versiones", async () => {
    const { user, dialog } = await openVersions();
    expect(within(dialog).getByText(/Sin versiones registradas/)).toBeInTheDocument();
    expect(within(dialog).getByLabelText(/^Versión/)).toHaveValue("1.0");
    expect(within(dialog).getByLabelText(/Elaborado por/)).toHaveValue("Ana Pérez");
    await user.type(within(dialog).getByLabelText("Cambios"), "Versión inicial");
    await user.click(within(dialog).getByRole("button", { name: "Registrar versión" }));

    const history = within(dialog).getByRole("region", { name: "Historial de versiones" });
    expect(await within(history).findByText("Versión inicial")).toBeInTheDocument();
    await waitFor(() => expect(within(dialog).getByLabelText(/^Versión/)).toHaveValue("1.1"));

    await user.click(within(history).getByRole("button", { name: "Editar versión 1.0" }));
    const changes = within(dialog).getByLabelText("Cambios");
    await user.clear(changes);
    await user.type(changes, "Se agregan casos de Pólizas");
    await user.click(within(dialog).getByRole("button", { name: "Guardar versión" }));
    expect(await within(history).findByText("Se agregan casos de Pólizas")).toBeInTheDocument();

    await user.click(within(history).getByRole("button", { name: "Eliminar versión 1.0" }));
    expect(await within(dialog).findByText(/Sin versiones registradas/)).toBeInTheDocument();
  });

  it("muestra los errores de validación y de versión repetida", async () => {
    const { user, dialog } = await openVersions("");
    await user.click(within(dialog).getByRole("button", { name: "Registrar versión" }));
    expect(within(dialog).getByText("«Elaborado por» es obligatorio.")).toBeInTheDocument();
    await user.type(within(dialog).getByLabelText(/Elaborado por/), "Ana");
    await user.click(within(dialog).getByRole("button", { name: "Registrar versión" }));
    await waitFor(() => expect(within(dialog).getByLabelText(/^Versión/)).toHaveValue("1.1"));
    const version = within(dialog).getByLabelText(/^Versión/);
    await user.clear(version);
    await user.type(version, "1.0");
    await user.click(within(dialog).getByRole("button", { name: "Registrar versión" }));
    expect(await within(dialog).findByText("La versión «1.0» ya existe en este proyecto.")).toBeInTheDocument();
  });
});
```

Run: `pnpm vitest run src/ui/screens/DocVersions.test.tsx`
Expected: FAIL (no existe la pestaña «Versiones»).

- [ ] **Step 8: Pestaña «Versiones»**

Crea `src/ui/screens/DocVersionsPanel.tsx`:

```tsx
import { Pencil, Trash2 } from "lucide-react";
import { useEffect, useState, type FormEvent } from "react";
import type { FormErrors } from "../../domain/cases";
import { emptyDocVersion, formatIsoDate, suggestNextVersion, todayIso, type DocVersion, type DocVersionInput } from "../../domain/doc-versions";
import { FormField } from "../components/FormField";
import { btn, iconButton, inputCls } from "../components/styles";
import { useWorkspace } from "../store/context";

const toInput = (v: DocVersion): DocVersionInput => ({
  version: v.version,
  date: v.date,
  author: v.author,
  reviewer: v.reviewer,
  approver: v.approver,
  changes: v.changes,
});

/** Control de versiones del documento (UC-05): alimenta la tabla de la carátula. */
export function DocVersionsPanel() {
  const docVersions = useWorkspace((s) => s.docVersions);
  const saveDocVersion = useWorkspace((s) => s.saveDocVersion);
  const deleteDocVersion = useWorkspace((s) => s.deleteDocVersion);
  const rememberedTester = useWorkspace((s) => s.rememberedTester);
  const blank = (): DocVersionInput => ({ ...emptyDocVersion(todayIso(), rememberedTester()), version: suggestNextVersion(docVersions) });
  const [editing, setEditing] = useState<string | null>(null);
  const [form, setForm] = useState<DocVersionInput>(blank);
  const [errors, setErrors] = useState<FormErrors>({});
  const [resets, setResets] = useState(0);
  useEffect(() => {
    // Al cargar o cambiar el historial, el formulario nuevo sugiere la siguiente versión.
    if (!editing) setForm((f) => (f.version ? f : { ...f, version: suggestNextVersion(docVersions) }));
  }, [docVersions, editing, resets]);
  const set = (key: keyof DocVersionInput) => (value: string) => setForm((f) => ({ ...f, [key]: value }));

  async function submit(e: FormEvent) {
    e.preventDefault();
    const result = await saveDocVersion(form, editing ?? undefined);
    if (!result.ok) {
      setErrors(result.errors);
      return;
    }
    setEditing(null);
    setErrors({});
    // Versión vacía: el efecto de arriba sugiere la siguiente con el historial ya recargado.
    setForm(emptyDocVersion(todayIso(), form.author));
    setResets((n) => n + 1);
  }

  function edit(v: DocVersion) {
    setEditing(v.id);
    setForm(toInput(v));
    setErrors({});
  }

  const field = (key: keyof DocVersionInput, label: string, required = false) => (
    <FormField label={label} required={required} htmlFor={`dv-${key}`} error={errors[key]}>
      <input id={`dv-${key}`} className={inputCls} value={form[key]} onChange={(e) => set(key)(e.target.value)} />
    </FormField>
  );

  return (
    <div className="space-y-6">
      <section aria-label="Historial de versiones">
        {docVersions.length === 0 ? (
          <p className="text-sm text-t4s-gray">Sin versiones registradas. La carátula del Excel muestra este historial.</p>
        ) : (
          <table className="w-full text-left text-sm">
            <thead className="text-xs text-t4s-gray">
              <tr>
                <th className="py-1">Versión</th>
                <th>Fecha</th>
                <th>Elaborado</th>
                <th>Revisado</th>
                <th>Aprobado</th>
                <th>Cambios</th>
                <th className="sr-only">Acciones</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-t4s-line">
              {docVersions.map((v) => (
                <tr key={v.id}>
                  <td className="py-2 font-medium">{v.version}</td>
                  <td>{formatIsoDate(v.date)}</td>
                  <td>{v.author}</td>
                  <td>{v.reviewer || "—"}</td>
                  <td>{v.approver || "—"}</td>
                  <td className="max-w-56 truncate" title={v.changes}>
                    {v.changes || "—"}
                  </td>
                  <td className="whitespace-nowrap text-right">
                    <button type="button" aria-label={`Editar versión ${v.version}`} className={iconButton} onClick={() => edit(v)}>
                      <Pencil size={16} />
                    </button>
                    <button type="button" aria-label={`Eliminar versión ${v.version}`} className={iconButton} onClick={() => void deleteDocVersion(v.id)}>
                      <Trash2 size={16} />
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>
      <form onSubmit={submit} noValidate className="grid grid-cols-3 gap-4 border-t border-t4s-line pt-4">
        <h3 className="col-span-3 text-sm font-medium">{editing ? `Editar versión ${form.version}` : "Nueva versión"}</h3>
        {field("version", "Versión", true)}
        <FormField label="Fecha" required htmlFor="dv-date" error={errors.date}>
          <input id="dv-date" type="date" className={inputCls} value={form.date} onChange={(e) => set("date")(e.target.value)} />
        </FormField>
        {field("author", "Elaborado por", true)}
        {field("reviewer", "Revisado por")}
        {field("approver", "Aprobado por")}
        <div />
        <FormField label="Cambios" htmlFor="dv-changes" error={errors.changes} className="col-span-3">
          <textarea id="dv-changes" rows={2} className={inputCls} value={form.changes} onChange={(e) => set("changes")(e.target.value)} />
        </FormField>
        {errors._form && (
          <p role="alert" className="col-span-3 text-sm text-red-700">
            {errors._form}
          </p>
        )}
        <div className="col-span-3 flex justify-end gap-2">
          {editing && (
            <button
              type="button"
              className={btn.ghost}
              onClick={() => {
                setEditing(null);
                setForm(blank());
                setErrors({});
              }}
            >
              Cancelar edición
            </button>
          )}
          <button type="submit" className={btn.primary}>
            {editing ? "Guardar versión" : "Registrar versión"}
          </button>
        </div>
      </form>
    </div>
  );
}
```

En `src/ui/screens/ProjectSettingsDialog.tsx`, después de:

```tsx
import { ColumnEditor } from "./ColumnEditor";
```

agrega:

```tsx
import { DocVersionsPanel } from "./DocVersionsPanel";
```

En `src/ui/screens/ProjectSettingsDialog.tsx`, reemplaza:

```tsx
const TABS = ["Datos", "Columnas", "Plantillas"] as const;
```

por:

```tsx
const TABS = ["Datos", "Columnas", "Versiones", "Plantillas"] as const;
```

En `src/ui/screens/ProjectSettingsDialog.tsx`, antes de:

```tsx
      {tab === "Plantillas" && <TemplatesPanel />}
```

agrega:

```tsx
      {tab === "Versiones" && <DocVersionsPanel />}
```

Run: `pnpm vitest run src/ui/screens/DocVersions.test.tsx src/ui/screens/ProjectSettings.test.tsx`
Expected: PASS.

- [ ] **Step 9: Verificación**

```bash
pnpm typecheck && pnpm test && pnpm build
```

Expected: todo en verde (158 tests).

- [ ] **Step 10: Commit**

```bash
git add src
git commit -m "feat(f4): control de versiones del documento en el front (pestaña Versiones)

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---


### Task 3: Base del Excel — dependencias, `AppError::Export`, datos del export (`excel/data.rs`) y ayudas de prueba (`excel/testing.rs`)

**Files:**
- Create: `src-tauri/src/excel/mod.rs`, `src-tauri/src/excel/data.rs`, `src-tauri/src/excel/testing.rs`
- Modify: `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `src-tauri/src/lib.rs`, `src-tauri/src/error.rs`, `src-tauri/src/dto.rs`

**Interfaces:**
- Consumes: `repo::{projects, cases, doc_versions}`, `media::storage::{EvidenceFiles, TempDir}`, `domain::{fields::is_iso_date, status::STATUSES, text::required_text}`, `repo::doc_versions::tests::input`.
- Produces:
  - `AppError::Export(String)`: código `"export"`, mensaje `No se pudo generar el Excel. Inténtalo nuevamente.`, y `impl From<rust_xlsxwriter::XlsxError> for AppError`.
  - `dto::ExportOptionsInput { tester: String, doc_version_id: Option<String>, include_evidence: bool, include_summary: bool, image_max_width: u32 }` (JSON camelCase, `deny_unknown_fields`).
  - `excel::data`:
    - `ExportEvidence { description, render_path: PathBuf }` y `ExportCase { case: CaseDto, evidence: Vec<ExportEvidence> }`.
    - `ExportData { project: ProjectDto /* solo columnas visibles, en orden */, cases: Vec<ExportCase>, versions: Vec<DocVersionDto> }`, con:
      - `ExportData::load(conn, files, project_id) -> AppResult<ExportData>`
      - `.field(key) -> Option<&FieldDto>`
      - `ExportData::value(&CaseDto, &FieldDto) -> &str`
      - `.evidence_total()`
      - `.distinct_values(key) -> Vec<String>`
    - `Kpis { total, by_status: [usize; 6], success_rate: Option<f64>, evidence }` con `Kpis::of(&ExportData)` y `.count(status)`.
    - `ExportSettings { tester, version: Option<DocVersionDto>, include_evidence, include_summary, image_max_width: u32, today: NaiveDate, template: bool }`.
    - `resolve_options(&ExportData, &ExportOptionsInput, template: bool, today: NaiveDate) -> AppResult<ExportSettings>`.
    - `IMAGE_WIDTH_RANGE = 400..=1600`.
    - `file_segment(raw, max) -> String`, `suggested_file_name(&ProjectDto, &ExportSettings) -> String`.
    - `parse_iso(&str) -> Option<NaiveDate>`, `format_dmy(NaiveDate) -> String`, `period(start, end) -> String`.
  - `excel::testing` (`#[cfg(test)]`):
    - Datos: `project()`, `fixture()` (4 casos: Aprobado, Aprobado, Fallido, No ejecutado; 3 evidencias), `case(&project, code, status, &[(key, value)])`, `evidence(description, path)`, `version(id, version, date)`, `write_png(dir, name, w, h, seed) -> PathBuf`.
    - `Book::save(&mut Workbook)` / `Book::at(path)`, con `sheet_names()`, `cell(sheet, row, col) -> calamine::Data`, `text(..)`, `row_texts(sheet, row, cols)`, `height(sheet)`, `find_row(sheet, text)`, `part(name)`, `sheet_xml(sheet)`, `hyperlinks(sheet) -> Vec<(celda, destino)>`, `pictures(sheet)`, `charts(sheet)`.

- [ ] **Step 1: Dependencias**

En `src-tauri/Cargo.toml`, después de `xcap = "0.9.8"` agrega:

```toml
# zip 9: crates.io retiró (yank) toda la serie zip < 9.0.3 y la 0.99.1 publicada todavía pide zip 8.
rust_xlsxwriter = { git = "https://github.com/jmcnamara/rust_xlsxwriter", rev = "ade0a017439d169b2a9c4848fe1cb66919c0ca3d" }
```

Y en `[dev-dependencies]`, después de `image = …`:

```toml
calamine = "0.36.1"
zip = { version = "9.0.4", default-features = false, features = ["deflate"] }

# calamine 0.36 (solo tests) pide zip ^8.6 y crates.io retiró toda la serie 8.x: se toma del tag.
[patch.crates-io]
zip8 = { package = "zip", git = "https://github.com/zip-rs/zip2", tag = "v8.6.0" }
```

Run: `cd src-tauri && cargo check --tests`
Expected: compila. En `Cargo.lock` aparecen dos `zip`: `9.0.4` (registry) y `8.6.0` (`git+https://github.com/zip-rs/zip2?tag=v8.6.0`). Si `cargo` dice que una versión está *yanked*, revisa que el `[patch]` haya quedado **después** de `[dev-dependencies]` y no dentro de `[profile.release]`.

- [ ] **Step 2: `AppError::Export` con su test**

En `src-tauri/src/error.rs`, reemplaza:

```rust
    #[error("captura: {0}")]
    Capture(String),
}
```

por:

```rust
    #[error("captura: {0}")]
    Capture(String),
    #[error("excel: {0}")]
    Export(String),
}

impl From<rust_xlsxwriter::XlsxError> for AppError {
    fn from(error: rust_xlsxwriter::XlsxError) -> Self {
        Self::Export(error.to_string())
    }
}
```

En `src-tauri/src/error.rs`, después de:

```rust
            Self::Capture(_) => "capture",
```

agrega:

```rust
            Self::Export(_) => "export",
```

En `src-tauri/src/error.rs`, después de:

```rust
            Self::Capture(_) => "No se pudo capturar la pantalla. Inténtalo nuevamente.".into(),
```

agrega:

```rust
            Self::Export(_) => "No se pudo generar el Excel. Inténtalo nuevamente.".into(),
```

En `src-tauri/src/error.rs`, después de:

```rust
        assert_eq!(json["detail"], "captura: xcap: monitor desconectado");
    }
```

agrega:

```rust
    #[test]
    fn excel_errors_have_their_own_code_and_a_friendly_message() {
        let export = serde_json::to_value(AppError::from(
            rust_xlsxwriter::XlsxError::SheetnameCannotBeBlank(String::new()),
        ))
        .unwrap();
        assert_eq!(export["code"], "export");
        assert_eq!(
            export["message"],
            "No se pudo generar el Excel. Inténtalo nuevamente."
        );
    }
```

Run: `cd src-tauri && cargo test error::`
Expected: PASS.

- [ ] **Step 3: Opciones del export (DTO)**

En `src-tauri/src/dto.rs`, antes de:

```rust
#[cfg(test)]
mod contract {
```

agrega:

```rust
/// Opciones del diálogo de exportación (UC-16 / UC-17).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportOptionsInput {
    pub tester: String,
    pub doc_version_id: Option<String>,
    pub include_evidence: bool,
    pub include_summary: bool,
    pub image_max_width: u32,
}
```

- [ ] **Step 4: Módulo `excel` y ayudas de prueba**

Crea `src-tauri/src/excel/mod.rs`:

```rust
//! Excel con carátula Teams4Soft (spec §7, UC-16 y UC-17), generado con `rust_xlsxwriter`.

// Hasta la Task 10 ningún comando usa este módulo: solo lo ejercitan los tests.
#![allow(dead_code)]

pub mod data;
#[cfg(test)]
pub(crate) mod testing;
```

En `src-tauri/src/lib.rs`, agrega `mod excel;` después de `mod error;`.

Crea `src-tauri/src/excel/testing.rs`:

```rust
//! Ayudas de prueba del libro: datos ficticios en memoria y lectura del xlsx con `calamine`
//! (celdas) y `zip` (vínculos, imágenes y gráficos, que calamine no expone).

use std::{
    collections::BTreeMap,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use calamine::{Data, Reader, Xlsx, open_workbook};
use rust_xlsxwriter::Workbook;

use super::data::{ExportCase, ExportData, ExportEvidence};
use crate::{
    domain::fields::FieldType,
    dto::{CaseDto, DocVersionDto, FieldDto, ProjectDto},
    media::storage::TempDir,
};

fn field(key: &str, label: &str, field_type: FieldType, options: &[&str], width: i64) -> FieldDto {
    FieldDto {
        id: format!("fld-{key}"),
        key: key.into(),
        label: label.into(),
        field_type,
        options: options.iter().map(|o| o.to_string()).collect(),
        required: false,
        clear_on_duplicate: false,
        width,
        position: 0,
        archived: false,
    }
}

pub fn project() -> ProjectDto {
    let mut fields = vec![
        field("entorno", "Entorno", FieldType::Text, &[], 21),
        field("modulo", "Módulo", FieldType::Text, &[], 22),
        field(
            "resultado_esperado",
            "Resultado esperado",
            FieldType::Longtext,
            &[],
            44,
        ),
        field(
            "resultado_obtenido",
            "Resultado obtenido",
            FieldType::Longtext,
            &[],
            44,
        ),
        field(
            "fecha_ejecucion",
            "Fecha de ejecución",
            FieldType::Date,
            &[],
            15,
        ),
        field(
            "severidad",
            "Severidad",
            FieldType::List,
            &["Alta", "Media", "Baja"],
            12,
        ),
        field("aceptado", "Aceptado", FieldType::Boolean, &[], 12),
        field("horas", "Horas", FieldType::Number, &[], 10),
    ];
    for (position, f) in fields.iter_mut().enumerate() {
        f.position = position as i64;
    }
    ProjectDto {
        id: "prj-1".into(),
        name: "Recibos".into(),
        client: "ACME".into(),
        test_type: "Funcional".into(),
        release_version: "2.4.0".into(),
        start_date: "2026-10-01".into(),
        end_date: "2026-10-31".into(),
        next_sequence: 5,
        source_template_id: Some("tpl-funcional".into()),
        created_at: String::new(),
        updated_at: String::new(),
        fields,
    }
}

/// Un caso con valores por `key` de columna.
pub fn case(project: &ProjectDto, code: &str, status: &str, values: &[(&str, &str)]) -> ExportCase {
    let values: BTreeMap<String, String> = values
        .iter()
        .map(|(key, value)| (format!("fld-{key}"), value.to_string()))
        .collect();
    ExportCase {
        case: CaseDto {
            id: format!("case-{code}"),
            project_id: project.id.clone(),
            code: code.into(),
            title: format!("Caso {code}"),
            status: status.into(),
            values,
            evidence_count: 0,
            created_at: String::new(),
            updated_at: String::new(),
        },
        evidence: Vec::new(),
    }
}

pub fn evidence(description: &str, path: &Path) -> ExportEvidence {
    ExportEvidence {
        description: description.into(),
        render_path: path.to_path_buf(),
    }
}

pub fn version(id: &str, version: &str, date: &str) -> DocVersionDto {
    DocVersionDto {
        id: id.into(),
        project_id: "prj-1".into(),
        version: version.into(),
        date: date.into(),
        author: "Ana Pérez".into(),
        reviewer: "Luis Gómez".into(),
        approver: String::new(),
        changes: "Versión inicial".into(),
        created_at: String::new(),
    }
}

/// 4 casos (2 aprobados, 1 fallido, 1 sin ejecutar) y 3 evidencias con rutas inexistentes:
/// sirve para todo lo que no lee imágenes.
pub fn fixture() -> ExportData {
    let project = project();
    let mut first = case(
        &project,
        "EJ-001",
        "Aprobado",
        &[
            ("entorno", "QA"),
            ("modulo", "Cobranzas"),
            ("resultado_esperado", "Se aplica el recibo"),
            ("resultado_obtenido", "Se aplicó el recibo"),
            ("fecha_ejecucion", "2026-10-16"),
            ("severidad", "Alta"),
            ("aceptado", "true"),
            ("horas", "1.5"),
        ],
    );
    first.evidence = vec![
        evidence("Listado de recibos", Path::new("no-existe-1.png")),
        evidence("Detalle del recibo", Path::new("no-existe-2.png")),
    ];
    let mut second = case(
        &project,
        "EJ-002",
        "Aprobado",
        &[("entorno", "UAT"), ("modulo", "Cobranzas")],
    );
    second.evidence = vec![evidence("Recibo en UAT", Path::new("no-existe-3.png"))];
    let third = case(
        &project,
        "EJ-003",
        "Fallido",
        &[("entorno", "QA"), ("modulo", "Pólizas")],
    );
    let fourth = case(&project, "EJ-004", "No ejecutado", &[("entorno", "QA")]);
    ExportData {
        project,
        cases: vec![first, second, third, fourth],
        versions: Vec::new(),
    }
}

/// PNG sintético con un patrón que cambia según `seed` (imágenes distintas no se deduplican en el xlsx).
pub fn write_png(dir: &Path, name: &str, width: u32, height: u32, seed: u8) -> PathBuf {
    let img = image::RgbImage::from_fn(width, height, |x, y| {
        image::Rgb([(x % 256) as u8, (y % 256) as u8, seed])
    });
    let path = dir.join(name);
    img.save(&path).expect("png de prueba");
    path
}

/// Un xlsx guardado en disco, listo para inspeccionar.
pub struct Book {
    path: PathBuf,
    _dir: Option<TempDir>,
}

impl Book {
    pub fn save(workbook: &mut Workbook) -> Self {
        let dir = TempDir::new();
        let path = dir.path().join("libro.xlsx");
        workbook.save(&path).expect("guardar xlsx");
        Self {
            path,
            _dir: Some(dir),
        }
    }

    pub fn at(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
            _dir: None,
        }
    }

    fn reader(&self) -> Xlsx<std::io::BufReader<File>> {
        open_workbook(&self.path).expect("calamine abre el libro")
    }

    pub fn sheet_names(&self) -> Vec<String> {
        self.reader().sheet_names()
    }

    pub fn cell(&self, sheet: &str, row: u32, col: u32) -> Data {
        self.reader()
            .worksheet_range(sheet)
            .expect("la hoja existe")
            .get_value((row, col))
            .cloned()
            .unwrap_or(Data::Empty)
    }

    pub fn text(&self, sheet: &str, row: u32, col: u32) -> String {
        self.cell(sheet, row, col).to_string()
    }

    pub fn row_texts(&self, sheet: &str, row: u32, cols: u32) -> Vec<String> {
        (0..cols).map(|col| self.text(sheet, row, col)).collect()
    }

    /// Filas con datos según calamine (la última fila usada + 1).
    pub fn height(&self, sheet: &str) -> u32 {
        self.reader()
            .worksheet_range(sheet)
            .expect("la hoja existe")
            .end()
            .map_or(0, |(row, _)| row + 1)
    }

    /// Busca la primera celda con ese texto exacto en la columna A.
    pub fn find_row(&self, sheet: &str, text: &str) -> Option<u32> {
        let range = self
            .reader()
            .worksheet_range(sheet)
            .expect("la hoja existe");
        let (end, _) = range.end()?;
        (0..=end).find(|row| {
            range
                .get_value((*row, 0))
                .is_some_and(|v| matches!(v, Data::String(s) if s == text))
        })
    }

    pub fn has_part(&self, name: &str) -> bool {
        let mut zip = zip::ZipArchive::new(File::open(&self.path).unwrap()).unwrap();
        zip.by_name(name).is_ok()
    }

    pub fn part(&self, name: &str) -> String {
        let mut zip = zip::ZipArchive::new(File::open(&self.path).unwrap()).unwrap();
        let mut out = String::new();
        zip.by_name(name)
            .unwrap_or_else(|_| panic!("falta {name}"))
            .read_to_string(&mut out)
            .unwrap();
        out
    }

    fn sheet_number(&self, sheet: &str) -> usize {
        self.sheet_names()
            .iter()
            .position(|s| s == sheet)
            .unwrap_or_else(|| panic!("no existe la hoja {sheet}"))
            + 1
    }

    pub fn sheet_xml(&self, sheet: &str) -> String {
        self.part(&format!(
            "xl/worksheets/sheet{}.xml",
            self.sheet_number(sheet)
        ))
    }

    /// `(celda, destino)` de cada vínculo interno de la hoja.
    pub fn hyperlinks(&self, sheet: &str) -> Vec<(String, String)> {
        self.sheet_xml(sheet)
            .split("<hyperlink ")
            .skip(1)
            .filter_map(|tag| Some((attr(tag, "ref")?, attr(tag, "location")?)))
            .collect()
    }

    fn drawing(&self, sheet: &str) -> Option<String> {
        let rels = format!(
            "xl/worksheets/_rels/sheet{}.xml.rels",
            self.sheet_number(sheet)
        );
        if !self.has_part(&rels) {
            return None;
        }
        let rels = self.part(&rels);
        let target = rels
            .split("Target=\"../drawings/")
            .nth(1)?
            .split('"')
            .next()?
            .to_string();
        Some(self.part(&format!("xl/drawings/{target}")))
    }

    pub fn pictures(&self, sheet: &str) -> usize {
        self.drawing(sheet)
            .map_or(0, |xml| xml.matches("<xdr:pic>").count())
    }

    pub fn charts(&self, sheet: &str) -> usize {
        self.drawing(sheet)
            .map_or(0, |xml| xml.matches("<xdr:graphicFrame").count())
    }
}

fn attr(tag: &str, name: &str) -> Option<String> {
    let key = format!("{name}=\"");
    let start = tag.find(&key)? + key.len();
    let end = tag[start..].find('"')? + start;
    Some(tag[start..end].to_string())
}
```

- [ ] **Step 5: Tests que fallan — `data.rs`**

Crea `src-tauri/src/excel/data.rs` solo con los tests:

```rust
#[cfg(test)]
mod tests {
    use rusqlite::params;

    use super::*;
    use crate::{
        excel::testing::{case, fixture, version},
        media::storage::TempDir,
        repo::{
            doc_versions::tests::input,
            test_support::{case as db_case, conn, field_id, project},
        },
    };

    fn options() -> ExportOptionsInput {
        ExportOptionsInput {
            tester: " Ana Pérez ".into(),
            doc_version_id: Some("ver-1".into()),
            include_evidence: true,
            include_summary: true,
            image_max_width: 940,
        }
    }

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 16).unwrap()
    }

    #[test]
    fn load_reads_visible_fields_cases_evidence_and_versions() {
        let conn = conn();
        let tmp = TempDir::new();
        let files = tmp.files();
        let p = project(&conn);
        conn.execute(
            "UPDATE project_field SET archived = 1 WHERE id = ?1",
            [field_id(&p, "menu")],
        )
        .unwrap();
        let first = db_case(&conn, &p);
        let second = db_case(&conn, &p);
        for (id, position, description) in [("e2", 1, "Detalle"), ("e1", 0, "Listado")] {
            conn.execute(
                "INSERT INTO evidence (id, case_id, position, description, original_path, rendered_path,
                                       annotations, width, height, captured_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, '', ?5, '{}', 10, 10, '', '')",
                params![id, first.id, position, description, format!("evidence/{}/{id}.render.png", p.id)],
            )
            .unwrap();
        }
        doc_versions::create(&conn, &p.id, &input("1.0", "2026-10-01")).unwrap();

        let data = ExportData::load(&conn, &files, &p.id).unwrap();
        assert_eq!(data.project.fields.len(), 10);
        assert!(data.field("menu").is_none());
        assert_eq!(data.cases.len(), 2);
        assert_eq!(data.cases[1].case.id, second.id);
        let descriptions: Vec<_> = data.cases[0]
            .evidence
            .iter()
            .map(|e| e.description.as_str())
            .collect();
        assert_eq!(descriptions, ["Listado", "Detalle"]);
        assert_eq!(
            data.cases[0].evidence[0].render_path,
            files.absolute(&format!("evidence/{}/e1.render.png", p.id))
        );
        assert!(data.cases[1].evidence.is_empty());
        assert_eq!(data.versions.len(), 1);
        assert_eq!(data.evidence_total(), 2);
        assert_eq!(
            ExportData::load(&conn, &files, "nope").unwrap_err().code(),
            "not_found"
        );
    }

    #[test]
    fn kpis_follow_the_success_rate_rule() {
        let mut data = fixture();
        let k = Kpis::of(&data);
        assert_eq!(k.total, 4);
        assert_eq!(k.count("Aprobado"), 2);
        assert_eq!(k.count("Fallido"), 1);
        assert_eq!(k.success_rate, Some(2.0 / 3.0));
        assert_eq!(k.evidence, 3);
        data.cases.retain(|c| c.case.status == "No ejecutado");
        assert_eq!(Kpis::of(&data).success_rate, None);
        data.cases.clear();
        assert_eq!(Kpis::of(&data).total, 0);
    }

    #[test]
    fn distinct_values_keep_first_appearance_and_skip_blanks() {
        let mut data = fixture();
        assert_eq!(data.distinct_values("entorno"), ["QA", "UAT"]);
        data.cases.push(case(
            &data.project,
            "EJ-005",
            "Aprobado",
            &[("entorno", "  ")],
        ));
        assert_eq!(data.distinct_values("entorno"), ["QA", "UAT"]);
        assert!(data.distinct_values("no_existe").is_empty());
    }

    #[test]
    fn resolve_options_validates_and_trims() {
        let mut data = fixture();
        data.versions.push(version("ver-1", "1.0", "2026-10-01"));
        let s = resolve_options(&data, &options(), false, today()).unwrap();
        assert_eq!(s.tester, "Ana Pérez");
        assert_eq!(s.version.as_ref().unwrap().version, "1.0");
        assert!(s.include_evidence && s.include_summary);

        let mut bad = options();
        bad.tester = " ".into();
        assert_eq!(
            resolve_options(&data, &bad, false, today())
                .unwrap_err()
                .to_string(),
            "«Tester» es obligatorio."
        );
        let mut bad = options();
        bad.image_max_width = 399;
        assert_eq!(
            resolve_options(&data, &bad, false, today())
                .unwrap_err()
                .to_string(),
            "El ancho máximo de imagen debe estar entre 400 y 1600 px."
        );
        let mut bad = options();
        bad.doc_version_id = None;
        assert_eq!(
            resolve_options(&data, &bad, false, today())
                .unwrap_err()
                .to_string(),
            "Elige la versión del documento."
        );
        bad.doc_version_id = Some("otra".into());
        assert_eq!(
            resolve_options(&data, &bad, false, today())
                .unwrap_err()
                .code(),
            "not_found"
        );
    }

    #[test]
    fn templates_need_no_version_and_skip_evidence_and_summary() {
        let data = fixture();
        let mut o = options();
        o.doc_version_id = None;
        let s = resolve_options(&data, &o, true, today()).unwrap();
        assert!(s.version.is_none());
        assert!(!s.include_evidence && !s.include_summary);
    }

    #[test]
    fn file_segments_are_safe_for_windows() {
        assert_eq!(file_segment("  Pólizas  de   Auto ", 60), "Pólizas-de-Auto");
        assert_eq!(file_segment("a<b>c:d\"e/f\\g|h?i*j", 60), "abcdefghij");
        assert_eq!(file_segment("...Recibos. ", 60), "Recibos");
        assert_eq!(file_segment("línea\nnueva\ttab", 60), "línea-nueva-tab");
        assert_eq!(file_segment(&"x".repeat(80), 40).len(), 40);
        assert_eq!(file_segment("???", 40), "");
    }

    #[test]
    fn suggested_names_follow_the_t4s_convention() {
        let mut data = fixture();
        data.versions.push(version("ver-1", "v1.0", "2026-10-01"));
        let s = resolve_options(&data, &options(), false, today()).unwrap();
        assert_eq!(
            suggested_file_name(&data.project, &s),
            "T4S_ACME_Recibos_Evidencias_20261016_v1.0.xlsx"
        );
        data.project.client = String::new();
        data.project.name = "Pólizas / Auto".into();
        assert_eq!(
            suggested_file_name(&data.project, &s),
            "T4S_Pólizas-Auto_Evidencias_20261016_v1.0.xlsx"
        );
        let mut o = options();
        o.doc_version_id = None;
        let t = resolve_options(&data, &o, true, today()).unwrap();
        assert_eq!(
            suggested_file_name(&data.project, &t),
            "T4S_Pólizas-Auto_Plantilla_20261016.xlsx"
        );
        data.project.name = "***".into();
        assert_eq!(
            suggested_file_name(&data.project, &t),
            "T4S_Proyecto_Plantilla_20261016.xlsx"
        );
    }

    #[test]
    fn periods_and_dates() {
        assert_eq!(
            period("2026-10-01", "2026-10-31"),
            "01/10/2026 – 31/10/2026"
        );
        assert_eq!(period("2026-10-01", ""), "Desde 01/10/2026");
        assert_eq!(period("", "2026-10-31"), "Hasta 31/10/2026");
        assert_eq!(period("", "basura"), "—");
        assert_eq!(parse_iso("2026-02-30"), None);
        assert_eq!(format_dmy(today()), "16/10/2026");
    }

    #[test]
    fn values_use_field_ids() {
        let conn = conn();
        let p = project(&conn);
        let c = db_case(&conn, &p);
        let entorno = p.fields.iter().find(|f| f.key == "entorno").unwrap();
        let modulo = p.fields.iter().find(|f| f.key == "modulo").unwrap();
        assert_eq!(ExportData::value(&c, entorno), "QA");
        assert_eq!(ExportData::value(&c, modulo), "");
    }
}
```

Run: `cd src-tauri && cargo test excel::data`
Expected: FAIL de compilación (`ExportData`, `Kpis`, `resolve_options`… no existen).

- [ ] **Step 6: Implementación**

Agrega al inicio de `src-tauri/src/excel/data.rs`:

```rust
use std::{collections::HashMap, ops::RangeInclusive, path::PathBuf};

use chrono::NaiveDate;
use rusqlite::Connection;

use crate::{
    domain::{fields::is_iso_date, status::STATUSES, text::required_text},
    dto::{CaseDto, DocVersionDto, ExportOptionsInput, FieldDto, ProjectDto},
    error::{AppError, AppResult, validation},
    media::storage::EvidenceFiles,
    repo::{cases, doc_versions, projects},
};

/// Ancho máximo de las capturas en la hoja Evidencias. El valor por defecto (940 px, el de la
/// Bitácora v1) lo pone el front; F6 lo hará configurable.
pub const IMAGE_WIDTH_RANGE: RangeInclusive<u32> = 400..=1600;

#[derive(Debug, Clone, PartialEq)]
pub struct ExportEvidence {
    pub description: String,
    pub render_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExportCase {
    pub case: CaseDto,
    pub evidence: Vec<ExportEvidence>,
}

/// Todo lo que necesita el libro, leído en una sola pasada para soltar la base de datos
/// antes de generar el archivo. `project.fields` solo trae las columnas visibles, en orden.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportData {
    pub project: ProjectDto,
    pub cases: Vec<ExportCase>,
    pub versions: Vec<DocVersionDto>,
}

impl ExportData {
    pub fn load(conn: &Connection, files: &EvidenceFiles, project_id: &str) -> AppResult<Self> {
        let mut project = projects::get(conn, project_id)?;
        project.fields.retain(|f| !f.archived);
        project.fields.sort_by_key(|f| f.position);
        let mut stmt = conn.prepare(
            "SELECT e.case_id, e.description, e.rendered_path FROM evidence e
             JOIN test_case c ON c.id = e.case_id
             WHERE c.project_id = ?1 ORDER BY e.position, e.captured_at",
        )?;
        let rows = stmt.query_map([project_id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?;
        let mut by_case: HashMap<String, Vec<ExportEvidence>> = HashMap::new();
        for row in rows {
            let (case_id, description, rel) = row?;
            by_case.entry(case_id).or_default().push(ExportEvidence {
                description,
                render_path: files.absolute(&rel),
            });
        }
        let cases = cases::list(conn, project_id)?
            .into_iter()
            .map(|case| ExportCase {
                evidence: by_case.remove(&case.id).unwrap_or_default(),
                case,
            })
            .collect();
        Ok(Self {
            project,
            cases,
            versions: doc_versions::list(conn, project_id)?,
        })
    }

    pub fn field(&self, key: &str) -> Option<&FieldDto> {
        self.project.fields.iter().find(|f| f.key == key)
    }

    pub fn value<'a>(case: &'a CaseDto, field: &FieldDto) -> &'a str {
        case.values.get(&field.id).map_or("", String::as_str)
    }

    pub fn evidence_total(&self) -> usize {
        self.cases.iter().map(|c| c.evidence.len()).sum()
    }

    /// Valores distintos y no vacíos de una columna, en orden de aparición.
    pub fn distinct_values(&self, key: &str) -> Vec<String> {
        let Some(field) = self.field(key) else {
            return Vec::new();
        };
        let mut seen = Vec::<String>::new();
        for c in &self.cases {
            let value = Self::value(&c.case, field).trim();
            if !value.is_empty() && !seen.iter().any(|s| s == value) {
                seen.push(value.to_string());
            }
        }
        seen
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Kpis {
    pub total: usize,
    /// En el orden de `STATUSES`.
    pub by_status: [usize; 6],
    /// Spec D15: Aprobado / (Aprobado + Fallido + Bloqueado); `None` si no hay veredictos.
    pub success_rate: Option<f64>,
    pub evidence: usize,
}

impl Kpis {
    pub fn of(data: &ExportData) -> Self {
        let mut by_status = [0; 6];
        for c in &data.cases {
            if let Some(i) = STATUSES.iter().position(|s| *s == c.case.status) {
                by_status[i] += 1;
            }
        }
        let mut kpis = Self {
            total: data.cases.len(),
            by_status,
            success_rate: None,
            evidence: data.evidence_total(),
        };
        let finished = kpis.count("Aprobado") + kpis.count("Fallido") + kpis.count("Bloqueado");
        if finished > 0 {
            kpis.success_rate = Some(kpis.count("Aprobado") as f64 / finished as f64);
        }
        kpis
    }

    pub fn count(&self, status: &str) -> usize {
        STATUSES
            .iter()
            .position(|s| *s == status)
            .map_or(0, |i| self.by_status[i])
    }
}

/// Opciones ya validadas y resueltas contra los datos del proyecto.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportSettings {
    pub tester: String,
    pub version: Option<DocVersionDto>,
    pub include_evidence: bool,
    pub include_summary: bool,
    pub image_max_width: u32,
    pub today: NaiveDate,
    pub template: bool,
}

pub fn resolve_options(
    data: &ExportData,
    input: &ExportOptionsInput,
    template: bool,
    today: NaiveDate,
) -> AppResult<ExportSettings> {
    let tester = required_text("Tester", &input.tester, 100)?;
    if !IMAGE_WIDTH_RANGE.contains(&input.image_max_width) {
        return Err(validation(format!(
            "El ancho máximo de imagen debe estar entre {} y {} px.",
            IMAGE_WIDTH_RANGE.start(),
            IMAGE_WIDTH_RANGE.end()
        )));
    }
    let version = match input.doc_version_id.as_deref() {
        Some(id) => Some(
            data.versions
                .iter()
                .find(|v| v.id == id)
                .cloned()
                .ok_or_else(|| AppError::NotFound("La versión del documento no existe.".into()))?,
        ),
        None if template => None,
        None => return Err(validation("Elige la versión del documento.")),
    };
    Ok(ExportSettings {
        tester,
        version,
        include_evidence: input.include_evidence && !template,
        include_summary: input.include_summary && !template,
        image_max_width: input.image_max_width,
        today,
        template,
    })
}

/// Un tramo seguro para el nombre de archivo en Windows: sin `<>:"/\|?*` ni controles,
/// espacios convertidos en `-` y sin puntos ni guiones en los extremos.
pub fn file_segment(raw: &str, max: usize) -> String {
    let mut out = String::new();
    for ch in raw.trim().chars() {
        if ch.is_whitespace() {
            if !out.ends_with('-') {
                out.push('-');
            }
        } else if !ch.is_control() && !"<>:\"/\\|?*".contains(ch) {
            out.push(ch);
        }
    }
    let limited: String = out.chars().take(max).collect();
    limited.trim_matches(['-', '.', ' ']).to_string()
}

/// `T4S_<Cliente>_<Proyecto>_Evidencias_<AAAAMMDD>_v<versión>.xlsx` (spec §6.5);
/// la plantilla vacía usa `Plantilla` y no lleva versión.
pub fn suggested_file_name(project: &ProjectDto, settings: &ExportSettings) -> String {
    let mut parts = vec!["T4S".to_string()];
    let client = file_segment(&project.client, 40);
    if !client.is_empty() {
        parts.push(client);
    }
    let name = file_segment(&project.name, 60);
    parts.push(if name.is_empty() {
        "Proyecto".into()
    } else {
        name
    });
    parts.push(
        if settings.template {
            "Plantilla"
        } else {
            "Evidencias"
        }
        .into(),
    );
    parts.push(settings.today.format("%Y%m%d").to_string());
    if let (false, Some(v)) = (settings.template, &settings.version) {
        let version = file_segment(v.version.trim_start_matches(['v', 'V']), 20);
        if !version.is_empty() {
            parts.push(format!("v{version}"));
        }
    }
    format!("{}.xlsx", parts.join("_"))
}

pub fn parse_iso(value: &str) -> Option<NaiveDate> {
    if is_iso_date(value) {
        NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()
    } else {
        None
    }
}

pub fn format_dmy(date: NaiveDate) -> String {
    date.format("%d/%m/%Y").to_string()
}

pub fn period(start: &str, end: &str) -> String {
    let show = |v: &str| parse_iso(v).map(format_dmy);
    match (show(start), show(end)) {
        (Some(s), Some(e)) => format!("{s} – {e}"),
        (Some(s), None) => format!("Desde {s}"),
        (None, Some(e)) => format!("Hasta {e}"),
        (None, None) => "—".into(),
    }
}
```

Run: `cd src-tauri && cargo test excel::data`
Expected: PASS (9 tests).

- [ ] **Step 7: Verificación y commit**

```bash
cd src-tauri && cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test && cd ..
git add src-tauri
git commit -m "feat(f4): base del Excel — datos del export, opciones, nombre sugerido y lectura con calamine

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---


### Task 4: Estilos T4S (`excel/styles.rs`)

**Files:**
- Create: `src-tauri/src/excel/styles.rs`
- Modify: `src-tauri/src/excel/mod.rs`

**Interfaces:**
- Consumes: `excel::testing::Book`.
- Produces:
  - Constantes `FONT`, `BLUE`, `LIME`, `CHARCOAL`, `CREAM`, `GRAY`, `LINE`, `WHITE`, `ROW_PX = 17` (alto por defecto de una fila con Arial 10), `DATE_FORMAT = "dd/mm/yyyy"`, `PERCENT_FORMAT = "0 %"`.
  - `STATUS_COLORS: [(estado, relleno, fuente); 3]` (Aprobado, Fallido y Bloqueado) y `status_format(fill, font) -> Format`.
  - `workbook() -> AppResult<Workbook>`: libro con Arial 10 como formato por defecto. **Todo libro de F4 se crea con esta función.**
  - `Styles::new()`, con los campos públicos `title`, `subtitle`, `section`, `header`, `label`, `value`, `kpi_label`, `kpi_value`, `kpi_rate`, `band`, `band_soft`, `text`, `link`, `number`, `percent`, `total` y `footer`, más `row(index) -> &RowStyles { text, date, number, link }` (las filas alternas llevan fondo crema).
  - `text_lines(text, width) -> u32` y `wrapped_height(iter<(&str, width)>, min) -> f64`: alto de fila entre `min` y 409.

- [ ] **Step 1: Tests que fallan**

En `src-tauri/src/excel/mod.rs`, agrega `pub mod styles;` después de `pub mod data;`. Crea `src-tauri/src/excel/styles.rs` solo con los tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::excel::testing::Book;

    #[test]
    fn every_font_is_arial_and_the_palette_is_t4s() {
        let styles = Styles::new();
        let mut workbook = workbook().unwrap();
        let ws = workbook.add_worksheet();
        let formats = [
            &styles.title,
            &styles.header,
            &styles.label,
            &styles.kpi_label,
            &styles.kpi_rate,
            &styles.band_soft,
            &styles.footer,
            &styles.row(1).date,
        ];
        for (row, format) in formats.into_iter().enumerate() {
            ws.write_string_with_format(row as u32, 0, "x", format)
                .unwrap();
        }
        let xml = Book::save(&mut workbook).part("xl/styles.xml");
        let fonts: Vec<_> = xml
            .match_indices("<name val=\"")
            .map(|(i, _)| &xml[i + 11..i + 16])
            .collect();
        assert!(!fonts.is_empty());
        assert!(fonts.iter().all(|f| *f == "Arial"), "{fonts:?}");
        for color in ["FF2B629E", "FFBAC82D", "FF292B2D", "FFF6F4F1", "FF596069"] {
            assert!(xml.contains(color), "falta {color}");
        }
        assert!(xml.contains("formatCode=\"dd/mm/yyyy\""));
        assert!(xml.contains("formatCode=\"0 %\""));
    }

    #[test]
    fn alternate_rows_switch_to_cream() {
        let styles = Styles::new();
        assert!(styles.row(0).text != styles.row(1).text);
        assert!(styles.row(0).text == styles.row(2).text);
    }

    #[test]
    fn heights_grow_with_text_and_stop_at_excel_limit() {
        assert_eq!(text_lines("", 10.0), 1);
        assert_eq!(text_lines(&"a".repeat(22), 10.0), 2);
        assert_eq!(text_lines("uno\ndos\ntres", 40.0), 3);
        assert_eq!(wrapped_height([("corto", 40.0)], 18.0), 19.0);
        assert_eq!(wrapped_height([("a\nb\nc", 40.0), ("x", 40.0)], 18.0), 45.0);
        assert_eq!(
            wrapped_height([(&"a\n".repeat(100) as &str, 40.0)], 18.0),
            409.0
        );
    }
}
```

Run: `cd src-tauri && cargo test excel::styles`
Expected: FAIL de compilación.

- [ ] **Step 2: Implementación**

Agrega al inicio de `src-tauri/src/excel/styles.rs`:

```rust
use rust_xlsxwriter::{Format, FormatAlign, FormatBorder, FormatUnderline, Workbook};

use crate::error::AppResult;

// Paleta Teams4Soft (spec §1). Todo el libro usa Arial: es segura en Excel y WPS.
pub const FONT: &str = "Arial";
pub const BLUE: &str = "#2B629E";
pub const LIME: &str = "#BAC82D";
pub const CHARCOAL: &str = "#292B2D";
pub const CREAM: &str = "#F6F4F1";
pub const GRAY: &str = "#596069";
pub const LINE: &str = "#E3E0DA";
pub const WHITE: &str = "#FFFFFF";

/// Alto por defecto de una fila con Arial 10, en píxeles (12,75 pt).
pub const ROW_PX: u32 = 17;

pub const DATE_FORMAT: &str = "dd/mm/yyyy";
pub const PERCENT_FORMAT: &str = "0 %";

/// Formato condicional de la columna Estado (spec §7): Aprobado verde, Fallido rojo, Bloqueado amarillo.
pub const STATUS_COLORS: [(&str, &str, &str); 3] = [
    ("Aprobado", "#C6EFCE", "#006100"),
    ("Fallido", "#FFC7CE", "#9C0006"),
    ("Bloqueado", "#FFEB9C", "#9C5700"),
];

pub fn status_format(fill: &str, font: &str) -> Format {
    Format::new()
        .set_background_color(fill)
        .set_font_color(font)
}

/// Formatos de una fila de la matriz; las filas alternas llevan fondo crema.
#[derive(Clone)]
pub struct RowStyles {
    pub text: Format,
    pub date: Format,
    pub number: Format,
    pub link: Format,
}

pub struct Styles {
    pub title: Format,
    pub subtitle: Format,
    pub section: Format,
    pub header: Format,
    pub label: Format,
    pub value: Format,
    pub kpi_label: Format,
    pub kpi_value: Format,
    pub kpi_rate: Format,
    pub band: Format,
    pub band_soft: Format,
    pub text: Format,
    pub link: Format,
    pub number: Format,
    pub percent: Format,
    pub total: Format,
    pub footer: Format,
    rows: [RowStyles; 2],
}

/// Libro con Arial 10 como formato por defecto, también en las celdas vacías que el usuario
/// llene después. Arial 10 usa 7 px por dígito: ancho de columna por defecto 64 px.
pub fn workbook() -> AppResult<Workbook> {
    let mut workbook = Workbook::new();
    workbook.set_default_format(
        &Format::new().set_font_name(FONT).set_font_size(10),
        ROW_PX,
        64,
    )?;
    Ok(workbook)
}

fn base() -> Format {
    Format::new()
        .set_font_name(FONT)
        .set_font_size(10)
        .set_font_color(CHARCOAL)
}

fn boxed() -> Format {
    base().set_border(FormatBorder::Thin).set_border_color(LINE)
}

fn row_styles(fill: Option<&str>) -> RowStyles {
    let cell = || {
        let f = boxed().set_text_wrap().set_align(FormatAlign::Top);
        match fill {
            Some(color) => f.set_background_color(color),
            None => f,
        }
    };
    RowStyles {
        text: cell(),
        date: cell()
            .set_num_format(DATE_FORMAT)
            .set_align(FormatAlign::Left),
        number: cell().set_align(FormatAlign::Right),
        link: cell()
            .set_font_color(BLUE)
            .set_underline(FormatUnderline::Single),
    }
}

impl Styles {
    pub fn new() -> Self {
        let number = boxed().set_align(FormatAlign::Center);
        let kpi_value = base()
            .set_font_size(20)
            .set_bold()
            .set_font_color(BLUE)
            .set_background_color(CREAM)
            .set_align(FormatAlign::Center)
            .set_align(FormatAlign::VerticalCenter);
        Self {
            title: base().set_font_size(18).set_bold().set_font_color(BLUE),
            subtitle: base().set_italic().set_font_color(GRAY),
            section: base()
                .set_bold()
                .set_font_color(WHITE)
                .set_background_color(BLUE)
                .set_align(FormatAlign::VerticalCenter)
                .set_indent(1),
            header: base()
                .set_bold()
                .set_font_color(WHITE)
                .set_background_color(BLUE)
                .set_text_wrap()
                .set_align(FormatAlign::VerticalCenter)
                .set_border(FormatBorder::Thin)
                .set_border_color(WHITE),
            label: boxed()
                .set_bold()
                .set_font_color(BLUE)
                .set_background_color(CREAM)
                .set_align(FormatAlign::VerticalCenter)
                .set_indent(1),
            value: boxed()
                .set_text_wrap()
                .set_align(FormatAlign::VerticalCenter)
                .set_indent(1),
            kpi_label: base()
                .set_font_size(9)
                .set_font_color(GRAY)
                .set_background_color(CREAM)
                .set_align(FormatAlign::Center)
                .set_border_top(FormatBorder::Medium)
                .set_border_top_color(LIME),
            kpi_rate: kpi_value.clone().set_num_format(PERCENT_FORMAT),
            kpi_value,
            band: base()
                .set_bold()
                .set_font_color(WHITE)
                .set_background_color(BLUE)
                .set_align(FormatAlign::VerticalCenter)
                .set_indent(1),
            band_soft: base()
                .set_bold()
                .set_font_color(BLUE)
                .set_background_color(CREAM)
                .set_align(FormatAlign::VerticalCenter)
                .set_indent(1),
            text: base().set_text_wrap().set_align(FormatAlign::Top),
            link: base()
                .set_font_color(BLUE)
                .set_underline(FormatUnderline::Single),
            percent: number.clone().set_num_format(PERCENT_FORMAT),
            total: number.clone().set_bold().set_background_color(CREAM),
            number,
            footer: base()
                .set_font_size(8)
                .set_italic()
                .set_font_color(GRAY)
                .set_align(FormatAlign::Center),
            rows: [row_styles(None), row_styles(Some(CREAM))],
        }
    }

    /// Formatos de la fila de datos `index` (0 = primera fila de datos).
    pub fn row(&self, index: usize) -> &RowStyles {
        &self.rows[index % 2]
    }
}

/// Líneas que ocupa un texto envuelto en una columna de `width` caracteres (Arial 10).
pub fn text_lines(text: &str, width: f64) -> u32 {
    let per_line = (width * 1.1).max(1.0) as usize;
    text.split('\n')
        .map(|line| line.chars().count().div_ceil(per_line).max(1) as u32)
        .sum()
}

/// Alto de fila (puntos) para que se lean los textos dados; Excel admite hasta 409.
pub fn wrapped_height<'a>(texts: impl IntoIterator<Item = (&'a str, f64)>, min: f64) -> f64 {
    let lines = texts
        .into_iter()
        .map(|(text, width)| text_lines(text, width))
        .max()
        .unwrap_or(1);
    (f64::from(lines) * 13.0 + 6.0).clamp(min, 409.0)
}
```

Run: `cd src-tauri && cargo test excel::styles`
Expected: PASS (3 tests). El primero comprueba en `xl/styles.xml` que **todas** las fuentes son Arial (incluida la por defecto) y que está la paleta T4S.

- [ ] **Step 3: Verificación y commit**

```bash
cd src-tauri && cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test && cd ..
git add src-tauri/src/excel
git commit -m "feat(f4): estilos T4S del Excel (Arial, paleta y alturas de fila)

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: Carátula (`excel/cover.rs`)

**Files:**
- Create: `src-tauri/src/excel/cover.rs`
- Modify: `src-tauri/src/excel/mod.rs`

**Interfaces:**
- Consumes:
  - `data::{ExportData, ExportSettings, Kpis, format_dmy, parse_iso, period}`
  - `styles::{self, Styles, wrapped_height, workbook}`
  - `testing::{Book, fixture, version}`
  - `src/assets/brand/t4s-logo.png` y `t4s-network-banner.png`
- Produces:
  - `cover::SHEET = "Carátula"`, `cover::FOOTER`, `cover::TITLE_ROW = 7`, `cover::DATA_ROW = 10`.
  - `cover::write(ws, &ExportData, &ExportSettings, &Styles) -> AppResult<()>`.
  - Diseño (columnas A–H):
    - logo en A1 y banner en E1;
    - título y cliente · proyecto (filas 8 y 9);
    - «Datos del proyecto» con 8 filas etiqueta | valor;
    - «Resumen de resultados» (solo si no es plantilla): 4 KPI (Ejecuciones, % de éxito, Evidencias, Con incidencias) y los 6 estados con su conteo;
    - «Control de versiones»;
    - pie de página.

- [ ] **Step 1: Tests que fallan**

En `src-tauri/src/excel/mod.rs`, agrega `pub mod cover;` antes de `pub mod data;`. Crea `src-tauri/src/excel/cover.rs` solo con los tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::excel::testing::{Book, fixture, version};
    use calamine::Data;
    use chrono::NaiveDate;

    fn settings(template: bool) -> ExportSettings {
        ExportSettings {
            tester: "Ana Pérez".into(),
            version: None,
            include_evidence: true,
            include_summary: true,
            image_max_width: 940,
            today: NaiveDate::from_ymd_opt(2026, 10, 16).unwrap(),
            template,
        }
    }

    fn render(data: &ExportData, template: bool) -> Book {
        let mut workbook = styles::workbook().unwrap();
        write(
            workbook.add_worksheet(),
            data,
            &settings(template),
            &Styles::new(),
        )
        .unwrap();
        Book::save(&mut workbook)
    }

    #[test]
    fn shows_logo_banner_title_and_project_data() {
        let book = render(&fixture(), false);
        assert_eq!(book.sheet_names(), [SHEET]);
        assert_eq!(book.pictures(SHEET), 2);
        assert_eq!(
            book.text(SHEET, TITLE_ROW, 0),
            "Matriz de pruebas y evidencias"
        );
        assert_eq!(book.text(SHEET, TITLE_ROW + 1, 0), "ACME · Recibos");
        let data: Vec<_> = (1..=8)
            .map(|i| {
                (
                    book.text(SHEET, DATA_ROW + i, 0),
                    book.text(SHEET, DATA_ROW + i, 2),
                )
            })
            .collect();
        let expected = [
            ("Cliente", "ACME"),
            ("Proyecto / Módulo", "Recibos"),
            ("Tipo de prueba", "Funcional"),
            ("Versión / release probado", "2.4.0"),
            ("Entorno(s)", "QA, UAT"),
            ("Periodo", "01/10/2026 – 31/10/2026"),
            ("Tester", "Ana Pérez"),
            ("Fecha de generación", "16/10/2026"),
        ];
        for ((label, value), (exp_label, exp_value)) in data.iter().zip(expected) {
            assert_eq!((label.as_str(), value.as_str()), (exp_label, exp_value));
        }
    }

    #[test]
    fn kpis_show_totals_rate_evidence_and_each_status() {
        let book = render(&fixture(), false);
        let start = book.find_row(SHEET, "Resumen de resultados").unwrap();
        assert_eq!(
            book.row_texts(SHEET, start + 1, 8),
            [
                "Ejecuciones",
                "",
                "% de éxito",
                "",
                "Evidencias",
                "",
                "Con incidencias",
                ""
            ]
        );
        assert_eq!(book.cell(SHEET, start + 2, 0), Data::Float(4.0));
        assert_eq!(book.cell(SHEET, start + 2, 2), Data::Float(2.0 / 3.0));
        assert_eq!(book.cell(SHEET, start + 2, 4), Data::Float(3.0));
        assert_eq!(book.cell(SHEET, start + 2, 6), Data::Float(1.0));
        assert_eq!(book.text(SHEET, start + 4, 2), "No ejecutado");
        assert_eq!(book.cell(SHEET, start + 5, 2), Data::Float(1.0));
        assert_eq!(book.text(SHEET, start + 4, 5), "Aprobado");
        assert_eq!(book.cell(SHEET, start + 5, 5), Data::Float(2.0));
    }

    #[test]
    fn rate_is_a_dash_without_verdicts() {
        let mut data = fixture();
        data.cases.retain(|c| c.case.status == "No ejecutado");
        let book = render(&data, false);
        let start = book.find_row(SHEET, "Resumen de resultados").unwrap();
        assert_eq!(book.text(SHEET, start + 2, 2), "—");
    }

    #[test]
    fn version_table_lists_the_history_and_footer_closes_the_page() {
        let mut data = fixture();
        data.versions = vec![
            version("v1", "1.0", "2026-10-01"),
            version("v2", "1.1", "2026-10-15"),
        ];
        let book = render(&data, false);
        let start = book.find_row(SHEET, "Control de versiones").unwrap();
        assert_eq!(
            book.row_texts(SHEET, start + 1, 6),
            [
                "Versión",
                "Fecha",
                "Elaborado",
                "Revisado",
                "Aprobado",
                "Cambios"
            ]
        );
        assert_eq!(
            book.row_texts(SHEET, start + 2, 6),
            [
                "1.0",
                "01/10/2026",
                "Ana Pérez",
                "Luis Gómez",
                "—",
                "Versión inicial"
            ]
        );
        assert_eq!(book.text(SHEET, start + 3, 0), "1.1");
        assert_eq!(book.text(SHEET, start + 5, 0), FOOTER);
        assert!(
            book.sheet_xml(SHEET)
                .contains("Teams4Soft · Inteligencia Híbrida · Confidencial</oddFooter>")
        );
    }

    #[test]
    fn empty_values_show_a_dash_and_no_versions_is_explicit() {
        let mut data = fixture();
        data.project.client = String::new();
        data.project.start_date = String::new();
        data.project.end_date = String::new();
        let book = render(&data, false);
        assert_eq!(book.text(SHEET, TITLE_ROW + 1, 0), "Recibos");
        assert_eq!(book.text(SHEET, DATA_ROW + 1, 2), "—");
        assert_eq!(book.text(SHEET, DATA_ROW + 6, 2), "—");
        let start = book.find_row(SHEET, "Control de versiones").unwrap();
        assert_eq!(book.text(SHEET, start + 2, 0), "Sin versiones registradas.");
    }

    #[test]
    fn template_cover_has_no_results() {
        let book = render(&fixture(), true);
        assert_eq!(book.text(SHEET, TITLE_ROW, 0), "Plantilla de pruebas");
        assert!(book.find_row(SHEET, "Resumen de resultados").is_none());
        assert!(book.find_row(SHEET, "Control de versiones").is_some());
    }
}
```

Run: `cd src-tauri && cargo test excel::cover`
Expected: FAIL de compilación.

- [ ] **Step 2: Implementación**

Agrega al inicio de `src-tauri/src/excel/cover.rs`:

```rust
use rust_xlsxwriter::{Image, Worksheet};

use super::{
    data::{ExportData, ExportSettings, Kpis, format_dmy, parse_iso, period},
    styles::{self, Styles, wrapped_height},
};
use crate::{domain::status::STATUSES, dto::DocVersionDto, error::AppResult};

pub const SHEET: &str = "Carátula";
pub const FOOTER: &str = "Teams4Soft · Inteligencia Híbrida · Confidencial";
pub const TITLE_ROW: u32 = 7;
/// Fila de la banda «Datos del proyecto»; los 8 datos van debajo.
pub const DATA_ROW: u32 = 10;

// El logo y el banner van dentro del binario (spec §7): el Excel no depende de archivos externos.
const LOGO: &[u8] = include_bytes!("../../../src/assets/brand/t4s-logo.png");
const BANNER: &[u8] = include_bytes!("../../../src/assets/brand/t4s-network-banner.png");
const LAST_COL: u16 = 7;
const WIDTHS: [f64; 8] = [16.0, 14.0, 18.0, 18.0, 18.0, 14.0, 14.0, 14.0];

pub fn write(
    ws: &mut Worksheet,
    data: &ExportData,
    settings: &ExportSettings,
    styles: &Styles,
) -> AppResult<()> {
    ws.set_name(SHEET)?;
    ws.set_screen_gridlines(false);
    ws.set_tab_color(styles::BLUE);
    for (col, width) in WIDTHS.iter().enumerate() {
        ws.set_column_width(col as u16, *width)?;
    }
    let logo = Image::new_from_buffer(LOGO)?
        .set_scale_to_size(80, 92, true)
        .set_alt_text("Teams4Soft · Inteligencia Híbrida");
    ws.insert_image_with_offset(0, 0, &logo, 8, 8)?;
    let banner = Image::new_from_buffer(BANNER)?
        .set_scale_to_size(430, 140, true)
        .set_decorative(true);
    ws.insert_image(0, 4, &banner)?;

    let title = if settings.template {
        "Plantilla de pruebas"
    } else {
        "Matriz de pruebas y evidencias"
    };
    ws.set_row_height(TITLE_ROW, 30)?;
    ws.merge_range(TITLE_ROW, 0, TITLE_ROW, LAST_COL, title, &styles.title)?;
    let p = &data.project;
    let heading = if p.client.is_empty() {
        p.name.clone()
    } else {
        format!("{} · {}", p.client, p.name)
    };
    ws.merge_range(
        TITLE_ROW + 1,
        0,
        TITLE_ROW + 1,
        LAST_COL,
        &heading,
        &styles.subtitle,
    )?;

    section(ws, DATA_ROW, "Datos del proyecto", styles)?;
    let environments = data.distinct_values("entorno").join(", ");
    let dates = period(&p.start_date, &p.end_date);
    let generated = format_dmy(settings.today);
    let rows = [
        ("Cliente", p.client.as_str()),
        ("Proyecto / Módulo", p.name.as_str()),
        ("Tipo de prueba", p.test_type.as_str()),
        ("Versión / release probado", p.release_version.as_str()),
        ("Entorno(s)", environments.as_str()),
        ("Periodo", dates.as_str()),
        ("Tester", settings.tester.as_str()),
        ("Fecha de generación", generated.as_str()),
    ];
    let mut row = DATA_ROW;
    for (label, value) in rows {
        row += 1;
        ws.set_row_height(row, wrapped_height([(value, 90.0)], 20.0))?;
        ws.merge_range(row, 0, row, 1, label, &styles.label)?;
        ws.merge_range(row, 2, row, LAST_COL, or_dash(value), &styles.value)?;
    }
    row += 2;
    if !settings.template {
        row = kpis(ws, row, data, styles)? + 2;
    }
    row = versions(ws, row, &data.versions, styles)? + 2;
    ws.merge_range(row, 0, row, LAST_COL, FOOTER, &styles.footer)?;
    ws.set_footer(format!("&C&\"Arial\"&8{FOOTER}"));
    ws.set_paper_size(9).set_print_fit_to_pages(1, 1);
    Ok(())
}

fn or_dash(value: &str) -> &str {
    if value.trim().is_empty() {
        "—"
    } else {
        value
    }
}

fn section(ws: &mut Worksheet, row: u32, title: &str, styles: &Styles) -> AppResult<()> {
    ws.set_row_height(row, 22)?;
    ws.merge_range(row, 0, row, LAST_COL, title, &styles.section)?;
    Ok(())
}

/// Bloque de KPI; devuelve la última fila que usa.
fn kpis(ws: &mut Worksheet, start: u32, data: &ExportData, styles: &Styles) -> AppResult<u32> {
    let k = Kpis::of(data);
    section(ws, start, "Resumen de resultados", styles)?;
    let (labels, values) = (start + 1, start + 2);
    ws.set_row_height(values, 34)?;
    let incidents = (k.count("Fallido") + k.count("Bloqueado")) as f64;
    let tiles = [
        ("Ejecuciones", Some(k.total as f64), &styles.kpi_value),
        ("% de éxito", k.success_rate, &styles.kpi_rate),
        ("Evidencias", Some(k.evidence as f64), &styles.kpi_value),
        ("Con incidencias", Some(incidents), &styles.kpi_value),
    ];
    for (i, (label, value, format)) in tiles.into_iter().enumerate() {
        let col = (i * 2) as u16;
        ws.merge_range(labels, col, labels, col + 1, label, &styles.kpi_label)?;
        ws.merge_range(values, col, values, col + 1, "", format)?;
        match value {
            Some(v) => ws.write_number_with_format(values, col, v, format)?,
            None => ws.write_string_with_format(values, col, "—", format)?,
        };
    }
    let (header, counts) = (start + 4, start + 5);
    ws.set_row_height(header, 28)?;
    ws.merge_range(header, 0, counts, 1, "Por estado", &styles.label)?;
    for (i, status) in STATUSES.iter().enumerate() {
        let col = 2 + i as u16;
        ws.write_string_with_format(header, col, *status, &styles.header)?;
        ws.write_number_with_format(counts, col, k.by_status[i] as f64, &styles.number)?;
    }
    Ok(counts)
}

/// Tabla de control de versiones (UC-05); devuelve la última fila que usa.
fn versions(
    ws: &mut Worksheet,
    start: u32,
    versions: &[DocVersionDto],
    styles: &Styles,
) -> AppResult<u32> {
    section(ws, start, "Control de versiones", styles)?;
    let head = start + 1;
    for (col, title) in ["Versión", "Fecha", "Elaborado", "Revisado", "Aprobado"]
        .into_iter()
        .enumerate()
    {
        ws.write_string_with_format(head, col as u16, title, &styles.header)?;
    }
    ws.merge_range(head, 5, head, LAST_COL, "Cambios", &styles.header)?;
    if versions.is_empty() {
        ws.merge_range(
            head + 1,
            0,
            head + 1,
            LAST_COL,
            "Sin versiones registradas.",
            &styles.value,
        )?;
        return Ok(head + 1);
    }
    let mut row = head;
    for v in versions {
        row += 1;
        let date = parse_iso(&v.date).map_or_else(|| v.date.clone(), format_dmy);
        let cells = [
            v.version.as_str(),
            date.as_str(),
            v.author.as_str(),
            v.reviewer.as_str(),
            v.approver.as_str(),
        ];
        for (col, text) in cells.into_iter().enumerate() {
            ws.write_string_with_format(row, col as u16, or_dash(text), &styles.value)?;
        }
        ws.merge_range(row, 5, row, LAST_COL, or_dash(&v.changes), &styles.value)?;
        ws.set_row_height(row, wrapped_height([(v.changes.as_str(), 42.0)], 20.0))?;
    }
    Ok(row)
}
```

Run: `cd src-tauri && cargo test excel::cover`
Expected: PASS (6 tests).

- [ ] **Step 3: Verificación y commit**

```bash
cd src-tauri && cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test && cd ..
git add src-tauri/src/excel
git commit -m "feat(f4): carátula con logo, datos del proyecto, KPI y control de versiones

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6: Matriz (`excel/matrix.rs`)

**Files:**
- Create: `src-tauri/src/excel/matrix.rs`, `src-tauri/src/excel/evidence_sheet.rs` (solo el nombre de la hoja; la Task 7 lo completa)
- Modify: `src-tauri/src/excel/mod.rs`

**Interfaces:**
- Consumes: `data::{ExportCase, ExportData, parse_iso}`, `styles::{RowStyles, STATUS_COLORS, Styles, status_format, wrapped_height}`, `evidence_sheet::SHEET`.
- Produces:
  - `matrix::SHEET = "Matriz"`, `HEADER_ROW = 3`, `FIRST_ROW = 4`, `TEMPLATE_ROWS = 100`, `row_of(case_index) -> u32`.
  - `matrix::Column<'a> { Code, Title, Field(&FieldDto), Status, Evidence }` con `label()`, y `columns(&ExportData)`: ID · Caso de prueba · columnas visibles · Estado · Evidencias.
  - `matrix::write(ws, &ExportData, links: Option<&[u32]>, template: bool, &Styles) -> AppResult<()>`. `links[i]` es la fila (base 0) donde empieza el caso `i` en la hoja Evidencias.

- [ ] **Step 1: Tests que fallan**

Crea `src-tauri/src/excel/evidence_sheet.rs`:

```rust
// La Task 7 completa esta hoja; la Matriz ya necesita su nombre para los vínculos.
pub const SHEET: &str = "Evidencias";
```

En `src-tauri/src/excel/mod.rs`, agrega `pub mod evidence_sheet;` después de `pub mod data;` y `pub mod matrix;` antes de `pub mod styles;`. Crea `src-tauri/src/excel/matrix.rs` solo con los tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::excel::{
        styles,
        testing::{Book, fixture},
    };
    use calamine::{Data, DataType};

    fn render(data: &ExportData, links: Option<&[u32]>, template: bool) -> Book {
        let mut workbook = styles::workbook().unwrap();
        write(
            workbook.add_worksheet(),
            data,
            links,
            template,
            &Styles::new(),
        )
        .unwrap();
        Book::save(&mut workbook)
    }

    const HEADERS: [&str; 12] = [
        "ID",
        "Caso de prueba",
        "Entorno",
        "Módulo",
        "Resultado esperado",
        "Resultado obtenido",
        "Fecha de ejecución",
        "Severidad",
        "Aceptado",
        "Horas",
        "Estado",
        "Evidencias",
    ];

    #[test]
    fn headers_are_the_visible_columns_in_order_and_one_row_per_case() {
        let data = fixture();
        let book = render(&data, None, false);
        assert_eq!(book.row_texts(SHEET, HEADER_ROW, 12), HEADERS);
        assert_eq!(book.text(SHEET, 0, 0), "ACME · Recibos");
        assert_eq!(book.height(SHEET), FIRST_ROW + 4);
        let codes: Vec<_> = (0..4).map(|i| book.text(SHEET, row_of(i), 0)).collect();
        assert_eq!(codes, ["EJ-001", "EJ-002", "EJ-003", "EJ-004"]);
        assert_eq!(book.text(SHEET, row_of(2), 10), "Fallido");
    }

    #[test]
    fn values_keep_their_excel_types() {
        let book = render(&fixture(), None, false);
        let row = row_of(0);
        let date = book.cell(SHEET, row, 6);
        assert_eq!(
            date.get_datetime().map(|d| d.as_f64()),
            Some(46311.0),
            "16/10/2026"
        );
        assert_eq!(book.cell(SHEET, row, 9), Data::Float(1.5));
        assert_eq!(book.text(SHEET, row, 8), "Sí");
        assert_eq!(book.text(SHEET, row, 7), "Alta");
        assert_eq!(book.text(SHEET, row, 4), "Se aplica el recibo");
        assert_eq!(book.cell(SHEET, row_of(1), 6), Data::Empty);
    }

    #[test]
    fn values_that_do_not_match_their_type_are_kept_as_text() {
        let mut data = fixture();
        let values = &mut data.cases[0].case.values;
        values.insert("fld-fecha_ejecucion".into(), "ayer".into());
        values.insert("fld-horas".into(), "NaN".into());
        let book = render(&data, None, false);
        assert_eq!(book.text(SHEET, row_of(0), 6), "ayer");
        assert_eq!(book.text(SHEET, row_of(0), 9), "NaN");
    }

    #[test]
    fn evidence_column_links_to_each_case_block() {
        let book = render(&fixture(), Some(&[3, 20, 40, 50]), false);
        assert_eq!(book.text(SHEET, row_of(0), 11), "Ver evidencias (2)");
        assert_eq!(book.text(SHEET, row_of(2), 11), "Ver sección");
        let links = book.hyperlinks(SHEET);
        assert_eq!(links.len(), 4);
        assert_eq!(links[0], ("L5".to_string(), "'Evidencias'!A4".to_string()));
        assert_eq!(links[3], ("L8".to_string(), "'Evidencias'!A51".to_string()));
    }

    #[test]
    fn without_evidence_sheet_the_column_is_a_count() {
        let book = render(&fixture(), None, false);
        assert_eq!(book.cell(SHEET, row_of(0), 11), Data::Float(2.0));
        assert!(book.hyperlinks(SHEET).is_empty());
    }

    #[test]
    fn lists_status_colors_filter_freeze_and_print_setup() {
        let book = render(&fixture(), None, false);
        let xml = book.sheet_xml(SHEET);
        assert_eq!(
            xml.matches("<dataValidation ").count(),
            3,
            "Severidad, Aceptado y Estado"
        );
        assert!(xml.contains("sqref=\"K5:K8\""));
        assert!(xml.contains(r#"<formula1>"No ejecutado,En ejecución,Por documentar,Aprobado,Fallido,Bloqueado"</formula1>"#));
        assert!(xml.contains(r#"<formula1>"Sí,No"</formula1>"#));
        assert_eq!(xml.matches("<cfRule ").count(), 3);
        assert!(xml.contains("<autoFilter ref=\"A4:L8\""));
        assert!(xml.contains("xSplit=\"2\" ySplit=\"4\""));
        assert!(xml.contains("state=\"frozen\""));
        assert!(xml.contains("orientation=\"landscape\""));
        assert!(xml.contains("paperSize=\"9\""));
        assert!(xml.contains("fitToHeight=\"0\""));
        assert!(book.part("xl/workbook.xml").contains("Matriz!$4:$4"));
    }

    #[test]
    fn long_or_comma_lists_skip_the_dropdown() {
        let mut data = fixture();
        let severidad = data
            .project
            .fields
            .iter_mut()
            .find(|f| f.key == "severidad")
            .unwrap();
        severidad.options = (0..30).map(|i| format!("Opción número {i}")).collect();
        let book = render(&data, None, false);
        assert_eq!(book.sheet_xml(SHEET).matches("<dataValidation ").count(), 2);
        let severidad = data
            .project
            .fields
            .iter_mut()
            .find(|f| f.key == "severidad")
            .unwrap();
        severidad.options = vec!["Alta, urgente".into(), "Baja".into()];
        let book = render(&data, None, false);
        assert_eq!(book.sheet_xml(SHEET).matches("<dataValidation ").count(), 2);
    }

    #[test]
    fn a_project_without_cases_says_so() {
        let mut data = fixture();
        data.cases.clear();
        let book = render(&data, None, false);
        assert_eq!(book.text(SHEET, FIRST_ROW, 0), "No hay casos registrados.");
        assert!(book.sheet_xml(SHEET).contains("<autoFilter ref=\"A4:L5\""));
    }

    #[test]
    fn template_has_headers_and_empty_formatted_rows() {
        let book = render(&fixture(), None, true);
        assert_eq!(book.row_texts(SHEET, HEADER_ROW, 12), HEADERS);
        assert_eq!(book.text(SHEET, FIRST_ROW, 0), "");
        let xml = book.sheet_xml(SHEET);
        assert!(xml.contains("sqref=\"K5:K104\""));
        assert!(xml.contains("<autoFilter ref=\"A4:L104\""));
    }
}
```

Run: `cd src-tauri && cargo test excel::matrix`
Expected: FAIL de compilación.

- [ ] **Step 2: Implementación**

Agrega al inicio de `src-tauri/src/excel/matrix.rs`:

```rust
use chrono::Datelike;
use rust_xlsxwriter::{
    ConditionalFormatCell, ConditionalFormatCellRule, DataValidation, ExcelDateTime, Url, Worksheet,
};

use super::{
    data::{ExportCase, ExportData, parse_iso},
    evidence_sheet,
    styles::{RowStyles, STATUS_COLORS, Styles, status_format, wrapped_height},
};
use crate::{
    domain::{fields::FieldType, status::STATUSES},
    dto::FieldDto,
    error::AppResult,
};

pub const SHEET: &str = "Matriz";
pub const HEADER_ROW: u32 = 3;
pub const FIRST_ROW: u32 = 4;
/// Filas con formato y listas desplegables que trae la plantilla vacía.
pub const TEMPLATE_ROWS: u32 = 100;
/// Excel no acepta listas de validación de más de 255 caracteres en total.
const MAX_LIST_CHARS: usize = 255;

pub fn row_of(case_index: usize) -> u32 {
    FIRST_ROW + case_index as u32
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Column<'a> {
    Code,
    Title,
    Field(&'a FieldDto),
    Status,
    Evidence,
}

impl Column<'_> {
    pub fn label(&self) -> &str {
        match self {
            Self::Code => "ID",
            Self::Title => "Caso de prueba",
            Self::Field(f) => &f.label,
            Self::Status => "Estado",
            Self::Evidence => "Evidencias",
        }
    }

    fn width(&self) -> f64 {
        match self {
            Self::Code => 12.0,
            Self::Title => 36.0,
            Self::Field(f) => f.width.clamp(6, 80) as f64,
            Self::Status => 16.0,
            Self::Evidence => 22.0,
        }
    }
}

/// ID · Caso de prueba · columnas visibles del proyecto (en su orden) · Estado · Evidencias.
pub fn columns(data: &ExportData) -> Vec<Column<'_>> {
    let mut cols = vec![Column::Code, Column::Title];
    cols.extend(data.project.fields.iter().map(Column::Field));
    cols.extend([Column::Status, Column::Evidence]);
    cols
}

/// `links`: fila donde empieza cada caso en la hoja Evidencias (`None` si el libro no la incluye).
pub fn write(
    ws: &mut Worksheet,
    data: &ExportData,
    links: Option<&[u32]>,
    template: bool,
    styles: &Styles,
) -> AppResult<()> {
    ws.set_name(SHEET)?;
    ws.set_screen_gridlines(false);
    let cols = columns(data);
    let last_col = (cols.len() - 1) as u16;
    for (i, c) in cols.iter().enumerate() {
        ws.set_column_width(i as u16, c.width())?;
    }
    let p = &data.project;
    let title = if p.client.is_empty() {
        p.name.clone()
    } else {
        format!("{} · {}", p.client, p.name)
    };
    ws.set_row_height(0, 26)?;
    ws.merge_range(0, 0, 0, last_col, &title, &styles.title)?;
    ws.merge_range(
        1,
        0,
        1,
        last_col,
        "Una fila por ejecución y entorno. «Ver evidencias» lleva a las capturas del caso.",
        &styles.subtitle,
    )?;
    ws.set_row_height(HEADER_ROW, 32)?;
    for (i, c) in cols.iter().enumerate() {
        ws.write_string_with_format(HEADER_ROW, i as u16, c.label(), &styles.header)?;
    }

    let rows = if template {
        TEMPLATE_ROWS
    } else {
        data.cases.len().max(1) as u32
    };
    let last_row = FIRST_ROW + rows - 1;
    if template {
        for i in 0..TEMPLATE_ROWS as usize {
            for col in 0..=last_col {
                ws.write_blank(row_of(i), col, &styles.row(i).text)?;
            }
        }
    } else if data.cases.is_empty() {
        ws.merge_range(
            FIRST_ROW,
            0,
            FIRST_ROW,
            last_col,
            "No hay casos registrados.",
            &styles.row(0).text,
        )?;
    } else {
        for (i, c) in data.cases.iter().enumerate() {
            write_case(ws, i, c, &cols, links, styles)?;
        }
    }
    validations(ws, &cols, last_row)?;
    ws.autofilter(HEADER_ROW, 0, last_row, last_col)?;
    ws.set_freeze_panes(FIRST_ROW, 2)?;
    ws.set_landscape()
        .set_paper_size(9)
        .set_print_fit_to_pages(1, 0);
    ws.set_repeat_rows(HEADER_ROW, HEADER_ROW)?;
    ws.set_footer("&C&\"Arial\"&8Página &P de &N");
    Ok(())
}

fn write_case(
    ws: &mut Worksheet,
    index: usize,
    c: &ExportCase,
    cols: &[Column],
    links: Option<&[u32]>,
    styles: &Styles,
) -> AppResult<()> {
    let row = row_of(index);
    let s = styles.row(index);
    let mut texts: Vec<(&str, f64)> = Vec::new();
    for (i, col) in cols.iter().enumerate() {
        let at = i as u16;
        match col {
            Column::Code => {
                ws.write_string_with_format(row, at, &c.case.code, &s.text)?;
            }
            Column::Title => {
                ws.write_string_with_format(row, at, &c.case.title, &s.text)?;
                texts.push((&c.case.title, col.width()));
            }
            Column::Status => {
                ws.write_string_with_format(row, at, &c.case.status, &s.text)?;
            }
            Column::Field(f) => {
                let value = ExportData::value(&c.case, f);
                write_value(ws, row, at, f, value, s)?;
                if f.field_type.is_textual() {
                    texts.push((value, col.width()));
                }
            }
            Column::Evidence => {
                let count = c.evidence.len();
                match links.and_then(|l| l.get(index)) {
                    Some(target) => {
                        let text = if count == 0 {
                            "Ver sección".to_string()
                        } else {
                            format!("Ver evidencias ({count})")
                        };
                        let url = Url::new(format!(
                            "internal:'{}'!A{}",
                            evidence_sheet::SHEET,
                            target + 1
                        ))
                        .set_text(text);
                        ws.write_url_with_format(row, at, url, &s.link)?;
                    }
                    None => {
                        ws.write_number_with_format(row, at, count as f64, &s.number)?;
                    }
                }
            }
        }
    }
    ws.set_row_height(row, wrapped_height(texts, 18.0))?;
    Ok(())
}

/// Fechas como fecha real de Excel, números como número y sí/no como texto; si un valor
/// viejo no cumple su tipo se escribe tal cual, sin perderlo.
fn write_value(
    ws: &mut Worksheet,
    row: u32,
    col: u16,
    field: &FieldDto,
    value: &str,
    s: &RowStyles,
) -> AppResult<()> {
    match field.field_type {
        FieldType::Date => {
            if let Some(d) = parse_iso(value) {
                let date =
                    ExcelDateTime::from_ymd(d.year() as u16, d.month() as u8, d.day() as u8)?;
                ws.write_datetime_with_format(row, col, &date, &s.date)?;
                return Ok(());
            }
        }
        FieldType::Number => {
            if let Some(n) = value.parse::<f64>().ok().filter(|n| n.is_finite()) {
                ws.write_number_with_format(row, col, n, &s.number)?;
                return Ok(());
            }
        }
        FieldType::Boolean => {
            let text = match value {
                "true" => "Sí",
                "false" => "No",
                other => other,
            };
            ws.write_string_with_format(row, col, text, &s.text)?;
            return Ok(());
        }
        _ => {}
    }
    if value.is_empty() {
        ws.write_blank(row, col, &s.text)?;
    } else {
        ws.write_string_with_format(row, col, value, &s.text)?;
    }
    Ok(())
}

fn fits_in_list(options: &[String]) -> bool {
    let chars: usize =
        options.iter().map(|o| o.chars().count()).sum::<usize>() + options.len().saturating_sub(1);
    chars <= MAX_LIST_CHARS && options.iter().all(|o| !o.contains(','))
}

fn validations(ws: &mut Worksheet, cols: &[Column], last_row: u32) -> AppResult<()> {
    for (i, col) in cols.iter().enumerate() {
        let at = i as u16;
        let list: Option<Vec<String>> = match col {
            Column::Status => Some(STATUSES.iter().map(|s| s.to_string()).collect()),
            Column::Field(f) if f.field_type == FieldType::List && !f.options.is_empty() => {
                Some(f.options.clone())
            }
            Column::Field(f) if f.field_type == FieldType::Boolean => {
                Some(vec!["Sí".into(), "No".into()])
            }
            _ => None,
        };
        if let Some(options) = list.filter(|o| fits_in_list(o)) {
            let rule = DataValidation::new()
                .allow_list_strings(&options)?
                .set_error_title("Valor no válido")?
                .set_error_message("Elige un valor de la lista.")?;
            ws.add_data_validation(FIRST_ROW, at, last_row, at, &rule)?;
        }
        if *col == Column::Status {
            for (status, fill, font) in STATUS_COLORS {
                let rule = ConditionalFormatCell::new()
                    .set_rule(ConditionalFormatCellRule::EqualTo(format!("\"{status}\"")))
                    .set_format(status_format(fill, font));
                ws.add_conditional_format(FIRST_ROW, at, last_row, at, &rule)?;
            }
        }
    }
    Ok(())
}
```

Run: `cd src-tauri && cargo test excel::matrix`
Expected: PASS (9 tests): encabezados = columnas visibles, una fila por caso, fecha real `dd/mm/yyyy` (serial 46311 = 16/10/2026), número, Sí/No, listas desplegables (Estado, listas y Sí/No), 3 reglas de color, autofiltro, paneles inmovilizados en `C5`, A4 horizontal ajustada al ancho y encabezado repetido (`Matriz!$4:$4`).

- [ ] **Step 3: Verificación y commit**

```bash
cd src-tauri && cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test && cd ..
git add src-tauri/src/excel
git commit -m "feat(f4): hoja Matriz con columnas dinámicas, tipos, listas, colores de estado e impresión A4

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 7: Imágenes reducidas en paralelo y hoja Evidencias (`excel/images.rs`, `excel/evidence_sheet.rs`)

**Files:**
- Create: `src-tauri/src/excel/images.rs`
- Modify: `src-tauri/src/excel/evidence_sheet.rs` (se reemplaza completo), `src-tauri/src/excel/mod.rs`

**Interfaces:**
- Consumes: `media::raster::dimensions`, `matrix::{SHEET, row_of}`, `styles::{ROW_PX, Styles, wrapped_height, workbook}`, `testing::{Book, fixture, write_png}`.
- Produces:
  - `images::SheetImage { png: Vec<u8>, width: u32, height: u32 }`.
  - `images::prepare(path, max_width) -> AppResult<SheetImage>`: nunca agranda; un PNG que ya cabe se usa tal cual.
  - `images::prepare_all(&[PathBuf], max_width, on_done: &(dyn Fn(usize) + Sync)) -> Vec<Option<SheetImage>>`: hasta 8 hilos, conserva el orden y deja `None` donde la imagen falta o está dañada.
  - `evidence_sheet::SHEET`, `evidence_sheet::MISSING_IMAGE`.
  - `evidence_sheet::write(ws, &ExportData, images: Vec<Option<SheetImage>>, &Styles) -> AppResult<Vec<u32>>`: devuelve la fila donde empieza cada caso.

- [ ] **Step 1: Tests que fallan — `images.rs`**

En `src-tauri/src/excel/mod.rs`, agrega `pub mod images;` después de `pub mod evidence_sheet;`. Crea `src-tauri/src/excel/images.rs` solo con los tests:

```rust
#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::{excel::testing::write_png, media::storage::TempDir};

    #[test]
    fn small_pngs_are_used_as_they_are() {
        let tmp = TempDir::new();
        let path = write_png(tmp.path(), "a.png", 800, 450, 1);
        let image = prepare(&path, 940).unwrap();
        assert_eq!((image.width, image.height), (800, 450));
        assert_eq!(image.png, fs::read(&path).unwrap());
    }

    #[test]
    fn wide_images_shrink_to_the_max_width_keeping_the_ratio() {
        let tmp = TempDir::new();
        let path = write_png(tmp.path(), "a.png", 2400, 1350, 1);
        let image = prepare(&path, 940).unwrap();
        assert_eq!((image.width, image.height), (940, 529));
        assert_eq!(raster::dimensions(&image.png).unwrap(), (940, 529));
        assert_eq!(image::guess_format(&image.png).unwrap(), ImageFormat::Png);
    }

    #[test]
    fn missing_or_broken_files_fail() {
        let tmp = TempDir::new();
        let broken = tmp.path().join("roto.png");
        fs::write(&broken, b"no es png").unwrap();
        assert!(prepare(&tmp.path().join("nada.png"), 940).is_err());
        assert!(prepare(&broken, 940).is_err());
    }

    #[test]
    fn prepare_all_keeps_order_reports_progress_and_marks_failures() {
        let tmp = TempDir::new();
        let paths = vec![
            write_png(tmp.path(), "a.png", 1200, 600, 1),
            tmp.path().join("falta.png"),
            write_png(tmp.path(), "c.png", 300, 200, 3),
        ];
        let seen = Mutex::new(Vec::new());
        let images = prepare_all(&paths, 600, &|n| seen.lock().unwrap().push(n));
        assert_eq!(images.len(), 3);
        assert_eq!(
            images[0].as_ref().map(|i| (i.width, i.height)),
            Some((600, 300))
        );
        assert!(images[1].is_none());
        assert_eq!(images[2].as_ref().map(|i| i.width), Some(300));
        let mut seen = seen.into_inner().unwrap();
        seen.sort_unstable();
        assert_eq!(seen, [1, 2, 3]);
        assert!(prepare_all(&[], 600, &|_| {}).is_empty());
    }
}
```

Run: `cd src-tauri && cargo test excel::images`
Expected: FAIL de compilación.

- [ ] **Step 2: Implementación — `images.rs`**

Agrega al inicio de `src-tauri/src/excel/images.rs`:

```rust
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

use image::{
    ImageFormat,
    codecs::png::{CompressionType, FilterType as PngFilter, PngEncoder},
    imageops::FilterType,
};

use crate::{
    error::{AppError, AppResult},
    media::raster,
};

/// Imagen lista para insertar en la hoja Evidencias.
#[derive(Debug, Clone, PartialEq)]
pub struct SheetImage {
    pub png: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// Lee un render y lo reduce a `max_width` px de ancho (nunca lo agranda). Un PNG que ya cabe
/// se usa tal cual: no se decodifica.
pub fn prepare(path: &Path, max_width: u32) -> AppResult<SheetImage> {
    let bytes = fs::read(path)?;
    let (width, height) = raster::dimensions(&bytes)?;
    let is_png = image::guess_format(&bytes).ok() == Some(ImageFormat::Png);
    if width <= max_width && is_png {
        return Ok(SheetImage {
            png: bytes,
            width,
            height,
        });
    }
    let img = image::load_from_memory(&bytes)
        .map_err(|e| AppError::Export(format!("imagen dañada: {e}")))?;
    let (w, h) = if width > max_width {
        let h = (f64::from(height) * f64::from(max_width) / f64::from(width)).round();
        (max_width, (h as u32).max(1))
    } else {
        (width, height)
    };
    let small = if (w, h) == (width, height) {
        img
    } else {
        img.resize_exact(w, h, FilterType::Triangle)
    };
    let mut png = Vec::new();
    small
        .write_with_encoder(PngEncoder::new_with_quality(
            &mut png,
            CompressionType::Fast,
            PngFilter::Adaptive,
        ))
        .map_err(|e| AppError::Export(e.to_string()))?;
    Ok(SheetImage {
        png,
        width: w,
        height: h,
    })
}

/// Prepara todas las imágenes en paralelo (hasta 8 hilos) conservando el orden.
/// Una imagen que falta o está dañada queda en `None`; el libro muestra un aviso en su lugar.
/// `on_done(n)` se llama cada vez que termina una, con el total terminado hasta ese momento.
pub fn prepare_all(
    paths: &[PathBuf],
    max_width: u32,
    on_done: &(dyn Fn(usize) + Sync),
) -> Vec<Option<SheetImage>> {
    let workers = std::thread::available_parallelism()
        .map_or(2, |n| n.get())
        .clamp(1, 8)
        .min(paths.len().max(1));
    let next = AtomicUsize::new(0);
    let done = AtomicUsize::new(0);
    let slots: Vec<Mutex<Option<SheetImage>>> = paths.iter().map(|_| Mutex::new(None)).collect();
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(path) = paths.get(i) else { break };
                    let image = match prepare(path, max_width) {
                        Ok(image) => Some(image),
                        Err(error) => {
                            eprintln!("No se pudo preparar {}: {error}", path.display());
                            None
                        }
                    };
                    *slots[i].lock().unwrap_or_else(|p| p.into_inner()) = image;
                    on_done(done.fetch_add(1, Ordering::Relaxed) + 1);
                }
            });
        }
    });
    slots
        .into_iter()
        .map(|slot| slot.into_inner().unwrap_or_else(|p| p.into_inner()))
        .collect()
}
```

Run: `cd src-tauri && cargo test excel::images`
Expected: PASS (4 tests).

- [ ] **Step 3: Tests que fallan — hoja Evidencias**

Reemplaza `src-tauri/src/excel/evidence_sheet.rs` por la constante de la Task 6 seguida de los tests:

```rust
pub const SHEET: &str = "Evidencias";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        excel::{
            images::prepare,
            styles,
            testing::{Book, fixture, write_png},
        },
        media::storage::TempDir,
    };

    fn render(data: &ExportData, images: Vec<Option<SheetImage>>) -> (Book, Vec<u32>) {
        let mut workbook = styles::workbook().unwrap();
        let anchors = write(workbook.add_worksheet(), data, images, &Styles::new()).unwrap();
        (Book::save(&mut workbook), anchors)
    }

    fn images(tmp: &TempDir, sizes: &[(u32, u32)]) -> Vec<Option<SheetImage>> {
        sizes
            .iter()
            .enumerate()
            .map(|(i, (w, h))| {
                let path = write_png(tmp.path(), &format!("{i}.png"), *w, *h, i as u8);
                Some(prepare(&path, 940).unwrap())
            })
            .collect()
    }

    #[test]
    fn one_block_per_case_with_back_links_and_key_fields() {
        let tmp = TempDir::new();
        let data = fixture();
        let (book, anchors) = render(&data, images(&tmp, &[(400, 200), (400, 300), (300, 100)]));
        assert_eq!(anchors.len(), 4);
        assert!(anchors.windows(2).all(|w| w[0] < w[1]));
        assert_eq!(book.text(SHEET, anchors[0], 0), "EJ-001 — Caso EJ-001");
        assert_eq!(book.text(SHEET, anchors[0] + 1, 0), "Volver a la matriz");
        assert_eq!(
            book.text(SHEET, anchors[0] + 2, 0),
            "Entorno: QA   ·   Estado: Aprobado"
        );
        assert_eq!(
            book.text(SHEET, anchors[0] + 3, 0),
            "Resultado esperado: Se aplica el recibo"
        );
        assert_eq!(
            book.text(SHEET, anchors[0] + 4, 0),
            "Resultado obtenido: Se aplicó el recibo"
        );
        assert_eq!(book.text(SHEET, anchors[0] + 5, 0), "Captura 1");
        assert_eq!(book.text(SHEET, anchors[0] + 6, 0), "Listado de recibos");
        let links = book.hyperlinks(SHEET);
        let targets: Vec<_> = links.iter().map(|(_, to)| to.as_str()).collect();
        assert_eq!(
            targets,
            ["'Matriz'!A5", "'Matriz'!A6", "'Matriz'!A7", "'Matriz'!A8"]
        );
        assert_eq!(links[0].0, format!("A{}", anchors[0] + 2));
    }

    #[test]
    fn inserts_every_image_and_leaves_room_below_it() {
        let tmp = TempDir::new();
        let data = fixture();
        let (book, anchors) = render(&data, images(&tmp, &[(400, 200), (400, 300), (300, 100)]));
        assert_eq!(book.pictures(SHEET), 3);
        // La imagen 1 empieza en anchor+7 y mide 200 px (+8 de margen): 13 filas de 17 px y 1 libre.
        assert_eq!(book.text(SHEET, anchors[0] + 7 + 14, 0), "Captura 2");
    }

    #[test]
    fn missing_images_and_cases_without_evidence_are_explicit() {
        let tmp = TempDir::new();
        let data = fixture();
        let mut imgs = images(&tmp, &[(400, 200), (400, 300), (300, 100)]);
        imgs[1] = None;
        let (book, anchors) = render(&data, imgs);
        assert_eq!(book.pictures(SHEET), 2);
        assert!(book.find_row(SHEET, MISSING_IMAGE).is_some());
        assert_eq!(
            book.text(SHEET, anchors[2] + 5, 0),
            "No se han registrado capturas."
        );
    }

    #[test]
    fn projects_without_the_key_columns_skip_them() {
        let mut data = fixture();
        data.project
            .fields
            .retain(|f| f.key != "entorno" && !f.key.starts_with("resultado_"));
        let (book, anchors) = render(&data, vec![None, None, None]);
        assert_eq!(book.text(SHEET, anchors[0] + 2, 0), "Estado: Aprobado");
        assert_eq!(book.text(SHEET, anchors[0] + 3, 0), "Captura 1");
    }

    #[test]
    fn an_empty_project_still_has_the_sheet() {
        let mut data = fixture();
        data.cases.clear();
        let (book, anchors) = render(&data, Vec::new());
        assert!(anchors.is_empty());
        assert_eq!(book.text(SHEET, 3, 0), "No hay casos registrados.");
    }
}
```

Run: `cd src-tauri && cargo test excel::evidence_sheet`
Expected: FAIL de compilación (`write`, `MISSING_IMAGE`…).

- [ ] **Step 4: Implementación — hoja Evidencias**

Reemplaza la línea `pub const SHEET: &str = "Evidencias";` por:

```rust
use rust_xlsxwriter::{Image, ObjectMovement, Url, Worksheet};

use super::{
    data::ExportData,
    images::SheetImage,
    matrix,
    styles::{ROW_PX, Styles, wrapped_height},
};
use crate::error::AppResult;

pub const SHEET: &str = "Evidencias";
pub const MISSING_IMAGE: &str = "No se pudo leer la imagen de esta evidencia.";
const LAST_COL: u16 = 9;
/// Columnas que se repiten junto a las capturas cuando el proyecto las tiene.
const KEY_FIELDS: [&str; 4] = [
    "resultado_esperado",
    "resultado_obtenido",
    "response_esperado",
    "response_obtenido",
];

/// Un bloque por caso: banda con código y título, «Volver a la matriz», entorno y estado,
/// campos clave y cada captura (descripción e imagen). `images` trae una entrada por
/// evidencia en el orden de `data.cases[..].evidence`; cada una se suelta al insertarla.
/// Devuelve la fila donde empieza cada caso, para los vínculos de la matriz.
pub fn write(
    ws: &mut Worksheet,
    data: &ExportData,
    images: Vec<Option<SheetImage>>,
    styles: &Styles,
) -> AppResult<Vec<u32>> {
    ws.set_name(SHEET)?;
    ws.set_screen_gridlines(false);
    for col in 0..=LAST_COL {
        ws.set_column_width(col, 14)?;
    }
    ws.set_row_height(0, 26)?;
    ws.merge_range(0, 0, 0, LAST_COL, "Evidencias de pruebas", &styles.title)?;
    ws.merge_range(
        1,
        0,
        1,
        LAST_COL,
        "Cada sección identifica la ejecución, su entorno y sus capturas.",
        &styles.subtitle,
    )?;
    let environment = data.field("entorno");
    let key_fields: Vec<_> = KEY_FIELDS.iter().filter_map(|k| data.field(k)).collect();
    let mut images = images.into_iter();
    let mut anchors = Vec::with_capacity(data.cases.len());
    let mut row = 3;
    if data.cases.is_empty() {
        ws.merge_range(
            row,
            0,
            row,
            LAST_COL,
            "No hay casos registrados.",
            &styles.text,
        )?;
    }
    for (index, c) in data.cases.iter().enumerate() {
        anchors.push(row);
        band(
            ws,
            row,
            &format!("{} — {}", c.case.code, c.case.title),
            &styles.band,
            24.0,
        )?;
        row += 1;
        let back = Url::new(format!(
            "internal:'{}'!A{}",
            matrix::SHEET,
            matrix::row_of(index) + 1
        ))
        .set_text("Volver a la matriz");
        ws.write_url_with_format(row, 0, back, &styles.link)?;
        row += 1;
        let mut status = format!("Estado: {}", c.case.status);
        if let Some(value) = environment
            .map(|f| ExportData::value(&c.case, f))
            .filter(|v| !v.is_empty())
        {
            status = format!("Entorno: {value}   ·   {status}");
        }
        ws.merge_range(row, 0, row, LAST_COL, &status, &styles.text)?;
        row += 1;
        for f in &key_fields {
            let text = format!("{}: {}", f.label, ExportData::value(&c.case, f));
            ws.set_row_height(row, wrapped_height([(text.as_str(), 140.0)], 18.0))?;
            ws.merge_range(row, 0, row, LAST_COL, &text, &styles.text)?;
            row += 1;
        }
        if c.evidence.is_empty() {
            band(
                ws,
                row,
                "No se han registrado capturas.",
                &styles.band_soft,
                22.0,
            )?;
            row += 1;
        }
        for (j, e) in c.evidence.iter().enumerate() {
            band(
                ws,
                row,
                &format!("Captura {}", j + 1),
                &styles.band_soft,
                22.0,
            )?;
            row += 1;
            ws.set_row_height(row, wrapped_height([(e.description.as_str(), 140.0)], 18.0))?;
            ws.merge_range(row, 0, row, LAST_COL, &e.description, &styles.text)?;
            row += 1;
            match images.next().flatten() {
                Some(img) => {
                    let alt: String = e.description.chars().take(200).collect();
                    let image = Image::new_from_buffer(&img.png)?
                        .set_alt_text(alt)
                        .set_object_movement(ObjectMovement::MoveButDontSizeWithCells);
                    ws.insert_image_with_offset(row, 0, &image, 4, 4)?;
                    row += (img.height + 8).div_ceil(ROW_PX) + 1;
                }
                None => {
                    ws.merge_range(row, 0, row, LAST_COL, MISSING_IMAGE, &styles.subtitle)?;
                    row += 2;
                }
            }
        }
        row += 2;
    }
    ws.set_landscape()
        .set_paper_size(9)
        .set_print_fit_to_pages(1, 0);
    Ok(anchors)
}

fn band(
    ws: &mut Worksheet,
    row: u32,
    text: &str,
    format: &rust_xlsxwriter::Format,
    height: f64,
) -> AppResult<()> {
    ws.set_row_height(row, height)?;
    ws.merge_range(row, 0, row, LAST_COL, text, format)?;
    Ok(())
}
```

Run: `cd src-tauri && cargo test excel::evidence_sheet && cargo test excel::matrix`
Expected: PASS (5 + 9 tests). Los vínculos «Volver a la matriz» apuntan a `'Matriz'!A5…A8`. Una imagen de 200 px ocupa 13 filas de 17 px más una libre.

- [ ] **Step 5: Verificación y commit**

```bash
cd src-tauri && cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test && cd ..
git add src-tauri/src/excel
git commit -m "feat(f4): hoja Evidencias con capturas reducidas en paralelo y vínculos de ida y vuelta

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 8: Resumen con gráfico nativo (`excel/summary.rs`)

**Files:**
- Create: `src-tauri/src/excel/summary.rs`
- Modify: `src-tauri/src/excel/mod.rs`

**Interfaces:**
- Consumes: `data::{ExportData, Kpis}`, `styles::{self, Styles, workbook}`, `testing::{Book, case, fixture}`.
- Produces:
  - `summary::SHEET = "Resumen"`, `STATUS_ROW = 3`, `GROUPS_ROW = 21`, `NO_VALUE = "(sin valor)"`.
  - `summary::write(ws, &ExportData, &Styles) -> AppResult<()>`. La hoja lleva:
    - tabla por estado (ejecuciones y % del total), Total y % de éxito;
    - gráfico de columnas nativo en `E4`;
    - tablas por Módulo y por Entorno, si esas columnas son visibles.

- [ ] **Step 1: Tests que fallan**

En `src-tauri/src/excel/mod.rs`, agrega `pub mod summary;` después de `pub mod styles;`. Crea `src-tauri/src/excel/summary.rs` solo con los tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::excel::testing::{Book, case, fixture};
    use calamine::Data;

    fn render(data: &ExportData) -> Book {
        let mut workbook = styles::workbook().unwrap();
        write(workbook.add_worksheet(), data, &Styles::new()).unwrap();
        Book::save(&mut workbook)
    }

    #[test]
    fn status_table_with_totals_rate_and_a_native_chart() {
        let book = render(&fixture());
        assert_eq!(
            book.row_texts(SHEET, STATUS_ROW, 3),
            ["Estado", "Ejecuciones", "% del total"]
        );
        assert_eq!(book.text(SHEET, STATUS_ROW + 4, 0), "Aprobado");
        assert_eq!(book.cell(SHEET, STATUS_ROW + 4, 1), Data::Float(2.0));
        assert_eq!(book.cell(SHEET, STATUS_ROW + 4, 2), Data::Float(0.5));
        assert_eq!(book.text(SHEET, STATUS_ROW + 7, 0), "Total");
        assert_eq!(book.cell(SHEET, STATUS_ROW + 7, 1), Data::Float(4.0));
        assert_eq!(book.cell(SHEET, STATUS_ROW + 8, 1), Data::Float(2.0 / 3.0));
        assert_eq!(book.charts(SHEET), 1);
        let chart = book.part("xl/charts/chart1.xml");
        assert!(chart.contains("<c:barDir val=\"col\"/>"));
        assert!(chart.contains("Resumen!$A$5:$A$10"));
        assert!(chart.contains("Resumen!$B$5:$B$10"));
    }

    #[test]
    fn groups_by_module_and_environment_with_blank_values_last() {
        let book = render(&fixture());
        assert_eq!(book.text(SHEET, GROUPS_ROW, 0), "Módulo");
        assert_eq!(book.text(SHEET, GROUPS_ROW, 7), "Total");
        let modules: Vec<_> = (1..=3)
            .map(|i| book.text(SHEET, GROUPS_ROW + i, 0))
            .collect();
        assert_eq!(modules, ["Cobranzas", "Pólizas", NO_VALUE]);
        assert_eq!(
            book.cell(SHEET, GROUPS_ROW + 1, 4),
            Data::Float(2.0),
            "Cobranzas · Aprobado"
        );
        assert_eq!(book.cell(SHEET, GROUPS_ROW + 1, 7), Data::Float(2.0));
        let env = GROUPS_ROW + 3 + 3;
        assert_eq!(book.text(SHEET, env, 0), "Entorno");
        assert_eq!(book.text(SHEET, env + 1, 0), "QA");
        assert_eq!(book.cell(SHEET, env + 1, 7), Data::Float(3.0));
    }

    #[test]
    fn without_those_columns_there_are_no_group_tables() {
        let mut data = fixture();
        data.project
            .fields
            .retain(|f| f.key != "modulo" && f.key != "entorno");
        let book = render(&data);
        assert_eq!(book.text(SHEET, GROUPS_ROW, 0), "");
    }

    #[test]
    fn an_empty_project_has_zero_shares_and_no_rate() {
        let mut data = fixture();
        data.cases.clear();
        let book = render(&data);
        assert_eq!(book.cell(SHEET, STATUS_ROW + 1, 2), Data::Float(0.0));
        assert_eq!(book.text(SHEET, STATUS_ROW + 8, 1), "—");
        data.cases
            .push(case(&data.project, "EJ-001", "Bloqueado", &[]));
        assert_eq!(
            render(&data).cell(SHEET, STATUS_ROW + 8, 1),
            Data::Float(0.0)
        );
    }
}
```

Run: `cd src-tauri && cargo test excel::summary`
Expected: FAIL de compilación.

- [ ] **Step 2: Implementación**

Agrega al inicio de `src-tauri/src/excel/summary.rs`:

```rust
use rust_xlsxwriter::{Chart, ChartFormat, ChartSolidFill, ChartType, Worksheet};

use super::{
    data::{ExportData, Kpis},
    styles::{self, Styles},
};
use crate::{domain::status::STATUSES, dto::FieldDto, error::AppResult};

pub const SHEET: &str = "Resumen";
pub const STATUS_ROW: u32 = 3;
/// Debajo del gráfico (que ocupa unas 15 filas desde `STATUS_ROW`).
pub const GROUPS_ROW: u32 = 21;
pub const NO_VALUE: &str = "(sin valor)";
/// Tablas por grupo, si el proyecto tiene esas columnas visibles (spec §7).
const GROUPS: [&str; 2] = ["modulo", "entorno"];

pub fn write(ws: &mut Worksheet, data: &ExportData, styles: &Styles) -> AppResult<()> {
    ws.set_name(SHEET)?;
    ws.set_screen_gridlines(false);
    ws.set_column_width(0, 24)?;
    for col in 1..=7 {
        ws.set_column_width(col, 13)?;
    }
    ws.set_row_height(0, 26)?;
    ws.merge_range(0, 0, 0, 7, "Resumen de resultados", &styles.title)?;
    let p = &data.project;
    let heading = if p.client.is_empty() {
        p.name.clone()
    } else {
        format!("{} · {}", p.client, p.name)
    };
    ws.merge_range(1, 0, 1, 7, &heading, &styles.subtitle)?;

    let k = Kpis::of(data);
    for (col, title) in ["Estado", "Ejecuciones", "% del total"]
        .into_iter()
        .enumerate()
    {
        ws.write_string_with_format(STATUS_ROW, col as u16, title, &styles.header)?;
    }
    let share = |n: usize| {
        if k.total == 0 {
            0.0
        } else {
            n as f64 / k.total as f64
        }
    };
    for (i, status) in STATUSES.iter().enumerate() {
        let row = STATUS_ROW + 1 + i as u32;
        ws.write_string_with_format(row, 0, *status, &styles.value)?;
        ws.write_number_with_format(row, 1, k.by_status[i] as f64, &styles.number)?;
        ws.write_number_with_format(row, 2, share(k.by_status[i]), &styles.percent)?;
    }
    let total = STATUS_ROW + 7;
    ws.write_string_with_format(total, 0, "Total", &styles.total)?;
    ws.write_number_with_format(total, 1, k.total as f64, &styles.total)?;
    ws.write_number_with_format(total, 2, share(k.total), &styles.total)?;
    let rate = total + 1;
    ws.write_string_with_format(rate, 0, "% de éxito", &styles.label)?;
    match k.success_rate {
        Some(r) => ws.write_number_with_format(rate, 1, r, &styles.percent)?,
        None => ws.write_string_with_format(rate, 1, "—", &styles.number)?,
    };

    let mut chart = Chart::new(ChartType::Column);
    chart
        .add_series()
        .set_name("Ejecuciones")
        .set_categories((SHEET, STATUS_ROW + 1, 0, STATUS_ROW + 6, 0))
        .set_values((SHEET, STATUS_ROW + 1, 1, STATUS_ROW + 6, 1))
        .set_format(
            ChartFormat::new().set_solid_fill(ChartSolidFill::new().set_color(styles::BLUE)),
        );
    chart.title().set_name("Ejecuciones por estado");
    chart.legend().set_hidden();
    ws.insert_chart(STATUS_ROW, 4, &chart)?;

    let mut row = GROUPS_ROW;
    for key in GROUPS {
        if let Some(field) = data.field(key) {
            row = group_table(ws, row, data, field, styles)? + 3;
        }
    }
    ws.set_paper_size(9).set_print_fit_to_pages(1, 0);
    Ok(())
}

/// Conteo por estado de cada valor de la columna; devuelve la última fila usada.
fn group_table(
    ws: &mut Worksheet,
    start: u32,
    data: &ExportData,
    field: &FieldDto,
    styles: &Styles,
) -> AppResult<u32> {
    ws.write_string_with_format(start, 0, &field.label, &styles.header)?;
    for (i, status) in STATUSES.iter().enumerate() {
        ws.write_string_with_format(start, 1 + i as u16, *status, &styles.header)?;
    }
    ws.write_string_with_format(start, 7, "Total", &styles.header)?;
    ws.set_row_height(start, 28)?;
    let mut groups: Vec<(String, [usize; 6])> = Vec::new();
    for c in &data.cases {
        let raw = ExportData::value(&c.case, field).trim();
        let name = if raw.is_empty() { NO_VALUE } else { raw };
        let index = match groups.iter().position(|(g, _)| g == name) {
            Some(i) => i,
            None => {
                groups.push((name.to_string(), [0; 6]));
                groups.len() - 1
            }
        };
        if let Some(s) = STATUSES.iter().position(|s| *s == c.case.status) {
            groups[index].1[s] += 1;
        }
    }
    // Alfabético sin distinguir mayúsculas; «(sin valor)» siempre al final.
    groups.sort_by_key(|(name, _)| (name == NO_VALUE, name.to_lowercase()));
    let mut row = start;
    for (name, counts) in &groups {
        row += 1;
        ws.write_string_with_format(row, 0, name, &styles.value)?;
        for (i, n) in counts.iter().enumerate() {
            ws.write_number_with_format(row, 1 + i as u16, *n as f64, &styles.number)?;
        }
        ws.write_number_with_format(row, 7, counts.iter().sum::<usize>() as f64, &styles.total)?;
    }
    Ok(row)
}
```

Run: `cd src-tauri && cargo test excel::summary`
Expected: PASS (4 tests).

- [ ] **Step 3: Verificación y commit**

```bash
cd src-tauri && cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test && cd ..
git add src-tauri/src/excel
git commit -m "feat(f4): hoja Resumen por estado, módulo y entorno con gráfico de barras nativo

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---


### Task 9: Libro completo, escritura atómica y «archivo abierto en Excel» (`excel/save.rs`, `excel/mod.rs`)

**Files:**
- Create: `src-tauri/src/excel/save.rs`
- Modify: `src-tauri/src/excel/mod.rs` (se reemplaza completo), `src-tauri/src/error.rs`, `src-tauri/src/dto.rs`

**Interfaces:**
- Consumes: todo `excel::*` de las Tasks 3–8.
- Produces:
  - `AppError::FileLocked(String)`: código `"file_locked"`, mensaje `Cierra el archivo en Excel y vuelve a intentar.`
  - `dto::ExportResultDto { path, file_name, cases: i64, evidence: i64, missing_images: i64 }`.
  - `dto::ExportStage { Preparing, Images, Writing, Saving }` (JSON en minúsculas) y `dto::ExportProgressDto { stage, current: i64, total: i64 }`.
  - `excel::save`:
    - `is_locked(&io::Error)`, `ensure_writable(dest)`, `save_atomic(&mut Workbook, dest)`;
    - `with_xlsx_extension(PathBuf) -> PathBuf`;
    - `RecentExports` (`Default`), con `.remember(&Path)` y `.resolve(&str) -> AppResult<PathBuf>`.
  - `excel::Progress<'a> = &'a (dyn Fn(ExportProgressDto) + Sync)`.
  - `excel::build(&ExportData, &ExportSettings, Progress) -> AppResult<Built { workbook, missing_images }>`.
  - `excel::export(&ExportData, &ExportSettings, dest: &Path, Progress) -> AppResult<ExportResultDto>`: avisa del bloqueo antes de generar, arma el libro y lo guarda de forma atómica.

- [ ] **Step 1: Error y DTO**

En `src-tauri/src/error.rs`, antes de:

```rust
    #[error("excel: {0}")]
    Export(String),
}
```

agrega:

```rust
    #[error("archivo en uso: {0}")]
    FileLocked(String),
```

En `src-tauri/src/error.rs`, antes de:

```rust
            Self::Export(_) => "export",
```

agrega:

```rust
            Self::FileLocked(_) => "file_locked",
```

En `src-tauri/src/error.rs`, antes de:

```rust
            Self::Export(_) => "No se pudo generar el Excel. Inténtalo nuevamente.".into(),
```

agrega:

```rust
            Self::FileLocked(_) => "Cierra el archivo en Excel y vuelve a intentar.".into(),
```

En `src-tauri/src/error.rs`, antes de:

```rust
    #[test]
    fn excel_errors_have_their_own_code_and_a_friendly_message() {
```

agrega:

```rust
    #[test]
    fn a_locked_file_asks_to_close_it_in_excel() {
        let locked = serde_json::to_value(AppError::FileLocked("matriz.xlsx".into())).unwrap();
        assert_eq!(locked["code"], "file_locked");
        assert_eq!(
            locked["message"],
            "Cierra el archivo en Excel y vuelve a intentar."
        );
        assert_eq!(locked["detail"], "archivo en uso: matriz.xlsx");
    }
```

En `src-tauri/src/dto.rs`, antes de:

```rust
#[cfg(test)]
mod contract {
```

agrega:

```rust
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResultDto {
    pub path: String,
    pub file_name: String,
    pub cases: i64,
    pub evidence: i64,
    pub missing_images: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportStage {
    Preparing,
    Images,
    Writing,
    Saving,
}

/// Payload del evento `export:progress` (Rust → ventana principal).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportProgressDto {
    pub stage: ExportStage,
    pub current: i64,
    pub total: i64,
}
```

Run: `cd src-tauri && cargo test error::`
Expected: PASS (3 tests).

- [ ] **Step 2: Tests que fallan — `save.rs`**

En `src-tauri/src/excel/mod.rs`, agrega `pub mod save;` antes de `pub mod styles;`. Crea `src-tauri/src/excel/save.rs` solo con los tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::storage::TempDir;

    fn book(text: &str) -> Workbook {
        let mut workbook = Workbook::new();
        workbook.add_worksheet().write_string(0, 0, text).unwrap();
        workbook
    }

    fn files_in(dir: &Path) -> Vec<String> {
        let mut names: Vec<_> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn writes_and_replaces_without_leftovers() {
        let tmp = TempDir::new();
        let dest = tmp.path().join("matriz.xlsx");
        save_atomic(&mut book("uno"), &dest).unwrap();
        save_atomic(&mut book("dos"), &dest).unwrap();
        assert_eq!(files_in(tmp.path()), ["matriz.xlsx"]);
        assert_eq!(
            crate::excel::testing::Book::at(&dest).text("Sheet1", 0, 0),
            "dos"
        );
    }

    #[cfg(windows)]
    #[test]
    fn a_file_open_in_excel_is_reported_and_left_intact() {
        use std::os::windows::fs::OpenOptionsExt;

        let tmp = TempDir::new();
        let dest = tmp.path().join("matriz.xlsx");
        save_atomic(&mut book("original"), &dest).unwrap();
        let before = fs::read(&dest).unwrap();
        // Excel abre el libro sin compartir escritura: se simula con share_mode(0).
        let _excel = OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(&dest)
            .unwrap();
        let error = save_atomic(&mut book("nuevo"), &dest).unwrap_err();
        assert_eq!(error.code(), "file_locked");
        assert_eq!(
            error.user_message(),
            "Cierra el archivo en Excel y vuelve a intentar."
        );
        drop(_excel);
        assert_eq!(fs::read(&dest).unwrap(), before);
        assert_eq!(files_in(tmp.path()), ["matriz.xlsx"]);
    }

    #[test]
    fn read_only_files_are_not_overwritten() {
        let tmp = TempDir::new();
        let dest = tmp.path().join("matriz.xlsx");
        save_atomic(&mut book("original"), &dest).unwrap();
        let mut permissions = fs::metadata(&dest).unwrap().permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&dest, permissions.clone()).unwrap();
        let error = save_atomic(&mut book("nuevo"), &dest).unwrap_err();
        assert_eq!(
            error.to_string(),
            "No se puede sobrescribir el archivo: es de solo lectura o no tienes permisos."
        );
        #[allow(clippy::permissions_set_readonly_false)]
        permissions.set_readonly(false);
        fs::set_permissions(&dest, permissions).unwrap();
    }

    #[test]
    fn a_missing_folder_fails_without_leaving_files() {
        let tmp = TempDir::new();
        let dest = tmp.path().join("no-existe").join("matriz.xlsx");
        assert!(save_atomic(&mut book("x"), &dest).is_err());
        assert!(files_in(tmp.path()).is_empty());
    }

    #[test]
    fn the_extension_is_always_xlsx() {
        let p = |s: &str| PathBuf::from(s);
        assert_eq!(
            with_xlsx_extension(p("C:\\a\\matriz.xlsx")),
            p("C:\\a\\matriz.xlsx")
        );
        assert_eq!(
            with_xlsx_extension(p("C:\\a\\matriz.XLSX")),
            p("C:\\a\\matriz.XLSX")
        );
        assert_eq!(
            with_xlsx_extension(p("C:\\a\\matriz")),
            p("C:\\a\\matriz.xlsx")
        );
        assert_eq!(with_xlsx_extension(p("C:\\a\\v1.0")), p("C:\\a\\v1.0.xlsx"));
    }

    #[test]
    fn only_files_exported_in_this_session_can_be_opened() {
        let tmp = TempDir::new();
        let dest = tmp.path().join("matriz.xlsx");
        let recent = RecentExports::default();
        let raw = dest.to_string_lossy().into_owned();
        assert_eq!(
            recent.resolve(&raw).unwrap_err().to_string(),
            "Solo se pueden abrir archivos exportados en esta sesión."
        );
        recent.remember(&dest);
        assert_eq!(recent.resolve(&raw).unwrap_err().code(), "not_found");
        save_atomic(&mut book("x"), &dest).unwrap();
        assert_eq!(recent.resolve(&raw).unwrap(), dest);
        assert!(recent.resolve("C:\\Windows\\notepad.exe").is_err());
    }
}
```

Run: `cd src-tauri && cargo test excel::save`
Expected: FAIL de compilación.

- [ ] **Step 3: Implementación — `save.rs`**

Agrega al inicio de `src-tauri/src/excel/save.rs`:

```rust
use std::{
    collections::HashSet,
    fs::{self, OpenOptions},
    io::ErrorKind,
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard},
};

use rust_xlsxwriter::Workbook;

use crate::{
    db::new_id,
    error::{AppError, AppResult, validation},
};

/// `ERROR_SHARING_VIOLATION` (32) y `ERROR_LOCK_VIOLATION` (33): Windows los devuelve cuando
/// Excel tiene abierto el archivo (spec §8).
pub fn is_locked(error: &std::io::Error) -> bool {
    matches!(error.raw_os_error(), Some(32 | 33))
}

fn locked(path: &Path) -> AppError {
    AppError::FileLocked(path.display().to_string())
}

/// Falla antes de generar nada si el destino existe y no se puede sobrescribir.
pub fn ensure_writable(dest: &Path) -> AppResult<()> {
    if !dest.exists() {
        return Ok(());
    }
    match OpenOptions::new().append(true).open(dest) {
        Ok(_) => Ok(()),
        Err(e) if is_locked(&e) => Err(locked(dest)),
        Err(e) if e.kind() == ErrorKind::PermissionDenied => Err(validation(
            "No se puede sobrescribir el archivo: es de solo lectura o no tienes permisos.",
        )),
        Err(e) => Err(e.into()),
    }
}

/// Escribe el libro en un temporal junto al destino y lo renombra: nunca deja un xlsx a medias
/// ni temporales sueltos.
pub fn save_atomic(workbook: &mut Workbook, dest: &Path) -> AppResult<()> {
    ensure_writable(dest)?;
    let tmp = dest.with_extension(format!("{}.tmp", new_id()));
    if let Err(error) = workbook.save(&tmp) {
        let _ = fs::remove_file(&tmp);
        return Err(error.into());
    }
    if let Err(error) = fs::rename(&tmp, dest) {
        let _ = fs::remove_file(&tmp);
        return Err(
            if is_locked(&error) || (error.kind() == ErrorKind::PermissionDenied && dest.exists()) {
                locked(dest)
            } else {
                error.into()
            },
        );
    }
    Ok(())
}

/// El diálogo «Guardar como» deja escribir cualquier nombre: el libro siempre termina en `.xlsx`.
pub fn with_xlsx_extension(path: PathBuf) -> PathBuf {
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("xlsx"))
    {
        return path;
    }
    let mut raw = path.into_os_string();
    raw.push(".xlsx");
    PathBuf::from(raw)
}

/// Archivos exportados en esta sesión: lo único que «Abrir archivo» y «Abrir carpeta» aceptan
/// (la ventana no tiene acceso genérico al disco, spec §9).
#[derive(Default)]
pub struct RecentExports {
    paths: Mutex<HashSet<PathBuf>>,
}

impl RecentExports {
    fn lock(&self) -> MutexGuard<'_, HashSet<PathBuf>> {
        self.paths.lock().unwrap_or_else(|p| p.into_inner())
    }

    pub fn remember(&self, path: &Path) {
        self.lock().insert(path.to_path_buf());
    }

    pub fn resolve(&self, raw: &str) -> AppResult<PathBuf> {
        let path = PathBuf::from(raw);
        if !self.lock().contains(&path) {
            return Err(validation(
                "Solo se pueden abrir archivos exportados en esta sesión.",
            ));
        }
        if !path.is_file() {
            return Err(AppError::NotFound(
                "No se encontró el archivo exportado. Puede que se haya movido o eliminado.".into(),
            ));
        }
        Ok(path)
    }
}
```

Run: `cd src-tauri && cargo test excel::save`
Expected: PASS (6 tests). `a_file_open_in_excel_is_reported_and_left_intact` abre el archivo con `share_mode(0)`, igual que Excel, y comprueba el mensaje, que el original sigue igual y que no queda ningún `.tmp`.

- [ ] **Step 4: Tests que fallan — el libro completo**

Agrega al final de `src-tauri/src/excel/mod.rs`:

```rust
#[cfg(test)]
mod tests {
    use std::{sync::Mutex, time::Instant};

    use chrono::NaiveDate;

    use super::*;
    use crate::{
        excel::testing::{Book, case, evidence, fixture, version, write_png},
        media::storage::TempDir,
    };

    fn settings() -> ExportSettings {
        ExportSettings {
            tester: "Ana Pérez".into(),
            version: Some(version("ver-1", "1.0", "2026-10-01")),
            include_evidence: true,
            include_summary: true,
            image_max_width: 940,
            today: NaiveDate::from_ymd_opt(2026, 10, 16).unwrap(),
            template: false,
        }
    }

    /// El fixture con imágenes reales en disco.
    fn with_images(tmp: &TempDir) -> ExportData {
        let mut data = fixture();
        let mut seed = 0;
        for c in &mut data.cases {
            for e in &mut c.evidence {
                seed += 1;
                e.render_path = write_png(tmp.path(), &format!("{seed}.png"), 1200, 700, seed);
            }
        }
        data
    }

    #[test]
    fn exports_four_sheets_that_calamine_reads_back() {
        let tmp = TempDir::new();
        let data = with_images(&tmp);
        let dest = tmp.path().join("T4S_ACME_Recibos.xlsx");
        let stages = Mutex::new(Vec::new());
        let result = export(&data, &settings(), &dest, &|p| {
            stages.lock().unwrap().push(p.stage)
        })
        .unwrap();
        assert_eq!(result.file_name, "T4S_ACME_Recibos.xlsx");
        assert_eq!(
            (result.cases, result.evidence, result.missing_images),
            (4, 3, 0)
        );

        let book = Book::at(&dest);
        assert_eq!(
            book.sheet_names(),
            ["Carátula", "Matriz", "Evidencias", "Resumen"]
        );
        let headers = book.row_texts(matrix::SHEET, matrix::HEADER_ROW, 12);
        let mut expected = vec!["ID".to_string(), "Caso de prueba".to_string()];
        expected.extend(data.project.fields.iter().map(|f| f.label.clone()));
        expected.extend(["Estado".to_string(), "Evidencias".to_string()]);
        assert_eq!(headers, expected);
        assert_eq!(
            book.height(matrix::SHEET),
            matrix::FIRST_ROW + data.cases.len() as u32
        );
        assert_eq!(book.pictures(evidence_sheet::SHEET), 3);
        assert_eq!(book.pictures(cover::SHEET), 2);
        assert_eq!(book.charts(summary::SHEET), 1);

        let stages = stages.into_inner().unwrap();
        assert_eq!(stages.first(), Some(&ExportStage::Preparing));
        assert!(stages.contains(&ExportStage::Images));
        assert_eq!(stages.last(), Some(&ExportStage::Saving));
    }

    #[test]
    fn links_go_both_ways_between_matrix_and_evidence() {
        let tmp = TempDir::new();
        let dest = tmp.path().join("libro.xlsx");
        export(&with_images(&tmp), &settings(), &dest, &|_| {}).unwrap();
        let book = Book::at(&dest);
        let target_row = |location: &str, sheet: &str| -> u32 {
            let prefix = format!("'{sheet}'!A");
            location
                .strip_prefix(&prefix)
                .unwrap()
                .parse::<u32>()
                .unwrap()
                - 1
        };
        for (i, (cell, to)) in book.hyperlinks(matrix::SHEET).iter().enumerate() {
            assert_eq!(cell, &format!("L{}", matrix::row_of(i) + 1));
            let block = target_row(to, evidence_sheet::SHEET);
            let code = format!("EJ-00{}", i + 1);
            assert!(
                book.text(evidence_sheet::SHEET, block, 0)
                    .starts_with(&code),
                "{to}"
            );
            let (_, back) = &book.hyperlinks(evidence_sheet::SHEET)[i];
            let row = target_row(back, matrix::SHEET);
            assert_eq!(book.text(matrix::SHEET, row, 0), code);
        }
    }

    #[test]
    fn sheets_can_be_left_out_and_missing_images_are_counted() {
        let tmp = TempDir::new();
        let mut data = with_images(&tmp);
        data.cases[1].evidence[0].render_path = tmp.path().join("borrada.png");
        let dest = tmp.path().join("libro.xlsx");
        let result = export(&data, &settings(), &dest, &|_| {}).unwrap();
        assert_eq!(result.missing_images, 1);

        let mut only_matrix = settings();
        only_matrix.include_evidence = false;
        only_matrix.include_summary = false;
        let result = export(&data, &only_matrix, &dest, &|_| {}).unwrap();
        assert_eq!(result.evidence, 0);
        assert_eq!(Book::at(&dest).sheet_names(), ["Carátula", "Matriz"]);
    }

    #[test]
    fn template_has_cover_and_empty_matrix() {
        let tmp = TempDir::new();
        let dest = tmp.path().join("plantilla.xlsx");
        let mut template = settings();
        template.template = true;
        template.include_evidence = false;
        template.include_summary = false;
        let result = export(&fixture(), &template, &dest, &|_| {}).unwrap();
        assert_eq!(result.cases, 0);
        let book = Book::at(&dest);
        assert_eq!(book.sheet_names(), ["Carátula", "Matriz"]);
        assert_eq!(
            book.text(cover::SHEET, cover::TITLE_ROW, 0),
            "Plantilla de pruebas"
        );
        assert_eq!(book.text(matrix::SHEET, matrix::FIRST_ROW, 0), "");
    }

    #[test]
    fn workbook_properties_name_the_tester() {
        let tmp = TempDir::new();
        let dest = tmp.path().join("libro.xlsx");
        export(&fixture(), &settings(), &dest, &|_| {}).unwrap();
        let core = Book::at(&dest).part("docProps/core.xml");
        assert!(core.contains("<dc:creator>Ana Pérez</dc:creator>"));
        assert!(core.contains("<dc:title>Matriz de pruebas y evidencias</dc:title>"));
    }

    /// Criterio de aceptación F4: 300 evidencias en menos de 20 s y con memoria acotada.
    /// Se mide en release: `cargo test --release excel::tests::exports_300_evidences -- --ignored --nocapture`
    #[test]
    #[ignore = "medición de rendimiento: correr en release"]
    fn exports_300_evidences_in_under_20_seconds() {
        let tmp = TempDir::new();
        let mut data = fixture();
        data.cases.clear();
        for i in 0..30 {
            let mut c = case(
                &data.project,
                &format!("EJ-{i:03}"),
                "Aprobado",
                &[("entorno", "QA")],
            );
            for j in 0..10 {
                let path = write_png(
                    tmp.path(),
                    &format!("{i}-{j}.png"),
                    1920,
                    1080,
                    (i * 10 + j) as u8,
                );
                c.evidence.push(evidence(&format!("Captura {j}"), &path));
            }
            data.cases.push(c);
        }
        let dest = tmp.path().join("grande.xlsx");
        let started = Instant::now();
        let result = export(&data, &settings(), &dest, &|_| {}).unwrap();
        let elapsed = started.elapsed();
        let size = std::fs::metadata(&dest).unwrap().len();
        println!(
            "300 evidencias: {elapsed:?}, {:.1} MB",
            size as f64 / 1_048_576.0
        );
        assert_eq!(result.evidence, 300);
        assert_eq!(Book::at(&dest).pictures(evidence_sheet::SHEET), 300);
        assert!(elapsed.as_secs_f64() < 20.0, "tardó {elapsed:?}");
        // Cada captura entra reducida a 940 px: el libro (y lo que se tiene en memoria) queda acotado.
        assert!(size < 300 * 1024 * 1024, "{size} bytes");
    }
}
```

Run: `cd src-tauri && cargo test excel::tests`
Expected: FAIL de compilación (`export`, `build`… no existen).

- [ ] **Step 5: Implementación — `build` y `export`**

Reemplaza todo lo que está **antes** de `#[cfg(test)]\nmod tests` en `src-tauri/src/excel/mod.rs` por:

```rust
//! Excel con carátula Teams4Soft (spec §7, UC-16 y UC-17), generado con `rust_xlsxwriter`.

// Hasta la Task 10 ningún comando usa este módulo: solo lo ejercitan los tests.
#![allow(dead_code)]

pub mod cover;
pub mod data;
pub mod evidence_sheet;
pub mod images;
pub mod matrix;
pub mod save;
pub mod styles;
pub mod summary;
#[cfg(test)]
pub(crate) mod testing;

use std::path::Path;

use rust_xlsxwriter::{DocProperties, Workbook};

use self::{
    data::{ExportData, ExportSettings},
    styles::Styles,
};
use crate::{
    dto::{ExportProgressDto, ExportResultDto, ExportStage},
    error::AppResult,
};

/// Recibe el avance; se llama desde varios hilos mientras se preparan las imágenes.
pub type Progress<'a> = &'a (dyn Fn(ExportProgressDto) + Sync);

fn report(progress: Progress, stage: ExportStage, current: usize, total: usize) {
    progress(ExportProgressDto {
        stage,
        current: current as i64,
        total: total as i64,
    });
}

pub struct Built {
    pub workbook: Workbook,
    pub missing_images: usize,
}

const COVER: usize = 0;
const MATRIX: usize = 1;
const EVIDENCE: usize = 2;

/// Arma el libro: Carátula, Matriz y, según las opciones, Evidencias y Resumen.
/// Las hojas se crean primero (en su orden final y con el formato por defecto Arial) y la
/// hoja Evidencias se llena antes que la Matriz, que necesita saber dónde empieza cada caso.
pub fn build(data: &ExportData, settings: &ExportSettings, progress: Progress) -> AppResult<Built> {
    let styles = Styles::new();
    report(progress, ExportStage::Preparing, 0, 0);
    let mut workbook = styles::workbook()?;
    let title = if settings.template {
        "Plantilla de pruebas"
    } else {
        "Matriz de pruebas y evidencias"
    };
    workbook.set_properties(
        &DocProperties::new()
            .set_title(title)
            .set_author(&settings.tester)
            .set_company("Teams4Soft"),
    );
    let sheets = 2 + usize::from(settings.include_evidence) + usize::from(settings.include_summary);
    for _ in 0..sheets {
        workbook.add_worksheet();
    }

    let mut missing_images = 0;
    let mut anchors = None;
    if settings.include_evidence {
        let paths: Vec<_> = data
            .cases
            .iter()
            .flat_map(|c| c.evidence.iter().map(|e| e.render_path.clone()))
            .collect();
        let total = paths.len();
        let prepared = images::prepare_all(&paths, settings.image_max_width, &|done| {
            if done == total || done % 10 == 0 {
                report(progress, ExportStage::Images, done, total);
            }
        });
        missing_images = prepared.iter().filter(|i| i.is_none()).count();
        report(progress, ExportStage::Writing, 0, 0);
        let ws = workbook.worksheet_from_index(EVIDENCE)?;
        anchors = Some(evidence_sheet::write(ws, data, prepared, &styles)?);
    } else {
        report(progress, ExportStage::Writing, 0, 0);
    }
    cover::write(
        workbook.worksheet_from_index(COVER)?,
        data,
        settings,
        &styles,
    )?;
    let ws = workbook.worksheet_from_index(MATRIX)?;
    matrix::write(ws, data, anchors.as_deref(), settings.template, &styles)?;
    if settings.include_summary {
        summary::write(workbook.worksheet_from_index(sheets - 1)?, data, &styles)?;
    }
    Ok(Built {
        workbook,
        missing_images,
    })
}

/// Genera el libro y lo guarda en `dest` de forma atómica.
pub fn export(
    data: &ExportData,
    settings: &ExportSettings,
    dest: &Path,
    progress: Progress,
) -> AppResult<ExportResultDto> {
    // Si Excel tiene abierto el archivo, se avisa antes de procesar cientos de imágenes.
    save::ensure_writable(dest)?;
    let mut built = build(data, settings, progress)?;
    report(progress, ExportStage::Saving, 0, 0);
    save::save_atomic(&mut built.workbook, dest)?;
    Ok(ExportResultDto {
        path: dest.to_string_lossy().into_owned(),
        file_name: dest
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        cases: if settings.template {
            0
        } else {
            data.cases.len() as i64
        },
        evidence: if settings.include_evidence {
            data.evidence_total() as i64
        } else {
            0
        },
        missing_images: built.missing_images as i64,
    })
}
```

Run: `cd src-tauri && cargo test excel::`
Expected: PASS. `exports_four_sheets_that_calamine_reads_back` cubre el primer criterio de aceptación: 4 hojas, encabezados iguales a las columnas visibles, N filas para N casos e imágenes contadas. `links_go_both_ways_between_matrix_and_evidence` sigue cada vínculo de ida y de vuelta.

- [ ] **Step 6: Rendimiento (segundo criterio de aceptación)**

```bash
cd src-tauri && cargo test --release excel::tests::exports_300_evidences_in_under_20_seconds -- --ignored --nocapture
```

Expected: PASS y una línea `300 evidencias: <tiempo>, <tamaño> MB` con menos de 20 s (en el equipo donde se preparó el plan, ~3 s con capturas de 1920×1080). Anota el tiempo en el commit.

- [ ] **Step 7: Verificación y commit**

```bash
cd src-tauri && cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test && cd ..
git add src-tauri/src
git commit -m "feat(f4): libro completo con escritura atómica, aviso de archivo abierto y prueba de 300 evidencias

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 10: Comandos `export_project`, `export_template`, `open_export` y `reveal_export` con «Guardar como» nativo

**Files:**
- Create: `src-tauri/src/commands/export.rs`
- Modify:
  - `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`
  - `src-tauri/src/state.rs`, `src-tauri/src/lib.rs`
  - `src-tauri/src/commands/mod.rs`, `src-tauri/src/commands/names.rs`, `src-tauri/capabilities/main.json`
  - `src-tauri/src/excel/mod.rs`, `src-tauri/src/dto.rs`
  - `src/infrastructure/tauri/__fixtures__/contracts.json` (regenerado)

**Interfaces:**
- Consumes: `excel::{export, data::{ExportData, resolve_options, suggested_file_name}, save::{with_xlsx_extension, RecentExports}}`.
- Produces:
  - `export_project(projectId, options: ExportOptionsInput) -> ExportResultDto | null` y `export_template(projectId, options) -> ExportResultDto | null`. `null` significa que el usuario cerró «Guardar como».
  - `open_export(path)` y `reveal_export(path)`: solo aceptan rutas exportadas en la sesión.
  - Evento `export:progress` (`commands::export::EXPORT_PROGRESS_EVENT`) con `ExportProgressDto`, solo hacia `main`.
  - `AppState::exports() -> &RecentExports`.
  - Fixture con `exportResult`, `exportProgress`, `lockedError`, `exportError` e `inputs.exportOptions`.

- [ ] **Step 1: Dependencias**

En `src-tauri/Cargo.toml`, después de la línea de `rust_xlsxwriter`:

```toml
tauri-plugin-dialog = "2.8.3"
tauri-plugin-opener = "2.7.2"
```

- [ ] **Step 2: Estado, comandos y registro**

En `src-tauri/src/state.rs`, antes de:

```rust
use crate::media::storage::EvidenceFiles;
```

agrega:

```rust
use crate::excel::save::RecentExports;
```

En `src-tauri/src/state.rs`, reemplaza:

```rust
    capture: CaptureController,
}
```

por:

```rust
    capture: CaptureController,
    exports: RecentExports,
}
```

En `src-tauri/src/state.rs`, después de:

```rust
            capture: CaptureController::new(HIDE_DELAY, Some(READY_TIMEOUT)),
```

agrega:

```rust
            exports: RecentExports::default(),
```

En `src-tauri/src/state.rs`, después de:

```rust
    pub fn files(&self) -> &EvidenceFiles {
        &self.files
    }
```

agrega:

```rust
    pub fn exports(&self) -> &RecentExports {
        &self.exports
    }
```

Crea `src-tauri/src/commands/export.rs`:

```rust
use std::path::PathBuf;

use tauri::{AppHandle, Emitter, State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

use crate::{
    dto::{ExportOptionsInput, ExportProgressDto, ExportResultDto},
    error::{AppError, AppResult},
    excel::{
        self,
        data::{ExportData, resolve_options, suggested_file_name},
        save::with_xlsx_extension,
    },
    state::AppState,
};

pub const EXPORT_PROGRESS_EVENT: &str = "export:progress";

/// `None`: el usuario cerró «Guardar como» sin elegir archivo.
fn run(
    app: &AppHandle,
    window: &WebviewWindow,
    state: &AppState,
    project_id: &str,
    options: &ExportOptionsInput,
    template: bool,
) -> AppResult<Option<ExportResultDto>> {
    let today = chrono::Local::now().date_naive();
    // La base se lee de una vez y se suelta antes del diálogo y de generar el libro.
    let data = ExportData::load(&state.db(), state.files(), project_id)?;
    let settings = resolve_options(&data, options, template, today)?;
    let name = suggested_file_name(&data.project, &settings);
    let Some(dest) = ask_destination(window, &name) else {
        return Ok(None);
    };
    let progress = |p: ExportProgressDto| {
        let _ = app.emit_to("main", EXPORT_PROGRESS_EVENT, p);
    };
    let result = excel::export(&data, &settings, &dest, &progress)?;
    state.exports().remember(&dest);
    Ok(Some(result))
}

fn ask_destination(window: &WebviewWindow, name: &str) -> Option<PathBuf> {
    let path = window
        .dialog()
        .file()
        .set_parent(window)
        .set_title("Guardar Excel")
        .set_file_name(name)
        .add_filter("Libro de Excel", &["xlsx"])
        .blocking_save_file()?
        .into_path()
        .ok()?;
    Some(with_xlsx_extension(path))
}

fn opener_error(error: tauri_plugin_opener::Error) -> AppError {
    AppError::Io(std::io::Error::other(error.to_string()))
}

#[tauri::command(async)]
pub fn export_project(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, AppState>,
    project_id: String,
    options: ExportOptionsInput,
) -> AppResult<Option<ExportResultDto>> {
    run(&app, &window, &state, &project_id, &options, false)
}

#[tauri::command(async)]
pub fn export_template(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, AppState>,
    project_id: String,
    options: ExportOptionsInput,
) -> AppResult<Option<ExportResultDto>> {
    run(&app, &window, &state, &project_id, &options, true)
}

#[tauri::command(async)]
pub fn open_export(app: AppHandle, state: State<'_, AppState>, path: String) -> AppResult<()> {
    let path = state.exports().resolve(&path)?;
    app.opener()
        .open_path(path.to_string_lossy(), None::<&str>)
        .map_err(opener_error)
}

#[tauri::command(async)]
pub fn reveal_export(app: AppHandle, state: State<'_, AppState>, path: String) -> AppResult<()> {
    let path = state.exports().resolve(&path)?;
    app.opener().reveal_item_in_dir(path).map_err(opener_error)
}
```

En `src-tauri/src/commands/mod.rs`, después de:

```rust
pub mod evidence;
```

agrega:

```rust
pub mod export;
```

En `src-tauri/src/commands/names.rs`, reemplaza:

```rust
    "delete_doc_version",
];
```

por:

```rust
    "delete_doc_version",
    "export_project",
    "export_template",
    "open_export",
    "reveal_export",
];
```

En `src-tauri/src/lib.rs`, antes de:

```rust
        .setup(|app| {
```

agrega:

```rust
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
```

En `src-tauri/src/lib.rs`, después de:

```rust
            commands::doc_versions::delete_doc_version,
```

agrega:

```rust
            commands::export::export_project,
            commands::export::export_template,
            commands::export::open_export,
            commands::export::reveal_export,
```

En `src-tauri/capabilities/main.json`, reemplaza:

```json
    "allow-delete-doc-version"
```

por:

```json
    "allow-delete-doc-version",
    "allow-export-project",
    "allow-export-template",
    "allow-open-export",
    "allow-reveal-export"
```

Quita de `src-tauri/src/excel/mod.rs` el comentario `// Hasta la Task 10 ningún comando usa este módulo…` y la línea `#![allow(dead_code)]`. Desde esta task todo se usa, también las ayudas de `excel/testing.rs`.

- [ ] **Step 3: Contrato**

En `src-tauri/src/dto.rs`, reemplaza:

```rust
                                 "reviewer": "Luis Gómez", "approver": "", "changes": "Versión inicial" }
        })
```

por:

```rust
                                 "reviewer": "Luis Gómez", "approver": "", "changes": "Versión inicial" },
            "exportOptions": { "tester": "Ana Pérez", "docVersionId": "ver-1", "includeEvidence": true,
                               "includeSummary": true, "imageMaxWidth": 940 }
        })
```

En `src-tauri/src/dto.rs`, reemplaza:

```rust
                                          changes: "Versión inicial".into(), created_at: "2026-10-16T12:00:00.000Z".into() },
        })
```

por:

```rust
                                          changes: "Versión inicial".into(), created_at: "2026-10-16T12:00:00.000Z".into() },
            "exportResult": ExportResultDto { path: "C:\\Users\\ana\\Documents\\T4S_ACME_Recibos_Evidencias_20261016_v1.0.xlsx".into(),
                                              file_name: "T4S_ACME_Recibos_Evidencias_20261016_v1.0.xlsx".into(),
                                              cases: 10, evidence: 20, missing_images: 0 },
            "exportProgress": ExportProgressDto { stage: ExportStage::Images, current: 12, total: 20 },
            "lockedError": AppError::FileLocked("C:\\Users\\ana\\Documents\\matriz.xlsx".into()),
            "exportError": AppError::Export("rust_xlsxwriter: formato inválido".into()),
        })
```

En `src-tauri/src/dto.rs`, después de:

```rust
        serde_json::from_value::<DocVersionInput>(i["docVersionInput"].clone()).unwrap();
```

agrega:

```rust
        serde_json::from_value::<ExportOptionsInput>(i["exportOptions"].clone()).unwrap();
```

```bash
cd src-tauri && UPDATE_CONTRACTS=1 cargo test contract
```

- [ ] **Step 4: Verificación**

```bash
cd src-tauri && cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test && cd ..
pnpm typecheck && pnpm test
```

Expected: todo en verde. Los tests de `names.rs` confirman que los 4 comandos nuevos están en `names.rs`, en `lib.rs` y en `main.json`, y que el overlay sigue sin acceso a ellos.

- [ ] **Step 5: Smoke manual del comando**

`pnpm tauri dev`. Abre un proyecto con casos y evidencias. Desde la consola del WebView (F12), registra una versión y exporta:

```js
const { invoke } = window.__TAURI_INTERNALS__;
const [p] = await invoke("list_projects");
const v = await invoke("create_doc_version", { projectId: p.id, input: { version: "0.9", date: "2026-10-16", author: "Ana", reviewer: "", approver: "", changes: "Smoke" } });
await invoke("export_project", { projectId: p.id, options: { tester: "Ana", docVersionId: v.id, includeEvidence: true, includeSummary: true, imageMaxWidth: 940 } });
```

Expected: se abre «Guardar como» con el nombre `T4S_<Cliente>_<Proyecto>_Evidencias_<hoy>_v0.9.xlsx` y la promesa devuelve `{ path, fileName, cases, evidence, missingImages }`. Si cierras el diálogo, devuelve `null`. Abre el archivo en Excel: no muestra advertencias. Con el archivo abierto, exporta otra vez al mismo nombre: la promesa se rechaza con `code: "file_locked"`. Después borra la versión de prueba con `invoke("delete_doc_version", { id: v.id })`.

- [ ] **Step 6: Commit**

```bash
git add src-tauri src/infrastructure/tauri/__fixtures__/contracts.json
git commit -m "feat(f4): comandos de exportación con «Guardar como» nativo, avance y apertura del archivo

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---


### Task 11: Exportación en el front — dominio, puerto `Exporter`, adaptadores, caso de uso, store y toasts con acciones

**Files:**
- Create: `src/domain/export.ts`, `src/domain/export.test.ts`
- Modify (se reemplazan completos):
  - `src/ports/export.ts`
  - `src/infrastructure/tauri/export.ts`
  - `src/infrastructure/memory/export.ts`, `src/infrastructure/memory/export.test.ts`
  - `src/ui/store/export-slice.ts`, `src/ui/store/export-slice.test.ts`
  - `src/ui/components/Toasts.tsx`
- Modify (cambios puntuales):
  - `src/ports/repositories.ts`, `src/ports/errors.ts`
  - `src/infrastructure/tauri/dto.ts`, `src/infrastructure/tauri/repositories.ts`, `src/infrastructure/tauri/contract.test.ts`
  - `src/infrastructure/memory/repositories.ts`
  - `src/application/use-cases.ts`, `src/ui/store/workspace.ts`

**Interfaces:**
- Consumes: comandos y fixture de la Task 10; `docVersions` y `saveDocVersion` de la Task 2.
- Produces:
  - `domain/export.ts`:
    - `IMAGE_WIDTH = { min: 400, max: 1600, default: 940 }`, `ExportKind = "project" | "template"`;
    - `ExportOptions { tester, docVersionId: string | null, includeEvidence, includeSummary, imageMaxWidth }`, `ExportResult`, `EXPORT_STAGES`, `ExportProgress`;
    - `defaultExportOptions(tester)`, `validateExportOptions(options, kind)`, `progressLabel(p)`, `progressPercent(p)`.
  - `ports/export.ts`: `Exporter { exportProject, exportTemplate (→ ExportResult | null), onProgress(handler) → unsubscribe, openFile(path), revealFile(path) }`. `Repositories.exporter`.
  - `AppErrorCode` suma `"file_locked"` y `"export"`.
  - `application/use-cases.ts`:
    - `ExportRequest { kind, options, newVersion?: DocVersionInput }`;
    - `exportWorkbook(repos, projectId, request, onProgress) → Outcome<ExportResult | null>`. `file_locked` vuelve como `{ _form }`.
  - `MemoryExporter`, con `mode: "save" | "cancel" | "locked"`, `calls`, `opened`, `revealed`, `missingImages` y `emit(progress)`.
  - Store:
    - `exporting: ExportProgress | null` y `exportExcel(request) → FormResult`. Si se canceló «Guardar como», devuelve `{ ok: false, errors: {} }`;
    - `Toast.actions?: ToastAction[]` y `pushToast(kind, message, actions?)`.

- [ ] **Step 1: Test que falla — dominio**

Crea `src/domain/export.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { defaultExportOptions, progressLabel, progressPercent, validateExportOptions } from "./export";

describe("opciones de exportación", () => {
  const ok = { ...defaultExportOptions("Ana Pérez"), docVersionId: "ver-1" };

  it("exigen tester y, para el proyecto, versión", () => {
    expect(validateExportOptions(ok, "project")).toEqual({});
    expect(validateExportOptions({ ...ok, tester: "  ", docVersionId: null }, "project")).toEqual({
      tester: "«Tester» es obligatorio.",
      docVersionId: "Elige la versión del documento.",
    });
    expect(validateExportOptions({ ...ok, docVersionId: null }, "template")).toEqual({});
    expect(validateExportOptions({ ...ok, tester: "a".repeat(101) }, "project").tester).toBe("«Tester» supera 100 caracteres.");
  });

  it("acotan el ancho de imagen", () => {
    expect(ok.imageMaxWidth).toBe(940);
    for (const bad of [399, 1601, 900.5]) {
      expect(validateExportOptions({ ...ok, imageMaxWidth: bad }, "project").imageMaxWidth).toBe(
        "El ancho máximo de imagen debe estar entre 400 y 1600 px.",
      );
    }
  });

  it("describen el avance", () => {
    expect(progressLabel(null)).toBe("");
    expect(progressLabel({ stage: "images", current: 120, total: 300 })).toBe("Procesando imágenes 120 de 300…");
    expect(progressLabel({ stage: "saving", current: 0, total: 0 })).toBe("Guardando el archivo…");
    expect(progressPercent({ stage: "images", current: 150, total: 300 })).toBe(45);
    expect(progressPercent({ stage: "images", current: 0, total: 0 })).toBe(5);
    expect(progressPercent({ stage: "writing", current: 0, total: 0 })).toBe(85);
  });
});
```

Run: `pnpm vitest run src/domain/export.test.ts`
Expected: FAIL (`./export` no existe).

- [ ] **Step 2: Dominio**

Crea `src/domain/export.ts`:

```ts
import type { FormErrors } from "./cases";
import { charLength } from "./fields";

/** Ancho máximo de las capturas en la hoja Evidencias. F6 lo llevará a Settings. */
export const IMAGE_WIDTH = { min: 400, max: 1600, default: 940 } as const;

export type ExportKind = "project" | "template";

export interface ExportOptions {
  tester: string;
  /** Obligatoria para el proyecto; la plantilla vacía no lleva versión en el nombre. */
  docVersionId: string | null;
  includeEvidence: boolean;
  includeSummary: boolean;
  imageMaxWidth: number;
}

export interface ExportResult {
  path: string;
  fileName: string;
  cases: number;
  evidence: number;
  missingImages: number;
}

export const EXPORT_STAGES = ["preparing", "images", "writing", "saving"] as const;
export type ExportStage = (typeof EXPORT_STAGES)[number];

export interface ExportProgress {
  stage: ExportStage;
  current: number;
  total: number;
}

export const defaultExportOptions = (tester: string): ExportOptions => ({
  tester,
  docVersionId: null,
  includeEvidence: true,
  includeSummary: true,
  imageMaxWidth: IMAGE_WIDTH.default,
});

/** Espejo de `excel::data::resolve_options` para avisar antes de abrir «Guardar como». */
export function validateExportOptions(options: ExportOptions, kind: ExportKind): FormErrors {
  const errors: FormErrors = {};
  const tester = options.tester.trim();
  if (!tester) errors.tester = "«Tester» es obligatorio.";
  else if (charLength(tester) > 100) errors.tester = "«Tester» supera 100 caracteres.";
  if (kind === "project" && !options.docVersionId) errors.docVersionId = "Elige la versión del documento.";
  const width = options.imageMaxWidth;
  if (!Number.isInteger(width) || width < IMAGE_WIDTH.min || width > IMAGE_WIDTH.max) {
    errors.imageMaxWidth = `El ancho máximo de imagen debe estar entre ${IMAGE_WIDTH.min} y ${IMAGE_WIDTH.max} px.`;
  }
  return errors;
}

export function progressLabel(progress: ExportProgress | null): string {
  if (!progress) return "";
  switch (progress.stage) {
    case "preparing":
      return "Preparando los datos…";
    case "images":
      return `Procesando imágenes ${progress.current} de ${progress.total}…`;
    case "writing":
      return "Escribiendo el libro…";
    case "saving":
      return "Guardando el archivo…";
  }
}

/** 0–100 para la barra de progreso; las etapas sin conteo avanzan por tramos. */
export function progressPercent(progress: ExportProgress | null): number {
  if (!progress) return 0;
  const base = { preparing: 0, images: 5, writing: 85, saving: 95 }[progress.stage];
  if (progress.stage !== "images" || progress.total === 0) return base;
  return Math.round(base + (80 * progress.current) / progress.total);
}
```

Run: `pnpm vitest run src/domain/export.test.ts`
Expected: PASS (3 tests).

- [ ] **Step 3: Tests que fallan — adaptadores**

Reemplaza `src/infrastructure/memory/export.test.ts` por:

```ts
import { describe, expect, it, vi } from "vitest";
import { emptyDocVersion } from "../../domain/doc-versions";
import { defaultExportOptions } from "../../domain/export";
import { emptyMeta } from "../../domain/project";
import { createMemoryRepositories } from "./repositories";

async function setup() {
  const repos = createMemoryRepositories();
  const p = await repos.projects.create({ meta: { ...emptyMeta(), name: "Recibos", client: "ACME" }, templateId: "tpl-funcional" });
  return { repos, p };
}

const input = (version: string, date = "2026-10-16") => ({ ...emptyDocVersion(date, "Ana Pérez"), version });

describe("versiones del documento en memoria", () => {
  it("lista por fecha, valida y no repite versiones", async () => {
    const { repos, p } = await setup();
    await repos.docVersions.create(p.id, input(" 1.1 ", "2026-10-20"));
    const first = await repos.docVersions.create(p.id, input("1.0", "2026-10-01"));
    expect((await repos.docVersions.list(p.id)).map((v) => v.version)).toEqual(["1.0", "1.1"]);
    await expect(repos.docVersions.create(p.id, input("V1.1"))).resolves.toBeTruthy();
    await expect(repos.docVersions.create(p.id, input("1.0"))).rejects.toMatchObject({ message: "La versión «1.0» ya existe en este proyecto." });
    await expect(repos.docVersions.create(p.id, input(""))).rejects.toMatchObject({ message: "«Versión» es obligatorio." });
    const updated = await repos.docVersions.update(first.id, { ...input("1.0"), changes: "Casos de Pólizas" });
    expect(updated.changes).toBe("Casos de Pólizas");
    await repos.docVersions.remove(first.id);
    await expect(repos.docVersions.remove(first.id)).rejects.toMatchObject({ code: "not_found" });
  });

  it("se borran con el proyecto", async () => {
    const { repos, p } = await setup();
    await repos.docVersions.create(p.id, input("1.0"));
    await repos.projects.remove(p.id);
    await expect(repos.docVersions.list(p.id)).rejects.toMatchObject({ code: "not_found" });
  });
});

describe("exportador en memoria", () => {
  it("exporta, emite avance y solo abre lo exportado", async () => {
    const { repos, p } = await setup();
    const progress = vi.fn();
    const off = await repos.exporter.onProgress(progress);
    const result = await repos.exporter.exportProject(p.id, { ...defaultExportOptions("Ana"), docVersionId: "ver-1" });
    expect(result).toMatchObject({ fileName: "T4S_ACME_Recibos_Evidencias_20261016_v1.0.xlsx", cases: 0, evidence: 0 });
    expect(progress).toHaveBeenCalledWith({ stage: "saving", current: 0, total: 0 });
    off();
    await repos.exporter.openFile(result!.path);
    expect(repos.exporter.opened).toEqual([result!.path]);
    await expect(repos.exporter.revealFile("C:\\otro.xlsx")).rejects.toMatchObject({ code: "validation" });
  });

  it("simula cancelar «Guardar como» y el archivo abierto en Excel", async () => {
    const { repos, p } = await setup();
    repos.exporter.mode = "cancel";
    await expect(repos.exporter.exportTemplate(p.id, defaultExportOptions("Ana"))).resolves.toBeNull();
    repos.exporter.mode = "locked";
    await expect(repos.exporter.exportProject(p.id, defaultExportOptions("Ana"))).rejects.toMatchObject({
      code: "file_locked",
      message: "Cierra el archivo en Excel y vuelve a intentar.",
    });
    expect(repos.exporter.calls.map((c) => c.kind)).toEqual(["template", "project"]);
  });
});
```

En `src/infrastructure/tauri/contract.test.ts`, después de:

```ts
import { emptyDocVersion } from "../../domain/doc-versions";
```

agrega:

```ts
import { defaultExportOptions } from "../../domain/export";
```

En `src/infrastructure/tauri/contract.test.ts`, reemplaza:

```ts
import { appErrorSchema, capturedEventSchema, caseSchema, docVersionSchema, evidenceSchema, fieldSchema,
```

por:

```ts
import { appErrorSchema, capturedEventSchema, caseSchema, docVersionSchema, evidenceSchema, exportProgressSchema, exportResultSchema, fieldSchema,
```

En `src/infrastructure/tauri/contract.test.ts`, después de:

```ts
    ["docVersion", docVersionSchema],
```

agrega:

```ts
    ["exportResult", exportResultSchema],
    ["exportProgress", exportProgressSchema],
    ["lockedError", appErrorSchema],
    ["exportError", appErrorSchema],
```

En `src/infrastructure/tauri/contract.test.ts`, después de:

```ts
    expect(sortedKeys(emptyDocVersion(""))).toEqual(sortedKeys(fixture.inputs.docVersionInput));
```

agrega:

```ts
    expect(sortedKeys(defaultExportOptions(""))).toEqual(sortedKeys(fixture.inputs.exportOptions));
```

Run: `pnpm vitest run src/infrastructure`
Expected: FAIL (`repos.exporter` no existe; faltan los esquemas `exportResultSchema` y `exportProgressSchema`).

- [ ] **Step 4: Puerto y adaptadores**

Reemplaza `src/ports/export.ts` por:

```ts
import type { DocVersion, DocVersionInput } from "../domain/doc-versions";
import type { ExportOptions, ExportProgress, ExportResult } from "../domain/export";

/** Control de versiones del documento (UC-05). */
export interface DocVersionRepository {
  list(projectId: string): Promise<DocVersion[]>;
  create(projectId: string, input: DocVersionInput): Promise<DocVersion>;
  update(id: string, input: DocVersionInput): Promise<DocVersion>;
  remove(id: string): Promise<void>;
}

/** Excel con carátula (UC-16) y plantilla vacía (UC-17). Rust abre «Guardar como». */
export interface Exporter {
  /** `null`: el usuario cerró «Guardar como» sin elegir archivo. */
  exportProject(projectId: string, options: ExportOptions): Promise<ExportResult | null>;
  exportTemplate(projectId: string, options: ExportOptions): Promise<ExportResult | null>;
  /** Suscribe al evento `export:progress`; devuelve la función para desuscribirse. */
  onProgress(handler: (progress: ExportProgress) => void): Promise<() => void>;
  /** Solo acepta archivos exportados en esta sesión. */
  openFile(path: string): Promise<void>;
  revealFile(path: string): Promise<void>;
}
```

Reemplaza `src/infrastructure/memory/export.ts` por:

```ts
import { trimDocVersion, validateDocVersion, type DocVersion, type DocVersionInput } from "../../domain/doc-versions";
import type { ExportOptions, ExportProgress, ExportResult } from "../../domain/export";
import { AppError } from "../../ports/errors";
import type { DocVersionRepository, Exporter } from "../../ports/export";

export interface MemoryDocVersionDeps {
  ensureProject(projectId: string): void;
  touch(projectId: string): void;
  id(prefix: string): string;
  now(): string;
}

export interface MemoryDocVersions extends DocVersionRepository {
  removeForProject(projectId: string): void;
}

/** Replica en memoria las reglas de `repo/doc_versions.rs`. */
export function createMemoryDocVersions(deps: MemoryDocVersionDeps): MemoryDocVersions {
  const items = new Map<string, DocVersion>();
  const fail = (message: string): never => {
    throw new AppError("validation", message);
  };
  const get = (id: string) => {
    const found = items.get(id);
    if (!found) throw new AppError("not_found", "La versión del documento no existe.");
    return found;
  };
  const check = (projectId: string, raw: DocVersionInput, except?: string) => {
    const first = Object.values(validateDocVersion(raw))[0];
    if (first) fail(first);
    const input = trimDocVersion(raw);
    const taken = [...items.values()].some(
      (v) => v.projectId === projectId && v.id !== except && v.version.toLowerCase() === input.version.toLowerCase(),
    );
    if (taken) fail(`La versión «${input.version}» ya existe en este proyecto.`);
    return input;
  };
  const sorted = (projectId: string) =>
    [...items.values()]
      .filter((v) => v.projectId === projectId)
      .sort((a, b) => a.date.localeCompare(b.date) || a.createdAt.localeCompare(b.createdAt));

  return {
    async list(projectId) {
      deps.ensureProject(projectId);
      return structuredClone(sorted(projectId));
    },
    async create(projectId, raw) {
      deps.ensureProject(projectId);
      const input = check(projectId, raw);
      const version: DocVersion = { ...input, id: deps.id("ver"), projectId, createdAt: deps.now() };
      items.set(version.id, version);
      deps.touch(projectId);
      return structuredClone(version);
    },
    async update(id, raw) {
      const current = get(id);
      Object.assign(current, check(current.projectId, raw, id));
      deps.touch(current.projectId);
      return structuredClone(current);
    },
    async remove(id) {
      const current = get(id);
      items.delete(id);
      deps.touch(current.projectId);
    },
    removeForProject(projectId) {
      for (const v of sorted(projectId)) items.delete(v.id);
    },
  };
}

/** save: guarda · cancel: el usuario cierra «Guardar como» · locked: el archivo está abierto en Excel. */
export type MemoryExportMode = "save" | "cancel" | "locked";

export interface MemoryExporter extends Exporter {
  mode: MemoryExportMode;
  calls: { kind: "project" | "template"; projectId: string; options: ExportOptions }[];
  opened: string[];
  revealed: string[];
  /** Imágenes que el próximo export no pudo incluir. */
  missingImages: number;
  emit(progress: ExportProgress): void;
}

export function createMemoryExporter(countFor: (projectId: string) => { cases: number; evidence: number }): MemoryExporter {
  const handlers = new Set<(progress: ExportProgress) => void>();
  const exported = new Set<string>();
  const run = async (kind: "project" | "template", projectId: string, options: ExportOptions): Promise<ExportResult | null> => {
    exporter.calls.push({ kind, projectId, options: structuredClone(options) });
    if (exporter.mode === "cancel") return null;
    if (exporter.mode === "locked") throw new AppError("file_locked", "Cierra el archivo en Excel y vuelve a intentar.");
    exporter.emit({ stage: "preparing", current: 0, total: 0 });
    exporter.emit({ stage: "saving", current: 0, total: 0 });
    const fileName = kind === "project" ? "T4S_ACME_Recibos_Evidencias_20261016_v1.0.xlsx" : "T4S_ACME_Recibos_Plantilla_20261016.xlsx";
    const path = `C:\\Users\\ana\\Documents\\${fileName}`;
    exported.add(path);
    const counts = kind === "project" ? countFor(projectId) : { cases: 0, evidence: 0 };
    return { path, fileName, ...counts, missingImages: exporter.missingImages };
  };
  const known = (path: string) => {
    if (!exported.has(path)) throw new AppError("validation", "Solo se pueden abrir archivos exportados en esta sesión.");
  };
  const exporter: MemoryExporter = {
    mode: "save",
    calls: [],
    opened: [],
    revealed: [],
    missingImages: 0,
    exportProject: (projectId, options) => run("project", projectId, options),
    exportTemplate: (projectId, options) => run("template", projectId, options),
    async onProgress(handler) {
      handlers.add(handler);
      return () => void handlers.delete(handler);
    },
    async openFile(path) {
      known(path);
      exporter.opened.push(path);
    },
    async revealFile(path) {
      known(path);
      exporter.revealed.push(path);
    },
    emit(progress) {
      handlers.forEach((handler) => handler(progress));
    },
  };
  return exporter;
}
```

Reemplaza `src/infrastructure/tauri/export.ts` por:

```ts
import { listen } from "@tauri-apps/api/event";
import { z } from "zod";
import type { DocVersionRepository, Exporter } from "../../ports/export";
import { call } from "./client";
import { docVersionSchema, exportProgressSchema, exportResultSchema } from "./dto";

const nothing = z.null();

export const EXPORT_PROGRESS_EVENT = "export:progress";

export function tauriDocVersions(): DocVersionRepository {
  return {
    list: (projectId) => call("list_doc_versions", { projectId }, z.array(docVersionSchema)),
    create: (projectId, input) => call("create_doc_version", { projectId, input }, docVersionSchema),
    update: (id, input) => call("update_doc_version", { id, input }, docVersionSchema),
    remove: async (id) => {
      await call("delete_doc_version", { id }, nothing);
    },
  };
}

export function tauriExporter(): Exporter {
  return {
    exportProject: (projectId, options) => call("export_project", { projectId, options }, exportResultSchema.nullable()),
    exportTemplate: (projectId, options) => call("export_template", { projectId, options }, exportResultSchema.nullable()),
    onProgress: (handler) =>
      listen<unknown>(EXPORT_PROGRESS_EVENT, (event) => {
        const parsed = exportProgressSchema.safeParse(event.payload);
        if (parsed.success) handler(parsed.data);
      }),
    openFile: async (path) => {
      await call("open_export", { path }, nothing);
    },
    revealFile: async (path) => {
      await call("reveal_export", { path }, nothing);
    },
  };
}
```

Agrega al final de `src/infrastructure/tauri/dto.ts`:

```ts
export const exportResultSchema = z.strictObject({
  path: z.string(),
  fileName: z.string(),
  cases: z.number().int(),
  evidence: z.number().int(),
  missingImages: z.number().int(),
});

export const exportProgressSchema = z.strictObject({
  stage: z.enum(EXPORT_STAGES),
  current: z.number().int(),
  total: z.number().int(),
});
```

En `src/ports/repositories.ts`, reemplaza:

```ts
import type { DocVersionRepository } from "./export";
```

por:

```ts
import type { DocVersionRepository, Exporter } from "./export";
```

En `src/ports/repositories.ts`, reemplaza:

```ts
  docVersions: DocVersionRepository;
}
```

por:

```ts
  docVersions: DocVersionRepository;
  exporter: Exporter;
}
```

En `src/ports/errors.ts`, reemplaza:

```ts
export type AppErrorCode = "validation" | "not_found" | "db" | "io" | "capture" | "internal" | "contract";
```

por:

```ts
export type AppErrorCode = "validation" | "not_found" | "db" | "io" | "capture" | "file_locked" | "export" | "internal" | "contract";
```

En `src/infrastructure/tauri/dto.ts`, antes de:

```ts
import { STATUSES } from "../../domain/status";
```

agrega:

```ts
import { EXPORT_STAGES } from "../../domain/export";
```

En `src/infrastructure/tauri/dto.ts`, reemplaza:

```ts
  code: z.enum(["validation", "not_found", "db", "io", "capture", "internal"]),
```

por:

```ts
  code: z.enum(["validation", "not_found", "db", "io", "capture", "file_locked", "export", "internal"]),
```

En `src/infrastructure/tauri/repositories.ts`, reemplaza:

```ts
import { tauriDocVersions } from "./export";
```

por:

```ts
import { tauriDocVersions, tauriExporter } from "./export";
```

En `src/infrastructure/tauri/repositories.ts`, reemplaza:

```ts
    docVersions: tauriDocVersions(),
  };
```

por:

```ts
    docVersions: tauriDocVersions(),
    exporter: tauriExporter(),
  };
```

En `src/infrastructure/memory/repositories.ts`, reemplaza:

```ts
import { createMemoryDocVersions, type MemoryDocVersions } from "./export";
```

por:

```ts
import { createMemoryDocVersions, createMemoryExporter, type MemoryDocVersions, type MemoryExporter } from "./export";
```

En `src/infrastructure/memory/repositories.ts`, reemplaza:

```ts
  docVersions: MemoryDocVersions;
}
```

por:

```ts
  docVersions: MemoryDocVersions;
  exporter: MemoryExporter;
}
```

En `src/infrastructure/memory/repositories.ts`, reemplaza:

```ts
    docVersions,
  };
```

por:

```ts
    docVersions,
    exporter: createMemoryExporter((projectId) => {
      const list = projectCases(projectId);
      return { cases: list.length, evidence: list.reduce((n, c) => n + evidence.countFor(c.id), 0) };
    }),
  };
```

Run: `pnpm vitest run src/infrastructure`
Expected: PASS.

- [ ] **Step 5: Tests que fallan — store**

Reemplaza `src/ui/store/export-slice.test.ts` por:

```ts
import { describe, expect, it } from "vitest";
import { emptyDocVersion } from "../../domain/doc-versions";
import { defaultExportOptions, type ExportProgress } from "../../domain/export";
import { createMemoryRepositories } from "../../infrastructure/memory/repositories";
import { seedProject } from "../test-utils";
import { memoryPreferences } from "./preferences";
import { createWorkspaceStore } from "./workspace";

async function setup() {
  const repos = createMemoryRepositories();
  const p = await seedProject(repos);
  const prefs = memoryPreferences(p.id, "Ana Pérez");
  const store = createWorkspaceStore(repos, prefs);
  await store.getState().init();
  return { repos, store, prefs, p };
}

const newVersion = { ...emptyDocVersion("2026-10-16", "Ana Pérez"), version: "1.0", changes: "Versión inicial" };
const options = defaultExportOptions("Ana Pérez");

describe("slice de exportación", () => {
  it("carga las versiones del proyecto abierto y las guarda con toast", async () => {
    const { store } = await setup();
    expect(store.getState().docVersions).toEqual([]);
    expect(await store.getState().saveDocVersion(newVersion)).toEqual({ ok: true });
    expect(store.getState().docVersions.map((v) => v.version)).toEqual(["1.0"]);
    expect(store.getState().toasts.at(-1)?.message).toBe("Versión 1.0 registrada.");
    expect(await store.getState().saveDocVersion({ ...newVersion, author: " " })).toEqual({
      ok: false,
      errors: { author: "«Elaborado por» es obligatorio." },
    });
  });

  it("publica el avance mientras exporta y lo limpia al terminar", async () => {
    const { store } = await setup();
    const seen: (ExportProgress | null)[] = [];
    store.subscribe((s, prev) => {
      if (s.exporting !== prev.exporting) seen.push(s.exporting);
    });
    expect(await store.getState().exportExcel({ kind: "project", options, newVersion })).toEqual({ ok: true });
    expect(seen).toContainEqual({ stage: "saving", current: 0, total: 0 });
    expect(seen.at(-1)).toBeNull();
    expect(store.getState().toasts.at(-1)?.actions?.map((a) => a.label)).toEqual(["Abrir archivo", "Abrir carpeta"]);
  });

  it("rechaza un segundo export mientras corre el primero", async () => {
    const { store, repos } = await setup();
    const first = store.getState().exportExcel({ kind: "template", options });
    expect(await store.getState().exportExcel({ kind: "template", options })).toEqual({
      ok: false,
      errors: { _form: "Ya hay una exportación en curso." },
    });
    expect(await first).toEqual({ ok: true });
    expect(repos.exporter.calls).toHaveLength(1);
  });

  it("si se cancela «Guardar como» la versión nueva queda registrada una sola vez", async () => {
    const { store, repos, prefs } = await setup();
    repos.exporter.mode = "cancel";
    expect(await store.getState().exportExcel({ kind: "project", options: { ...options, tester: " Luis " }, newVersion })).toEqual({ ok: false, errors: {} });
    expect(store.getState().docVersions).toHaveLength(1);
    expect(prefs.getTester()).toBe("Luis");
    const [saved] = store.getState().docVersions;
    repos.exporter.mode = "save";
    expect(await store.getState().exportExcel({ kind: "project", options: { ...options, docVersionId: saved.id } })).toEqual({ ok: true });
    expect(store.getState().docVersions).toHaveLength(1);
  });

  it("el archivo abierto en Excel vuelve como error del formulario", async () => {
    const { store, repos } = await setup();
    repos.exporter.mode = "locked";
    expect(await store.getState().exportExcel({ kind: "project", options, newVersion })).toEqual({
      ok: false,
      errors: { _form: "Cierra el archivo en Excel y vuelve a intentar." },
    });
    expect(store.getState().exporting).toBeNull();
  });
});
```

Run: `pnpm vitest run src/ui/store/export-slice.test.ts`
Expected: FAIL (`exportExcel` no existe).

- [ ] **Step 6: Caso de uso, slice y toasts con acciones**

En `src/application/use-cases.ts`, después de:

```ts
import { validateDescription, type Evidence, type EvidenceUpdateInput, type NewEvidenceInput } from "../domain/evidence";
```

agrega:

```ts
import { validateExportOptions, type ExportKind, type ExportOptions, type ExportProgress, type ExportResult } from "../domain/export";
```

En `src/application/use-cases.ts`, reemplaza:

```ts
import { isValidationError } from "../ports/errors";
```

por:

```ts
import { AppError, isValidationError } from "../ports/errors";
```

Agrega al final de `src/application/use-cases.ts`:

```ts
export interface ExportRequest {
  kind: ExportKind;
  options: ExportOptions;
  /** Se registra antes de exportar y su id reemplaza a `options.docVersionId`. */
  newVersion?: DocVersionInput;
}

/**
 * UC-16 / UC-17. `value: null` = el usuario cerró «Guardar como». Una versión nueva queda
 * registrada aunque después se cancele. «Archivo abierto en Excel» vuelve como error del formulario.
 */
export async function exportWorkbook(
  repos: Repositories,
  projectId: string,
  request: ExportRequest,
  onProgress: (progress: ExportProgress) => void,
): Promise<Outcome<ExportResult | null>> {
  const { kind, options, newVersion } = request;
  const pending = newVersion ? { ...options, docVersionId: "nueva" } : options;
  const errors = { ...(newVersion ? validateDocVersion(newVersion) : {}), ...validateExportOptions(pending, kind) };
  if (invalid(errors)) return { ok: false, errors };
  try {
    return await guard(async () => {
      const docVersionId = newVersion ? (await repos.docVersions.create(projectId, trimDocVersion(newVersion))).id : options.docVersionId;
      const clean = { ...options, tester: options.tester.trim(), docVersionId };
      const off = await repos.exporter.onProgress(onProgress);
      try {
        return kind === "project" ? await repos.exporter.exportProject(projectId, clean) : await repos.exporter.exportTemplate(projectId, clean);
      } finally {
        off();
      }
    });
  } catch (error) {
    if (error instanceof AppError && error.code === "file_locked") return { ok: false, errors: { _form: error.message } };
    throw error;
  }
}
```

Reemplaza `src/ui/store/export-slice.ts` por:

```ts
import type { StoreApi } from "zustand/vanilla";
import * as useCases from "../../application/use-cases";
import type { DocVersion, DocVersionInput } from "../../domain/doc-versions";
import type { ExportProgress } from "../../domain/export";
import type { Repositories } from "../../ports/repositories";
import type { SliceHelpers } from "./evidence-slice";
import type { Preferences } from "./preferences";
import type { FormResult, WorkspaceState } from "./workspace";

export interface ExportSlice {
  docVersions: DocVersion[];
  /** Avance del export en curso; `null` si no hay ninguno. */
  exporting: ExportProgress | null;
  loadDocVersions(): Promise<void>;
  saveDocVersion(input: DocVersionInput, id?: string): Promise<FormResult>;
  deleteDocVersion(id: string): Promise<void>;
  rememberedTester(): string;
  exportExcel(request: useCases.ExportRequest): Promise<FormResult>;
}

type Set = StoreApi<WorkspaceState>["setState"];
type Get = StoreApi<WorkspaceState>["getState"];

const NO_PROJECT: FormResult = { ok: false, errors: { _form: "No hay un proyecto abierto." } };
const BUSY: FormResult = { ok: false, errors: { _form: "Ya hay una exportación en curso." } };

const missingMessage = (n: number) =>
  n === 1
    ? "1 imagen no se pudo incluir: en la hoja Evidencias aparece un aviso en su lugar."
    : `${n} imágenes no se pudieron incluir: en la hoja Evidencias aparece un aviso en su lugar.`;

export function createExportSlice(repos: Repositories, prefs: Preferences, set: Set, get: Get, h: SliceHelpers): ExportSlice {
  return {
    docVersions: [],
    exporting: null,

    async loadDocVersions() {
      const projectId = get().project?.id;
      if (!projectId) {
        set({ docVersions: [] });
        return;
      }
      try {
        const docVersions = await repos.docVersions.list(projectId);
        if (get().project?.id === projectId) set({ docVersions });
      } catch (error) {
        h.report(error);
      }
    },

    async saveDocVersion(input, id) {
      const current = get().project;
      if (!current) return NO_PROJECT;
      return h.form(
        () => useCases.saveDocVersion(repos, current.id, input, id),
        async (saved) => {
          await get().loadDocVersions();
          get().pushToast("success", id ? `Versión ${saved.version} actualizada.` : `Versión ${saved.version} registrada.`);
        },
      );
    },

    async deleteDocVersion(id) {
      const removed = get().docVersions.find((v) => v.id === id);
      try {
        await repos.docVersions.remove(id);
        await get().loadDocVersions();
        if (removed) get().pushToast("success", `Versión ${removed.version} eliminada.`);
      } catch (error) {
        h.report(error);
      }
    },

    rememberedTester: () => prefs.getTester(),

    async exportExcel(request) {
      const current = get().project;
      if (!current) return NO_PROJECT;
      if (get().exporting) return BUSY;
      set({ exporting: { stage: "preparing", current: 0, total: 0 } });
      try {
        const outcome = await useCases.exportWorkbook(repos, current.id, request, (exporting) => set({ exporting }));
        // Una versión nueva queda registrada aunque luego se cancele «Guardar como».
        if (request.newVersion) await get().loadDocVersions();
        if (!outcome.ok) return outcome;
        prefs.setTester(request.options.tester.trim());
        const result = outcome.value;
        if (!result) return { ok: false, errors: {} };
        const open = (run: (path: string) => Promise<void>) => () => void run(result.path).catch(h.report);
        get().pushToast("success", `Excel guardado: ${result.fileName}`, [
          { label: "Abrir archivo", run: open(repos.exporter.openFile) },
          { label: "Abrir carpeta", run: open(repos.exporter.revealFile) },
        ]);
        if (result.missingImages > 0) get().pushToast("error", missingMessage(result.missingImages));
        return { ok: true };
      } catch (error) {
        h.report(error);
        return { ok: false, errors: {} };
      } finally {
        set({ exporting: null });
      }
    },
  };
}
```

En `src/ui/store/workspace.ts`, reemplaza:

```ts
export interface Toast {
  id: number;
  kind: "success" | "error";
  message: string;
}
```

por:

```ts
export interface ToastAction {
  label: string;
  run(): void;
}
export interface Toast {
  id: number;
  kind: "success" | "error";
  message: string;
  actions?: ToastAction[];
}
```

En `src/ui/store/workspace.ts`, reemplaza:

```ts
  pushToast(kind: Toast["kind"], message: string): void;
```

por:

```ts
  pushToast(kind: Toast["kind"], message: string, actions?: ToastAction[]): void;
```

En `src/ui/store/workspace.ts`, reemplaza:

```ts
      pushToast: (kind, message) => set((s) => ({ toasts: [...s.toasts, { id: ++toastSeq, kind, message }] })),
```

por:

```ts
      pushToast: (kind, message, actions) => set((s) => ({ toasts: [...s.toasts, { id: ++toastSeq, kind, message, actions }] })),
```

Reemplaza `src/ui/components/Toasts.tsx` por:

```tsx
import { useEffect } from "react";
import { useWorkspace } from "../store/context";

/** Un toast con acciones («Abrir archivo») queda más tiempo para que se alcance a usar. */
const DURATION = { plain: 4000, actions: 10000 };

export function Toasts() {
  const toasts = useWorkspace((s) => s.toasts);
  const dismiss = useWorkspace((s) => s.dismissToast);

  useEffect(() => {
    if (toasts.length === 0) return;
    const first = toasts[0];
    const timer = setTimeout(() => dismiss(first.id), first.actions?.length ? DURATION.actions : DURATION.plain);
    return () => clearTimeout(timer);
  }, [toasts, dismiss]);

  return (
    <div role="status" aria-live="polite" className="fixed bottom-4 right-4 z-50 flex flex-col gap-2">
      {toasts.map((t) => (
        <div key={t.id} className={`rounded-lg px-4 py-3 text-sm text-white shadow-lg ${t.kind === "error" ? "bg-red-700" : "bg-t4s-charcoal"}`}>
          <p>{t.message}</p>
          {t.actions?.length ? (
            <div className="mt-2 flex gap-3">
              {t.actions.map((a) => (
                <button
                  key={a.label}
                  type="button"
                  className="font-medium text-t4s-lime underline-offset-2 hover:underline"
                  onClick={() => {
                    a.run();
                    dismiss(t.id);
                  }}
                >
                  {a.label}
                </button>
              ))}
            </div>
          ) : null}
        </div>
      ))}
    </div>
  );
}
```

Run: `pnpm vitest run src/ui/store`
Expected: PASS (5 tests de exportación, entre ellos «rechaza un segundo export mientras corre el primero» y «si se cancela «Guardar como» la versión nueva queda registrada una sola vez»).

- [ ] **Step 7: Verificación y commit**

```bash
pnpm typecheck && pnpm test && pnpm build
git add src
git commit -m "feat(f4): exportación en el front (puerto Exporter, caso de uso, avance y toast con acciones)

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

Expected: 171 tests en verde.

---

### Task 12: Diálogo «Exportar Excel» y botón en la barra superior

**Files:**
- Create: `src/ui/screens/ExportDialog.tsx`, `src/ui/screens/Export.test.tsx`
- Modify: `src/ui/screens/TopBar.tsx`, `src/ui/screens/AppShell.tsx`

**Interfaces:**
- Consumes: store (`docVersions`, `exporting`, `exportExcel`, `rememberedTester`), dominio (`latestVersion`, `suggestNextVersion`, `formatIsoDate`, `todayIso`, `IMAGE_WIDTH`, `progressLabel` y `progressPercent`), `seedCases` de `Evidence.test.tsx`.
- Produces:
  - Botón «Exportar» en `TopBar` (solo con proyecto abierto).
  - `ExportDialog`, que ofrece:
    - «Qué exportar»: Matriz con evidencias o Plantilla vacía;
    - Tester (precargado con el recordado);
    - Versión del documento: las existentes, de la más nueva a la más vieja, más «Nueva versión…» con número sugerido y cambios;
    - casillas «Incluir hoja Evidencias» e «Incluir hoja Resumen»;
    - barra de avance y error del formulario.
  - Mientras exporta, no se cierra con Esc ni con Cancelar.

- [ ] **Step 1: Tests que fallan**

Crea `src/ui/screens/Export.test.tsx`:

```tsx
import { screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { emptyDocVersion } from "../../domain/doc-versions";
import type { MemoryRepositories } from "../../infrastructure/memory/repositories";
import type { Repositories } from "../../ports/repositories";
import { renderApp } from "../test-utils";
import { seedCases } from "./Evidence.test";

const FILE = "T4S_ACME_Recibos_Evidencias_20261016_v1.0.xlsx";

async function openExport(options: { tester?: string; setup?: (repos: Repositories) => Promise<void> } = {}) {
  const ctx = await renderApp({ tester: options.tester ?? "Ana Pérez", setup: options.setup ?? (async (repos) => void (await seedCases(repos))) });
  await ctx.user.click(screen.getByRole("button", { name: /Exportar/ }));
  const repos = ctx.repos as MemoryRepositories;
  return { ...ctx, repos, dialog: screen.getByRole("dialog", { name: "Exportar Excel" }) };
}

describe("exportar a Excel", () => {
  it("registra la primera versión, exporta y ofrece abrir el archivo y la carpeta", async () => {
    const { user, repos, dialog, store } = await openExport();
    expect(within(dialog).getByLabelText(/Tester/)).toHaveValue("Ana Pérez");
    expect(within(dialog).getByLabelText(/Versión del documento/)).toHaveDisplayValue("Nueva versión…");
    expect(within(dialog).getByLabelText(/Número de versión/)).toHaveValue("1.0");
    expect(within(dialog).getByLabelText("Cambios")).toHaveValue("Versión inicial");
    await user.click(within(dialog).getByRole("button", { name: "Exportar Excel" }));

    await waitFor(() => expect(screen.queryByRole("dialog", { name: "Exportar Excel" })).not.toBeInTheDocument());
    const [call] = repos.exporter.calls;
    const [version] = store.getState().docVersions;
    expect(version).toMatchObject({ version: "1.0", author: "Ana Pérez", changes: "Versión inicial" });
    expect(call).toMatchObject({ kind: "project", options: { tester: "Ana Pérez", docVersionId: version.id, includeEvidence: true, includeSummary: true, imageMaxWidth: 940 } });
    expect(screen.getByText(`Excel guardado: ${FILE}`)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Abrir archivo" }));
    await waitFor(() => expect(repos.exporter.opened).toEqual([`C:\\Users\\ana\\Documents\\${FILE}`]));
    expect(screen.queryByText(`Excel guardado: ${FILE}`)).not.toBeInTheDocument();
  });

  it("usa la última versión registrada y deja quitar hojas", async () => {
    const { user, repos, dialog } = await openExport({
      setup: async (repos) => {
        const { p } = await seedCases(repos);
        await repos.docVersions.create(p.id, { ...emptyDocVersion("2026-10-01", "Ana"), version: "1.0" });
        await repos.docVersions.create(p.id, { ...emptyDocVersion("2026-10-15", "Ana"), version: "1.1" });
      },
    });
    expect(within(dialog).getByLabelText(/Versión del documento/)).toHaveDisplayValue("1.1 · 15/10/2026");
    await user.click(within(dialog).getByLabelText("Incluir hoja Resumen"));
    await user.click(within(dialog).getByRole("button", { name: "Exportar Excel" }));
    await waitFor(() => expect(repos.exporter.calls).toHaveLength(1));
    expect(repos.exporter.calls[0].options).toMatchObject({ includeEvidence: true, includeSummary: false });
  });

  it("con el archivo abierto en Excel muestra el mensaje y no cierra el diálogo", async () => {
    const { user, repos, dialog } = await openExport();
    repos.exporter.mode = "locked";
    await user.click(within(dialog).getByRole("button", { name: "Exportar Excel" }));
    expect(await within(dialog).findByRole("alert")).toHaveTextContent("Cierra el archivo en Excel y vuelve a intentar.");
    expect(screen.getByRole("dialog", { name: "Exportar Excel" })).toBeInTheDocument();
  });

  it("si se cancela «Guardar como» queda abierto y reutiliza la versión ya registrada", async () => {
    const { user, repos, dialog, store } = await openExport();
    repos.exporter.mode = "cancel";
    await user.click(within(dialog).getByRole("button", { name: "Exportar Excel" }));
    await waitFor(() => expect(within(dialog).getByLabelText(/Versión del documento/)).toHaveDisplayValue(/^1\.0 · /));
    repos.exporter.mode = "save";
    await user.click(within(dialog).getByRole("button", { name: "Exportar Excel" }));
    await waitFor(() => expect(screen.queryByRole("dialog", { name: "Exportar Excel" })).not.toBeInTheDocument());
    expect(store.getState().docVersions).toHaveLength(1);
    expect(repos.exporter.calls[1].options.docVersionId).toBe(store.getState().docVersions[0].id);
  });

  it("la plantilla vacía no pide versión ni hojas", async () => {
    const { user, repos, dialog } = await openExport();
    await user.click(within(dialog).getByRole("radio", { name: /Plantilla vacía/ }));
    expect(within(dialog).queryByLabelText(/Versión del documento/)).not.toBeInTheDocument();
    expect(within(dialog).queryByLabelText("Incluir hoja Evidencias")).not.toBeInTheDocument();
    await user.click(within(dialog).getByRole("button", { name: "Exportar plantilla" }));
    await waitFor(() => expect(repos.exporter.calls).toHaveLength(1));
    expect(repos.exporter.calls[0]).toMatchObject({ kind: "template", options: { docVersionId: null } });
  });

  it("exige el tester y lo recuerda para la próxima vez", async () => {
    const { user, repos, dialog } = await openExport({ tester: "" });
    await user.click(within(dialog).getByRole("button", { name: "Exportar Excel" }));
    expect(within(dialog).getByText("«Tester» es obligatorio.")).toBeInTheDocument();
    expect(repos.exporter.calls).toHaveLength(0);
    await user.type(within(dialog).getByLabelText(/Tester/), "  Luis Gómez ");
    await user.click(within(dialog).getByRole("button", { name: "Exportar Excel" }));
    await waitFor(() => expect(screen.queryByRole("dialog", { name: "Exportar Excel" })).not.toBeInTheDocument());
    await user.click(screen.getByRole("button", { name: /Exportar/ }));
    expect(within(screen.getByRole("dialog", { name: "Exportar Excel" })).getByLabelText(/Tester/)).toHaveValue("Luis Gómez");
  });

  it("avisa si alguna imagen no se pudo incluir", async () => {
    const { user, repos, dialog } = await openExport();
    repos.exporter.missingImages = 2;
    await user.click(within(dialog).getByRole("button", { name: "Exportar Excel" }));
    expect(await screen.findByText("2 imágenes no se pudieron incluir: en la hoja Evidencias aparece un aviso en su lugar.")).toBeInTheDocument();
  });
});
```

Run: `pnpm vitest run src/ui/screens/Export.test.tsx`
Expected: FAIL (no hay botón «Exportar»).

- [ ] **Step 2: Diálogo y botón**

Crea `src/ui/screens/ExportDialog.tsx`:

```tsx
import { useEffect, useState, type FormEvent } from "react";
import type { ExportRequest } from "../../application/use-cases";
import type { FormErrors } from "../../domain/cases";
import { emptyDocVersion, formatIsoDate, latestVersion, suggestNextVersion, todayIso } from "../../domain/doc-versions";
import { IMAGE_WIDTH, progressLabel, progressPercent, type ExportKind } from "../../domain/export";
import { Dialog, DialogFooter } from "../components/Dialog";
import { FormField } from "../components/FormField";
import { btn, inputCls } from "../components/styles";
import { useWorkspace } from "../store/context";

const NEW = "__nueva__";
const KINDS: { kind: ExportKind; label: string; hint: string }[] = [
  { kind: "project", label: "Matriz con evidencias", hint: "Carátula, matriz, capturas y resumen." },
  { kind: "template", label: "Plantilla vacía", hint: "Carátula y las columnas del proyecto, sin casos." },
];

/** UC-16 / UC-17: opciones del export; «Guardar como» lo abre Rust al confirmar. */
export function ExportDialog({ onClose }: { onClose: () => void }) {
  const project = useWorkspace((s) => s.project);
  const docVersions = useWorkspace((s) => s.docVersions);
  const exporting = useWorkspace((s) => s.exporting);
  const exportExcel = useWorkspace((s) => s.exportExcel);
  const rememberedTester = useWorkspace((s) => s.rememberedTester);
  const [kind, setKind] = useState<ExportKind>("project");
  const [tester, setTester] = useState(rememberedTester);
  const [versionId, setVersionId] = useState(() => latestVersion(docVersions)?.id ?? NEW);
  const [newVersion, setNewVersion] = useState(() => ({
    version: suggestNextVersion(docVersions),
    changes: docVersions.length ? "" : "Versión inicial",
  }));
  const [includeEvidence, setIncludeEvidence] = useState(true);
  const [includeSummary, setIncludeSummary] = useState(true);
  const [errors, setErrors] = useState<FormErrors>({});

  useEffect(() => {
    // Si la versión nueva ya quedó registrada (p. ej., se canceló «Guardar como»), se elige esa.
    if (versionId !== NEW) return;
    const typed = newVersion.version.trim().toLowerCase();
    const saved = docVersions.find((v) => v.version.toLowerCase() === typed);
    if (saved) setVersionId(saved.id);
  }, [docVersions, versionId, newVersion.version]);

  if (!project) return null;
  const busy = exporting !== null;

  async function submit(e: FormEvent) {
    e.preventDefault();
    const isNew = kind === "project" && versionId === NEW;
    const request: ExportRequest = {
      kind,
      options: {
        tester,
        docVersionId: kind === "project" && !isNew ? versionId : null,
        includeEvidence,
        includeSummary,
        imageMaxWidth: IMAGE_WIDTH.default,
      },
      newVersion: isNew ? { ...emptyDocVersion(todayIso(), tester.trim()), version: newVersion.version, changes: newVersion.changes } : undefined,
    };
    const result = await exportExcel(request);
    if (result.ok) onClose();
    else setErrors(result.errors);
  }

  return (
    <Dialog title="Exportar Excel" kicker={project.name} onClose={busy ? () => {} : onClose}>
      <form onSubmit={submit} noValidate className="grid gap-4">
        <fieldset>
          <legend className="mb-2 text-sm font-medium">Qué exportar</legend>
          <div role="radiogroup" className="grid grid-cols-2 gap-2">
            {KINDS.map((k) => (
              <label key={k.kind} className={`cursor-pointer rounded-lg border p-3 text-sm ${kind === k.kind ? "border-t4s-blue bg-t4s-blue/5" : "border-t4s-line"}`}>
                <input type="radio" name="export-kind" className="mr-2" checked={kind === k.kind} onChange={() => setKind(k.kind)} disabled={busy} />
                <span className="font-medium">{k.label}</span>
                <span className="mt-1 block text-xs text-t4s-gray">{k.hint}</span>
              </label>
            ))}
          </div>
        </fieldset>
        <FormField label="Tester" required htmlFor="export-tester" error={errors.tester}>
          <input id="export-tester" className={inputCls} value={tester} onChange={(e) => setTester(e.target.value)} disabled={busy} placeholder="Nombre que aparece en la carátula" />
        </FormField>
        {kind === "project" && (
          <>
            <FormField label="Versión del documento" required htmlFor="export-version" error={errors.docVersionId}>
              <select id="export-version" className={inputCls} value={versionId} onChange={(e) => setVersionId(e.target.value)} disabled={busy}>
                {[...docVersions].reverse().map((v) => (
                  <option key={v.id} value={v.id}>
                    {`${v.version} · ${formatIsoDate(v.date)}`}
                  </option>
                ))}
                <option value={NEW}>Nueva versión…</option>
              </select>
            </FormField>
            {versionId === NEW && (
              <div className="grid grid-cols-3 gap-4">
                <FormField label="Número de versión" required htmlFor="export-new-version" error={errors.version}>
                  <input
                    id="export-new-version"
                    className={inputCls}
                    value={newVersion.version}
                    onChange={(e) => setNewVersion((v) => ({ ...v, version: e.target.value }))}
                    disabled={busy}
                  />
                </FormField>
                <FormField label="Cambios" htmlFor="export-new-changes" error={errors.changes} className="col-span-2">
                  <input
                    id="export-new-changes"
                    className={inputCls}
                    value={newVersion.changes}
                    onChange={(e) => setNewVersion((v) => ({ ...v, changes: e.target.value }))}
                    disabled={busy}
                  />
                </FormField>
              </div>
            )}
            <fieldset className="flex gap-6 text-sm">
              <legend className="sr-only">Hojas opcionales</legend>
              <label className="flex items-center gap-2">
                <input type="checkbox" checked={includeEvidence} onChange={(e) => setIncludeEvidence(e.target.checked)} disabled={busy} />
                Incluir hoja Evidencias
              </label>
              <label className="flex items-center gap-2">
                <input type="checkbox" checked={includeSummary} onChange={(e) => setIncludeSummary(e.target.checked)} disabled={busy} />
                Incluir hoja Resumen
              </label>
            </fieldset>
          </>
        )}
        {busy && (
          <div>
            <div
              role="progressbar"
              aria-label="Avance de la exportación"
              aria-valuemin={0}
              aria-valuemax={100}
              aria-valuenow={progressPercent(exporting)}
              className="h-2 overflow-hidden rounded-full bg-t4s-cream"
            >
              <div className="h-full bg-t4s-blue transition-all" style={{ width: `${progressPercent(exporting)}%` }} />
            </div>
            <p className="mt-1 text-xs text-t4s-gray">{progressLabel(exporting)}</p>
          </div>
        )}
        {errors._form && (
          <p role="alert" className="text-sm text-red-700">
            {errors._form}
          </p>
        )}
        <DialogFooter>
          <button type="button" className={btn.secondary} onClick={onClose} disabled={busy}>
            Cancelar
          </button>
          <button type="submit" className={btn.primary} disabled={busy}>
            {kind === "project" ? "Exportar Excel" : "Exportar plantilla"}
          </button>
        </DialogFooter>
      </form>
    </Dialog>
  );
}
```

En `src/ui/screens/TopBar.tsx`, reemplaza:

```tsx
import { Plus, Settings } from "lucide-react";
```

por:

```tsx
import { FileSpreadsheet, Plus, Settings } from "lucide-react";
```

En `src/ui/screens/TopBar.tsx`, reemplaza:

```tsx
export function TopBar({ onNewProject, onSettings }: { onNewProject: () => void; onSettings?: () => void }) {
```

por:

```tsx
export function TopBar({ onNewProject, onSettings, onExport }: { onNewProject: () => void; onSettings?: () => void; onExport?: () => void }) {
```

En `src/ui/screens/TopBar.tsx`, después de:

```tsx
      <div className="ml-auto flex gap-2">
```

agrega:

```tsx
        {onExport && project && (
          <button type="button" className={btn.secondary} onClick={onExport}>
            <FileSpreadsheet size={16} /> Exportar
          </button>
        )}
```

En `src/ui/screens/AppShell.tsx`, después de:

```tsx
import { EmptyState } from "./EmptyState";
```

agrega:

```tsx
import { ExportDialog } from "./ExportDialog";
```

En `src/ui/screens/AppShell.tsx`, reemplaza:

```tsx
useState<"new-project" | "settings" | null>(null);
```

por:

```tsx
useState<"new-project" | "settings" | "export" | null>(null);
```

En `src/ui/screens/AppShell.tsx`, reemplaza:

```tsx
<TopBar onNewProject={() => setDialog("new-project")} onSettings={() => setDialog("settings")} />
```

por:

```tsx
<TopBar onNewProject={() => setDialog("new-project")} onSettings={() => setDialog("settings")} onExport={() => setDialog("export")} />
```

En `src/ui/screens/AppShell.tsx`, después de:

```tsx
      {dialog === "settings" && project && <ProjectSettingsDialog onClose={() => setDialog(null)} />}
```

agrega:

```tsx
      {dialog === "export" && project && <ExportDialog onClose={() => setDialog(null)} />}
```

Run: `pnpm vitest run src/ui/screens/Export.test.tsx`
Expected: PASS (7 tests).

- [ ] **Step 3: Verificación**

```bash
pnpm typecheck && pnpm test && pnpm build
```

Expected: 194 tests en verde.

- [ ] **Step 4: Smoke manual**

`pnpm tauri dev`, con un proyecto de prueba de 10 casos y 20 evidencias:

1. «Exportar» → el diálogo propone «Nueva versión…» 1.0 / «Versión inicial». Escribe el tester → «Exportar Excel». Se abre «Guardar como» con el nombre T4S, la barra avanza («Procesando imágenes …») y aparece el toast «Excel guardado: …». «Abrir archivo» abre Excel; «Abrir carpeta» abre el Explorador con el archivo seleccionado.
2. Vuelve a abrir el diálogo: el tester está recordado y la versión 1.0 aparece como «1.0 · dd/mm/aaaa».
3. Exporta otra vez al mismo archivo con Excel abierto: aparece «Cierra el archivo en Excel y vuelve a intentar.» dentro del diálogo.
4. «Plantilla vacía» → «Exportar plantilla»: el libro tiene Carátula y Matriz sin casos.

- [ ] **Step 5: Commit**

```bash
git add src
git commit -m "feat(f4): diálogo Exportar Excel con versión del documento, hojas y avance

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 13: Verificación integral, versión `0.4.0` y cierre técnico del contrato

**Files:**
- Modify: `package.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `src-tauri/tauri.conf.json` (versión `0.4.0`)
- Modify: `docs/superpowers/contracts/F4-excel-caratula.md`, `docs/superpowers/contracts/README.md`

- [ ] **Step 1: Versión**

Cambia `"version": "0.3.0"` → `"0.4.0"` en `package.json` y en `src-tauri/tauri.conf.json`, y `version = "0.3.0"` → `"0.4.0"` en `src-tauri/Cargo.toml`. Luego `cd src-tauri && cargo check` para actualizar `Cargo.lock`.

- [ ] **Step 2: Verificación completa**

```bash
cd src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test && cd ..
cd src-tauri && cargo test --release excel::tests::exports_300_evidences_in_under_20_seconds -- --ignored --nocapture && cd ..
pnpm typecheck && pnpm test && pnpm build
pnpm tauri build
```

Expected: todo en verde. El instalador queda en `src-tauri/target/release/bundle/nsis/Qapture_0.4.0_x64-setup.exe`. Instálalo y repite el smoke de la Task 12 con el build de release: la ACL de producción debe permitir los 8 comandos nuevos, y «Guardar como» debe abrirse sobre la ventana principal.

- [ ] **Step 3: Criterios de aceptación (automáticos)**

En `docs/superpowers/contracts/F4-excel-caratula.md`, marca los dos criterios con los tests que los cubren:
- xlsx releído con `calamine`: 4 hojas, encabezados iguales a las columnas visibles, N filas para N casos, vínculos válidos → `excel::tests::exports_four_sheets_that_calamine_reads_back` y `excel::tests::links_go_both_ways_between_matrix_and_evidence`.
- 300 evidencias en menos de 20 s → `excel::tests::exports_300_evidences_in_under_20_seconds` (en release, `--ignored`; anota el tiempo medido). La memoria se revisa en la prueba local 9.

En la tabla **Tasks** del contrato, agrega una columna `Estado` con `✅` en F4-T1…F4-T9. En `README.md`, cambia el estado de F4 a `🧪 En pruebas del equipo`.

- [ ] **Step 4: Commit**

```bash
git add -A package.json src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/tauri.conf.json docs
git commit -m "chore(f4): versión 0.4.0 y cierre técnico del contrato F4

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

- [ ] **Step 5: Entrega al equipo**

Pide a la persona usuaria que ejecute el **Checklist de pruebas locales** del contrato F4 (pruebas 1–9) con `Qapture_0.4.0_x64-setup.exe`:
- abrir el libro en **Excel** y en **WPS**;
- imprimir la Matriz;
- exportar con el archivo abierto;
- la plantilla vacía;
- un proyecto grande, con el Administrador de tareas abierto.

Si WPS no muestra el gráfico o el formato condicional, se anota como hallazgo y se corrige en esta misma rama. La fase se cierra cuando el checklist está firmado. Push y PR, solo cuando lo pida.

---

## Probar un build

```bash
git pull && pnpm install
pnpm tauri build
# src-tauri/target/release/bundle/nsis/Qapture_0.4.0_x64-setup.exe
```

Los datos de prueba están en `%APPDATA%\com.teams4soft.qapture\` (`qapture.db`, `evidence\`, `backups\`). Al primer arranque, la migración `0003` deja un respaldo de la base en `backups\qapture-v2-<fecha>.db`. El nombre del tester se guarda en el almacenamiento local del WebView, no en la base.

## Ejecución

Recomendado: **subagent-driven**. Las Tasks 3–9 (Rust puro, un módulo por task) son casi independientes y cada una trae sus tests con `calamine`, así que se pueden revisar por separado. La 10 une todo y la 11/12 dependen de nombres exactos del contrato (`export:progress`, campos camelCase, `null` al cancelar), por eso conviene un revisor por task. Las Tasks 3–9 también se pueden delegar a Codex (`/codex:rescue`) indicando la task y este plan; Claude revisa cada diff contra su sección.
