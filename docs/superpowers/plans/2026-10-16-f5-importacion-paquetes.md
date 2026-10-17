# F5 — Importación v1, paquetes `.qapture` y respaldos · Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Que el equipo QA migre sus datos de la Bitácora v1 (`matrices.json` o respaldo JSON, schema 1) sin pérdidas, comparta proyectos con un compañero mediante un paquete `.qapture` que siempre se importa como copia, y tenga los datos protegidos con un respaldo diario automático (se conservan 7), respaldos manuales, restauración con confirmación y un modo recuperación cuando la base no abre.

**Architecture:** Toda la lógica vive en Rust y es testeable sin Tauri. Las dos importaciones (v1 y paquete) traducen su archivo a un modelo intermedio común, `transfer::bundle::ProjectBundle`. Antes de abrir la transacción escriben las imágenes en `evidence/<proyecto-nuevo>/`. Después insertan todo en **una sola transacción** (`bundle::insert`). Si algo falla, la transacción se revierte y la guarda `ImportFiles` borra las carpetas creadas: todo o nada. Los respaldos usan la API de backup de SQLite (`rusqlite`, feature `backup`), se escriben en un temporal y se renombran. Restaurar copia el respaldo **sobre la conexión viva** (`Connection::restore`), migra si hace falta y aparta (no borra) las capturas que el respaldo no conoce. Al arrancar, `PRAGMA quick_check` detecta una base dañada. En ese caso la app arranca en **modo recuperación**: usa una base en memoria, no ejecuta limpiezas ni respaldos, y el front muestra una pantalla para restaurar un respaldo o empezar con una base vacía (e importar un paquete). En React, un menú **Datos** en la barra superior reúne importar v1, importar/exportar paquete y respaldos, y al terminar muestra un resumen.

**Tech Stack:** Tauri 2.12, Rust 2024, rusqlite 0.40 (`bundled`, `backup`), `zip` 9.0.4 (deflate, pasa de dev a dependencia normal), `base64` 0.22.1, image 0.25, `tauri-plugin-dialog` 2.8.3 · React 19, TypeScript 6, Zustand 5, zod 4, lucide-react · Vitest 5, Testing Library.

**Spec:** [`docs/superpowers/specs/2026-10-15-qapture-design.md`](../specs/2026-10-15-qapture-design.md) (§5 almacenamiento y `backups/`, §6.6 UC-19, §6.7 UC-18, §6.9 sección Datos, §8 «db corrupta» e «importación inválida», §9 privacidad del repo, §10 fixture v1 anonimizado) · **Contrato:** [`docs/superpowers/contracts/F5-importacion-paquetes.md`](../contracts/F5-importacion-paquetes.md) · **Referencia v1:** `app-local/store.mjs` (`validateState`) y `app-local/export.mjs` (`STATES`), fuera del repo.

## Global Constraints

- Rama `feat/f5-importacion-paquetes` creada desde `main` actualizado (F4 y CI/CD ya están en `main`). Un commit por task. Todos los mensajes terminan con una línea en blanco y `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>` (o la línea de la herramienta que haga el commit). **No hacer push ni merge sin pedirlo.**
- Plataforma Windows. Los comandos van en Git Bash desde la raíz `qapture/`; los de Rust, desde `qapture/src-tauri/`.
- Textos visibles **en español**. Mensajes fijos de esta fase (se usan tal cual en código y tests):
  - Importación (v1 y paquete), encabezado de la lista de problemas: `No se importó nada. Revisa estos problemas:`. Debajo van como máximo 10 líneas `• <dónde>: <qué>` y, si hay más, `• … y <N> problemas más.` (o `problema` si N = 1).
  - v1, archivo: `El archivo no es un respaldo válido de la Bitácora v1.` · `El respaldo no tiene proyectos para importar.` · `El archivo supera 200 MB.`
  - v1, reglas copiadas de `store.mjs`: `Texto inválido o demasiado largo.` · `Nombre o ID de proyecto inválido.` · `Demasiados casos.` · `Secuencia inválida.` · `Completa caso y entorno.` · `ID de ejecución repetido o inválido.` · `La secuencia debe superar los IDs usados.` · `Estado inválido.` · `Fecha inválida.` · `Demasiadas capturas por caso.` · `Captura repetida.` · `La captura necesita una descripción.` · `Dimensiones de imagen inválidas.` · `Formato de imagen inválido.` · `Contenido de imagen inválido.` Regla propia: `«Caso de prueba» supera 300 caracteres.`
  - Ubicación en v1: `Proyecto «<nombre>»`, `Proyecto «<nombre>», EJ-001` y `Proyecto «<nombre>», EJ-001, captura 2`. Si el dato que da el nombre no es válido, se usa `Proyecto 3`, `caso 4` o `captura 2` (base 1).
  - Paquete: `El archivo no es un paquete de Qapture válido.` · `El paquete se creó con una versión más nueva de Qapture. Actualiza la app para abrirlo.` · `El paquete supera el tamaño permitido.` · `El paquete está dañado: falta «<entrada>».` · `El paquete está dañado: no se pudo leer «<entrada>».` · `Ruta de imagen inválida o repetida.` · `Clave de columna inválida o repetida.` · `La columna «<clave>» no existe en el paquete.`
  - Nombre repetido al importar: `<nombre> (importado)`, luego `<nombre> (importado 2)`, `(importado 3)`… (sin distinguir mayúsculas, máximo 150 caracteres con el sufijo).
  - Respaldos: `El respaldo no existe.` · `El respaldo está dañado y no se puede restaurar.` · `El respaldo es de una versión más nueva de Qapture. Actualiza la app para restaurarlo.`
  - Recuperación: `Qapture está en modo recuperación: restaura un respaldo o empieza con una base vacía.` · `Qapture no está en modo recuperación.` · nuevo `AppError::Corrupt` (código `corrupt`, mensaje `La base de datos de Qapture está dañada.`).
  - UI: `Importado: 1 proyecto, 3 casos y 2 capturas.` (con singular y plural) · `Paquete guardado: <archivo>` con la acción `Abrir carpeta` · `<N> capturas no se incluyeron porque falta su imagen.` · `Respaldo creado: <archivo>` · `Se restauró el respaldo del <dd/mm/aaaa hh:mm>.` · `Base recuperada desde el respaldo.` · `Qapture empezó con una base vacía.`
- Archivos en el directorio de datos (`%APPDATA%\com.teams4soft.qapture\`):
  - `backups\qapture-diario-AAAAMMDD.db` (fecha local; se conservan los 7 más recientes).
  - `backups\qapture-manual-AAAAMMDD-HHMMSS.db` y `backups\qapture-antes-de-restaurar-AAAAMMDD-HHMMSS.db` (no rotan).
  - `backups\qapture-v<N>-AAAAMMDD-HHMMSS.db` (los de F1, antes de migrar; no rotan).
  - `qapture-danada-AAAAMMDD-HHMMSS.db` (la base dañada que se aparta en modo recuperación).
  - `evidence-sin-uso\AAAAMMDD-HHMMSS\<proyecto>\*.png` (capturas que el respaldo restaurado no conoce; nunca se borran solas).
- Paquete `.qapture` = ZIP con `project.json` (`formatVersion: 1`, JSON con sangría y `\n` final, comprimido) y `evidence/<código>-<n>.orig.png` / `.render.png` (sin comprimir, `n` desde 1 en el orden del caso). `project.json` **no lleva ids** ni la plantilla de origen: así exportar → importar → exportar da el mismo archivo byte a byte.
- Nombre sugerido del paquete: `T4S_<Cliente>_<Proyecto>_<AAAAMMDD>.qapture` (el cliente se omite si está vacío; mismas reglas de `excel::data::file_segment`).
- **ACL (F3):** cada comando nuevo se agrega en `src-tauri/src/commands/names.rs`, en `generate_handler!` (`lib.rs`) y en `capabilities/main.json` (`allow-*`). El test de `names.rs` verifica las tres cosas. La ventana sigue sin permisos JS de `dialog`: «Abrir» y «Guardar como» se llaman desde Rust.
- Dependencias nuevas o cambiadas, solo estas:
  - Rust: `base64 = "0.22.1"` (ya está en `Cargo.lock`), feature `backup` en `rusqlite`, y `zip = { version = "9.0.4", default-features = false, features = ["deflate"] }`, que sale de `[dev-dependencies]` y pasa a `[dependencies]`.
  - JS: ninguna.
- Fixtures y tests con datos ficticios ("ACME", "Recibos", "Pólizas", "Ana Pérez", recibos `1000001`). **Nunca** datos ni capturas de GMX: el fixture v1 se genera desde código (Task 2) y un test prohíbe los textos reales conocidos.
- El fixture `src/infrastructure/tauri/__fixtures__/contracts.json` **cambia en esta fase** (Task 8), y solo se regenera con `UPDATE_CONTRACTS=1 cargo test contract`.
- Cero warnings: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `pnpm typecheck`, `pnpm test` y `pnpm build` en verde al cerrar cada task.

## Review Focus

1. **Base dañada al arrancar:** la app **no** puede ejecutar la limpieza de evidencias huérfanas (`sweep_orphans`) ni el respaldo diario sobre la base vacía en memoria. Si lo hiciera, borraría todas las capturas o reemplazaría los respaldos buenos por copias vacías. El archivo dañado se conserva (`qapture-danada-*.db`) y las capturas se apartan, no se borran → tests en Task 7 (`db::recovery`) y Task 8 (arranque, `ensure_ready`).
2. **Un error a mitad de la importación** (imagen con cabecera PNG válida pero truncada, choque de restricción, disco lleno): no queda ningún proyecto, caso ni archivo en `evidence/`, aunque los proyectos anteriores del mismo archivo ya hayan escrito sus imágenes → tests en Task 1 (`ImportFiles`), Task 3 (v1) y Task 5 (paquete).
3. **Paquetes hostiles o de otra versión:** ZIP sin `project.json`, `formatVersion` 2, rutas como `../../qapture.db` dentro del JSON, entradas que dicen pesar poco y descomprimen gigas, imágenes que no son PNG. Nada se escribe fuera de `evidence/<proyecto-nuevo>/`, y el mensaje es claro → tests en Task 5.
4. **Restaurar un respaldo:** vuelve exactamente al estado del respaldo. Antes deja una copia `antes-de-restaurar`. Las capturas que el respaldo no conoce se mueven a `evidence-sin-uso\` en vez de perderse en la próxima limpieza. Un nombre inventado (`..\qapture.db`) o un archivo dañado no tocan nada → tests en Task 6.
5. **Datos v1 que caben en la v1 pero no en Qapture:** textos de más de 500 caracteres en columnas `text` (Menú, Módulo…), títulos largos, JPEG en lugar de PNG, nombres de proyecto repetidos. Se importan sin pérdida: la columna pasa a «Texto largo», el JPEG se convierte a PNG y el nombre recibe el sufijo `(importado)`. Lo que de verdad no cabe (título > 300) se informa → tests en Task 3.

## Decisiones respecto al contrato (se anotan en el contrato en la Task 1)

- **Mismo hash de imagen:** las capturas PNG de la v1 se guardan **byte a byte** como `.orig.png` y como `.render.png` (spec §6.6: «orig = render»), sin reducirlas a 2400 px. Las JPEG (la v1 las aceptaba) se convierten a PNG sin reducir. El criterio de «mismo hash» se verifica comparando los bytes con los del base64 decodificado (es más estricto que comparar un hash).
- **Validación v1 que reporta varios problemas:** `store.mjs` corta en el primer error. Qapture aplica las mismas reglas y mensajes, pero junta hasta 10 problemas con su ubicación (spec §8: «lista los primeros 10 problemas»).
- **Límites distintos v1 ↔ Qapture:** si un valor v1 de una columna `text` supera 500 caracteres, esa columna se crea como `longtext` en el proyecto importado. Las descripciones de captura de hasta 4000 caracteres (límite v1) se importan completas, también desde un paquete. Si después se editan, rige el límite de 2000 del editor. El título del caso se limita a 300 (regla del caso en Qapture) y, si lo supera, se informa.
- **El paquete no lleva ids ni la plantilla de origen:** los valores se guardan por clave de columna y las imágenes por código de caso. Al importar se crean ids nuevos y `source_template_id` queda vacío (la plantilla puede no existir en el otro equipo). Se conservan las fechas de creación y modificación del proyecto, los casos, las capturas y las versiones del documento, para que el viaje de ida y vuelta sea exacto.
- **Capturas sin imagen al exportar un paquete:** se omiten y se cuentan (`missingImages`), igual que en el Excel. Así una captura borrada a mano no impide compartir el proyecto.
- **Respaldos solo de la base:** spec §5 (`backups/` = copia de la db). Las capturas no se copian. Al restaurar, las capturas que el respaldo no referencia se **mueven** a `evidence-sin-uso\<fecha>\`, porque la limpieza del arranque las borraría. El resumen dice cuántas capturas del respaldo ya no tienen imagen.
- **Rotación:** solo los respaldos diarios rotan (7). Los manuales, los `antes-de-restaurar` y los de migración se conservan hasta que alguien los borre de la carpeta.
- **Respaldo diario con la app en bandeja:** se hace al arrancar y luego se revisa cada hora en un hilo (la app puede quedar abierta días en la bandeja desde F6).
- **Modo recuperación:** se activa si abrir la base falla por corrupción (`SQLITE_CORRUPT`, `SQLITE_NOTADB`) o si `PRAGMA quick_check` no devuelve `ok`. Otros errores (permisos, disco) siguen impidiendo el arranque, como hasta ahora. «Empezar vacía e importar un paquete» crea una base nueva y abre directamente el diálogo de importación.
- **Menú Datos en la barra superior:** F6 llevará estas mismas acciones a Settings › Datos (spec §6.9). Hasta entonces viven en el menú.

## File Structure

```
qapture/
├─ package.json                                          (T11 versión)
├─ src/
│  ├─ domain/data.ts  data.test.ts                       (T9) tipos, mensajes de resumen, fechas y tamaños
│  ├─ ports/data.ts  errors.ts  repositories.ts          (T9) puerto DataTransfer, código `corrupt`
│  ├─ infrastructure/
│  │  ├─ tauri/data.ts  dto.ts  repositories.ts  contract.test.ts       (T9)
│  │  └─ memory/data.ts  export.ts  repositories.ts                     (T9)
│  └─ ui/
│     ├─ store/data-slice.ts  data-slice.test.ts  evidence-slice.ts  workspace.ts   (T9)
│     ├─ screens/DataMenu.tsx  BackupsDialog.tsx  ImportIssuesDialog.tsx            (T10)
│     ├─ screens/RecoveryScreen.tsx  TopBar.tsx  AppShell.tsx                       (T10)
│     ├─ screens/Data.test.tsx  Recovery.test.tsx                                   (T10)
│     └─ test-utils.tsx                                  (T9) `setup` recibe MemoryRepositories
└─ src-tauri/
   ├─ Cargo.toml  Cargo.lock  tauri.conf.json            (T1, T11)
   ├─ capabilities/main.json                             (T8)
   ├─ fixtures/bitacora-v1.json                          (T2) fixture v1 anonimizado, generado por test
   └─ src/
      ├─ transfer/mod.rs                                 (T1)
      ├─ transfer/bundle.rs                              (T1) modelo común, ImportFiles, Problems, insert
      ├─ transfer/legacy.rs                              (T2 validación, T3 mapeo e importación)
      ├─ transfer/package.rs                             (T4 formato y export, T5 import)
      ├─ backup.rs                                       (T6) diario, rotación, manual, lista, restaurar
      ├─ db/recovery.rs  db/mod.rs                       (T6 prepare/latest_version, T7 quick_check y recuperación)
      ├─ media/raster.rs  media/storage.rs               (T3 to_png, T6 set_aside)
      ├─ repo/evidence.rs  repo/doc_versions.rs          (T5 validadores pub(crate), T6 references/missing_images)
      ├─ system/daily_backup.rs  system/mod.rs           (T8) respaldo diario cada hora
      ├─ commands/data.rs  commands/mod.rs  commands/names.rs   (T8)
      └─ dto.rs  error.rs  state.rs  lib.rs              (T1, T7, T8)
```

Correspondencia con el contrato: F5-T1 → Tasks 2 y 3 · F5-T2 → Task 2 · F5-T3 → Tasks 1, 4 y 5 · F5-T4 → Task 6 · F5-T5 → Task 7 · F5-T6 → Tasks 8, 9 y 10.

---

### Task 1: Rama, documentos, dependencias y núcleo común de importación (`transfer/bundle.rs`)

**Files:**
- Create: `src-tauri/src/transfer/mod.rs`, `src-tauri/src/transfer/bundle.rs`
- Modify: `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `src-tauri/src/lib.rs` (`mod transfer;`), `src-tauri/src/dto.rs`
- Modify: `docs/superpowers/contracts/README.md`, `docs/superpowers/contracts/F5-importacion-paquetes.md`; Add: `docs/superpowers/plans/2026-10-16-f5-importacion-paquetes.md` (este plan)

**Interfaces:**
- Produces:
  - `dto::ImportedProjectDto { id, name }` y `dto::ImportSummaryDto { projects: Vec<ImportedProjectDto>, cases: i64, evidence: i64 }` (Serialize, camelCase).
  - `transfer::bundle::{ProjectBundle, BundleField, BundleCase, BundleEvidence, BundleVersion}`: el modelo intermedio. Los valores de los casos van **por clave de columna**, y `ProjectBundle.id` lo asigna quien escribe las imágenes.
  - `transfer::bundle::ImportFiles<'a>`: `new(&EvidenceFiles)`, `write(&mut self, project_id, evidence_id, original: &[u8], rendered: &[u8]) -> AppResult<()>`. Si se suelta sin pasar por `insert`, borra las carpetas de proyecto que creó.
  - `transfer::bundle::Problems`: `add(&mut self, at: &str, message: &str)`, `total() -> usize`, `into_result(self) -> AppResult<()>`. Constante `IMPORT_FAILED`.
  - `transfer::bundle::{EMPTY_ANNOTATIONS, is_case_code(&str) -> bool, code_number(&str) -> Option<i64>, unique_name(&Connection, &str) -> AppResult<String>}`.
  - `transfer::bundle::insert(conn: &Connection, bundles: Vec<ProjectBundle>, files: ImportFiles) -> AppResult<ImportSummaryDto>`: una transacción; al confirmar, conserva los archivos.

- [ ] **Step 1: Rama y documentos**

```bash
git checkout main && git pull
git checkout -b feat/f5-importacion-paquetes
```

Este plan ya está en `docs/superpowers/plans/2026-10-16-f5-importacion-paquetes.md` (se escribió en `main` sin commitear, así que pasa a la rama).

En `docs/superpowers/contracts/README.md`, tabla **Estado**, cambia la fila F5 a:

```markdown
| F5 | [Importación v1 + paquetes + respaldo](F5-importacion-paquetes.md) | [plan](../plans/2026-10-16-f5-importacion-paquetes.md) | 🟡 En curso | `v0.5.0` |
```

En `docs/superpowers/contracts/F5-importacion-paquetes.md`, cambia la línea de cabecera por:

```markdown
**Depende de:** F2 (evidencias) · **Plan:** [2026-10-16-f5-importacion-paquetes.md](../plans/2026-10-16-f5-importacion-paquetes.md)
```

Agrega, antes de `## Criterios de aceptación`:

```markdown
## Notas de implementación (acordadas en el plan)
- Las capturas PNG de la v1 se guardan byte a byte (original = render, sin reducir). Las JPEG se convierten a PNG. «Mismo hash» se verifica comparando bytes.
- La validación v1 usa las reglas y mensajes de `store.mjs`, pero junta hasta 10 problemas con su ubicación (proyecto, caso, captura).
- Si un valor v1 supera 500 caracteres en una columna de texto corto, la columna se crea como «Texto largo». Las descripciones de hasta 4000 caracteres se importan completas.
- `project.json` no lleva ids ni la plantilla de origen. Las imágenes van como `evidence/<código>-<n>.orig.png` y `.render.png`. Exportar → importar → exportar da el mismo `project.json` byte a byte.
- Al exportar un paquete, las capturas sin imagen se omiten y se avisa cuántas fueron.
- Respaldos: `backups/qapture-diario-AAAAMMDD.db` (rotan 7), `qapture-manual-*` y `qapture-antes-de-restaurar-*` (no rotan). Restaurar deja antes una copia del estado actual y mueve a `evidence-sin-uso/<fecha>/` las capturas que el respaldo no conoce.
- Modo recuperación: base que no abre o que no pasa `PRAGMA quick_check`. La base dañada se conserva como `qapture-danada-<fecha>.db`. Mientras dura, no se limpian evidencias ni se hacen respaldos.
- Las acciones viven en el menú **Datos** de la barra superior hasta que F6 las lleve a Settings › Datos.

## Interfaces producidas
- Comandos `import_legacy`, `import_package`, `export_package(projectId)`, `list_backups`, `create_backup`, `restore_backup(fileName)`, `get_startup_status`, `recover_database(fileName | null)`. F6 los reutiliza en Settings › Datos.
- Formato `.qapture` `formatVersion: 1` (ver plan, Task 4). Un cambio incompatible sube `formatVersion`; las versiones viejas de la app rechazan los paquetes nuevos con un mensaje claro.
```

- [ ] **Step 2: Dependencias**

En `src-tauri/Cargo.toml`, en `[dependencies]`:

```toml
rusqlite = { version = "0.40", features = ["bundled", "backup"] }
base64 = "0.22.1"
# Paquetes .qapture (F5). zip 9: crates.io retiró (yank) toda la serie < 9.0.3.
zip = { version = "9.0.4", default-features = false, features = ["deflate"] }
```

(la línea de `rusqlite` reemplaza a la existente). En `[dev-dependencies]` borra la línea `zip = { version = "9.0.4", … }`: los tests de `excel` la siguen usando, ahora desde `[dependencies]`. El `[patch.crates-io]` de `zip8` (calamine) no cambia.

Run: `cd src-tauri && cargo check`
Expected: compila; `Cargo.lock` no agrega versiones nuevas de `base64` ni de `zip`.

- [ ] **Step 3: DTOs del resumen de importación**

En `src-tauri/src/dto.rs`, después de `ExportProgressDto`:

```rust
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedProjectDto {
    pub id: String,
    pub name: String,
}

/// Resumen de una importación (UC-18 / UC-19): los proyectos creados, en orden, y sus totales.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummaryDto {
    pub projects: Vec<ImportedProjectDto>,
    pub cases: i64,
    pub evidence: i64,
}
```

(El contrato con el front se actualiza en la Task 8, junto con los comandos).

- [ ] **Step 4: Escribir los tests del núcleo (fallan)**

Crea `src-tauri/src/transfer/mod.rs`:

```rust
//! Importación y exportación de datos (F5): Bitácora v1 y paquetes `.qapture`.
// Hasta la Task 8 el módulo solo lo usan sus tests; la Task 8 retira este allow.
#![allow(dead_code)]

pub mod bundle;
```

En `src-tauri/src/lib.rs`, agrega `mod transfer;` después de `mod system;`.

Crea `src-tauri/src/transfer/bundle.rs` solo con los tests:

```rust
#[cfg(test)]
pub(crate) mod tests {
    use std::{collections::BTreeMap, fs};

    use super::*;
    use crate::{
        dto::{DocVersionInput, ProjectMetaInput},
        media::{raster::sample_png, storage::TempDir},
        repo::{cases, doc_versions, evidence, projects, test_support},
    };

    const TS: &str = "2026-10-01T10:00:00.000Z";

    pub(crate) fn meta(name: &str) -> ProjectMetaInput {
        ProjectMetaInput {
            name: name.into(),
            client: "ACME".into(),
            test_type: "Funcional".into(),
            release_version: String::new(),
            start_date: String::new(),
            end_date: String::new(),
        }
    }

    fn field(key: &str, label: &str, field_type: FieldType, width: i64) -> BundleField {
        BundleField {
            key: key.into(),
            label: label.into(),
            field_type,
            options: Vec::new(),
            required: key == "entorno",
            clear_on_duplicate: false,
            width,
            archived: false,
        }
    }

    /// Un proyecto con 2 columnas, un caso con texto multilínea, una captura y una versión.
    pub(crate) fn sample_bundle(out: &mut ImportFiles, name: &str) -> ProjectBundle {
        let id = new_id();
        let evidence_id = new_id();
        let png = sample_png(40, 30);
        out.write(&id, &evidence_id, &png, &png).unwrap();
        ProjectBundle {
            id,
            meta: meta(name),
            next_sequence: 4,
            source_template_id: None,
            created_at: TS.into(),
            updated_at: "2026-10-02T10:00:00.000Z".into(),
            fields: vec![
                field("entorno", "Entorno", FieldType::Text, 21),
                field("datos_entrada", "Datos de entrada", FieldType::Longtext, 48),
            ],
            cases: vec![BundleCase {
                code: "EJ-003".into(),
                title: "Aplicar recibo".into(),
                status: "Aprobado".into(),
                created_at: TS.into(),
                updated_at: TS.into(),
                values: BTreeMap::from([
                    ("entorno".to_string(), "QA".to_string()),
                    ("datos_entrada".to_string(), "Recibo: 1000001\nSucursal: 2".to_string()),
                ]),
                evidence: vec![BundleEvidence {
                    id: evidence_id,
                    description: "Pantalla del recibo".into(),
                    annotations: EMPTY_ANNOTATIONS.into(),
                    width: 40,
                    height: 30,
                    captured_at: TS.into(),
                    updated_at: TS.into(),
                }],
            }],
            versions: vec![BundleVersion {
                input: DocVersionInput {
                    version: "1.0".into(),
                    date: "2026-10-01".into(),
                    author: "Ana Pérez".into(),
                    reviewer: String::new(),
                    approver: String::new(),
                    changes: "Versión inicial".into(),
                },
                created_at: TS.into(),
            }],
        }
    }

    pub(crate) fn project_dirs(files: &EvidenceFiles) -> usize {
        fs::read_dir(files.root()).map_or(0, |dir| dir.count())
    }

    #[test]
    fn inserts_a_bundle_with_values_evidence_and_versions() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let conn = test_support::conn();
        let mut out = ImportFiles::new(&files);
        let bundle = sample_bundle(&mut out, "Recibos");
        let summary = insert(&conn, vec![bundle], out).unwrap();

        assert_eq!(summary.cases, 1);
        assert_eq!(summary.evidence, 1);
        let p = projects::get(&conn, &summary.projects[0].id).unwrap();
        assert_eq!((p.name.as_str(), p.next_sequence), ("Recibos", 4));
        assert_eq!(p.created_at, TS);
        assert_eq!(p.source_template_id, None);
        assert_eq!(p.fields.len(), 2);
        assert_eq!(p.fields[1].field_type, FieldType::Longtext);
        let list = cases::list(&conn, &p.id).unwrap();
        assert_eq!(list[0].code, "EJ-003");
        let datos = test_support::field_id(&p, "datos_entrada");
        assert_eq!(list[0].values[&datos], "Recibo: 1000001\nSucursal: 2");
        let ev = evidence::list(&conn, &files, &list[0].id).unwrap();
        assert_eq!(ev[0].description, "Pantalla del recibo");
        assert_eq!(fs::read(&ev[0].original_path).unwrap(), sample_png(40, 30));
        assert_eq!(doc_versions::list(&conn, &p.id).unwrap()[0].version, "1.0");
    }

    #[test]
    fn repeated_names_get_the_importado_suffix() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let conn = test_support::conn();
        test_support::project(&conn); // «Recibos»
        let mut names = Vec::new();
        for wanted in ["Recibos", "recibos", "Pólizas"] {
            let mut out = ImportFiles::new(&files);
            let bundle = sample_bundle(&mut out, wanted);
            names.push(insert(&conn, vec![bundle], out).unwrap().projects[0].name.clone());
        }
        assert_eq!(names, ["Recibos (importado)", "recibos (importado 2)", "Pólizas"]);
    }

    #[test]
    fn long_names_keep_the_suffix_within_150_characters() {
        let conn = test_support::conn();
        let long = "Á".repeat(150);
        conn.execute(
            "INSERT INTO project (id, name, created_at, updated_at) VALUES ('p', ?1, '', '')",
            [&long],
        )
        .unwrap();
        let name = unique_name(&conn, &long).unwrap();
        assert_eq!(name.chars().count(), 150);
        assert!(name.ends_with("Á (importado)"));
    }

    #[test]
    fn a_failed_insert_leaves_no_rows_and_no_files() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let conn = test_support::conn();
        let mut out = ImportFiles::new(&files);
        let good = sample_bundle(&mut out, "Recibos");
        let mut bad = sample_bundle(&mut out, "Pólizas");
        bad.cases[0].values.insert("no_existe".into(), "x".into());
        assert_eq!(project_dirs(&files), 2);

        let err = insert(&conn, vec![good, bad], out).unwrap_err();
        assert_eq!(err.to_string(), "La columna «no_existe» no existe en el paquete.");
        assert!(projects::list(&conn).unwrap().is_empty());
        assert_eq!(project_dirs(&files), 0, "las imágenes escritas se borran");
    }

    #[test]
    fn dropping_import_files_without_insert_cleans_up() {
        let tmp = TempDir::new();
        let files = tmp.files();
        {
            let mut out = ImportFiles::new(&files);
            sample_bundle(&mut out, "Recibos");
            assert_eq!(project_dirs(&files), 1);
        }
        assert_eq!(project_dirs(&files), 0);
    }

    #[test]
    fn problems_list_at_most_ten_with_a_remainder() {
        let mut problems = Problems::default();
        assert!(Problems::default().into_result().is_ok());
        for n in 1..=12 {
            problems.add(&format!("EJ-{n:03}"), "Estado inválido.");
        }
        problems.add("", "Sin ubicación.");
        assert_eq!(problems.total(), 13);
        let text = problems.into_result().unwrap_err().to_string();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines[0], IMPORT_FAILED);
        assert_eq!(lines[1], "• EJ-001: Estado inválido.");
        assert_eq!(lines.len(), 12);
        assert_eq!(lines[11], "• … y 3 problemas más.");

        let mut one_more = Problems::default();
        for _ in 0..11 {
            one_more.add("", "x");
        }
        assert!(one_more.into_result().unwrap_err().to_string().ends_with("• … y 1 problema más."));
    }

    #[test]
    fn case_codes() {
        assert!(is_case_code("EJ-001"));
        assert!(is_case_code("EJ-1234"));
        for bad in ["EJ-01", "ej-001", "EJ-00a", "EJ-", "XX-001"] {
            assert!(!is_case_code(bad), "{bad}");
        }
        assert_eq!(code_number("EJ-012"), Some(12));
        assert_eq!(code_number("EJ-99999999999999999999999"), None);
    }
}
```

Run: `cd src-tauri && cargo test transfer::bundle`
Expected: FAIL al compilar (`ProjectBundle`, `ImportFiles`, `insert`… no existen).

- [ ] **Step 5: Implementar el núcleo**

Agrega al inicio de `src-tauri/src/transfer/bundle.rs` (encima del módulo de tests):

```rust
//! Modelo intermedio común a la importación de la v1 y de paquetes `.qapture`.
//! Las imágenes se escriben antes de la transacción (`ImportFiles`) y las filas se insertan
//! en una sola transacción (`insert`): o entra todo o no queda nada.

use std::collections::{BTreeMap, HashMap};

use rusqlite::{Connection, params};

use crate::{
    db::new_id,
    domain::fields::FieldType,
    dto::{DocVersionInput, FieldDto, ImportSummaryDto, ImportedProjectDto, ProjectMetaInput},
    error::{AppResult, validation},
    media::storage::{EvidenceFiles, write_atomic},
    repo::fields,
};

pub const IMPORT_FAILED: &str = "No se importó nada. Revisa estos problemas:";
pub const EMPTY_ANNOTATIONS: &str = r#"{"version":1,"crop":null,"shapes":[]}"#;
const MAX_LISTED: usize = 10;
const MAX_NAME: usize = 150;

#[derive(Debug, Clone, PartialEq)]
pub struct BundleField {
    pub key: String,
    pub label: String,
    pub field_type: FieldType,
    pub options: Vec<String>,
    pub required: bool,
    pub clear_on_duplicate: bool,
    pub width: i64,
    pub archived: bool,
}

/// Sus archivos ya están en `evidence/<proyecto>/<id>.orig.png` y `.render.png`.
#[derive(Debug, Clone, PartialEq)]
pub struct BundleEvidence {
    pub id: String,
    pub description: String,
    pub annotations: String,
    pub width: i64,
    pub height: i64,
    pub captured_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BundleCase {
    pub code: String,
    pub title: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    /// Clave de columna → valor. Los vacíos no se guardan.
    pub values: BTreeMap<String, String>,
    pub evidence: Vec<BundleEvidence>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BundleVersion {
    pub input: DocVersionInput,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectBundle {
    pub id: String,
    pub meta: ProjectMetaInput,
    pub next_sequence: i64,
    pub source_template_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub fields: Vec<BundleField>,
    pub cases: Vec<BundleCase>,
    pub versions: Vec<BundleVersion>,
}

/// Problemas de validación con su ubicación; se informan los primeros 10 (spec §8).
#[derive(Debug, Default)]
pub struct Problems {
    listed: Vec<String>,
    total: usize,
}

impl Problems {
    pub fn add(&mut self, at: &str, message: &str) {
        self.total += 1;
        if self.listed.len() < MAX_LISTED {
            self.listed.push(if at.is_empty() {
                message.to_string()
            } else {
                format!("{at}: {message}")
            });
        }
    }

    pub fn total(&self) -> usize {
        self.total
    }

    pub fn into_result(self) -> AppResult<()> {
        if self.total == 0 {
            return Ok(());
        }
        let mut text = String::from(IMPORT_FAILED);
        for item in &self.listed {
            text.push_str("\n• ");
            text.push_str(item);
        }
        let rest = self.total - self.listed.len();
        if rest > 0 {
            let noun = if rest == 1 { "problema" } else { "problemas" };
            text.push_str(&format!("\n• … y {rest} {noun} más."));
        }
        Err(validation(text))
    }
}

/// `EJ-` seguido de al menos 3 dígitos (regla de la v1).
pub fn is_case_code(code: &str) -> bool {
    code.strip_prefix("EJ-")
        .is_some_and(|digits| digits.len() >= 3 && digits.bytes().all(|b| b.is_ascii_digit()))
}

pub fn code_number(code: &str) -> Option<i64> {
    code.strip_prefix("EJ-")?.parse().ok()
}

/// Imágenes escritas antes de la transacción. Si la importación no llega a confirmarse,
/// al soltarse borra las carpetas de los proyectos nuevos (nunca toca proyectos existentes:
/// sus ids se acaban de generar).
pub struct ImportFiles<'a> {
    files: &'a EvidenceFiles,
    projects: Vec<String>,
    committed: bool,
}

impl<'a> ImportFiles<'a> {
    pub fn new(files: &'a EvidenceFiles) -> Self {
        Self {
            files,
            projects: Vec::new(),
            committed: false,
        }
    }

    pub fn write(
        &mut self,
        project_id: &str,
        evidence_id: &str,
        original: &[u8],
        rendered: &[u8],
    ) -> AppResult<()> {
        if !self.projects.iter().any(|p| p == project_id) {
            self.projects.push(project_id.to_string());
            std::fs::create_dir_all(self.files.project_dir(project_id))?;
        }
        let original_path = self
            .files
            .absolute(&EvidenceFiles::original_rel(project_id, evidence_id));
        write_atomic(&original_path, original)?;
        let render_path = self
            .files
            .absolute(&EvidenceFiles::render_rel(project_id, evidence_id));
        write_atomic(&render_path, rendered)
    }

    fn commit(mut self) {
        self.committed = true;
    }
}

impl Drop for ImportFiles<'_> {
    fn drop(&mut self) {
        if !self.committed {
            for project_id in &self.projects {
                self.files.remove_project(project_id);
            }
        }
    }
}

fn name_taken(conn: &Connection, name: &str) -> AppResult<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM project WHERE name = ?1 COLLATE NOCASE)",
        [name],
        |r| r.get(0),
    )?)
}

/// Importar nunca sobrescribe: si el nombre existe se agrega «(importado)», «(importado 2)»…
pub fn unique_name(conn: &Connection, wanted: &str) -> AppResult<String> {
    if !name_taken(conn, wanted)? {
        return Ok(wanted.to_string());
    }
    let mut n = 1;
    loop {
        let suffix = if n == 1 {
            " (importado)".to_string()
        } else {
            format!(" (importado {n})")
        };
        let room = MAX_NAME - suffix.chars().count();
        let base: String = wanted.chars().take(room).collect();
        let candidate = format!("{}{suffix}", base.trim_end());
        if !name_taken(conn, &candidate)? {
            return Ok(candidate);
        }
        n += 1;
    }
}

fn insert_project(conn: &Connection, b: &ProjectBundle, name: &str) -> AppResult<()> {
    conn.execute(
        "INSERT INTO project (id, name, client, test_type, release_version, start_date, end_date,
                              next_sequence, source_template_id, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            b.id,
            name,
            b.meta.client,
            b.meta.test_type,
            b.meta.release_version,
            b.meta.start_date,
            b.meta.end_date,
            b.next_sequence,
            b.source_template_id,
            b.created_at,
            b.updated_at
        ],
    )?;
    Ok(())
}

fn insert_fields(conn: &Connection, b: &ProjectBundle) -> AppResult<HashMap<String, String>> {
    let mut ids = HashMap::new();
    for (position, f) in b.fields.iter().enumerate() {
        let dto = FieldDto {
            id: String::new(),
            key: f.key.clone(),
            label: f.label.clone(),
            field_type: f.field_type,
            options: f.options.clone(),
            required: f.required,
            clear_on_duplicate: f.clear_on_duplicate,
            width: f.width,
            position: position as i64,
            archived: f.archived,
        };
        let id = fields::insert_project_field(conn, &b.id, &dto, position as i64)?;
        ids.insert(f.key.clone(), id);
    }
    Ok(ids)
}

fn insert_case(
    conn: &Connection,
    project_id: &str,
    field_ids: &HashMap<String, String>,
    c: &BundleCase,
) -> AppResult<()> {
    let case_id = new_id();
    conn.execute(
        "INSERT INTO test_case (id, project_id, code, title, status, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![case_id, project_id, c.code, c.title, c.status, c.created_at, c.updated_at],
    )?;
    for (key, value) in &c.values {
        if value.is_empty() {
            continue;
        }
        let field_id = field_ids
            .get(key)
            .ok_or_else(|| validation(format!("La columna «{key}» no existe en el paquete.")))?;
        conn.execute(
            "INSERT INTO field_value (case_id, field_id, value) VALUES (?1, ?2, ?3)",
            params![case_id, field_id, value],
        )?;
    }
    for (position, e) in c.evidence.iter().enumerate() {
        conn.execute(
            "INSERT INTO evidence (id, case_id, position, description, original_path, rendered_path,
                                   annotations, width, height, captured_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                e.id,
                case_id,
                position as i64,
                e.description,
                EvidenceFiles::original_rel(project_id, &e.id),
                EvidenceFiles::render_rel(project_id, &e.id),
                e.annotations,
                e.width,
                e.height,
                e.captured_at,
                e.updated_at
            ],
        )?;
    }
    Ok(())
}

fn insert_versions(conn: &Connection, b: &ProjectBundle) -> AppResult<()> {
    for v in &b.versions {
        conn.execute(
            "INSERT INTO doc_version (id, project_id, version, date, author, reviewer, approver, changes, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                new_id(),
                b.id,
                v.input.version,
                v.input.date,
                v.input.author,
                v.input.reviewer,
                v.input.approver,
                v.input.changes,
                v.created_at
            ],
        )?;
    }
    Ok(())
}

/// Inserta todos los proyectos en una sola transacción. Si algo falla, se revierte y
/// `files` (al soltarse) borra las imágenes ya escritas.
pub fn insert(
    conn: &Connection,
    bundles: Vec<ProjectBundle>,
    files: ImportFiles,
) -> AppResult<ImportSummaryDto> {
    let tx = conn.unchecked_transaction()?;
    let mut summary = ImportSummaryDto {
        projects: Vec::new(),
        cases: 0,
        evidence: 0,
    };
    for b in &bundles {
        let name = unique_name(&tx, &b.meta.name)?;
        insert_project(&tx, b, &name)?;
        let field_ids = insert_fields(&tx, b)?;
        for c in &b.cases {
            insert_case(&tx, &b.id, &field_ids, c)?;
            summary.cases += 1;
            summary.evidence += c.evidence.len() as i64;
        }
        insert_versions(&tx, b)?;
        summary.projects.push(ImportedProjectDto {
            id: b.id.clone(),
            name,
        });
    }
    tx.commit()?;
    files.commit();
    Ok(summary)
}
```

(`fields::insert_project_field` ya es `pub(crate)` y genera el id de la columna).

- [ ] **Step 6: Ejecutar los tests**

Run: `cd src-tauri && cargo test transfer::bundle && cargo clippy --all-targets -- -D warnings && cargo fmt --check`
Expected: PASS (7 tests), sin warnings.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/transfer src-tauri/src/lib.rs src-tauri/src/dto.rs docs
git commit -m "feat(f5): núcleo común de importación todo o nada con sufijo «(importado)»

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---
### Task 2: Fixture v1 anonimizado y validación del schema v1 (`transfer/legacy.rs`, parte 1)

**Files:**
- Create: `src-tauri/src/transfer/legacy.rs`, `src-tauri/fixtures/bitacora-v1.json` (generado por el test)
- Modify: `src-tauri/src/transfer/mod.rs`

**Interfaces:**
- Consumes: `bundle::{Problems, is_case_code, code_number}` (Task 1), `domain::fields::is_iso_date`, `domain::status::is_status`.
- Produces:
  - `transfer::legacy::parse(bytes: &[u8]) -> AppResult<Vec<LegacyProject>>`: valida con las reglas de `store.mjs` y junta hasta 10 problemas.
  - `LegacyProject { name, next_sequence: i64, cases: Vec<LegacyCase> }`, `LegacyCase { code, title, status, values: Vec<(&'static str, String)>, evidence: Vec<LegacyEvidence> }` con `value(&self, key) -> &str`, `LegacyEvidence { description, kind: ImageKind, bytes: Vec<u8> }` (base64 ya decodificado), `ImageKind::{Png, Jpeg}`.
  - `legacy::FIELD_MAP`: campo v1 → clave Funcional. Constantes `NOT_LEGACY`, `NO_PROJECTS`, `BAD_IMAGE`.
  - Solo en tests (`pub(crate)`): `legacy::tests::{fixture_value() -> Value, fixture_bytes() -> Vec<u8>, mutated(impl FnOnce(&mut Value)) -> Vec<u8>, jpeg(w, h) -> Vec<u8>}`.

- [ ] **Step 1: Escribir el generador del fixture y los tests (fallan)**

En `src-tauri/src/transfer/mod.rs`, agrega `pub mod legacy;` después de `pub mod bundle;`.

Crea `src-tauri/src/transfer/legacy.rs` solo con los tests:

```rust
#[cfg(test)]
pub(crate) mod tests {
    use std::{io::Cursor, path::PathBuf};

    use base64::engine::general_purpose::STANDARD;
    use serde_json::json;

    use super::*;
    use crate::{error::AppError, media::raster::sample_png};

    const FIXTURE: &str = "fixtures/bitacora-v1.json";

    fn data_url(mime: &str, bytes: &[u8]) -> String {
        format!("data:image/{mime};base64,{}", STANDARD.encode(bytes))
    }

    pub(crate) fn jpeg(width: u32, height: u32) -> Vec<u8> {
        let img = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
            width,
            height,
            image::Rgb([186, 200, 45]),
        ));
        let mut out = Vec::new();
        img.write_to(&mut Cursor::new(&mut out), image::ImageFormat::Jpeg)
            .unwrap();
        out
    }

    fn picture(id: &str, description: &str, url: String, width: u32, height: u32) -> Value {
        json!({ "id": id, "description": description, "dataUrl": url, "width": width, "height": height })
    }

    fn case(id: &str, code: &str, title: &str, status: &str, evidence: Vec<Value>) -> Value {
        json!({
            "id": id, "code": code, "title": title, "environment": "QA", "module": "Cobranza",
            "menu": "Aplicación", "submenu": "Aplicación de recibos", "functionality": "Primas por cobrar",
            "input": "", "expected": "El recibo queda aplicado.", "actual": "", "status": status,
            "date": "", "owner": "Ana Pérez", "evidence": evidence
        })
    }

    /// Bitácora v1 ficticia con la misma forma que `matrices.json` (spec §9: sin datos de clientes).
    pub(crate) fn fixture_value() -> Value {
        let mut first = case(
            "caso-1",
            "EJ-001",
            "Aplicación de recibos",
            "Por documentar",
            vec![
                picture("cap-1", "Pantalla de captura del recibo.", data_url("png", &sample_png(64, 48)), 64, 48),
                picture("cap-2", "Falta un SP.", data_url("jpeg", &jpeg(32, 24)), 32, 24),
            ],
        );
        first["environment"] = json!("Máquina Virtual");
        first["input"] = json!("Recibo: 1000001\nSucursal: 1\nImporte: $1,234.56 (Moneda Nacional)");
        let mut second = case(
            "caso-2",
            "EJ-002",
            "Aplicación de recibos",
            "Aprobado",
            vec![picture("cap-3", "Recibo aplicado.", data_url("png", &sample_png(120, 80)), 120, 80)],
        );
        second["date"] = json!("2026-10-14");
        let mut fourth = case("caso-4", "EJ-004", "Cancelar recibo", "Fallido", vec![]);
        fourth["actual"] = json!("Error 500 al cancelar.\nSe reintentó dos veces.");
        fourth["date"] = json!("2026-10-15");
        json!({
            "schema": 1,
            "revision": 3,
            "projects": [
                { "id": "recibos-demo", "name": "Aplicación de recibos", "nextSequence": 6, "cases": [first, second, fourth] },
                { "id": "polizas-demo", "name": "Pólizas", "nextSequence": 1, "cases": [] }
            ]
        })
    }

    fn fixture_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(FIXTURE)
    }

    pub(crate) fn fixture_bytes() -> Vec<u8> {
        std::fs::read(fixture_path())
            .expect("falta el fixture; ejecuta UPDATE_LEGACY_FIXTURE=1 cargo test legacy")
    }

    pub(crate) fn mutated(change: impl FnOnce(&mut Value)) -> Vec<u8> {
        let mut value = fixture_value();
        change(&mut value);
        serde_json::to_vec(&value).unwrap()
    }

    fn problems_of(bytes: &[u8]) -> String {
        match parse(bytes) {
            Err(AppError::Validation(message)) => message,
            other => panic!("se esperaba un error de validación: {other:?}"),
        }
    }

    /// Regenerar: `UPDATE_LEGACY_FIXTURE=1 cargo test legacy`
    #[test]
    fn fixture_is_generated_and_anonymous() {
        let actual = serde_json::to_string_pretty(&fixture_value()).unwrap() + "\n";
        if std::env::var("UPDATE_LEGACY_FIXTURE").is_ok() {
            std::fs::create_dir_all(fixture_path().parent().unwrap()).unwrap();
            std::fs::write(fixture_path(), &actual).unwrap();
        }
        let expected = String::from_utf8(fixture_bytes()).unwrap().replace("\r\n", "\n");
        assert_eq!(actual, expected, "El fixture cambió: regenéralo con UPDATE_LEGACY_FIXTURE=1");
        // Textos de la Bitácora real de GMX que nunca deben llegar al repo público.
        for banned in ["GMX", "4644564", "7043816", "25,834.23", "captura-original"] {
            assert!(!expected.contains(banned), "el fixture contiene «{banned}»");
        }
    }

    #[test]
    fn parses_the_fixture() {
        let projects = parse(&fixture_bytes()).unwrap();
        assert_eq!(projects.len(), 2);
        let recibos = &projects[0];
        assert_eq!((recibos.name.as_str(), recibos.next_sequence), ("Aplicación de recibos", 6));
        let codes: Vec<&str> = recibos.cases.iter().map(|c| c.code.as_str()).collect();
        assert_eq!(codes, ["EJ-001", "EJ-002", "EJ-004"]);
        let first = &recibos.cases[0];
        assert_eq!(first.value("entorno"), "Máquina Virtual");
        assert_eq!(
            first.value("datos_entrada"),
            "Recibo: 1000001\nSucursal: 1\nImporte: $1,234.56 (Moneda Nacional)"
        );
        assert_eq!(first.value("responsable"), "Ana Pérez");
        assert_eq!(first.evidence[0].kind, ImageKind::Png);
        assert_eq!(first.evidence[0].bytes, sample_png(64, 48));
        assert_eq!(first.evidence[1].kind, ImageKind::Jpeg);
        assert_eq!(first.evidence[1].description, "Falta un SP.");
        assert_eq!(recibos.cases[1].value("fecha_ejecucion"), "2026-10-14");
        assert!(projects[1].cases.is_empty());
    }

    #[test]
    fn every_v1_rule_reports_where_it_failed() {
        const R: &str = "Proyecto «Aplicación de recibos»";
        let rules: [(fn(&mut Value), String); 15] = [
            (|v| v["projects"][0]["cases"][0]["environment"] = json!("  "), format!("{R}, EJ-001: Completa caso y entorno.")),
            (|v| v["projects"][0]["cases"][0]["title"] = json!("x".repeat(301)), format!("{R}, EJ-001: «Caso de prueba» supera 300 caracteres.")),
            (|v| v["projects"][0]["cases"][1]["code"] = json!("EJ-001"), format!("{R}, EJ-001: ID de ejecución repetido o inválido.")),
            (|v| v["projects"][0]["cases"][1]["code"] = json!("EJ-2"), format!("{R}, EJ-2: ID de ejecución repetido o inválido.")),
            (|v| v["projects"][0]["nextSequence"] = json!(4), format!("{R}, EJ-004: La secuencia debe superar los IDs usados.")),
            (|v| v["projects"][0]["cases"][2]["status"] = json!("Cerrado"), format!("{R}, EJ-004: Estado inválido.")),
            (|v| v["projects"][0]["cases"][1]["date"] = json!("2026-02-30"), format!("{R}, EJ-002: Fecha inválida.")),
            (|v| v["projects"][0]["cases"][0]["menu"] = json!(7), format!("{R}, EJ-001: Texto inválido o demasiado largo.")),
            (|v| v["projects"][0]["cases"][0]["evidence"][1]["description"] = json!(" "), format!("{R}, EJ-001, captura 2: La captura necesita una descripción.")),
            (|v| v["projects"][0]["cases"][0]["evidence"][1]["id"] = json!("cap-1"), format!("{R}, EJ-001, captura 2: Captura repetida.")),
            (|v| v["projects"][0]["cases"][0]["evidence"][0]["width"] = json!(0), format!("{R}, EJ-001, captura 1: Dimensiones de imagen inválidas.")),
            (|v| v["projects"][0]["cases"][0]["evidence"][0]["dataUrl"] = json!("data:image/gif;base64,R0lGOD"), format!("{R}, EJ-001, captura 1: Formato de imagen inválido.")),
            (|v| v["projects"][0]["cases"][0]["evidence"][0]["dataUrl"] = json!("data:image/png;base64,/9j/4AAQ"), format!("{R}, EJ-001, captura 1: Contenido de imagen inválido.")),
            (|v| v["projects"][1]["id"] = json!("recibos-demo"), "Proyecto «Pólizas»: Nombre o ID de proyecto inválido.".to_string()),
            (|v| v["projects"][1]["nextSequence"] = json!(0), "Proyecto «Pólizas»: Secuencia inválida.".to_string()),
        ];
        for (change, expected) in rules {
            let message = problems_of(&mutated(change));
            assert!(message.starts_with(super::super::bundle::IMPORT_FAILED), "{message}");
            assert!(message.contains(&format!("\n• {expected}")), "falta «{expected}» en:\n{message}");
        }
    }

    #[test]
    fn a_project_without_a_valid_name_is_located_by_position() {
        let message = problems_of(&mutated(|v| v["projects"][1]["name"] = json!("   ")));
        assert!(message.contains("• Proyecto 2: Nombre o ID de proyecto inválido."), "{message}");
        let message = problems_of(&mutated(|v| v["projects"][0]["cases"][0]["code"] = json!(null)));
        assert!(message.contains("• Proyecto «Aplicación de recibos», caso 1: Texto inválido o demasiado largo."), "{message}");
    }

    #[test]
    fn lists_at_most_ten_problems() {
        let bad: Vec<Value> = (1..=12)
            .map(|n| {
                let mut c = case(&format!("c{n}"), &format!("EJ-{n:03}"), "", "", vec![]);
                c["environment"] = json!("");
                c
            })
            .collect();
        // Cada caso: sin título/entorno, código ≥ secuencia (1) y estado vacío → 36 problemas.
        let message = problems_of(&mutated(|v| v["projects"][1]["cases"] = json!(bad)));
        let lines: Vec<&str> = message.lines().collect();
        assert_eq!(lines.len(), 12);
        assert_eq!(lines[1], "• Proyecto «Pólizas», EJ-001: Completa caso y entorno.");
        assert_eq!(lines[11], "• … y 26 problemas más.");
    }

    #[test]
    fn rejects_files_that_are_not_v1() {
        let many: Vec<Value> = (0..101)
            .map(|n| json!({ "id": format!("p{n}"), "name": format!("P{n}"), "nextSequence": 1, "cases": [] }))
            .collect();
        for bytes in [
            b"hola".to_vec(),
            b"[]".to_vec(),
            br#"{"schema":2,"projects":[]}"#.to_vec(),
            br#"{"schema":1,"projects":{}}"#.to_vec(),
            serde_json::to_vec(&json!({ "schema": 1, "projects": many })).unwrap(),
        ] {
            assert_eq!(parse(&bytes).unwrap_err().to_string(), NOT_LEGACY);
        }
        assert_eq!(
            parse(br#"{"schema":1,"projects":[]}"#).unwrap_err().to_string(),
            NO_PROJECTS
        );
    }

    #[test]
    fn accepts_what_node_accepts() {
        let bytes = mutated(|v| {
            v["schema"] = json!(1.0);
            v["projects"][0]["nextSequence"] = json!(6.0);
            let url = v["projects"][0]["cases"][1]["evidence"][0]["dataUrl"]
                .as_str()
                .unwrap()
                .trim_end_matches('=')
                .to_string();
            v["projects"][0]["cases"][1]["evidence"][0]["dataUrl"] = json!(url);
        });
        let projects = parse(&bytes).unwrap();
        assert_eq!(projects[0].next_sequence, 6);
        assert_eq!(projects[0].cases[1].evidence[0].bytes, sample_png(120, 80));
    }
}
```

Run: `cd src-tauri && cargo test transfer::legacy`
Expected: FAIL al compilar (`parse`, `ImageKind`… no existen).

- [ ] **Step 2: Implementar la validación v1**

Agrega al inicio de `src-tauri/src/transfer/legacy.rs` (encima de los tests):

```rust
//! UC-19: importa `matrices.json` o un respaldo JSON de la Bitácora v1 (schema 1).
//! Valida con las reglas y mensajes de `app-local/store.mjs` (`validateState`), pero junta
//! hasta 10 problemas con su ubicación en lugar de cortar en el primero (spec §8).

use std::collections::{HashMap, HashSet};

use base64::{
    Engine, alphabet,
    engine::{DecodePaddingMode, GeneralPurpose, GeneralPurposeConfig},
};
use serde_json::Value;

use super::bundle::{Problems, code_number, is_case_code};
use crate::{
    domain::{fields::is_iso_date, status::is_status},
    error::{AppResult, validation},
};

pub const NOT_LEGACY: &str = "El archivo no es un respaldo válido de la Bitácora v1.";
pub const NO_PROJECTS: &str = "El respaldo no tiene proyectos para importar.";
pub const BAD_IMAGE: &str = "Contenido de imagen inválido.";

const BAD_TEXT: &str = "Texto inválido o demasiado largo.";
const BAD_PROJECT: &str = "Nombre o ID de proyecto inválido.";
const TOO_MANY_CASES: &str = "Demasiados casos.";
const BAD_SEQUENCE: &str = "Secuencia inválida.";
const MISSING_TITLE: &str = "Completa caso y entorno.";
const LONG_TITLE: &str = "«Caso de prueba» supera 300 caracteres.";
const BAD_CODE: &str = "ID de ejecución repetido o inválido.";
const LOW_SEQUENCE: &str = "La secuencia debe superar los IDs usados.";
const BAD_STATUS: &str = "Estado inválido.";
const BAD_DATE: &str = "Fecha inválida.";
const TOO_MANY_PICTURES: &str = "Demasiadas capturas por caso.";
const DUP_PICTURE: &str = "Captura repetida.";
const NO_DESCRIPTION: &str = "La captura necesita una descripción.";
const BAD_SIZE: &str = "Dimensiones de imagen inválidas.";
const BAD_IMAGE_FORMAT: &str = "Formato de imagen inválido.";

/// Campos de texto de cada caso v1 (`fields` en `store.mjs`).
const CASE_TEXTS: [&str; 14] = [
    "id", "code", "title", "environment", "module", "menu", "submenu", "functionality",
    "input", "expected", "actual", "status", "date", "owner",
];

/// Campo v1 → clave de la plantilla Funcional. `code`, `title` y `status` van al caso; `id` se descarta.
pub const FIELD_MAP: [(&str, &str); 10] = [
    ("environment", "entorno"),
    ("module", "modulo"),
    ("menu", "menu"),
    ("submenu", "submenu"),
    ("functionality", "funcionalidad"),
    ("input", "datos_entrada"),
    ("expected", "resultado_esperado"),
    ("actual", "resultado_obtenido"),
    ("date", "fecha_ejecucion"),
    ("owner", "responsable"),
];

const MAX_PROJECTS: usize = 100;
const MAX_CASES: usize = 10_000;
const MAX_TEXT: usize = 16_000;
const MAX_TITLE: usize = 300;
const MAX_PICTURES: usize = 200;
const MAX_DESCRIPTION: usize = 4_000;
const MAX_DATA_URL: usize = 16 * 1024 * 1024;
const MAX_SIDE: i64 = 12_000;
const MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991;
const PNG_MAGIC: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];

/// Como `Buffer.from(…, 'base64')` de Node: el relleno `=` es opcional.
const LENIENT: GeneralPurpose = GeneralPurpose::new(
    &alphabet::STANDARD,
    GeneralPurposeConfig::new()
        .with_decode_padding_mode(DecodePaddingMode::Indifferent)
        .with_decode_allow_trailing_bits(true),
);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageKind {
    Png,
    Jpeg,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LegacyEvidence {
    pub description: String,
    pub kind: ImageKind,
    /// El base64 del `dataUrl` ya decodificado.
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LegacyCase {
    pub code: String,
    pub title: String,
    pub status: String,
    /// Clave Funcional → valor tal como venía (con sus saltos de línea).
    pub values: Vec<(&'static str, String)>,
    pub evidence: Vec<LegacyEvidence>,
}

impl LegacyCase {
    pub fn value(&self, key: &str) -> &str {
        self.values
            .iter()
            .find(|(k, _)| *k == key)
            .map_or("", |(_, v)| v.as_str())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LegacyProject {
    pub name: String,
    pub next_sequence: i64,
    pub cases: Vec<LegacyCase>,
}

/// `typeof v === 'string' && v.length <= max`, con la longitud en UTF-16 como JavaScript.
fn text(value: Option<&Value>, max: usize) -> Option<&str> {
    value?
        .as_str()
        .filter(|s| s.encode_utf16().count() <= max)
}

/// `Number.isSafeInteger`: `6` y `6.0` valen; `6.5`, `"6"` o enteros enormes no.
fn integer(value: Option<&Value>) -> Option<i64> {
    let Value::Number(n) = value? else {
        return None;
    };
    let i = n.as_i64().or_else(|| {
        n.as_f64()
            .filter(|f| f.fract() == 0.0 && f.abs() <= MAX_SAFE_INTEGER as f64)
            .map(|f| f as i64)
    })?;
    (i.abs() <= MAX_SAFE_INTEGER).then_some(i)
}

/// `/^data:image\/(png|jpeg);base64,[A-Za-z0-9+/]+={0,2}$/`
fn split_data_url(url: &str) -> Option<(ImageKind, &str)> {
    let (kind, payload) = match url.strip_prefix("data:image/png;base64,") {
        Some(rest) => (ImageKind::Png, rest),
        None => (ImageKind::Jpeg, url.strip_prefix("data:image/jpeg;base64,")?),
    };
    let body = payload.trim_end_matches('=');
    let valid = payload.len() - body.len() <= 2
        && !body.is_empty()
        && body
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'/');
    valid.then_some((kind, payload))
}

fn content_matches(kind: ImageKind, bytes: &[u8]) -> bool {
    match kind {
        ImageKind::Png => bytes.starts_with(&PNG_MAGIC),
        ImageKind::Jpeg => bytes.starts_with(&[0xFF, 0xD8]),
    }
}

struct ProjectRules<'a> {
    at: &'a str,
    next_sequence: i64,
    ids: HashSet<String>,
    codes: HashSet<String>,
}

pub fn parse(bytes: &[u8]) -> AppResult<Vec<LegacyProject>> {
    let root: Value = serde_json::from_slice(bytes).map_err(|_| validation(NOT_LEGACY))?;
    let schema_ok = integer(root.get("schema")) == Some(1);
    let projects = match root.get("projects").and_then(Value::as_array) {
        Some(list) if schema_ok && list.len() <= MAX_PROJECTS => list,
        _ => return Err(validation(NOT_LEGACY)),
    };
    if projects.is_empty() {
        return Err(validation(NO_PROJECTS));
    }
    let mut problems = Problems::default();
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for (index, p) in projects.iter().enumerate() {
        if let Some(project) = parse_project(p, index, &mut seen, &mut problems) {
            out.push(project);
        }
    }
    problems.into_result()?;
    Ok(out)
}

fn parse_project(
    p: &Value,
    index: usize,
    seen: &mut HashSet<String>,
    problems: &mut Problems,
) -> Option<LegacyProject> {
    let name = text(p.get("name"), 150);
    let at = match name.map(str::trim) {
        Some(n) if !n.is_empty() => format!("Proyecto «{n}»"),
        _ => format!("Proyecto {}", index + 1),
    };
    let (Some(id), Some(name)) = (text(p.get("id"), 100), name) else {
        problems.add(&at, BAD_TEXT);
        return None;
    };
    if name.trim().is_empty() || !seen.insert(id.to_string()) {
        problems.add(&at, BAD_PROJECT);
        return None;
    }
    let Some(cases) = p
        .get("cases")
        .and_then(Value::as_array)
        .filter(|c| c.len() <= MAX_CASES)
    else {
        problems.add(&at, TOO_MANY_CASES);
        return None;
    };
    let Some(next_sequence) = integer(p.get("nextSequence")).filter(|n| *n > 0) else {
        problems.add(&at, BAD_SEQUENCE);
        return None;
    };
    let mut rules = ProjectRules {
        at: &at,
        next_sequence,
        ids: HashSet::new(),
        codes: HashSet::new(),
    };
    let before = problems.total();
    let cases: Vec<LegacyCase> = cases
        .iter()
        .enumerate()
        .filter_map(|(i, c)| parse_case(c, i, &mut rules, problems))
        .collect();
    (problems.total() == before).then(|| LegacyProject {
        name: name.trim().to_string(),
        next_sequence,
        cases,
    })
}

fn parse_case(
    c: &Value,
    index: usize,
    rules: &mut ProjectRules,
    problems: &mut Problems,
) -> Option<LegacyCase> {
    let label = text(c.get("code"), 20)
        .filter(|s| !s.trim().is_empty())
        .map_or_else(|| format!("caso {}", index + 1), str::to_string);
    let at = format!("{}, {label}", rules.at);
    let mut texts: HashMap<&str, &str> = HashMap::new();
    for key in CASE_TEXTS {
        let Some(value) = text(c.get(key), MAX_TEXT) else {
            problems.add(&at, BAD_TEXT);
            return None;
        };
        texts.insert(key, value);
    }
    let before = problems.total();
    let title = texts["title"].trim();
    if title.is_empty() || texts["environment"].trim().is_empty() {
        problems.add(&at, MISSING_TITLE);
    } else if title.chars().count() > MAX_TITLE {
        problems.add(&at, LONG_TITLE);
    }
    let code = texts["code"];
    if !is_case_code(code)
        || !rules.ids.insert(texts["id"].to_string())
        || !rules.codes.insert(code.to_string())
    {
        problems.add(&at, BAD_CODE);
    } else if code_number(code).is_none_or(|n| n >= rules.next_sequence) {
        problems.add(&at, LOW_SEQUENCE);
    }
    if !is_status(texts["status"]) {
        problems.add(&at, BAD_STATUS);
    }
    let date = texts["date"];
    if !date.is_empty() && !is_iso_date(date) {
        problems.add(&at, BAD_DATE);
    }
    let evidence = match c
        .get("evidence")
        .and_then(Value::as_array)
        .filter(|e| e.len() <= MAX_PICTURES)
    {
        Some(list) => parse_evidence(list, &at, problems),
        None => {
            problems.add(&at, TOO_MANY_PICTURES);
            Vec::new()
        }
    };
    (problems.total() == before).then(|| LegacyCase {
        code: code.to_string(),
        title: title.to_string(),
        status: texts["status"].to_string(),
        values: FIELD_MAP
            .iter()
            .map(|&(v1, key)| (key, texts[v1].to_string()))
            .collect(),
        evidence,
    })
}

fn parse_evidence(list: &[Value], case_at: &str, problems: &mut Problems) -> Vec<LegacyEvidence> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for (index, e) in list.iter().enumerate() {
        let at = format!("{case_at}, captura {}", index + 1);
        let Some(id) = text(e.get("id"), 100) else {
            problems.add(&at, BAD_TEXT);
            continue;
        };
        if !seen.insert(id.to_string()) {
            problems.add(&at, DUP_PICTURE);
            continue;
        }
        let Some(description) = text(e.get("description"), MAX_DESCRIPTION) else {
            problems.add(&at, BAD_TEXT);
            continue;
        };
        if description.trim().is_empty() {
            problems.add(&at, NO_DESCRIPTION);
            continue;
        }
        let side = |key: &str| integer(e.get(key)).filter(|n| (1..=MAX_SIDE).contains(n));
        if side("width").is_none() || side("height").is_none() {
            problems.add(&at, BAD_SIZE);
            continue;
        }
        let Some(url) = text(e.get("dataUrl"), MAX_DATA_URL) else {
            problems.add(&at, BAD_TEXT);
            continue;
        };
        let Some((kind, payload)) = split_data_url(url) else {
            problems.add(&at, BAD_IMAGE_FORMAT);
            continue;
        };
        let bytes = LENIENT.decode(payload).unwrap_or_default();
        if !content_matches(kind, &bytes) {
            problems.add(&at, BAD_IMAGE);
            continue;
        }
        out.push(LegacyEvidence {
            description: description.trim().to_string(),
            kind,
            bytes,
        });
    }
    out
}
```

- [ ] **Step 3: Generar el fixture y ejecutar los tests**

```bash
cd src-tauri
UPDATE_LEGACY_FIXTURE=1 cargo test transfer::legacy::tests::fixture_is_generated_and_anonymous
cargo test transfer::legacy
```

Expected: el primer comando crea `src-tauri/fixtures/bitacora-v1.json` (unos 5 KB). El segundo: PASS (7 tests). Abre el fixture y revisa a ojo que solo hay datos ficticios: «Aplicación de recibos», «Pólizas», «Ana Pérez», recibo `1000001`.

Run: `git check-ignore -v src-tauri/fixtures/bitacora-v1.json`
Expected: sin salida (el `.gitignore` no lo bloquea; solo bloquea `data/`, `evidence/`, `*.db` y `*.xlsx`).

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/transfer src-tauri/fixtures/bitacora-v1.json
git commit -m "feat(f5): fixture v1 anonimizado y validación del schema 1 con las reglas de store.mjs

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: Importación v1 completa — plantilla Funcional, base64 → PNG y todo o nada (`transfer/legacy.rs`, parte 2)

**Files:**
- Modify: `src-tauri/src/transfer/legacy.rs`, `src-tauri/src/media/raster.rs`

**Interfaces:**
- Consumes: `legacy::parse` (Task 2); `bundle::{ImportFiles, insert, BundleField, BundleCase, BundleEvidence, ProjectBundle, EMPTY_ANNOTATIONS, IMPORT_FAILED}` (Task 1); `repo::templates::get`; `domain::fields::normalize_value`.
- Produces:
  - `media::raster::to_png(bytes: &[u8]) -> AppResult<NormalizedImage>`: decodifica la imagen **completa** (así detecta archivos truncados) y la devuelve como PNG del mismo tamaño, sin reducirla.
  - `transfer::legacy::{FUNCIONAL, MAX_FILE_BYTES, template_fields(&Connection) -> AppResult<Vec<BundleField>>, import(conn: &Connection, files: &EvidenceFiles, bytes: &[u8]) -> AppResult<ImportSummaryDto>}`.

- [ ] **Step 1: Test de `to_png` (falla)**

En `src-tauri/src/media/raster.rs`, dentro de `mod tests`:

```rust
    #[test]
    fn to_png_keeps_the_size_and_detects_truncated_files() {
        let out = to_png(&encode(rgb(3000, 20), ImageFormat::Jpeg)).unwrap();
        assert_eq!((out.width, out.height), (3000, 20), "no se reduce a 2400");
        assert_eq!(image::guess_format(&out.png).unwrap(), ImageFormat::Png);
        let png = sample_png(64, 48);
        assert!(to_png(&png[..png.len() / 2]).is_err());
        assert!(to_png(b"hola").is_err());
    }
```

Run: `cd src-tauri && cargo test raster::tests::to_png`
Expected: FAIL (`to_png` no existe).

- [ ] **Step 2: Implementar `to_png`**

En `src-tauri/src/media/raster.rs`, después de `normalize`:

```rust
/// Importación de la v1: decodifica la imagen completa (un PNG truncado falla aquí) y la
/// devuelve como PNG del mismo tamaño. A diferencia de `normalize`, no reduce a 2400 px.
pub fn to_png(bytes: &[u8]) -> AppResult<NormalizedImage> {
    let (width, height) = dimensions(bytes)?;
    check_side(width, height)?;
    let img = reader(bytes)?
        .decode()
        .map_err(|_| validation("La imagen está dañada o incompleta."))?;
    Ok(NormalizedImage {
        png: encode_png(&img)?,
        width,
        height,
    })
}
```

Run: `cd src-tauri && cargo test raster`
Expected: PASS.

- [ ] **Step 3: Tests de la importación (fallan)**

En `src-tauri/src/transfer/legacy.rs`, dentro de `mod tests`, agrega estos `use` y tests:

```rust
    use std::{collections::BTreeMap, fs};

    use crate::{
        domain::fields::FieldType,
        dto::CaseInput,
        media::storage::TempDir,
        repo::{cases, evidence, projects, test_support},
        transfer::bundle::tests::project_dirs,
    };

    fn decoded(value: &Value) -> Vec<u8> {
        let url = value["dataUrl"].as_str().unwrap();
        STANDARD.decode(url.split_once(',').unwrap().1).unwrap()
    }

    #[test]
    fn imports_the_fixture_keeping_codes_sequence_text_and_images() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let conn = test_support::conn();
        let summary = import(&conn, &files, &fixture_bytes()).unwrap();

        let names: Vec<&str> = summary.projects.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["Aplicación de recibos", "Pólizas"]);
        assert_eq!((summary.cases, summary.evidence), (3, 3));

        let recibos = projects::get(&conn, &summary.projects[0].id).unwrap();
        assert_eq!(recibos.next_sequence, 6);
        assert_eq!(recibos.test_type, "Funcional");
        assert_eq!(recibos.source_template_id.as_deref(), Some(FUNCIONAL));
        assert_eq!(recibos.fields.len(), 11, "plantilla Funcional completa");
        let list = cases::list(&conn, &recibos.id).unwrap();
        let codes: Vec<&str> = list.iter().map(|c| c.code.as_str()).collect();
        assert_eq!(codes, ["EJ-001", "EJ-002", "EJ-004"]);
        let statuses: Vec<&str> = list.iter().map(|c| c.status.as_str()).collect();
        assert_eq!(statuses, ["Por documentar", "Aprobado", "Fallido"]);
        let value = |case: usize, key: &str| {
            list[case]
                .values
                .get(&test_support::field_id(&recibos, key))
                .cloned()
                .unwrap_or_default()
        };
        assert_eq!(
            value(0, "datos_entrada"),
            "Recibo: 1000001\nSucursal: 1\nImporte: $1,234.56 (Moneda Nacional)"
        );
        assert_eq!(value(2, "resultado_obtenido"), "Error 500 al cancelar.\nSe reintentó dos veces.");
        assert_eq!(value(1, "fecha_ejecucion"), "2026-10-14");
        assert_eq!(value(0, "entorno"), "Máquina Virtual");
        assert_eq!(value(0, "resultado_obtenido"), "", "los vacíos no se guardan");

        let fixture = fixture_value();
        let pictures = &fixture["projects"][0]["cases"][0]["evidence"];
        let ev = evidence::list(&conn, &files, &list[0].id).unwrap();
        assert_eq!(ev[0].description, "Pantalla de captura del recibo.");
        let original = fs::read(&ev[0].original_path).unwrap();
        assert_eq!(original, decoded(&pictures[0]), "PNG byte a byte (mismo hash)");
        assert_eq!(fs::read(&ev[0].rendered_path).unwrap(), original, "render = original");
        assert_eq!((ev[0].width, ev[0].height), (64, 48));
        assert_eq!(ev[0].annotations, crate::transfer::bundle::EMPTY_ANNOTATIONS);
        let from_jpeg = fs::read(&ev[1].original_path).unwrap();
        assert_eq!(image::guess_format(&from_jpeg).unwrap(), image::ImageFormat::Png);
        assert_eq!((ev[1].width, ev[1].height), (32, 24));
        let second = evidence::list(&conn, &files, &list[1].id).unwrap();
        let second_picture = &fixture["projects"][0]["cases"][1]["evidence"][0];
        assert_eq!(fs::read(&second[0].original_path).unwrap(), decoded(second_picture));

        let next = cases::create(
            &conn,
            &recibos.id,
            &CaseInput {
                title: "Nuevo caso".into(),
                status: "No ejecutado".into(),
                values: BTreeMap::from([(test_support::field_id(&recibos, "entorno"), "QA".into())]),
            },
        )
        .unwrap();
        assert_eq!(next.code, "EJ-006", "la secuencia continúa donde la dejó la v1");
    }

    #[test]
    fn a_rejected_file_writes_nothing() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let conn = test_support::conn();
        let bytes = mutated(|v| v["projects"][0]["cases"][2]["status"] = json!("Cerrado"));
        assert!(import(&conn, &files, &bytes).is_err());
        assert!(projects::list(&conn).unwrap().is_empty());
        assert_eq!(project_dirs(&files), 0);
    }

    #[test]
    fn a_truncated_png_in_the_second_project_rolls_back_everything() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let conn = test_support::conn();
        // Cabecera PNG válida (pasa la regla de store.mjs) pero archivo incompleto.
        let truncated = &sample_png(64, 48)[..40];
        let bytes = mutated(|v| {
            let mut broken = case("p-1", "EJ-001", "Emitir póliza", "Aprobado", vec![]);
            broken["evidence"] = json!([picture("x", "Póliza emitida.", data_url("png", truncated), 64, 48)]);
            v["projects"][1]["nextSequence"] = json!(2);
            v["projects"][1]["cases"] = json!([broken]);
        });
        let err = import(&conn, &files, &bytes).unwrap_err().to_string();
        assert_eq!(
            err,
            format!("{}\n• Proyecto «Pólizas», EJ-001, captura 1: Contenido de imagen inválido.", crate::transfer::bundle::IMPORT_FAILED)
        );
        assert!(projects::list(&conn).unwrap().is_empty());
        assert_eq!(project_dirs(&files), 0, "las capturas del primer proyecto también se borran");
    }

    #[test]
    fn long_values_turn_the_column_into_long_text() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let conn = test_support::conn();
        let long = "m".repeat(600);
        let bytes = mutated(|v| v["projects"][0]["cases"][0]["menu"] = json!(long));
        let summary = import(&conn, &files, &bytes).unwrap();
        let p = projects::get(&conn, &summary.projects[0].id).unwrap();
        let menu = p.fields.iter().find(|f| f.key == "menu").unwrap();
        assert_eq!(menu.field_type, FieldType::Longtext);
        let modulo = p.fields.iter().find(|f| f.key == "modulo").unwrap();
        assert_eq!(modulo.field_type, FieldType::Text);
        let list = cases::list(&conn, &p.id).unwrap();
        assert_eq!(list[0].values[&menu.id].chars().count(), 600);
        let polizas = projects::get(&conn, &summary.projects[1].id).unwrap();
        let other_menu = polizas.fields.iter().find(|f| f.key == "menu").unwrap();
        assert_eq!(other_menu.field_type, FieldType::Text, "solo cambia en el proyecto afectado");
    }

    #[test]
    fn importing_twice_adds_the_importado_suffix() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let conn = test_support::conn();
        import(&conn, &files, &fixture_bytes()).unwrap();
        let again = import(&conn, &files, &fixture_bytes()).unwrap();
        let names: Vec<&str> = again.projects.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["Aplicación de recibos (importado)", "Pólizas (importado)"]);
        assert_eq!(projects::list(&conn).unwrap().len(), 4);
    }
```

Run: `cd src-tauri && cargo test transfer::legacy`
Expected: FAIL al compilar (`import`, `FUNCIONAL` no existen).

- [ ] **Step 4: Implementar el mapeo y la importación**

En `src-tauri/src/transfer/legacy.rs`, completa los `use` del inicio:

```rust
use rusqlite::Connection;

use super::bundle::{
    self, BundleCase, BundleEvidence, BundleField, EMPTY_ANNOTATIONS, IMPORT_FAILED, ImportFiles,
    Problems, ProjectBundle, code_number, is_case_code,
};
use crate::{
    db::{new_id, now},
    domain::{
        fields::{FieldType, is_iso_date, normalize_value},
        status::is_status,
    },
    dto::{ImportSummaryDto, ProjectMetaInput},
    error::{AppResult, validation},
    media::{raster, storage::EvidenceFiles},
    repo::templates,
};
```

(reemplazan a los `use super::bundle::…` y `use crate::…` de la Task 2) y agrega, después de `parse_evidence`:

```rust
pub const FUNCIONAL: &str = "tpl-funcional";
pub const MAX_FILE_BYTES: u64 = 200 * 1024 * 1024;

/// Columnas de la plantilla Funcional tal como están en esta base (spec §5).
pub fn template_fields(conn: &Connection) -> AppResult<Vec<BundleField>> {
    Ok(templates::get(conn, FUNCIONAL)?
        .fields
        .into_iter()
        .map(|f| BundleField {
            key: f.key,
            label: f.label,
            field_type: f.field_type,
            options: f.options,
            required: f.required,
            clear_on_duplicate: f.clear_on_duplicate,
            width: f.width,
            archived: false,
        })
        .collect())
}

/// La plantilla, con «Texto largo» en las columnas de texto corto cuyo valor v1 no cabe.
fn columns_for(template: &[BundleField], cases: &[LegacyCase]) -> Vec<BundleField> {
    template
        .iter()
        .cloned()
        .map(|mut f| {
            let too_long = cases
                .iter()
                .any(|c| c.value(&f.key).chars().count() > FieldType::Text.max_len());
            if f.field_type == FieldType::Text && too_long {
                f.field_type = FieldType::Longtext;
            }
            f
        })
        .collect()
}

/// PNG: los mismos bytes (mismo hash). JPEG: convertido a PNG sin reducir.
fn as_png(at: &str, kind: ImageKind, bytes: Vec<u8>) -> AppResult<(Vec<u8>, u32, u32)> {
    let bad = || validation(format!("{IMPORT_FAILED}\n• {at}: {BAD_IMAGE}"));
    let decoded = raster::to_png(&bytes).map_err(|_| bad())?;
    let (width, height) = (decoded.width, decoded.height);
    let png = match kind {
        ImageKind::Png => bytes,
        ImageKind::Jpeg => decoded.png,
    };
    Ok((png, width, height))
}

fn to_bundles(
    projects: Vec<LegacyProject>,
    template: &[BundleField],
    out: &mut ImportFiles,
) -> AppResult<Vec<ProjectBundle>> {
    let ts = now();
    let mut bundles = Vec::with_capacity(projects.len());
    for p in projects {
        let id = new_id();
        let at = format!("Proyecto «{}»", p.name);
        let fields = columns_for(template, &p.cases);
        let mut cases = Vec::with_capacity(p.cases.len());
        for c in p.cases {
            let mut evidence = Vec::with_capacity(c.evidence.len());
            for (index, e) in c.evidence.into_iter().enumerate() {
                let place = format!("{at}, {}, captura {}", c.code, index + 1);
                let (png, width, height) = as_png(&place, e.kind, e.bytes)?;
                let evidence_id = new_id();
                out.write(&id, &evidence_id, &png, &png)?;
                evidence.push(BundleEvidence {
                    id: evidence_id,
                    description: e.description,
                    annotations: EMPTY_ANNOTATIONS.into(),
                    width: width.into(),
                    height: height.into(),
                    captured_at: ts.clone(),
                    updated_at: ts.clone(),
                });
            }
            let values = c
                .values
                .iter()
                .map(|(key, raw)| {
                    let field_type = fields
                        .iter()
                        .find(|f| f.key == *key)
                        .map_or(FieldType::Text, |f| f.field_type);
                    (key.to_string(), normalize_value(field_type, raw))
                })
                .filter(|(_, value)| !value.is_empty())
                .collect();
            cases.push(BundleCase {
                code: c.code,
                title: c.title,
                status: c.status,
                created_at: ts.clone(),
                updated_at: ts.clone(),
                values,
                evidence,
            });
        }
        bundles.push(ProjectBundle {
            id,
            meta: ProjectMetaInput {
                name: p.name,
                client: String::new(),
                test_type: "Funcional".into(),
                release_version: String::new(),
                start_date: String::new(),
                end_date: String::new(),
            },
            next_sequence: p.next_sequence,
            source_template_id: Some(FUNCIONAL.into()),
            created_at: ts.clone(),
            updated_at: ts.clone(),
            fields,
            cases,
            versions: Vec::new(),
        });
    }
    Ok(bundles)
}

/// UC-19 completo: valida, escribe las capturas y crea los proyectos en una transacción.
pub fn import(conn: &Connection, files: &EvidenceFiles, bytes: &[u8]) -> AppResult<ImportSummaryDto> {
    let projects = parse(bytes)?;
    let template = template_fields(conn)?;
    let mut out = ImportFiles::new(files);
    let bundles = to_bundles(projects, &template, &mut out)?;
    bundle::insert(conn, bundles, out)
}
```

- [ ] **Step 5: Ejecutar los tests**

Run: `cd src-tauri && cargo test transfer && cargo clippy --all-targets -- -D warnings && cargo fmt --check`
Expected: PASS (todos los de `transfer::bundle` y `transfer::legacy`).

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/transfer/legacy.rs src-tauri/src/media/raster.rs
git commit -m "feat(f5): importar Bitácora v1 a la plantilla Funcional conservando códigos, secuencia e imágenes

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---
### Task 4: Paquete `.qapture` — formato `formatVersion: 1` y exportación (`transfer/package.rs`, parte 1)

**Files:**
- Create: `src-tauri/src/transfer/package.rs`
- Modify: `src-tauri/src/transfer/mod.rs`

**Interfaces:**
- Consumes: `repo::{projects::get, cases::list, doc_versions::list}`, `excel::data::file_segment`, `excel::save::ensure_writable`, `media::storage::EvidenceFiles`.
- Produces:
  - Formato (serde, camelCase, `deny_unknown_fields`): `PackageDoc { format_version, project: PackageProject, fields: Vec<PackageField>, cases: Vec<PackageCase>, doc_versions: Vec<PackageDocVersion> }`.
    - `PackageProject { name, client, test_type, release_version, start_date, end_date, next_sequence, created_at, updated_at }`.
    - `PackageField { key, label, type, options, required, clear_on_duplicate, width, archived }`, en el orden del proyecto (activas y luego archivadas).
    - `PackageCase { code, title, status, created_at, updated_at, values: BTreeMap<clave, valor>, evidence: Vec<PackageEvidence> }`.
    - `PackageEvidence { description, annotations, width, height, captured_at, updated_at, original, rendered }`, con `original`/`rendered` como rutas dentro del ZIP.
    - `PackageDocVersion { version, date, author, reviewer, approver, changes, created_at }`.
  - `package::{FORMAT_VERSION = 1, PROJECT_JSON = "project.json", EXTENSION = "qapture"}`.
  - `package::collect(conn, files, project_id) -> AppResult<PackageExport>`, con `PackageExport { doc, images: Vec<(String /*ruta en el zip*/, PathBuf /*archivo*/)>, missing_images: i64 }` y `evidence_count(&self) -> i64`.
  - `package::write(export: &PackageExport, dest: &Path) -> AppResult<()>`: atómico (temporal + renombrar), con aviso de archivo abierto.
  - `package::project_json(&PackageDoc) -> AppResult<Vec<u8>>`, `suggested_file_name(client, name, today) -> String`, `with_package_extension(PathBuf) -> PathBuf`.
  - Solo en tests (`pub(crate)`): `package::tests::seeded(conn, files) -> ProjectDto`.

- [ ] **Step 1: Tests de la exportación (fallan)**

En `src-tauri/src/transfer/mod.rs`, agrega `pub mod package;`.

Crea `src-tauri/src/transfer/package.rs` solo con los tests:

```rust
#[cfg(test)]
pub(crate) mod tests {
    use std::{collections::BTreeMap, io::Read};

    use zip::ZipArchive;

    use super::*;
    use crate::{
        dto::{CaseInput, FieldInput, NewEvidenceMeta, ProjectDto},
        media::{raster::sample_png, storage::TempDir},
        repo::{evidence, fields, test_support},
        transfer::bundle::EMPTY_ANNOTATIONS,
    };

    /// «Recibos» con una columna de lista, una archivada con datos, 2 casos (uno duplicado),
    /// 2 capturas en EJ-001 y una versión del documento.
    pub(crate) fn seeded(conn: &Connection, files: &EvidenceFiles) -> ProjectDto {
        let p = test_support::project(conn);
        let mut inputs: Vec<FieldInput> = p
            .fields
            .iter()
            .map(|f| FieldInput {
                id: Some(f.id.clone()),
                label: f.label.clone(),
                field_type: f.field_type,
                options: f.options.clone(),
                required: f.required,
                clear_on_duplicate: f.clear_on_duplicate,
                width: f.width,
                archived: false,
            })
            .collect();
        inputs.push(FieldInput {
            id: None,
            label: "Severidad".into(),
            field_type: FieldType::List,
            options: vec!["Alta".into(), "Baja".into()],
            required: false,
            clear_on_duplicate: false,
            width: 14,
            archived: false,
        });
        fields::save(conn, &p.id, inputs).unwrap();
        let p = projects::get(conn, &p.id).unwrap();
        let id = |key: &str| test_support::field_id(&p, key);
        let first = cases::create(
            conn,
            &p.id,
            &CaseInput {
                title: "Aplicar recibo".into(),
                status: "Aprobado".into(),
                values: BTreeMap::from([
                    (id("entorno"), "QA".into()),
                    (id("datos_entrada"), "Recibo: 1000001\nSucursal: 2".into()),
                    (id("severidad"), "Alta".into()),
                    (id("observaciones"), "Revisar con el líder.".into()),
                ]),
            },
        )
        .unwrap();
        cases::duplicate(conn, &first.id).unwrap();
        for description in ["Pantalla inicial", "Recibo aplicado"] {
            let staged = evidence::stage(files, &sample_png(320, 200)).unwrap();
            let meta = NewEvidenceMeta {
                case_id: first.id.clone(),
                staged_path: staged.path,
                description: description.into(),
                annotations: EMPTY_ANNOTATIONS.into(),
            };
            evidence::add(conn, files, &meta, &sample_png(320, 200)).unwrap();
        }
        conn.execute(
            "UPDATE project_field SET archived = 1 WHERE project_id = ?1 AND key = 'observaciones'",
            [&p.id],
        )
        .unwrap();
        doc_versions::create(conn, &p.id, &doc_versions::tests::input("1.0", "2026-10-16")).unwrap();
        projects::get(conn, &p.id).unwrap()
    }

    pub(crate) fn entry_names(path: &Path) -> Vec<String> {
        let zip = ZipArchive::new(File::open(path).unwrap()).unwrap();
        zip.file_names().map(str::to_string).collect()
    }

    #[test]
    fn collect_describes_the_project_without_ids() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let conn = test_support::conn();
        let p = seeded(&conn, &files);
        let export = collect(&conn, &files, &p.id).unwrap();
        let doc = &export.doc;

        assert_eq!(doc.format_version, 1);
        assert_eq!((doc.project.name.as_str(), doc.project.client.as_str()), ("Recibos", "ACME"));
        assert_eq!(doc.project.next_sequence, 3);
        // 11 de la plantilla Funcional + «Severidad»; «Observaciones» quedó archivada.
        assert_eq!(doc.fields.len(), 12);
        assert_eq!(doc.fields[10].key, "severidad");
        assert_eq!(doc.fields[10].options, ["Alta", "Baja"]);
        assert!(doc.fields[11].archived, "las archivadas van al final");
        assert_eq!(doc.fields[11].key, "observaciones");
        let codes: Vec<&str> = doc.cases.iter().map(|c| c.code.as_str()).collect();
        assert_eq!(codes, ["EJ-001", "EJ-002"]);
        let first = &doc.cases[0];
        assert_eq!(first.values["datos_entrada"], "Recibo: 1000001\nSucursal: 2");
        assert_eq!(first.values["observaciones"], "Revisar con el líder.");
        assert_eq!(first.evidence.len(), 2);
        assert_eq!(first.evidence[1].description, "Recibo aplicado");
        assert_eq!(first.evidence[0].original, "evidence/EJ-001-1.orig.png");
        assert_eq!(first.evidence[1].rendered, "evidence/EJ-001-2.render.png");
        assert_eq!((first.evidence[0].width, first.evidence[0].height), (320, 200));
        assert_eq!(doc.doc_versions[0].version, "1.0");
        assert_eq!(export.images.len(), 4);
        assert_eq!((export.evidence_count(), export.missing_images), (2, 0));

        let json = String::from_utf8(project_json(doc).unwrap()).unwrap();
        assert!(json.ends_with("}\n"));
        assert!(!json.contains(&p.id), "sin id del proyecto");
        for f in &p.fields {
            assert!(!json.contains(&f.id), "sin ids de columnas");
        }
        assert!(!json.contains("sourceTemplateId"));
    }

    #[test]
    fn write_creates_the_zip_with_project_json_and_stored_images() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let conn = test_support::conn();
        let p = seeded(&conn, &files);
        let export = collect(&conn, &files, &p.id).unwrap();
        let dest = tmp.path().join("Recibos.qapture");
        write(&export, &dest).unwrap();

        assert_eq!(
            entry_names(&dest).into_iter().collect::<std::collections::BTreeSet<_>>(),
            [
                "project.json",
                "evidence/EJ-001-1.orig.png",
                "evidence/EJ-001-1.render.png",
                "evidence/EJ-001-2.orig.png",
                "evidence/EJ-001-2.render.png",
            ]
            .into_iter()
            .map(str::to_string)
            .collect()
        );
        let mut zip = ZipArchive::new(File::open(&dest).unwrap()).unwrap();
        let mut json = Vec::new();
        zip.by_name(PROJECT_JSON).unwrap().read_to_end(&mut json).unwrap();
        assert_eq!(json, project_json(&export.doc).unwrap());
        for (name, source) in &export.images {
            let mut entry = zip.by_name(name).unwrap();
            assert_eq!(entry.compression(), zip::CompressionMethod::Stored, "{name}");
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).unwrap();
            assert_eq!(bytes, fs::read(source).unwrap(), "{name}");
        }
        let leftovers = fs::read_dir(tmp.path())
            .unwrap()
            .filter(|e| e.as_ref().unwrap().file_name().to_string_lossy().ends_with(".tmp"))
            .count();
        assert_eq!(leftovers, 0);
        write(&export, &dest).unwrap();
        assert_eq!(entry_names(&dest).len(), 5, "sobrescribir deja un zip válido");
    }

    #[test]
    fn evidence_without_an_image_is_skipped_and_counted() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let conn = test_support::conn();
        let p = seeded(&conn, &files);
        let case_id = cases::list(&conn, &p.id).unwrap()[0].id.clone();
        let first = evidence::list(&conn, &files, &case_id).unwrap().remove(0);
        fs::remove_file(&first.rendered_path).unwrap();

        let export = collect(&conn, &files, &p.id).unwrap();
        assert_eq!((export.evidence_count(), export.missing_images), (1, 1));
        let kept = &export.doc.cases[0].evidence[0];
        assert_eq!(kept.description, "Recibo aplicado");
        assert_eq!(kept.original, "evidence/EJ-001-1.orig.png");
    }

    #[test]
    fn suggested_name_and_extension() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 16).unwrap();
        assert_eq!(suggested_file_name("ACME", "Recibos / Q4", today), "T4S_ACME_Recibos-Q4_20261016.qapture");
        assert_eq!(suggested_file_name("", "Pólizas", today), "T4S_Pólizas_20261016.qapture");
        assert_eq!(suggested_file_name("", "///", today), "T4S_Proyecto_20261016.qapture");
        assert_eq!(with_package_extension(PathBuf::from("a")), PathBuf::from("a.qapture"));
        assert_eq!(with_package_extension(PathBuf::from("a.QAPTURE")), PathBuf::from("a.QAPTURE"));
        assert_eq!(with_package_extension(PathBuf::from("a.zip")), PathBuf::from("a.zip.qapture"));
    }
}
```

Run: `cd src-tauri && cargo test transfer::package`
Expected: FAIL al compilar.

- [ ] **Step 2: Implementar el formato y la exportación**

Agrega al inicio de `src-tauri/src/transfer/package.rs`:

```rust
//! UC-18: paquete `.qapture` = ZIP con `project.json` (`formatVersion: 1`) y las capturas en
//! `evidence/<código>-<n>.orig.png` / `.render.png`. `project.json` no lleva ids ni la plantilla de
//! origen: exportar → importar → exportar da el mismo archivo. Importar crea siempre una copia.

use std::{
    collections::{BTreeMap, HashMap},
    fs::{self, File},
    io::{self, BufWriter, Write},
    path::{Path, PathBuf},
};

use chrono::NaiveDate;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

use crate::{
    db::new_id,
    domain::fields::FieldType,
    error::{AppError, AppResult},
    excel::{data::file_segment, save::ensure_writable},
    media::storage::EvidenceFiles,
    repo::{cases, doc_versions, projects},
};

pub const FORMAT_VERSION: i64 = 1;
pub const PROJECT_JSON: &str = "project.json";
pub const EXTENSION: &str = "qapture";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackageDoc {
    pub format_version: i64,
    pub project: PackageProject,
    pub fields: Vec<PackageField>,
    pub cases: Vec<PackageCase>,
    pub doc_versions: Vec<PackageDocVersion>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackageProject {
    pub name: String,
    pub client: String,
    pub test_type: String,
    pub release_version: String,
    pub start_date: String,
    pub end_date: String,
    pub next_sequence: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackageField {
    pub key: String,
    pub label: String,
    #[serde(rename = "type")]
    pub field_type: FieldType,
    pub options: Vec<String>,
    pub required: bool,
    pub clear_on_duplicate: bool,
    pub width: i64,
    pub archived: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackageCase {
    pub code: String,
    pub title: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    /// Clave de columna → valor.
    pub values: BTreeMap<String, String>,
    pub evidence: Vec<PackageEvidence>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackageEvidence {
    pub description: String,
    pub annotations: String,
    pub width: i64,
    pub height: i64,
    pub captured_at: String,
    pub updated_at: String,
    /// Rutas dentro del ZIP.
    pub original: String,
    pub rendered: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackageDocVersion {
    pub version: String,
    pub date: String,
    pub author: String,
    pub reviewer: String,
    pub approver: String,
    pub changes: String,
    pub created_at: String,
}

/// Lo que se escribe en el ZIP. Se arma con la base bloqueada y se escribe sin ella.
pub struct PackageExport {
    pub doc: PackageDoc,
    /// Ruta dentro del ZIP → archivo en `evidence/`.
    pub images: Vec<(String, PathBuf)>,
    /// Capturas omitidas porque falta su original o su render.
    pub missing_images: i64,
}

impl PackageExport {
    pub fn evidence_count(&self) -> i64 {
        self.doc.cases.iter().map(|c| c.evidence.len() as i64).sum()
    }
}

struct StoredEvidence {
    description: String,
    annotations: String,
    width: i64,
    height: i64,
    captured_at: String,
    updated_at: String,
    original_rel: String,
    render_rel: String,
}

fn evidence_by_case(
    conn: &Connection,
    project_id: &str,
) -> AppResult<HashMap<String, Vec<StoredEvidence>>> {
    let mut stmt = conn.prepare(
        "SELECT e.case_id, e.description, e.annotations, e.width, e.height, e.captured_at,
                e.updated_at, e.original_path, e.rendered_path
         FROM evidence e JOIN test_case c ON c.id = e.case_id
         WHERE c.project_id = ?1 ORDER BY e.position, e.captured_at",
    )?;
    let rows = stmt.query_map([project_id], |r| {
        Ok((
            r.get::<_, String>(0)?,
            StoredEvidence {
                description: r.get(1)?,
                annotations: r.get(2)?,
                width: r.get(3)?,
                height: r.get(4)?,
                captured_at: r.get(5)?,
                updated_at: r.get(6)?,
                original_rel: r.get(7)?,
                render_rel: r.get(8)?,
            },
        ))
    })?;
    let mut out: HashMap<String, Vec<StoredEvidence>> = HashMap::new();
    for row in rows {
        let (case_id, stored) = row?;
        out.entry(case_id).or_default().push(stored);
    }
    Ok(out)
}

pub fn collect(conn: &Connection, files: &EvidenceFiles, project_id: &str) -> AppResult<PackageExport> {
    let project = projects::get(conn, project_id)?;
    let keys: HashMap<&str, &str> = project
        .fields
        .iter()
        .map(|f| (f.id.as_str(), f.key.as_str()))
        .collect();
    let mut by_case = evidence_by_case(conn, project_id)?;
    let mut images = Vec::new();
    let mut missing_images = 0;
    let mut package_cases = Vec::new();
    for case in cases::list(conn, project_id)? {
        let mut evidence = Vec::new();
        for stored in by_case.remove(&case.id).unwrap_or_default() {
            let original = files.absolute(&stored.original_rel);
            let rendered = files.absolute(&stored.render_rel);
            if !original.is_file() || !rendered.is_file() {
                missing_images += 1;
                continue;
            }
            let n = evidence.len() + 1;
            let original_name = format!("evidence/{}-{n}.orig.png", case.code);
            let rendered_name = format!("evidence/{}-{n}.render.png", case.code);
            images.push((original_name.clone(), original));
            images.push((rendered_name.clone(), rendered));
            evidence.push(PackageEvidence {
                description: stored.description,
                annotations: stored.annotations,
                width: stored.width,
                height: stored.height,
                captured_at: stored.captured_at,
                updated_at: stored.updated_at,
                original: original_name,
                rendered: rendered_name,
            });
        }
        let values = case
            .values
            .iter()
            .filter_map(|(field_id, value)| {
                keys.get(field_id.as_str())
                    .map(|key| (key.to_string(), value.clone()))
            })
            .collect();
        package_cases.push(PackageCase {
            code: case.code,
            title: case.title,
            status: case.status,
            created_at: case.created_at,
            updated_at: case.updated_at,
            values,
            evidence,
        });
    }
    let doc = PackageDoc {
        format_version: FORMAT_VERSION,
        project: PackageProject {
            name: project.name.clone(),
            client: project.client.clone(),
            test_type: project.test_type.clone(),
            release_version: project.release_version.clone(),
            start_date: project.start_date.clone(),
            end_date: project.end_date.clone(),
            next_sequence: project.next_sequence,
            created_at: project.created_at.clone(),
            updated_at: project.updated_at.clone(),
        },
        fields: project
            .fields
            .iter()
            .map(|f| PackageField {
                key: f.key.clone(),
                label: f.label.clone(),
                field_type: f.field_type,
                options: f.options.clone(),
                required: f.required,
                clear_on_duplicate: f.clear_on_duplicate,
                width: f.width,
                archived: f.archived,
            })
            .collect(),
        cases: package_cases,
        doc_versions: doc_versions::list(conn, project_id)?
            .into_iter()
            .map(|v| PackageDocVersion {
                version: v.version,
                date: v.date,
                author: v.author,
                reviewer: v.reviewer,
                approver: v.approver,
                changes: v.changes,
                created_at: v.created_at,
            })
            .collect(),
    };
    Ok(PackageExport {
        doc,
        images,
        missing_images,
    })
}

pub fn project_json(doc: &PackageDoc) -> AppResult<Vec<u8>> {
    let mut out = serde_json::to_vec_pretty(doc)?;
    out.push(b'\n');
    Ok(out)
}

fn zip_error(error: zip::result::ZipError) -> AppError {
    AppError::Io(io::Error::other(error.to_string()))
}

fn write_zip(export: &PackageExport, path: &Path) -> AppResult<()> {
    let mut zip = ZipWriter::new(BufWriter::new(File::create(path)?));
    let json = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    // Los PNG ya vienen comprimidos: guardarlos tal cual es más rápido y no ocupa más.
    let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    zip.start_file(PROJECT_JSON, json).map_err(zip_error)?;
    zip.write_all(&project_json(&export.doc)?)?;
    for (name, source) in &export.images {
        zip.start_file(name.as_str(), stored).map_err(zip_error)?;
        io::copy(&mut File::open(source)?, &mut zip)?;
    }
    zip.finish().map_err(zip_error)?.flush()?;
    Ok(())
}

/// Escribe en un temporal junto al destino y lo renombra: nunca deja un paquete a medias.
pub fn write(export: &PackageExport, dest: &Path) -> AppResult<()> {
    ensure_writable(dest)?;
    let tmp = dest.with_extension(format!("{}.tmp", new_id()));
    let result = write_zip(export, &tmp).and_then(|()| Ok(fs::rename(&tmp, dest)?));
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

/// `T4S_<Cliente>_<Proyecto>_<AAAAMMDD>.qapture` (el cliente se omite si está vacío).
pub fn suggested_file_name(client: &str, name: &str, today: NaiveDate) -> String {
    let mut parts = vec!["T4S".to_string()];
    let client = file_segment(client, 40);
    if !client.is_empty() {
        parts.push(client);
    }
    let name = file_segment(name, 60);
    parts.push(if name.is_empty() { "Proyecto".into() } else { name });
    parts.push(today.format("%Y%m%d").to_string());
    format!("{}.{EXTENSION}", parts.join("_"))
}

/// «Guardar como» deja escribir cualquier nombre: el paquete siempre termina en `.qapture`.
pub fn with_package_extension(path: PathBuf) -> PathBuf {
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case(EXTENSION))
    {
        return path;
    }
    let mut raw = path.into_os_string();
    raw.push(".");
    raw.push(EXTENSION);
    PathBuf::from(raw)
}
```

`excel::data::file_segment` y `excel::save::ensure_writable` ya son `pub` en módulos `pub` de `excel`, y `repo::doc_versions::tests` es `pub(crate)` (F4): no hace falta tocarlos.

- [ ] **Step 3: Ejecutar los tests**

Run: `cd src-tauri && cargo test transfer::package && cargo clippy --all-targets -- -D warnings && cargo fmt --check`
Expected: PASS (4 tests).

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/transfer src-tauri/src/excel
git commit -m "feat(f5): exportar paquete .qapture con project.json sin ids y capturas por código de caso

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: Paquete `.qapture` — importación como copia y viaje de ida y vuelta (`transfer/package.rs`, parte 2)

**Files:**
- Modify: `src-tauri/src/transfer/package.rs`, `src-tauri/src/media/raster.rs`, `src-tauri/src/repo/evidence.rs` (`validate_annotations` → `pub(crate)`), `src-tauri/src/repo/doc_versions.rs` (`validate` → `pub(crate)`)

**Interfaces:**
- Consumes: `bundle::{ImportFiles, insert, Problems, is_case_code, code_number, IMPORT_FAILED}` (Task 1); `PackageDoc` y `collect`/`write` (Task 4); `projects::validate_meta`, `evidence::validate_annotations`, `doc_versions::validate`, `domain::fields::{validate_value, FieldRule}`.
- Produces:
  - `media::raster::verify_png(bytes) -> AppResult<(u32, u32)>`: PNG que decodifica entero y mide como mucho 12000 px por lado.
  - `transfer::package::import(conn: &Connection, files: &EvidenceFiles, path: &Path) -> AppResult<ImportSummaryDto>`.
  - Constantes `NOT_PACKAGE`, `NEWER`, `TOO_BIG`.

- [ ] **Step 1: Test de `verify_png` (falla)**

En `src-tauri/src/media/raster.rs`, dentro de `mod tests`:

```rust
    #[test]
    fn verify_png_requires_a_complete_png() {
        assert_eq!(verify_png(&sample_png(64, 48)).unwrap(), (64, 48));
        let png = sample_png(64, 48);
        assert!(verify_png(&png[..png.len() - 20]).is_err(), "truncado");
        assert!(verify_png(&encode(rgb(10, 10), ImageFormat::Jpeg)).is_err(), "no es PNG");
        assert!(verify_png(&sample_png(12_001, 1)).is_err());
    }
```

Run: `cd src-tauri && cargo test raster::tests::verify_png`
Expected: FAIL.

- [ ] **Step 2: Implementar `verify_png`**

En `src-tauri/src/media/raster.rs`, después de `to_png`:

```rust
/// Paquetes `.qapture`: el archivo debe ser un PNG completo de hasta 12000 px por lado.
pub fn verify_png(bytes: &[u8]) -> AppResult<(u32, u32)> {
    if image::guess_format(bytes).ok() != Some(ImageFormat::Png) {
        return Err(validation(UNSUPPORTED));
    }
    let (width, height) = dimensions(bytes)?;
    check_side(width, height)?;
    reader(bytes)?
        .decode()
        .map_err(|_| validation("La imagen está dañada o incompleta."))?;
    Ok((width, height))
}
```

Run: `cd src-tauri && cargo test raster`
Expected: PASS.

- [ ] **Step 3: Tests de la importación (fallan)**

En `src-tauri/src/repo/evidence.rs`, cambia `fn validate_annotations` a `pub(crate) fn validate_annotations`. En `src-tauri/src/repo/doc_versions.rs`, cambia `fn validate` a `pub(crate) fn validate`.

En `src-tauri/src/transfer/package.rs`, dentro de `mod tests`, agrega:

```rust
    use std::io::Write as _;

    use serde_json::json;
    use zip::{ZipWriter, write::SimpleFileOptions};

    use crate::transfer::bundle::{IMPORT_FAILED, tests::project_dirs};

    fn exported(conn: &Connection, files: &EvidenceFiles, dir: &Path) -> (ProjectDto, PathBuf) {
        let p = seeded(conn, files);
        let path = dir.join("Recibos.qapture");
        write(&collect(conn, files, &p.id).unwrap(), &path).unwrap();
        (p, path)
    }

    /// Copia el ZIP cambiando entradas: `None` la quita, `Some(bytes)` la reemplaza.
    fn rewrite(src: &Path, dest: &Path, change: impl Fn(&str, Vec<u8>) -> Option<Vec<u8>>) {
        let mut input = ZipArchive::new(File::open(src).unwrap()).unwrap();
        let mut output = ZipWriter::new(File::create(dest).unwrap());
        for index in 0..input.len() {
            let mut entry = input.by_index(index).unwrap();
            let name = entry.name().to_string();
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).unwrap();
            drop(entry);
            if let Some(bytes) = change(&name, bytes) {
                output.start_file(name, SimpleFileOptions::default()).unwrap();
                output.write_all(&bytes).unwrap();
            }
        }
        output.finish().unwrap();
    }

    fn with_json(src: &Path, dest: &Path, edit: impl Fn(&mut serde_json::Value)) {
        rewrite(src, dest, |name, bytes| {
            if name != PROJECT_JSON {
                return Some(bytes);
            }
            let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            edit(&mut value);
            Some(serde_json::to_vec(&value).unwrap())
        });
    }

    #[test]
    fn export_import_export_gives_the_same_project_json() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let conn = test_support::conn();
        let (p, path) = exported(&conn, &files, tmp.path());
        let first = collect(&conn, &files, &p.id).unwrap();

        let other = TempDir::new();
        let other_files = other.files();
        let other_conn = test_support::conn();
        let summary = import(&other_conn, &other_files, &path).unwrap();
        assert_eq!(summary.projects[0].name, "Recibos");
        assert_eq!((summary.cases, summary.evidence), (2, 2));
        let imported = projects::get(&other_conn, &summary.projects[0].id).unwrap();
        assert_ne!(imported.id, p.id, "es una copia con ids nuevos");
        assert_eq!(imported.source_template_id, None);

        let second = collect(&other_conn, &other_files, &imported.id).unwrap();
        assert_eq!(project_json(&first.doc).unwrap(), project_json(&second.doc).unwrap());
        for ((name_a, file_a), (name_b, file_b)) in first.images.iter().zip(&second.images) {
            assert_eq!(name_a, name_b);
            assert_eq!(fs::read(file_a).unwrap(), fs::read(file_b).unwrap(), "{name_a}");
        }
        let again = other.path().join("again.qapture");
        write(&second, &again).unwrap();
        let mut zip = ZipArchive::new(File::open(&again).unwrap()).unwrap();
        let mut json = Vec::new();
        zip.by_name(PROJECT_JSON).unwrap().read_to_end(&mut json).unwrap();
        assert_eq!(json, project_json(&first.doc).unwrap(), "el project.json releído es idéntico");
    }

    #[test]
    fn importing_the_same_package_twice_creates_copies() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let conn = test_support::conn();
        let (p, path) = exported(&conn, &files, tmp.path());
        let a = import(&conn, &files, &path).unwrap();
        let b = import(&conn, &files, &path).unwrap();
        assert_eq!(a.projects[0].name, "Recibos (importado)");
        assert_eq!(b.projects[0].name, "Recibos (importado 2)");
        assert_eq!(projects::list(&conn).unwrap().len(), 3);
        assert_eq!(cases::list(&conn, &p.id).unwrap().len(), 2, "el original no cambia");
    }

    #[test]
    fn foreign_newer_or_damaged_packages_import_nothing() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let conn = test_support::conn();
        let (_, good) = exported(&conn, &files, tmp.path());
        let bad = tmp.path().join("bad.qapture");
        let not_zip = tmp.path().join("hola.qapture");
        fs::write(&not_zip, b"hola").unwrap();

        type Prepare = Box<dyn Fn(&Path, &Path)>;
        let cases: Vec<(Prepare, String)> = vec![
            (Box::new(|_: &Path, dest: &Path| fs::copy(dest.with_file_name("hola.qapture"), dest).map(|_| ()).unwrap()), NOT_PACKAGE.to_string()),
            (Box::new(|src: &Path, dest: &Path| rewrite(src, dest, |name, b| (name != PROJECT_JSON).then_some(b))), NOT_PACKAGE.to_string()),
            (Box::new(|src: &Path, dest: &Path| with_json(src, dest, |v| v["formatVersion"] = json!(2))), NEWER.to_string()),
            (Box::new(|src: &Path, dest: &Path| with_json(src, dest, |v| v["formatVersion"] = json!("1"))), NOT_PACKAGE.to_string()),
            (Box::new(|src: &Path, dest: &Path| with_json(src, dest, |v| v["extra"] = json!(true))), NOT_PACKAGE.to_string()),
            (
                Box::new(|src: &Path, dest: &Path| rewrite(src, dest, |name, b| (name != "evidence/EJ-001-2.orig.png").then_some(b))),
                "El paquete está dañado: falta «evidence/EJ-001-2.orig.png».".to_string(),
            ),
            (
                Box::new(|src: &Path, dest: &Path| rewrite(src, dest, |name, b| Some(if name == "evidence/EJ-001-2.render.png" { b"no es png".to_vec() } else { b }))),
                format!("{IMPORT_FAILED}\n• EJ-001, captura 2: Contenido de imagen inválido."),
            ),
            (
                Box::new(|src: &Path, dest: &Path| with_json(src, dest, |v| v["cases"][0]["evidence"][0]["original"] = json!("../../qapture.db"))),
                format!("{IMPORT_FAILED}\n• EJ-001, captura 1: Ruta de imagen inválida o repetida."),
            ),
            (
                Box::new(|src: &Path, dest: &Path| with_json(src, dest, |v| v["cases"][1]["status"] = json!("Cerrado"))),
                format!("{IMPORT_FAILED}\n• EJ-002: Estado inválido: «Cerrado»."),
            ),
            (
                Box::new(|src: &Path, dest: &Path| with_json(src, dest, |v| v["cases"][1]["code"] = json!("EJ-001"))),
                format!("{IMPORT_FAILED}\n• EJ-001: ID de ejecución repetido o inválido."),
            ),
            (
                Box::new(|src: &Path, dest: &Path| with_json(src, dest, |v| v["cases"][0]["values"]["severidad"] = json!("Media"))),
                format!("{IMPORT_FAILED}\n• EJ-001: «Severidad» debe ser una de las opciones de la lista."),
            ),
            (
                Box::new(|src: &Path, dest: &Path| with_json(src, dest, |v| v["cases"][0]["values"]["no_existe"] = json!("x"))),
                format!("{IMPORT_FAILED}\n• EJ-001: La columna «no_existe» no existe en el paquete."),
            ),
        ];
        for (prepare, expected) in cases {
            prepare(&good, &bad);
            let other = TempDir::new();
            let other_files = other.files();
            let other_conn = test_support::conn();
            let err = import(&other_conn, &other_files, &bad).unwrap_err().to_string();
            assert_eq!(err, expected);
            assert!(projects::list(&other_conn).unwrap().is_empty(), "{expected}");
            assert_eq!(project_dirs(&other_files), 0, "{expected}");
        }
    }

    #[test]
    fn an_entry_that_inflates_beyond_the_limit_is_rejected() {
        let tmp = TempDir::new();
        let path = tmp.path().join("bomba.qapture");
        let mut zip = ZipWriter::new(File::create(&path).unwrap());
        let deflated = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        zip.start_file(PROJECT_JSON, deflated).unwrap();
        zip.write_all(&vec![b' '; 65 * 1024 * 1024]).unwrap();
        zip.finish().unwrap();
        assert!(fs::metadata(&path).unwrap().len() < 1024 * 1024, "comprimido pesa poco");
        let conn = test_support::conn();
        assert_eq!(import(&conn, &tmp.files(), &path).unwrap_err().to_string(), TOO_BIG);
    }
```

Run: `cd src-tauri && cargo test transfer::package`
Expected: FAIL al compilar (`import`, `NOT_PACKAGE`… no existen).

- [ ] **Step 4: Implementar la importación**

En `src-tauri/src/transfer/package.rs`, completa los `use` del inicio:

```rust
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs::{self, File},
    io::{self, BufWriter, Read, Seek, Write},
    path::{Path, PathBuf},
};

use zip::{CompressionMethod, ZipArchive, ZipWriter, write::SimpleFileOptions};

use super::bundle::{
    self, BundleCase, BundleEvidence, BundleField, BundleVersion, IMPORT_FAILED, ImportFiles,
    Problems, ProjectBundle, code_number, is_case_code,
};
use crate::{
    db::new_id,
    domain::{
        fields::{FieldRule, FieldType, validate_value},
        status::is_status,
        text::required_text,
    },
    dto::{DocVersionInput, ImportSummaryDto, ProjectMetaInput},
    error::{AppError, AppResult, validation},
    excel::{data::file_segment, save::ensure_writable},
    media::{raster, storage::EvidenceFiles},
    repo::{cases, doc_versions, evidence, projects},
};
```

(reemplazan a los de la Task 4; `chrono`, `rusqlite` y `serde` quedan igual) y agrega al final, antes de los tests:

```rust
pub const NOT_PACKAGE: &str = "El archivo no es un paquete de Qapture válido.";
pub const NEWER: &str =
    "El paquete se creó con una versión más nueva de Qapture. Actualiza la app para abrirlo.";
pub const TOO_BIG: &str = "El paquete supera el tamaño permitido.";

const MAX_ENTRIES: usize = 20_001;
const MAX_JSON_BYTES: u64 = 64 * 1024 * 1024;
const MAX_IMAGE_BYTES: u64 = raster::MAX_INPUT_BYTES as u64;
/// Tope de todo lo descomprimido: protege el disco de un ZIP «bomba».
const MAX_TOTAL_BYTES: u64 = 4 * 1024 * 1024 * 1024;
const MAX_FIELDS: usize = 60;
const MAX_CASES: usize = 10_000;
const MAX_EVIDENCE_PER_CASE: usize = 500;
/// La v1 admitía descripciones de 4000 caracteres y se importan completas (Task 3).
const MAX_DESCRIPTION: usize = 4_000;
const MAX_ARCHIVED_VALUE: usize = 16_000;

/// Lee entradas por nombre (nunca extrae rutas del ZIP al disco) con topes por entrada y en total.
struct Reader<R> {
    zip: ZipArchive<R>,
    left: u64,
}

impl<R: Read + Seek> Reader<R> {
    fn has(&mut self, name: &str) -> bool {
        self.zip.by_name(name).is_ok()
    }

    fn read(&mut self, name: &str, max: u64) -> AppResult<Vec<u8>> {
        let entry = self
            .zip
            .by_name(name)
            .map_err(|_| validation(format!("El paquete está dañado: falta «{name}».")))?;
        let limit = max.min(self.left);
        let mut bytes = Vec::new();
        entry
            .take(limit + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| validation(format!("El paquete está dañado: no se pudo leer «{name}».")))?;
        if bytes.len() as u64 > limit {
            return Err(validation(TOO_BIG));
        }
        self.left -= bytes.len() as u64;
        Ok(bytes)
    }
}

fn read_doc<R: Read + Seek>(reader: &mut Reader<R>) -> AppResult<PackageDoc> {
    if !reader.has(PROJECT_JSON) {
        return Err(validation(NOT_PACKAGE));
    }
    let bytes = reader.read(PROJECT_JSON, MAX_JSON_BYTES)?;
    let value: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|_| validation(NOT_PACKAGE))?;
    match value.get("formatVersion").and_then(serde_json::Value::as_i64) {
        Some(FORMAT_VERSION) => {}
        Some(v) if v > FORMAT_VERSION => return Err(validation(NEWER)),
        _ => return Err(validation(NOT_PACKAGE)),
    }
    serde_json::from_value(value).map_err(|_| validation(NOT_PACKAGE))
}

fn meta_of(p: &PackageProject) -> ProjectMetaInput {
    ProjectMetaInput {
        name: p.name.clone(),
        client: p.client.clone(),
        test_type: p.test_type.clone(),
        release_version: p.release_version.clone(),
        start_date: p.start_date.clone(),
        end_date: p.end_date.clone(),
    }
}

fn version_input(v: &PackageDocVersion) -> DocVersionInput {
    DocVersionInput {
        version: v.version.clone(),
        date: v.date.clone(),
        author: v.author.clone(),
        reviewer: v.reviewer.clone(),
        approver: v.approver.clone(),
        changes: v.changes.clone(),
    }
}

/// `evidence/<archivo>.png`, sin subcarpetas ni `..`.
fn is_image_entry(name: &str) -> bool {
    name.strip_prefix("evidence/").is_some_and(|file| {
        file.len() > 4
            && file.ends_with(".png")
            && !file.starts_with('.')
            && file
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
    })
}

fn is_field_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 60
        && key
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

fn validate_fields(fields: &[PackageField], problems: &mut Problems) {
    if fields.is_empty() || fields.len() > MAX_FIELDS {
        problems.add("Columnas", "El paquete debe tener entre 1 y 60 columnas.");
    }
    let mut keys = HashSet::new();
    for f in fields {
        let label = f.label.trim();
        let at = format!("Columna «{label}»");
        if !is_field_key(&f.key) || !keys.insert(f.key.as_str()) {
            problems.add(&at, "Clave de columna inválida o repetida.");
        }
        if let Err(e) = required_text("Nombre de la columna", &f.label, 100) {
            problems.add(&at, &e.to_string());
        }
        if !(8..=100).contains(&f.width) {
            problems.add(&at, &format!("El ancho de «{label}» debe estar entre 8 y 100."));
        }
        if f.field_type == FieldType::List && f.options.is_empty() {
            problems.add(&at, &format!("La lista «{label}» necesita al menos una opción."));
        }
    }
}

fn validate_value_of(doc: &PackageDoc, key: &str, value: &str, at: &str, problems: &mut Problems) {
    let Some(f) = doc.fields.iter().find(|f| f.key == key) else {
        problems.add(at, &format!("La columna «{key}» no existe en el paquete."));
        return;
    };
    if f.archived {
        // Una columna archivada puede guardar valores de un tipo anterior: solo se limita el largo.
        if value.chars().count() > MAX_ARCHIVED_VALUE {
            problems.add(at, &format!("«{}» supera {MAX_ARCHIVED_VALUE} caracteres.", f.label));
        }
        return;
    }
    let rule = FieldRule {
        label: &f.label,
        field_type: f.field_type,
        options: &f.options,
        required: false,
    };
    if let Err(message) = validate_value(&rule, value) {
        problems.add(at, &message);
    }
}

fn validate_cases(doc: &PackageDoc, problems: &mut Problems) {
    if doc.cases.len() > MAX_CASES {
        problems.add("Casos", "Demasiados casos.");
    }
    let mut codes = HashSet::new();
    let mut images = HashSet::new();
    for (index, c) in doc.cases.iter().enumerate() {
        let valid_code = is_case_code(&c.code);
        let at = if valid_code {
            c.code.clone()
        } else {
            format!("Caso {}", index + 1)
        };
        if !valid_code || !codes.insert(c.code.as_str()) {
            problems.add(&at, "ID de ejecución repetido o inválido.");
        } else if code_number(&c.code).is_none_or(|n| n >= doc.project.next_sequence) {
            problems.add(&at, "La secuencia debe superar los IDs usados.");
        }
        if let Err(e) = required_text("Caso de prueba", &c.title, 300) {
            problems.add(&at, &e.to_string());
        }
        if !is_status(&c.status) {
            problems.add(&at, &format!("Estado inválido: «{}».", c.status));
        }
        for (key, value) in &c.values {
            validate_value_of(doc, key, value, &at, problems);
        }
        if c.evidence.len() > MAX_EVIDENCE_PER_CASE {
            problems.add(&at, "Demasiadas capturas por caso.");
        }
        for (i, e) in c.evidence.iter().enumerate() {
            let at = format!("{at}, captura {}", i + 1);
            if let Err(err) = required_text("Descripción", &e.description, MAX_DESCRIPTION) {
                problems.add(&at, &err.to_string());
            }
            if let Err(err) = evidence::validate_annotations(&e.annotations) {
                problems.add(&at, &err.to_string());
            }
            for name in [&e.original, &e.rendered] {
                if !is_image_entry(name) || !images.insert(name.as_str()) {
                    problems.add(&at, "Ruta de imagen inválida o repetida.");
                }
            }
        }
    }
}

fn validate_versions(versions: &[PackageDocVersion], problems: &mut Problems) {
    let mut seen = HashSet::new();
    for v in versions {
        let at = format!("Versión «{}»", v.version.trim());
        match doc_versions::validate(&version_input(v)) {
            Err(e) => problems.add(&at, &e.to_string()),
            Ok(clean) if !seen.insert(clean.version.to_lowercase()) => problems.add(
                &at,
                &format!("La versión «{}» ya existe en este proyecto.", clean.version),
            ),
            Ok(_) => {}
        }
    }
}

fn validate(doc: &PackageDoc) -> AppResult<()> {
    let mut problems = Problems::default();
    if let Err(e) = projects::validate_meta(&meta_of(&doc.project)) {
        problems.add("Proyecto", &e.to_string());
    }
    if doc.project.next_sequence < 1 {
        problems.add("Proyecto", "Secuencia inválida.");
    }
    validate_fields(&doc.fields, &mut problems);
    validate_cases(doc, &mut problems);
    validate_versions(&doc.doc_versions, &mut problems);
    problems.into_result()
}

fn bad_image(at: &str) -> AppError {
    validation(format!("{IMPORT_FAILED}\n• {at}: Contenido de imagen inválido."))
}

fn to_bundle<R: Read + Seek>(
    doc: PackageDoc,
    reader: &mut Reader<R>,
    out: &mut ImportFiles,
) -> AppResult<ProjectBundle> {
    let id = new_id();
    let meta = projects::validate_meta(&meta_of(&doc.project))?;
    let mut bundle_cases = Vec::with_capacity(doc.cases.len());
    for c in doc.cases {
        let mut evidence = Vec::with_capacity(c.evidence.len());
        for (index, e) in c.evidence.into_iter().enumerate() {
            let at = format!("{}, captura {}", c.code, index + 1);
            let original = reader.read(&e.original, MAX_IMAGE_BYTES)?;
            let rendered = reader.read(&e.rendered, MAX_IMAGE_BYTES)?;
            let (width, height) = raster::verify_png(&original).map_err(|_| bad_image(&at))?;
            raster::verify_png(&rendered).map_err(|_| bad_image(&at))?;
            let evidence_id = new_id();
            out.write(&id, &evidence_id, &original, &rendered)?;
            evidence.push(BundleEvidence {
                id: evidence_id,
                description: e.description.trim().to_string(),
                annotations: e.annotations,
                width: width.into(),
                height: height.into(),
                captured_at: e.captured_at,
                updated_at: e.updated_at,
            });
        }
        bundle_cases.push(BundleCase {
            code: c.code,
            title: c.title.trim().to_string(),
            status: c.status,
            created_at: c.created_at,
            updated_at: c.updated_at,
            values: c.values,
            evidence,
        });
    }
    Ok(ProjectBundle {
        id,
        meta,
        next_sequence: doc.project.next_sequence,
        source_template_id: None,
        created_at: doc.project.created_at,
        updated_at: doc.project.updated_at,
        fields: doc
            .fields
            .into_iter()
            .map(|f| BundleField {
                key: f.key,
                label: f.label.trim().to_string(),
                field_type: f.field_type,
                options: f.options,
                required: f.required,
                clear_on_duplicate: f.clear_on_duplicate,
                width: f.width,
                archived: f.archived,
            })
            .collect(),
        cases: bundle_cases,
        versions: doc
            .doc_versions
            .iter()
            .map(|v| BundleVersion {
                input: version_input(v),
                created_at: v.created_at.clone(),
            })
            .collect(),
    })
}

/// UC-18: importa el paquete como un proyecto nuevo (nunca sobrescribe). Todo o nada.
pub fn import(conn: &Connection, files: &EvidenceFiles, path: &Path) -> AppResult<ImportSummaryDto> {
    let zip = ZipArchive::new(File::open(path)?).map_err(|_| validation(NOT_PACKAGE))?;
    if zip.len() > MAX_ENTRIES {
        return Err(validation(TOO_BIG));
    }
    let mut reader = Reader {
        zip,
        left: MAX_TOTAL_BYTES,
    };
    let doc = read_doc(&mut reader)?;
    validate(&doc)?;
    let mut out = ImportFiles::new(files);
    let bundle = to_bundle(doc, &mut reader, &mut out)?;
    bundle::insert(conn, vec![bundle], out)
}
```

- [ ] **Step 5: Ejecutar los tests**

Run: `cd src-tauri && cargo test transfer && cargo clippy --all-targets -- -D warnings && cargo fmt --check`
Expected: PASS. `export_import_export_gives_the_same_project_json` es el criterio de aceptación 3 del contrato.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/transfer/package.rs src-tauri/src/media/raster.rs src-tauri/src/repo/evidence.rs src-tauri/src/repo/doc_versions.rs
git commit -m "feat(f5): importar paquete .qapture como copia, validado y con topes contra zips hostiles

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---
### Task 6: Respaldos — diario con rotación de 7, manual, lista y restauración con copia previa (`backup.rs`)

**Files:**
- Create: `src-tauri/src/backup.rs`
- Modify: `src-tauri/src/lib.rs` (`mod backup;`), `src-tauri/src/dto.rs`, `src-tauri/src/db/mod.rs`, `src-tauri/src/media/storage.rs`, `src-tauri/src/repo/evidence.rs`

**Interfaces:**
- Consumes: `rusqlite::{Connection::backup, Connection::restore, MAIN_DB}` (feature `backup`, Task 1), `db::schema_version`.
- Produces:
  - `dto::BackupKind::{Daily, Manual, BeforeRestore, BeforeMigration}` (JSON `"daily" | "manual" | "before-restore" | "before-migration"`), `dto::BackupDto { file_name, kind, created_at /*"AAAA-MM-DDTHH:MM:SS" local, fecha de modificación*/, size_bytes }`, `dto::RestoreResultDto { restored: BackupDto, safety_copy: String, missing_images: i64, set_aside_files: i64 }`.
  - `db::{DB_FILE = "qapture.db", latest_version() -> i64, prepare(&Connection) -> AppResult<()>}`. `prepare` activa las claves foráneas, migra y siembra las plantillas; `init` lo usa.
  - `media::storage::{UNUSED_DIR = "evidence-sin-uso", EvidenceFiles::set_aside(&self, referenced, projects, stamp) -> AppResult<usize>}`.
  - `repo::evidence::{references(conn) -> AppResult<(HashSet<String>, HashSet<String>)>, missing_images(conn, files) -> AppResult<i64>}`. `sweep_orphans` pasa a usar `references`.
  - `backup::Backups`:
    - `new(data_dir: &Path)`, `dir() -> &Path`.
    - `ensure_daily(&self, conn, today: NaiveDate) -> AppResult<Option<BackupDto>>`.
    - `create_manual(&self, conn, now: NaiveDateTime) -> AppResult<BackupDto>`.
    - `list(&self) -> AppResult<Vec<BackupDto>>`, `resolve(&self, file_name) -> AppResult<PathBuf>`.
    - `Backups::check(path) -> AppResult<()>`.
    - `restore(&self, conn: &mut Connection, files, file_name, now) -> AppResult<RestoreResultDto>`.
  - `backup::KEEP_DAILY = 7`.

- [ ] **Step 1: DTOs, `db::prepare` y referencias de evidencias**

En `src-tauri/src/dto.rs`, después de `ImportSummaryDto`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BackupKind {
    Daily,
    Manual,
    BeforeRestore,
    BeforeMigration,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupDto {
    pub file_name: String,
    pub kind: BackupKind,
    /// Hora local de la copia (`AAAA-MM-DDTHH:MM:SS`).
    pub created_at: String,
    pub size_bytes: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreResultDto {
    pub restored: BackupDto,
    /// Copia del estado anterior, por si hay que deshacer.
    pub safety_copy: String,
    /// Capturas del respaldo cuyo archivo ya no existe.
    pub missing_images: i64,
    /// Capturas que el respaldo no conoce, movidas a `evidence-sin-uso/`.
    pub set_aside_files: i64,
}
```

En `src-tauri/src/db/mod.rs`:

```rust
pub const DB_FILE: &str = "qapture.db";

/// Versión de esquema que deja esta app (`PRAGMA user_version` tras migrar).
pub fn latest_version() -> i64 {
    MIGRATIONS.len() as i64
}

/// Claves foráneas, migraciones y plantillas incluidas. También se usa tras restaurar un respaldo.
pub fn prepare(conn: &Connection) -> AppResult<()> {
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrate(conn)?;
    seed::ensure_builtin_templates(conn)
}
```

y cambia `init` por:

```rust
fn init(conn: Connection) -> AppResult<Connection> {
    prepare(&conn)?;
    Ok(conn)
}
```

En `backup_before_migrate`, reemplaza `MIGRATIONS.len() as i64` por `latest_version()`.

En `src-tauri/src/repo/evidence.rs`, reemplaza `sweep_orphans` por:

```rust
/// Rutas relativas que la base referencia y los ids de proyecto existentes.
pub fn references(conn: &Connection) -> AppResult<(HashSet<String>, HashSet<String>)> {
    let mut referenced = HashSet::new();
    let mut stmt = conn.prepare("SELECT original_path, rendered_path FROM evidence")?;
    for row in stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))? {
        let (original, rendered) = row?;
        referenced.insert(original);
        referenced.insert(rendered);
    }
    let mut stmt = conn.prepare("SELECT id FROM project")?;
    let projects = stmt
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<Result<HashSet<_>, _>>()?;
    Ok((referenced, projects))
}

/// Al arrancar: borra archivos de `evidence/` que la base no referencia.
pub fn sweep_orphans(conn: &Connection, files: &EvidenceFiles) -> AppResult<usize> {
    let (referenced, projects) = references(conn)?;
    files.sweep(&referenced, &projects)
}

/// Capturas cuyo original ya no está en disco (p. ej. tras restaurar un respaldo antiguo).
pub fn missing_images(conn: &Connection, files: &EvidenceFiles) -> AppResult<i64> {
    let mut stmt = conn.prepare("SELECT original_path FROM evidence")?;
    let mut missing = 0;
    for rel in stmt.query_map([], |r| r.get::<_, String>(0))? {
        if !files.absolute(&rel?).is_file() {
            missing += 1;
        }
    }
    Ok(missing)
}
```

Run: `cd src-tauri && cargo test db:: && cargo test repo::evidence`
Expected: PASS (refactor sin cambio de comportamiento).

- [ ] **Step 2: Test de `set_aside` (falla)**

En `src-tauri/src/media/storage.rs`, dentro de `mod tests`:

```rust
    #[test]
    fn set_aside_moves_unreferenced_files_instead_of_deleting_them() {
        let tmp = TempDir::new();
        let files = tmp.files();
        files.stage(b"x").unwrap();
        let keep = EvidenceFiles::original_rel("p1", "e1");
        let extra = EvidenceFiles::render_rel("p1", "e2");
        let gone = EvidenceFiles::original_rel("p2", "e3");
        for rel in [&keep, &extra, &gone] {
            put(&files, rel);
        }
        let moved = files
            .set_aside(
                &HashSet::from([keep.clone()]),
                &HashSet::from(["p1".to_string()]),
                "20261016-100000",
            )
            .unwrap();
        assert_eq!(moved, 2);
        assert!(files.absolute(&keep).exists());
        let target = tmp.path().join(UNUSED_DIR).join("20261016-100000");
        assert!(target.join("p1").join("e2.render.png").exists());
        assert!(target.join("p2").join("e3.orig.png").exists());
        assert!(!files.project_dir("p2").exists());
        assert!(files.staging_dir().exists(), "_staging no se toca");
        assert_eq!(files.set_aside(&HashSet::new(), &HashSet::new(), "otra").unwrap(), 1, "solo queda e1");
    }
```

Run: `cd src-tauri && cargo test storage::tests::set_aside`
Expected: FAIL.

- [ ] **Step 3: Implementar `set_aside`**

En `src-tauri/src/media/storage.rs`, agrega la constante junto a las demás y el método en `impl EvidenceFiles`, después de `sweep`:

```rust
pub const UNUSED_DIR: &str = "evidence-sin-uso";
```

```rust
    /// Como `sweep`, pero mueve lo no referenciado a `evidence-sin-uso/<stamp>/<proyecto>/` en vez
    /// de borrarlo: restaurar un respaldo o recuperar la base nunca debe perder capturas.
    pub fn set_aside(
        &self,
        referenced: &HashSet<String>,
        projects: &HashSet<String>,
        stamp: &str,
    ) -> AppResult<usize> {
        let root = self.root();
        if !root.exists() {
            return Ok(0);
        }
        let target = self.data_dir.join(UNUSED_DIR).join(stamp);
        let mut moved = 0;
        for entry in fs::read_dir(&root)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let path = entry.path();
            if name == STAGING_DIR || !path.is_dir() {
                continue;
            }
            let known_project = projects.contains(&name);
            for file in fs::read_dir(&path)? {
                let file = file?;
                let rel = format!("{EVIDENCE_DIR}/{name}/{}", file.file_name().to_string_lossy());
                if known_project && referenced.contains(&rel) {
                    continue;
                }
                let dest = target.join(&name);
                fs::create_dir_all(&dest)?;
                fs::rename(file.path(), dest.join(file.file_name()))?;
                moved += 1;
            }
            if !known_project {
                let _ = fs::remove_dir(&path);
            }
        }
        Ok(moved)
    }
```

Run: `cd src-tauri && cargo test storage`
Expected: PASS.

- [ ] **Step 4: Tests de respaldos (fallan)**

Agrega `mod backup;` en `src-tauri/src/lib.rs` (después de `mod capture;`). Crea `src-tauri/src/backup.rs` solo con los tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        dto::NewEvidenceMeta,
        media::{raster::sample_png, storage::{TempDir, UNUSED_DIR}},
        repo::{evidence, projects, test_support},
        transfer::bundle::EMPTY_ANNOTATIONS,
    };

    fn day(d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, d).unwrap()
    }

    fn at(h: u32, m: u32, s: u32) -> NaiveDateTime {
        day(16).and_hms_opt(h, m, s).unwrap()
    }

    fn open(tmp: &TempDir) -> Connection {
        db::open(&tmp.path().join(db::DB_FILE)).unwrap()
    }

    fn project_names(path: &Path) -> Vec<String> {
        let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let mut stmt = conn.prepare("SELECT name FROM project ORDER BY name").unwrap();
        stmt.query_map([], |r| r.get(0)).unwrap().map(Result::unwrap).collect()
    }

    fn sorted_names(backups: &Backups) -> Vec<String> {
        let mut names: Vec<String> = backups.list().unwrap().into_iter().map(|b| b.file_name).collect();
        names.sort();
        names
    }

    #[test]
    fn daily_backup_runs_once_per_day_and_keeps_the_last_seven() {
        let tmp = TempDir::new();
        let conn = open(&tmp);
        let backups = Backups::new(tmp.path());
        let first = backups.ensure_daily(&conn, day(1)).unwrap().unwrap();
        assert_eq!((first.file_name.as_str(), first.kind), ("qapture-diario-20261001.db", BackupKind::Daily));
        assert!(backups.ensure_daily(&conn, day(1)).unwrap().is_none(), "una vez por día");
        for d in 2..=9 {
            backups.ensure_daily(&conn, day(d)).unwrap();
        }
        let expected: Vec<String> = (3..=9).map(|d| format!("qapture-diario-202610{d:02}.db")).collect();
        assert_eq!(sorted_names(&backups), expected);
    }

    #[test]
    fn manual_backups_are_readable_copies_and_do_not_rotate() {
        let tmp = TempDir::new();
        let conn = open(&tmp);
        test_support::project(&conn);
        let backups = Backups::new(tmp.path());
        let a = backups.create_manual(&conn, at(9, 30, 0)).unwrap();
        let b = backups.create_manual(&conn, at(9, 30, 0)).unwrap();
        assert_eq!(a.file_name, "qapture-manual-20261016-093000.db");
        assert_eq!(b.file_name, "qapture-manual-20261016-093000-2.db", "no pisa una copia del mismo segundo");
        assert_eq!(a.kind, BackupKind::Manual);
        assert!(a.size_bytes > 0);
        assert_eq!(a.created_at.len(), 19);
        assert_eq!(project_names(&backups.dir().join(&a.file_name)), ["Recibos"]);
        for d in 1..=9 {
            backups.ensure_daily(&conn, day(d)).unwrap();
        }
        assert_eq!(backups.list().unwrap().iter().filter(|b| b.kind == BackupKind::Manual).count(), 2);
        let leftovers: Vec<String> = fs::read_dir(backups.dir())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(".tmp") || n.ends_with("-wal") || n.ends_with("-shm"))
            .collect();
        assert!(leftovers.is_empty(), "{leftovers:?}");
        let journal: String = Connection::open_with_flags(
            backups.dir().join(&a.file_name),
            OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap()
        .query_row("PRAGMA journal_mode", [], |r| r.get(0))
        .unwrap();
        assert_eq!(journal, "delete", "un solo archivo, sin WAL");
    }

    #[test]
    fn list_shows_every_kind_and_ignores_other_files() {
        let tmp = TempDir::new();
        let backups = Backups::new(tmp.path());
        fs::create_dir_all(backups.dir()).unwrap();
        for name in [
            "qapture-diario-20261016.db",
            "qapture-manual-20261016-093000.db",
            "qapture-antes-de-restaurar-20261016-100000.db",
            "qapture-v2-20261015-120000.db",
            "notas.txt",
            "qapture-diario-20261016.db.abc.tmp",
            "otra-base.db",
        ] {
            fs::write(backups.dir().join(name), b"x").unwrap();
        }
        let mut kinds: Vec<BackupKind> = backups.list().unwrap().into_iter().map(|b| b.kind).collect();
        kinds.sort_by_key(|k| format!("{k:?}"));
        assert_eq!(
            kinds,
            [BackupKind::BeforeMigration, BackupKind::BeforeRestore, BackupKind::Daily, BackupKind::Manual]
        );
        assert!(Backups::new(&tmp.path().join("nada")).list().unwrap().is_empty());
    }

    #[test]
    fn restore_returns_to_the_backup_and_keeps_a_safety_copy() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let mut conn = open(&tmp);
        test_support::project(&conn); // «Recibos»
        let backups = Backups::new(tmp.path());
        let saved = backups.create_manual(&conn, at(9, 0, 0)).unwrap();

        let polizas = test_support::other_project(&conn);
        let case = test_support::case(&conn, &polizas);
        let staged = evidence::stage(&files, &sample_png(80, 60)).unwrap();
        let meta = NewEvidenceMeta {
            case_id: case.id.clone(),
            staged_path: staged.path,
            description: "Póliza emitida".into(),
            annotations: EMPTY_ANNOTATIONS.into(),
        };
        evidence::add(&conn, &files, &meta, &sample_png(80, 60)).unwrap();

        let result = backups.restore(&mut conn, &files, &saved.file_name, at(10, 0, 0)).unwrap();
        assert_eq!(result.restored.file_name, saved.file_name);
        assert_eq!(result.safety_copy, "qapture-antes-de-restaurar-20261016-100000.db");
        assert_eq!((result.missing_images, result.set_aside_files), (0, 2));

        let names: Vec<String> = projects::list(&conn).unwrap().into_iter().map(|p| p.name).collect();
        assert_eq!(names, ["Recibos"]);
        assert_eq!(project_names(&backups.dir().join(&result.safety_copy)), ["Pólizas", "Recibos"]);
        let aside = tmp.path().join(UNUSED_DIR).join("20261016-100000").join(&polizas.id);
        assert_eq!(fs::read_dir(aside).unwrap().count(), 2, "orig y render de Pólizas, apartados");
        assert!(!files.project_dir(&polizas.id).exists());
        assert_eq!(db::schema_version(&conn).unwrap(), db::latest_version());
        let fk: i64 = conn.query_row("PRAGMA foreign_keys", [], |r| r.get(0)).unwrap();
        assert_eq!(fk, 1);
        test_support::case(&conn, &projects::get(&conn, &projects::list(&conn).unwrap()[0].id).unwrap());
    }

    #[test]
    fn restore_reports_images_that_no_longer_exist() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let mut conn = open(&tmp);
        let p = test_support::project(&conn);
        let case = test_support::case(&conn, &p);
        let staged = evidence::stage(&files, &sample_png(80, 60)).unwrap();
        let meta = NewEvidenceMeta {
            case_id: case.id.clone(),
            staged_path: staged.path,
            description: "Recibo".into(),
            annotations: EMPTY_ANNOTATIONS.into(),
        };
        let ev = evidence::add(&conn, &files, &meta, &sample_png(80, 60)).unwrap();
        let backups = Backups::new(tmp.path());
        let saved = backups.create_manual(&conn, at(9, 0, 0)).unwrap();
        evidence::delete(&conn, &files, &ev.id).unwrap();
        let result = backups.restore(&mut conn, &files, &saved.file_name, at(10, 0, 0)).unwrap();
        assert_eq!(result.missing_images, 1);
    }

    #[test]
    fn restore_rejects_unknown_names_and_damaged_files_without_touching_anything() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let mut conn = open(&tmp);
        test_support::project(&conn);
        let backups = Backups::new(tmp.path());
        fs::create_dir_all(backups.dir()).unwrap();
        fs::write(backups.dir().join("qapture-manual-20261016-080000.db"), b"no soy una base").unwrap();
        for (name, message) in [
            ("..\\qapture.db", "El respaldo no existe."),
            ("../qapture.db", "El respaldo no existe."),
            ("qapture.db", "El respaldo no existe."),
            ("qapture-manual-20991231-000000.db", "El respaldo no existe."),
            ("qapture-manual-20261016-080000.db", "El respaldo está dañado y no se puede restaurar."),
        ] {
            let err = backups.restore(&mut conn, &files, name, at(10, 0, 0)).unwrap_err();
            assert_eq!(err.to_string(), message, "{name}");
        }
        assert_eq!(projects::list(&conn).unwrap().len(), 1);
        assert_eq!(backups.list().unwrap().len(), 1, "no se creó copia previa");
    }

    #[test]
    fn a_backup_from_a_newer_app_is_refused() {
        let tmp = TempDir::new();
        let conn = open(&tmp);
        let backups = Backups::new(tmp.path());
        let saved = backups.create_manual(&conn, at(9, 0, 0)).unwrap();
        let path = backups.dir().join(&saved.file_name);
        Connection::open(&path)
            .unwrap()
            .pragma_update(None, "user_version", db::latest_version() + 1)
            .unwrap();
        assert_eq!(
            Backups::check(&path).unwrap_err().to_string(),
            "El respaldo es de una versión más nueva de Qapture. Actualiza la app para restaurarlo."
        );
    }
}
```

Run: `cd src-tauri && cargo test backup::`
Expected: FAIL al compilar.

- [ ] **Step 5: Implementar `Backups`**

Agrega al inicio de `src-tauri/src/backup.rs`:

```rust
//! UC-20: respaldo diario de la base (se conservan 7), respaldo manual y restauración.
//! Las copias se hacen con la API de backup de SQLite, que da una copia consistente aunque
//! la base esté en uso, y se escriben en un temporal que luego se renombra.

use std::{
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

use chrono::{DateTime, Local, NaiveDate, NaiveDateTime};
use rusqlite::{Connection, MAIN_DB, OpenFlags, backup::Progress};

use crate::{
    db::{self, new_id},
    dto::{BackupDto, BackupKind, RestoreResultDto},
    error::{AppError, AppResult, validation},
    media::storage::EvidenceFiles,
    repo::evidence,
};

pub const BACKUP_DIR: &str = "backups";
pub const KEEP_DAILY: usize = 7;
const DAILY: &str = "qapture-diario-";
const MANUAL: &str = "qapture-manual";
const BEFORE_RESTORE: &str = "qapture-antes-de-restaurar";

fn not_found() -> AppError {
    AppError::NotFound("El respaldo no existe.".into())
}

fn damaged() -> AppError {
    validation("El respaldo está dañado y no se puede restaurar.")
}

/// Tipo de respaldo según su nombre; `None` para cualquier otro archivo de la carpeta.
fn kind_of(name: &str) -> Option<BackupKind> {
    let stem = name.strip_suffix(".db")?;
    if stem.starts_with(DAILY) {
        Some(BackupKind::Daily)
    } else if stem.starts_with(&format!("{MANUAL}-")) {
        Some(BackupKind::Manual)
    } else if stem.starts_with(&format!("{BEFORE_RESTORE}-")) {
        Some(BackupKind::BeforeRestore)
    } else {
        // `qapture-v<N>-…`: los deja db::open antes de migrar (F1).
        let (version, _) = stem.strip_prefix("qapture-v")?.split_once('-')?;
        (!version.is_empty() && version.bytes().all(|b| b.is_ascii_digit()))
            .then_some(BackupKind::BeforeMigration)
    }
}

/// La copia hereda de la base viva la marca de WAL en la cabecera. Pasarla a `DELETE` la deja
/// en un solo archivo (sin `-wal`/`-shm`): se abre en solo lectura y se copia tal cual al recuperar.
fn standalone(path: &Path) -> AppResult<()> {
    let conn = Connection::open(path)?;
    conn.query_row("PRAGMA journal_mode = DELETE", [], |_| Ok(()))?;
    Ok(())
}

pub struct Backups {
    dir: PathBuf,
}

impl Backups {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            dir: data_dir.join(BACKUP_DIR),
        }
    }

    #[cfg(test)]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn write(&self, conn: &Connection, name: &str) -> AppResult<PathBuf> {
        fs::create_dir_all(&self.dir)?;
        let dest = self.dir.join(name);
        let tmp = self.dir.join(format!("{name}.{}.tmp", new_id()));
        let result = conn
            .backup(MAIN_DB, &tmp, None)
            .map_err(AppError::from)
            .and_then(|()| standalone(&tmp))
            .and_then(|()| Ok(fs::rename(&tmp, &dest)?));
        if let Err(error) = result {
            let _ = fs::remove_file(&tmp);
            return Err(error);
        }
        Ok(dest)
    }

    /// `<prefijo>-AAAAMMDD-HHMMSS.db`, con `-2`, `-3`… si ya existe uno en el mismo segundo.
    fn stamped(&self, prefix: &str, now: NaiveDateTime) -> String {
        let base = format!("{prefix}-{}", now.format("%Y%m%d-%H%M%S"));
        let mut name = format!("{base}.db");
        let mut n = 2;
        while self.dir.join(&name).exists() {
            name = format!("{base}-{n}.db");
            n += 1;
        }
        name
    }

    fn describe(&self, path: &Path) -> AppResult<BackupDto> {
        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let kind = kind_of(&file_name).ok_or_else(not_found)?;
        let meta = fs::metadata(path)?;
        let modified: DateTime<Local> = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH).into();
        Ok(BackupDto {
            file_name,
            kind,
            created_at: modified.format("%Y-%m-%dT%H:%M:%S").to_string(),
            size_bytes: meta.len() as i64,
        })
    }

    fn names(&self) -> AppResult<Vec<String>> {
        if !self.dir.exists() {
            return Ok(Vec::new());
        }
        let mut names = Vec::new();
        for entry in fs::read_dir(&self.dir)? {
            let name = entry?.file_name().to_string_lossy().into_owned();
            if kind_of(&name).is_some() {
                names.push(name);
            }
        }
        Ok(names)
    }

    /// Una copia por día (fecha local). Solo rotan los diarios: quedan los 7 más recientes.
    pub fn ensure_daily(&self, conn: &Connection, today: NaiveDate) -> AppResult<Option<BackupDto>> {
        let name = format!("{DAILY}{}.db", today.format("%Y%m%d"));
        if self.dir.join(&name).exists() {
            return Ok(None);
        }
        let path = self.write(conn, &name)?;
        let mut daily: Vec<String> = self
            .names()?
            .into_iter()
            .filter(|n| kind_of(n) == Some(BackupKind::Daily))
            .collect();
        daily.sort();
        let excess = daily.len().saturating_sub(KEEP_DAILY);
        for old in &daily[..excess] {
            fs::remove_file(self.dir.join(old))?;
        }
        Ok(Some(self.describe(&path)?))
    }

    pub fn create_manual(&self, conn: &Connection, now: NaiveDateTime) -> AppResult<BackupDto> {
        let path = self.write(conn, &self.stamped(MANUAL, now))?;
        self.describe(&path)
    }

    /// Del más reciente al más antiguo.
    pub fn list(&self) -> AppResult<Vec<BackupDto>> {
        let mut list = self
            .names()?
            .into_iter()
            .map(|name| self.describe(&self.dir.join(name)))
            .collect::<AppResult<Vec<_>>>()?;
        list.sort_by(|a, b| {
            b.created_at
                .cmp(&a.created_at)
                .then_with(|| b.file_name.cmp(&a.file_name))
        });
        Ok(list)
    }

    /// Solo nombres simples de la carpeta `backups/` con forma de respaldo: el nombre llega del webview.
    pub fn resolve(&self, file_name: &str) -> AppResult<PathBuf> {
        let plain = !file_name.contains(['/', '\\']) && !file_name.contains("..");
        if !plain || kind_of(file_name).is_none() {
            return Err(not_found());
        }
        let path = self.dir.join(file_name);
        if path.is_file() { Ok(path) } else { Err(not_found()) }
    }

    /// Un respaldo restaurable: base SQLite íntegra de Qapture y no más nueva que esta app.
    pub fn check(path: &Path) -> AppResult<()> {
        let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|_| damaged())?;
        let integrity: String = conn
            .query_row("PRAGMA quick_check", [], |r| r.get(0))
            .map_err(|_| damaged())?;
        let has_projects: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'project')",
                [],
                |r| r.get(0),
            )
            .map_err(|_| damaged())?;
        let version = db::schema_version(&conn).map_err(|_| damaged())?;
        if integrity != "ok" || !has_projects || version < 1 {
            return Err(damaged());
        }
        if version > db::latest_version() {
            return Err(validation(
                "El respaldo es de una versión más nueva de Qapture. Actualiza la app para restaurarlo.",
            ));
        }
        Ok(())
    }

    /// Vuelve al estado del respaldo sobre la conexión viva. Antes guarda una copia del estado
    /// actual y, al final, aparta las capturas que el respaldo no conoce.
    pub fn restore(
        &self,
        conn: &mut Connection,
        files: &EvidenceFiles,
        file_name: &str,
        now: NaiveDateTime,
    ) -> AppResult<RestoreResultDto> {
        let source = self.resolve(file_name)?;
        Self::check(&source)?;
        let restored = self.describe(&source)?;
        let safety = self.write(conn, &self.stamped(BEFORE_RESTORE, now))?;
        conn.restore(MAIN_DB, &source, None::<fn(Progress)>)?;
        db::prepare(conn)?;
        let (referenced, projects) = evidence::references(conn)?;
        let stamp = now.format("%Y%m%d-%H%M%S").to_string();
        let set_aside_files = files.set_aside(&referenced, &projects, &stamp)? as i64;
        Ok(RestoreResultDto {
            restored,
            safety_copy: self.describe(&safety)?.file_name,
            missing_images: evidence::missing_images(conn, files)?,
            set_aside_files,
        })
    }
}
```

Como el módulo todavía no se usa fuera de sus tests, agrega arriba de `mod backup;` en `lib.rs` el atributo `#[allow(dead_code)] // Lo usan los comandos de la Task 8, que retira este allow.`

- [ ] **Step 6: Ejecutar los tests**

Run: `cd src-tauri && cargo test backup:: && cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check`
Expected: PASS. Si `restore_returns_to_the_backup_and_keeps_a_safety_copy` falla porque la conexión WAL tiene lectores abiertos, revisa que ningún `Statement` del test siga vivo antes de `restore` (las variables `stmt` deben quedar en su propio bloque).

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/backup.rs src-tauri/src/lib.rs src-tauri/src/dto.rs src-tauri/src/db/mod.rs src-tauri/src/media/storage.rs src-tauri/src/repo/evidence.rs
git commit -m "feat(f5): respaldo diario con rotación de 7, manual y restauración con copia previa

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 7: Modo recuperación al arrancar con la base dañada (`db/recovery.rs`, `AppState`)

**Files:**
- Create: `src-tauri/src/db/recovery.rs`
- Modify: `src-tauri/src/db/mod.rs`, `src-tauri/src/error.rs`, `src-tauri/src/state.rs`, `src-tauri/src/lib.rs`

**Interfaces:**
- Consumes: `Backups::{resolve, check}` (Task 6), `EvidenceFiles::set_aside`, `repo::evidence::references`.
- Produces:
  - `AppError::Corrupt(String)`: código `corrupt`, mensaje `La base de datos de Qapture está dañada.`
  - `db::check_integrity(&Connection) -> AppResult<()>` (`PRAGMA quick_check`). `db::open` lo ejecuta antes de migrar. `db::open_in_memory()` deja de ser solo de tests.
  - `db::recovery::{Startup::{Ready(Connection), Recovery(String)}, is_corruption(&AppError) -> bool, open_or_recover(path) -> AppResult<Startup>, set_aside_corrupt(path, now) -> AppResult<Option<PathBuf>>, recover(path, backups, files, backup: Option<&str>, now) -> AppResult<Connection>}`.
  - `AppState::new(conn, files, data_dir: &Path, recovery: Option<String>)`. Métodos nuevos: `backups()`, `db_path()`, `recovery() -> Option<String>`, `ensure_ready() -> AppResult<()>`, `finish_recovery(conn)`.

- [ ] **Step 1: Tests (fallan)**

Agrega `pub mod recovery;` en `src-tauri/src/db/mod.rs` (después de `pub mod seed;`). Crea `src-tauri/src/db/recovery.rs` solo con los tests:

```rust
#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;
    use crate::{
        media::storage::{TempDir, UNUSED_DIR},
        repo::{projects, test_support},
    };

    const GARBAGE: &[u8] = b"esto no es una base de datos de Qapture ";

    fn now() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 10, 16).unwrap().and_hms_opt(10, 0, 0).unwrap()
    }

    fn db_path(tmp: &TempDir) -> PathBuf {
        tmp.path().join(db::DB_FILE)
    }

    #[test]
    fn a_healthy_db_opens_normally() {
        let tmp = TempDir::new();
        assert!(matches!(open_or_recover(&db_path(&tmp)).unwrap(), Startup::Ready(_)));
    }

    #[test]
    fn a_file_that_is_not_a_database_opens_in_recovery_and_is_kept() {
        let tmp = TempDir::new();
        let garbage = GARBAGE.repeat(200);
        fs::write(db_path(&tmp), &garbage).unwrap();
        let Startup::Recovery(detail) = open_or_recover(&db_path(&tmp)).unwrap() else {
            panic!("debía entrar en modo recuperación");
        };
        assert!(!detail.is_empty());
        assert_eq!(fs::read(db_path(&tmp)).unwrap(), garbage, "no se toca la base dañada");
    }

    #[test]
    fn damaged_pages_are_detected_by_quick_check() {
        let tmp = TempDir::new();
        {
            let conn = db::open(&db_path(&tmp)).unwrap();
            conn.execute_batch("CREATE TABLE relleno (x TEXT);").unwrap();
            for _ in 0..300 {
                conn.execute("INSERT INTO relleno (x) VALUES (?1)", ["x".repeat(1000)])
                    .unwrap();
            }
        }
        let mut bytes = fs::read(db_path(&tmp)).unwrap();
        let len = bytes.len();
        bytes[len - 3 * 4096..].fill(0xFF);
        fs::write(db_path(&tmp), bytes).unwrap();
        assert!(matches!(open_or_recover(&db_path(&tmp)).unwrap(), Startup::Recovery(_)));
    }

    #[test]
    fn is_corruption_only_matches_damaged_databases() {
        assert!(is_corruption(&AppError::Corrupt("x".into())));
        assert!(!is_corruption(&AppError::Validation("x".into())));
        assert!(!is_corruption(&AppError::Io(std::io::Error::other("disco lleno"))));
    }

    #[test]
    fn recover_with_a_backup_sets_the_damaged_file_aside() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let backups = Backups::new(tmp.path());
        let saved = {
            let conn = db::open(&db_path(&tmp)).unwrap();
            test_support::project(&conn);
            backups.create_manual(&conn, now()).unwrap()
        };
        fs::write(db_path(&tmp), GARBAGE.repeat(200)).unwrap();

        let conn = recover(&db_path(&tmp), &backups, &files, Some(&saved.file_name), now()).unwrap();
        let names: Vec<String> = projects::list(&conn).unwrap().into_iter().map(|p| p.name).collect();
        assert_eq!(names, ["Recibos"]);
        let aside = tmp.path().join("qapture-danada-20261016-100000.db");
        assert_eq!(fs::read(aside).unwrap(), GARBAGE.repeat(200), "la base dañada se conserva");
    }

    #[test]
    fn recover_without_a_backup_starts_empty_and_keeps_the_images_aside() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let image = files.absolute(&EvidenceFiles::original_rel("p1", "e1"));
        fs::create_dir_all(image.parent().unwrap()).unwrap();
        fs::write(&image, b"png").unwrap();
        fs::write(db_path(&tmp), GARBAGE.repeat(200)).unwrap();

        let conn = recover(&db_path(&tmp), &Backups::new(tmp.path()), &files, None, now()).unwrap();
        assert!(projects::list(&conn).unwrap().is_empty());
        assert_eq!(db::schema_version(&conn).unwrap(), db::latest_version());
        let aside = tmp.path().join(UNUSED_DIR).join("20261016-100000").join("p1").join("e1.orig.png");
        assert_eq!(fs::read(aside).unwrap(), b"png", "las capturas no se pierden");
    }

    #[test]
    fn a_damaged_backup_is_refused_before_touching_the_current_file() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let backups = Backups::new(tmp.path());
        fs::create_dir_all(backups.dir()).unwrap();
        fs::write(backups.dir().join("qapture-manual-20261016-080000.db"), b"roto").unwrap();
        fs::write(db_path(&tmp), GARBAGE.repeat(200)).unwrap();
        let err = recover(&db_path(&tmp), &backups, &files, Some("qapture-manual-20261016-080000.db"), now())
            .unwrap_err();
        assert_eq!(err.to_string(), "El respaldo está dañado y no se puede restaurar.");
        assert_eq!(fs::read(db_path(&tmp)).unwrap(), GARBAGE.repeat(200));
        assert!(!tmp.path().join("qapture-danada-20261016-100000.db").exists());
    }
}
```

En `src-tauri/src/state.rs`, agrega al final:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{db, media::storage::TempDir};

    #[test]
    fn recovery_blocks_data_commands_until_a_new_db_is_installed() {
        let tmp = TempDir::new();
        let state = AppState::new(
            db::open_in_memory().unwrap(),
            tmp.files(),
            tmp.path(),
            Some("base dañada".into()),
        );
        assert_eq!(state.recovery().as_deref(), Some("base dañada"));
        assert_eq!(
            state.ensure_ready().unwrap_err().to_string(),
            "Qapture está en modo recuperación: restaura un respaldo o empieza con una base vacía."
        );
        assert_eq!(state.db_path(), tmp.path().join(db::DB_FILE));
        let fresh = db::open(&tmp.path().join(db::DB_FILE)).unwrap();
        fresh
            .execute("INSERT INTO project (id, name, created_at, updated_at) VALUES ('p', 'Nueva', '', '')", [])
            .unwrap();
        state.finish_recovery(fresh);
        assert!(state.recovery().is_none());
        assert!(state.ensure_ready().is_ok());
        let count: i64 = state.db().query_row("SELECT COUNT(*) FROM project", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1, "los comandos usan la base nueva");
    }
}
```

Run: `cd src-tauri && cargo test db::recovery && cargo test state::`
Expected: FAIL al compilar.

- [ ] **Step 2: `AppError::Corrupt` y `check_integrity`**

En `src-tauri/src/error.rs`, agrega la variante (después de `Export`):

```rust
    #[error("base dañada: {0}")]
    Corrupt(String),
```

En `code()`: `Self::Corrupt(_) => "corrupt",`. En `user_message()`: `Self::Corrupt(_) => "La base de datos de Qapture está dañada.".into(),` (antes del `_ =>`).

En `src-tauri/src/db/mod.rs`:
- Quita `#[cfg(test)]` de `open_in_memory` (ahora lo usa el arranque en modo recuperación).
- Agrega:

```rust
/// `PRAGMA quick_check`: una base con páginas dañadas no se migra ni se usa (spec §8).
pub fn check_integrity(conn: &Connection) -> AppResult<()> {
    let mut stmt = conn.prepare("PRAGMA quick_check")?;
    let rows = stmt
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    if rows.len() == 1 && rows[0] == "ok" {
        Ok(())
    } else {
        Err(AppError::Corrupt(rows.join("; ")))
    }
}
```

- En `open`, después de `PRAGMA journal_mode = WAL`, agrega `check_integrity(&conn)?;`. Importa `AppError` (`use crate::error::{AppError, AppResult};`).

- [ ] **Step 3: Implementar `db::recovery`**

Agrega al inicio de `src-tauri/src/db/recovery.rs`:

```rust
//! Spec §8: si la base no abre por corrupción, la app arranca en modo recuperación y ofrece
//! restaurar un respaldo o empezar con una base vacía. La base dañada nunca se borra.

use std::{
    fs,
    path::{Path, PathBuf},
};

use chrono::NaiveDateTime;
use rusqlite::{Connection, ErrorCode};

use crate::{
    backup::Backups,
    db,
    error::{AppError, AppResult},
    media::storage::EvidenceFiles,
    repo::evidence,
};

pub enum Startup {
    Ready(Connection),
    /// Detalle técnico del fallo, para el registro y la pantalla de recuperación.
    Recovery(String),
}

pub fn is_corruption(error: &AppError) -> bool {
    match error {
        AppError::Corrupt(_) => true,
        AppError::Db(rusqlite::Error::SqliteFailure(failure, _)) => matches!(
            failure.code,
            ErrorCode::DatabaseCorrupt | ErrorCode::NotADatabase
        ),
        _ => false,
    }
}

/// Otros errores (permisos, disco) siguen impidiendo el arranque, como antes de F5.
pub fn open_or_recover(path: &Path) -> AppResult<Startup> {
    match db::open(path) {
        Ok(conn) => Ok(Startup::Ready(conn)),
        Err(error) if is_corruption(&error) => Ok(Startup::Recovery(error.to_string())),
        Err(error) => Err(error),
    }
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut raw = path.as_os_str().to_owned();
    raw.push(suffix);
    PathBuf::from(raw)
}

/// Renombra la base (y sus `-wal`/`-shm`) a `qapture-danada-<fecha>.db` en la misma carpeta.
pub fn set_aside_corrupt(path: &Path, now: NaiveDateTime) -> AppResult<Option<PathBuf>> {
    if !path.exists() {
        return Ok(None);
    }
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let target = dir.join(format!("qapture-danada-{}.db", now.format("%Y%m%d-%H%M%S")));
    fs::rename(path, &target)?;
    for suffix in ["-wal", "-shm"] {
        let side = with_suffix(path, suffix);
        if side.exists() {
            let _ = fs::rename(&side, with_suffix(&target, suffix));
        }
    }
    Ok(Some(target))
}

/// Sale del modo recuperación con una base nueva en `path`: la copia de `backup` o, sin él,
/// una vacía. El respaldo se valida antes de mover nada. Las capturas que la base nueva no
/// conoce se apartan en `evidence-sin-uso/` (la limpieza del arranque las borraría).
pub fn recover(
    path: &Path,
    backups: &Backups,
    files: &EvidenceFiles,
    backup: Option<&str>,
    now: NaiveDateTime,
) -> AppResult<Connection> {
    let source = match backup {
        Some(name) => {
            let source = backups.resolve(name)?;
            Backups::check(&source)?;
            Some(source)
        }
        None => None,
    };
    set_aside_corrupt(path, now)?;
    if let Some(source) = &source {
        fs::copy(source, path)?;
    }
    let conn = db::open(path)?;
    let (referenced, projects) = evidence::references(&conn)?;
    files.set_aside(&referenced, &projects, &now.format("%Y%m%d-%H%M%S").to_string())?;
    Ok(conn)
}
```

- [ ] **Step 4: `AppState` con modo recuperación**

Reemplaza `src-tauri/src/state.rs` (manteniendo el módulo de tests del Step 1 al final):

```rust
use std::{
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard},
};

use rusqlite::Connection;

use crate::{
    backup::Backups,
    capture::session::{CaptureController, HIDE_DELAY, READY_TIMEOUT},
    db::DB_FILE,
    error::{AppResult, validation},
    excel::save::RecentExports,
    media::storage::EvidenceFiles,
};

pub struct AppState {
    db: Mutex<Connection>,
    files: EvidenceFiles,
    capture: CaptureController,
    exports: RecentExports,
    backups: Backups,
    db_path: PathBuf,
    /// `Some(detalle)` mientras la base está dañada y la app usa una base vacía en memoria.
    recovery: Mutex<Option<String>>,
}

impl AppState {
    pub fn new(
        conn: Connection,
        files: EvidenceFiles,
        data_dir: &Path,
        recovery: Option<String>,
    ) -> Self {
        Self {
            db: Mutex::new(conn),
            files,
            capture: CaptureController::new(HIDE_DELAY, Some(READY_TIMEOUT)),
            exports: RecentExports::default(),
            backups: Backups::new(data_dir),
            db_path: data_dir.join(DB_FILE),
            recovery: Mutex::new(recovery),
        }
    }

    /// Una sola conexión serializada: SQLite + WAL es suficiente para una app de escritorio de un usuario.
    pub fn db(&self) -> MutexGuard<'_, Connection> {
        self.db
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn capture(&self) -> &CaptureController {
        &self.capture
    }

    pub fn files(&self) -> &EvidenceFiles {
        &self.files
    }

    pub fn exports(&self) -> &RecentExports {
        &self.exports
    }

    pub fn backups(&self) -> &Backups {
        &self.backups
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    pub fn recovery(&self) -> Option<String> {
        self.recovery
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// Importar, exportar paquetes y respaldar no corren sobre la base vacía del modo recuperación.
    pub fn ensure_ready(&self) -> AppResult<()> {
        match self.recovery() {
            Some(_) => Err(validation(
                "Qapture está en modo recuperación: restaura un respaldo o empieza con una base vacía.",
            )),
            None => Ok(()),
        }
    }

    pub fn finish_recovery(&self, conn: Connection) {
        *self.db() = conn;
        *self
            .recovery
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
    }
}
```

- [ ] **Step 5: Arranque con modo recuperación**

En `src-tauri/src/lib.rs`, reemplaza el cuerpo de `.setup(|app| { … })` por:

```rust
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let files = media::storage::EvidenceFiles::new(&data_dir);
            let state = match db::recovery::open_or_recover(&data_dir.join(db::DB_FILE))? {
                db::recovery::Startup::Ready(conn) => {
                    // Restos de sesiones anteriores: capturas sin guardar y archivos sin fila en la db.
                    if let Err(error) = files.clear_staging() {
                        eprintln!("No se pudo limpiar evidence/_staging: {error}");
                    }
                    if let Err(error) = repo::evidence::sweep_orphans(&conn, &files) {
                        eprintln!("No se pudieron limpiar evidencias huérfanas: {error}");
                    }
                    state::AppState::new(conn, files, &data_dir, None)
                }
                db::recovery::Startup::Recovery(detail) => {
                    // Nada de limpiezas ni respaldos: la base vacía en memoria no refleja los datos reales.
                    eprintln!("Base de datos dañada, se abre el modo recuperación: {detail}");
                    state::AppState::new(db::open_in_memory()?, files, &data_dir, Some(detail))
                }
            };
            app.manage(state);
            system::tray::setup_tray(app)?;
            Ok(())
        })
```

Deja el `#[allow(dead_code)]` de `mod backup;`: `create_manual`, `list` y `restore` solo los usan los comandos, y la Task 8 retira ese `allow`. Por la misma razón, hasta la Task 8 no tienen uso fuera de los tests:
- `db::recovery::recover`: agrega `#[allow(dead_code)] // Lo usa recover_database (Task 8).` encima de su `pub fn`;
- los métodos `backups`, `db_path`, `recovery`, `ensure_ready` y `finish_recovery` de `AppState`: agrega el mismo atributo, con su comentario, encima de `impl AppState`.

La Task 8 retira los tres `allow`.

- [ ] **Step 6: Ejecutar los tests**

Run: `cd src-tauri && cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check`
Expected: PASS, incluidos los tests de `db` de F1 (`open` ahora también ejecuta `quick_check`).

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/db src-tauri/src/error.rs src-tauri/src/state.rs src-tauri/src/lib.rs
git commit -m "feat(f5): modo recuperación al arrancar con la base dañada, sin limpiezas ni respaldos

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---
### Task 8: Comandos de Datos, ACL, contrato y respaldo diario programado

**Files:**
- Create: `src-tauri/src/commands/data.rs`, `src-tauri/src/system/daily_backup.rs`
- Modify: `src-tauri/src/commands/mod.rs`, `src-tauri/src/commands/names.rs`, `src-tauri/capabilities/main.json`, `src-tauri/src/lib.rs`, `src-tauri/src/system/mod.rs`, `src-tauri/src/dto.rs`, `src-tauri/src/transfer/mod.rs` (retira el `allow`), `src/infrastructure/tauri/__fixtures__/contracts.json` (regenerado)

**Interfaces:**
- Consumes: `transfer::{legacy::import, package::{collect, write, import, suggested_file_name, with_package_extension}}`, `Backups`, `db::recovery::recover`, `AppState::{ensure_ready, recovery, finish_recovery, backups, db_path, exports}`.
- Produces:
  - `dto::PackageExportResultDto { path, file_name, cases, evidence, missing_images }` y `dto::StartupStatusDto { recovery: bool, detail: String }`.
  - Comandos (JS, argumentos camelCase). `null` significa que el usuario cerró el diálogo de archivo:
    - `get_startup_status() -> StartupStatusDto`
    - `import_legacy() -> ImportSummaryDto | null`
    - `import_package() -> ImportSummaryDto | null`
    - `export_package({ projectId }) -> PackageExportResultDto | null`
    - `list_backups() -> BackupDto[]`, `create_backup() -> BackupDto`
    - `restore_backup({ fileName }) -> RestoreResultDto`
    - `recover_database({ fileName: string | null }) -> null`
  - `system::daily_backup::spawn(AppHandle)`: un respaldo al arrancar y una revisión cada hora (nunca en modo recuperación).
  - Claves nuevas en el fixture de contrato: `importSummary`, `packageExportResult`, `backup`, `restoreResult`, `startupStatus`, `corruptError`.

- [ ] **Step 1: DTOs y fixture de contrato (fallan)**

En `src-tauri/src/dto.rs`, después de `RestoreResultDto`:

```rust
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageExportResultDto {
    pub path: String,
    pub file_name: String,
    pub cases: i64,
    pub evidence: i64,
    pub missing_images: i64,
}

/// Lo primero que pide el front: si la base está dañada, muestra la pantalla de recuperación.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupStatusDto {
    pub recovery: bool,
    pub detail: String,
}
```

En el módulo `contract` de `dto.rs`, agrega después de `export_samples`:

```rust
    /// F5: en un objeto aparte para no superar el límite de recursión de `json!`.
    fn data_samples() -> Value {
        let backup = BackupDto {
            file_name: "qapture-diario-20261016.db".into(),
            kind: BackupKind::Daily,
            created_at: "2026-10-16T09:00:00".into(),
            size_bytes: 98_304,
        };
        json!({
            "importSummary": ImportSummaryDto {
                projects: vec![ImportedProjectDto { id: "prj-2".into(), name: "Aplicación de recibos".into() }],
                cases: 3, evidence: 2,
            },
            "packageExportResult": PackageExportResultDto {
                path: "C:\\Users\\ana\\Documents\\T4S_ACME_Recibos_20261016.qapture".into(),
                file_name: "T4S_ACME_Recibos_20261016.qapture".into(),
                cases: 10, evidence: 20, missing_images: 0,
            },
            "backup": backup.clone(),
            "restoreResult": RestoreResultDto {
                restored: backup,
                safety_copy: "qapture-antes-de-restaurar-20261016-100000.db".into(),
                missing_images: 0, set_aside_files: 2,
            },
            "startupStatus": StartupStatusDto { recovery: true, detail: "base dañada: database disk image is malformed".into() },
            "corruptError": AppError::Corrupt("database disk image is malformed".into()),
        })
    }
```

y reemplaza `sample()` por:

```rust
    fn sample() -> Value {
        let mut sample = base_sample();
        if let Some(target) = sample.as_object_mut() {
            for extra in [export_samples(), data_samples()] {
                if let Value::Object(extra) = extra {
                    target.extend(extra);
                }
            }
        }
        sample
    }
```

Run: `cd src-tauri && cargo test contract`
Expected: FAIL en `dto_contract_matches_fixture` («El contrato cambió»).

- [ ] **Step 2: Regenerar el fixture**

Run: `cd src-tauri && UPDATE_CONTRACTS=1 cargo test contract && cargo test contract`
Expected: PASS. `git diff --stat` muestra `src/infrastructure/tauri/__fixtures__/contracts.json` con las 6 claves nuevas.

- [ ] **Step 3: Registrar los comandos en la ACL (falla)**

En `src-tauri/src/commands/names.rs`, agrega al final de `APP_COMMANDS`:

```rust
    "get_startup_status",
    "import_legacy",
    "import_package",
    "export_package",
    "list_backups",
    "create_backup",
    "restore_backup",
    "recover_database",
```

Run: `cd src-tauri && cargo test names`
Expected: FAIL (`main_window_gets_core_and_every_non_overlay_command` y `every_registered_handler_is_listed_and_vice_versa`).

- [ ] **Step 4: Implementar los comandos**

Crea `src-tauri/src/commands/data.rs`:

```rust
use std::{fs, path::PathBuf};

use chrono::{Local, NaiveDateTime};
use tauri::{State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;

use crate::{
    db::recovery,
    dto::{BackupDto, ImportSummaryDto, PackageExportResultDto, RestoreResultDto, StartupStatusDto},
    error::{AppResult, validation},
    state::AppState,
    transfer::{legacy, package},
};

fn now() -> NaiveDateTime {
    Local::now().naive_local()
}

/// `None`: el usuario cerró el diálogo sin elegir archivo.
fn pick_file(window: &WebviewWindow, title: &str, filter: &str, extension: &str) -> Option<PathBuf> {
    window
        .dialog()
        .file()
        .set_parent(window)
        .set_title(title)
        .add_filter(filter, &[extension])
        .blocking_pick_file()?
        .into_path()
        .ok()
}

#[tauri::command(async)]
pub fn get_startup_status(state: State<'_, AppState>) -> AppResult<StartupStatusDto> {
    let detail = state.recovery();
    Ok(StartupStatusDto {
        recovery: detail.is_some(),
        detail: detail.unwrap_or_default(),
    })
}

#[tauri::command(async)]
pub fn import_legacy(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> AppResult<Option<ImportSummaryDto>> {
    state.ensure_ready()?;
    let Some(path) = pick_file(&window, "Importar Bitácora v1", "Respaldo de la Bitácora v1", "json")
    else {
        return Ok(None);
    };
    if fs::metadata(&path)?.len() > legacy::MAX_FILE_BYTES {
        return Err(validation("El archivo supera 200 MB."));
    }
    let bytes = fs::read(&path)?;
    legacy::import(&state.db(), state.files(), &bytes).map(Some)
}

#[tauri::command(async)]
pub fn import_package(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> AppResult<Option<ImportSummaryDto>> {
    state.ensure_ready()?;
    let Some(path) = pick_file(&window, "Importar paquete", "Paquete de Qapture", package::EXTENSION)
    else {
        return Ok(None);
    };
    package::import(&state.db(), state.files(), &path).map(Some)
}

#[tauri::command(async)]
pub fn export_package(
    window: WebviewWindow,
    state: State<'_, AppState>,
    project_id: String,
) -> AppResult<Option<PackageExportResultDto>> {
    state.ensure_ready()?;
    // La base se lee de una vez y se suelta antes del diálogo y de escribir el ZIP.
    let export = package::collect(&state.db(), state.files(), &project_id)?;
    let name = package::suggested_file_name(
        &export.doc.project.client,
        &export.doc.project.name,
        Local::now().date_naive(),
    );
    let Some(dest) = window
        .dialog()
        .file()
        .set_parent(&window)
        .set_title("Exportar paquete")
        .set_file_name(&name)
        .add_filter("Paquete de Qapture", &[package::EXTENSION])
        .blocking_save_file()
        .and_then(|p| p.into_path().ok())
    else {
        return Ok(None);
    };
    let dest = package::with_package_extension(dest);
    package::write(&export, &dest)?;
    // «Abrir carpeta» reutiliza `reveal_export`, que solo acepta rutas exportadas en la sesión.
    state.exports().remember(&dest);
    Ok(Some(PackageExportResultDto {
        path: dest.to_string_lossy().into_owned(),
        file_name: dest
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        cases: export.doc.cases.len() as i64,
        evidence: export.evidence_count(),
        missing_images: export.missing_images,
    }))
}

/// También en modo recuperación: la pantalla de recuperación lista los respaldos.
#[tauri::command(async)]
pub fn list_backups(state: State<'_, AppState>) -> AppResult<Vec<BackupDto>> {
    state.backups().list()
}

#[tauri::command(async)]
pub fn create_backup(state: State<'_, AppState>) -> AppResult<BackupDto> {
    state.ensure_ready()?;
    state.backups().create_manual(&state.db(), now())
}

#[tauri::command(async)]
pub fn restore_backup(state: State<'_, AppState>, file_name: String) -> AppResult<RestoreResultDto> {
    state.ensure_ready()?;
    let mut db = state.db();
    state.backups().restore(&mut db, state.files(), &file_name, now())
}

/// Sale del modo recuperación: `fileName` restaura ese respaldo; `null` empieza con una base vacía.
#[tauri::command(async)]
pub fn recover_database(state: State<'_, AppState>, file_name: Option<String>) -> AppResult<()> {
    if state.recovery().is_none() {
        return Err(validation("Qapture no está en modo recuperación."));
    }
    let conn = recovery::recover(
        state.db_path(),
        state.backups(),
        state.files(),
        file_name.as_deref(),
        now(),
    )?;
    state.finish_recovery(conn);
    Ok(())
}
```

En `src-tauri/src/commands/mod.rs`, agrega `pub mod data;` (orden alfabético, después de `pub mod cases;`).

En `src-tauri/src/lib.rs`, dentro de `generate_handler![…]`, después de `commands::export::reveal_export,`:

```rust
            commands::data::get_startup_status,
            commands::data::import_legacy,
            commands::data::import_package,
            commands::data::export_package,
            commands::data::list_backups,
            commands::data::create_backup,
            commands::data::restore_backup,
            commands::data::recover_database,
```

En `src-tauri/capabilities/main.json`, agrega al final de `permissions`:

```json
    "allow-get-startup-status",
    "allow-import-legacy",
    "allow-import-package",
    "allow-export-package",
    "allow-list-backups",
    "allow-create-backup",
    "allow-restore-backup",
    "allow-recover-database"
```

(con la coma correspondiente después de `"allow-reveal-export"`).

- [ ] **Step 5: Respaldo diario programado (test primero)**

En `src-tauri/src/system/mod.rs`, agrega `pub mod daily_backup;`. Crea `src-tauri/src/system/daily_backup.rs`:

```rust
use std::{thread, time::Duration};

use tauri::{AppHandle, Manager};

use crate::state::AppState;

const CHECK_EVERY: Duration = Duration::from_secs(60 * 60);

/// Spec §5: copia diaria de la base. Se revisa al arrancar y luego cada hora, porque la app
/// puede quedar días abierta en la bandeja.
pub fn spawn(app: AppHandle) {
    thread::spawn(move || {
        loop {
            run_once(&app.state::<AppState>());
            thread::sleep(CHECK_EVERY);
        }
    });
}

pub(crate) fn run_once(state: &AppState) {
    // Nunca respaldar la base vacía del modo recuperación: rotaría las copias buenas.
    if state.recovery().is_some() {
        return;
    }
    let conn = state.db();
    if let Err(error) = state
        .backups()
        .ensure_daily(&conn, chrono::Local::now().date_naive())
    {
        eprintln!("No se pudo crear el respaldo diario: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{db, media::storage::TempDir};

    #[test]
    fn skips_the_daily_backup_in_recovery_mode() {
        let tmp = TempDir::new();
        let state = AppState::new(
            db::open_in_memory().unwrap(),
            tmp.files(),
            tmp.path(),
            Some("base dañada".into()),
        );
        run_once(&state);
        assert!(state.backups().list().unwrap().is_empty());
        state.finish_recovery(db::open(&tmp.path().join(db::DB_FILE)).unwrap());
        run_once(&state);
        run_once(&state);
        assert_eq!(state.backups().list().unwrap().len(), 1, "uno por día");
    }
}
```

En `src-tauri/src/lib.rs`, después de `app.manage(state);`:

```rust
            system::daily_backup::spawn(app.handle().clone());
```

Retira los `allow(dead_code)` temporales, cada uno con su comentario:
- `#![allow(dead_code)]` de `src-tauri/src/transfer/mod.rs` (Task 1);
- el de `mod backup;` en `lib.rs` (Task 6);
- los de `db::recovery::recover` y de `impl AppState` (Task 7).

- [ ] **Step 6: Ejecutar todo**

Run: `cd src-tauri && cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check`
Expected: PASS, sin warnings de código sin uso.

- [ ] **Step 7: Smoke en desarrollo**

Run: `pnpm tauri dev`
Expected:
- arranca normal;
- en `%APPDATA%\com.teams4soft.qapture\backups\` aparece `qapture-diario-<hoy>.db`;
- desde la consola del WebView, `await window.__TAURI_INTERNALS__.invoke("get_startup_status")` devuelve `{ recovery: false, detail: "" }`.

Cierra la app desde la bandeja.

- [ ] **Step 8: Commit**

```bash
git add src-tauri src/infrastructure/tauri/__fixtures__/contracts.json
git commit -m "feat(f5): comandos de importación, paquetes, respaldos y recuperación con ACL y respaldo diario

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 9: Datos en el front — dominio, puerto `DataTransfer`, adaptadores y store

**Files:**
- Create: `src/domain/data.ts`, `src/domain/data.test.ts`, `src/ports/data.ts`, `src/infrastructure/tauri/data.ts`, `src/infrastructure/memory/data.ts`, `src/ui/store/data-slice.ts`, `src/ui/store/data-slice.test.ts`
- Modify: `src/ports/errors.ts`, `src/ports/repositories.ts`, `src/infrastructure/tauri/dto.ts`, `src/infrastructure/tauri/repositories.ts`, `src/infrastructure/tauri/contract.test.ts`, `src/infrastructure/memory/export.ts`, `src/infrastructure/memory/repositories.ts`, `src/ui/store/evidence-slice.ts` (`SliceHelpers.reload`), `src/ui/store/workspace.ts`, `src/ui/test-utils.tsx`

**Interfaces:**
- Consumes: los comandos y el fixture de la Task 8.
- Produces:
  - `domain/data.ts`:
    - Tipos: `ImportedProject`, `ImportSummary`, `PackageExportResult`, `BACKUP_KINDS`, `BackupKind`, `Backup`, `RestoreResult`, `StartupStatus`, `ImportIssues { title, items }`.
    - Textos: `importSummaryMessage`, `parseImportIssues`, `restoreMessage`, `BACKUP_KIND_LABEL`.
    - Formatos: `formatBackupDate`, `formatSize`.
  - `ports/data.ts`: `DataTransfer { startupStatus, importLegacy, importPackage, exportPackage, listBackups, createBackup, restoreBackup, recover }`. `Repositories.data: DataTransfer`.
  - `infrastructure/memory/data.ts`: `createMemoryData(deps) -> MemoryData`.
    - Controles para los tests: `mode: "ok" | "cancel" | "invalid"`, `status`, `backups`, `calls: string[]`.
    - `MemoryExporter.remember(path)`.
  - Store (`DataSlice`):
    - Estado: `recovery: StartupStatus | null`, `backups`, `importIssues`, `dataBusy`.
    - Acciones: `loadBackups()`, `importLegacy()`, `importPackage()`, `exportPackage()`, `createBackup()`, `restoreBackup(fileName) -> Promise<boolean>`, `recover(fileName | null, thenImportPackage?)`, `dismissImportIssues()`.
  - `SliceHelpers.reload(projectId?: string | null)`: recarga listas y abre ese proyecto, el actual o el primero.
  - `renderApp({ setup })`: ahora `setup` recibe `MemoryRepositories`.

- [ ] **Step 1: Dominio (test primero)**

Crea `src/domain/data.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { formatBackupDate, formatSize, importSummaryMessage, parseImportIssues, restoreMessage, type Backup } from "./data";

const backup: Backup = { fileName: "qapture-diario-20261015.db", kind: "daily", createdAt: "2026-10-15T09:00:05", sizeBytes: 98304 };

describe("importSummaryMessage", () => {
  it("usa singular y plural", () => {
    expect(importSummaryMessage({ projects: [{ id: "p1", name: "A" }], cases: 1, evidence: 1 })).toBe("Importado: 1 proyecto, 1 caso y 1 captura.");
    expect(importSummaryMessage({ projects: [{ id: "p1", name: "A" }, { id: "p2", name: "B" }], cases: 3, evidence: 0 })).toBe(
      "Importado: 2 proyectos, 3 casos y 0 capturas.",
    );
  });
});

describe("parseImportIssues", () => {
  it("separa el encabezado de las viñetas", () => {
    const issues = parseImportIssues("No se importó nada. Revisa estos problemas:\n• EJ-001: Fecha inválida.\n• … y 3 problemas más.");
    expect(issues).toEqual({ title: "No se importó nada. Revisa estos problemas:", items: ["EJ-001: Fecha inválida.", "… y 3 problemas más."] });
  });

  it("un mensaje de una línea queda como título", () => {
    expect(parseImportIssues("El archivo no es un paquete de Qapture válido.")).toEqual({ title: "El archivo no es un paquete de Qapture válido.", items: [] });
  });
});

describe("formatos", () => {
  it("fecha del respaldo en formato local", () => {
    expect(formatBackupDate("2026-10-15T09:00:05")).toBe("15/10/2026 09:00");
    expect(formatBackupDate("raro")).toBe("raro");
  });

  it("tamaño legible", () => {
    expect(formatSize(512)).toBe("512 B");
    expect(formatSize(98304)).toBe("96 KB");
    expect(formatSize(1.5 * 1024 * 1024)).toBe("1,5 MB");
  });

  it("mensaje al restaurar", () => {
    const result = { restored: backup, safetyCopy: "qapture-antes-de-restaurar-20261016-100000.db", missingImages: 0, setAsideFiles: 0 };
    expect(restoreMessage(result)).toBe("Se restauró el respaldo del 15/10/2026 09:00.");
    expect(restoreMessage({ ...result, missingImages: 2 })).toBe("Se restauró el respaldo del 15/10/2026 09:00. 2 capturas del respaldo ya no tienen imagen.");
    expect(restoreMessage({ ...result, missingImages: 1 })).toBe("Se restauró el respaldo del 15/10/2026 09:00. 1 captura del respaldo ya no tiene imagen.");
  });
});
```

Run: `pnpm test src/domain/data.test.ts`
Expected: FAIL (no existe `./data`).

Crea `src/domain/data.ts`:

```ts
/** UC-18 / UC-19 / UC-20: importación, paquetes `.qapture` y respaldos. */

export interface ImportedProject {
  id: string;
  name: string;
}

export interface ImportSummary {
  projects: ImportedProject[];
  cases: number;
  evidence: number;
}

export interface PackageExportResult {
  path: string;
  fileName: string;
  cases: number;
  evidence: number;
  missingImages: number;
}

export const BACKUP_KINDS = ["daily", "manual", "before-restore", "before-migration"] as const;
export type BackupKind = (typeof BACKUP_KINDS)[number];

export interface Backup {
  fileName: string;
  kind: BackupKind;
  /** Hora local `AAAA-MM-DDTHH:MM:SS`. */
  createdAt: string;
  sizeBytes: number;
}

export interface RestoreResult {
  restored: Backup;
  safetyCopy: string;
  missingImages: number;
  setAsideFiles: number;
}

export interface StartupStatus {
  recovery: boolean;
  detail: string;
}

export interface ImportIssues {
  title: string;
  items: string[];
}

export const BACKUP_KIND_LABEL: Record<BackupKind, string> = {
  daily: "Diario",
  manual: "Manual",
  "before-restore": "Antes de restaurar",
  "before-migration": "Antes de actualizar",
};

const count = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;

export function importSummaryMessage(summary: ImportSummary): string {
  const projects = count(summary.projects.length, "proyecto", "proyectos");
  return `Importado: ${projects}, ${count(summary.cases, "caso", "casos")} y ${count(summary.evidence, "captura", "capturas")}.`;
}

/** Rust envía «encabezado\n• problema\n• problema…» (máximo 10 + resto). */
export function parseImportIssues(message: string): ImportIssues {
  const [title = "", ...rest] = message.split("\n");
  return { title, items: rest.filter((line) => line.startsWith("• ")).map((line) => line.slice(2)) };
}

/** `2026-10-15T09:00:05` → `15/10/2026 09:00`. */
export function formatBackupDate(iso: string): string {
  const m = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2})/.exec(iso);
  return m ? `${m[3]}/${m[2]}/${m[1]} ${m[4]}:${m[5]}` : iso;
}

export function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const kb = bytes / 1024;
  if (kb < 1024) return `${Math.round(kb)} KB`;
  return `${(kb / 1024).toFixed(1).replace(".", ",")} MB`;
}

export function restoreMessage(result: RestoreResult): string {
  const base = `Se restauró el respaldo del ${formatBackupDate(result.restored.createdAt)}.`;
  const n = result.missingImages;
  if (n === 0) return base;
  return `${base} ${count(n, "captura", "capturas")} del respaldo ya no ${n === 1 ? "tiene" : "tienen"} imagen.`;
}
```

Run: `pnpm test src/domain/data.test.ts`
Expected: PASS.

- [ ] **Step 2: Puerto, esquemas y adaptador Tauri**

En `src/ports/errors.ts`, agrega `"corrupt"` a `AppErrorCode` (después de `"export"`).

Crea `src/ports/data.ts`:

```ts
import type { Backup, ImportSummary, PackageExportResult, RestoreResult, StartupStatus } from "../domain/data";

/** Importar (v1 y paquetes), exportar paquetes y respaldos. Los diálogos de archivo los abre Rust. */
export interface DataTransfer {
  startupStatus(): Promise<StartupStatus>;
  /** `null`: el usuario cerró el diálogo sin elegir archivo. */
  importLegacy(): Promise<ImportSummary | null>;
  importPackage(): Promise<ImportSummary | null>;
  exportPackage(projectId: string): Promise<PackageExportResult | null>;
  listBackups(): Promise<Backup[]>;
  createBackup(): Promise<Backup>;
  restoreBackup(fileName: string): Promise<RestoreResult>;
  /** Sale del modo recuperación con ese respaldo o, con `null`, con una base vacía. */
  recover(fileName: string | null): Promise<void>;
}
```

En `src/ports/repositories.ts`, importa `DataTransfer` y agrega `data: DataTransfer;` a `Repositories`.

En `src/infrastructure/tauri/dto.ts`:
- agrega `"corrupt"` al `z.enum` de `appErrorSchema`;
- importa `BACKUP_KINDS` de `../../domain/data`;
- agrega:

```ts
export const importSummarySchema = z.strictObject({
  projects: z.array(z.strictObject({ id: z.string(), name: z.string() })),
  cases: z.number().int(),
  evidence: z.number().int(),
});

export const packageExportResultSchema = z.strictObject({
  path: z.string(),
  fileName: z.string(),
  cases: z.number().int(),
  evidence: z.number().int(),
  missingImages: z.number().int(),
});

export const backupSchema = z.strictObject({
  fileName: z.string(),
  kind: z.enum(BACKUP_KINDS),
  createdAt: z.string(),
  sizeBytes: z.number().int(),
});

export const restoreResultSchema = z.strictObject({
  restored: backupSchema,
  safetyCopy: z.string(),
  missingImages: z.number().int(),
  setAsideFiles: z.number().int(),
});

export const startupStatusSchema = z.strictObject({
  recovery: z.boolean(),
  detail: z.string(),
});
```

Crea `src/infrastructure/tauri/data.ts`:

```ts
import { z } from "zod";
import type { DataTransfer } from "../../ports/data";
import { call } from "./client";
import { backupSchema, importSummarySchema, packageExportResultSchema, restoreResultSchema, startupStatusSchema } from "./dto";

export function tauriData(): DataTransfer {
  return {
    startupStatus: () => call("get_startup_status", {}, startupStatusSchema),
    importLegacy: () => call("import_legacy", {}, importSummarySchema.nullable()),
    importPackage: () => call("import_package", {}, importSummarySchema.nullable()),
    exportPackage: (projectId) => call("export_package", { projectId }, packageExportResultSchema.nullable()),
    listBackups: () => call("list_backups", {}, z.array(backupSchema)),
    createBackup: () => call("create_backup", {}, backupSchema),
    restoreBackup: (fileName) => call("restore_backup", { fileName }, restoreResultSchema),
    recover: async (fileName) => {
      await call("recover_database", { fileName }, z.null());
    },
  };
}
```

En `src/infrastructure/tauri/repositories.ts`, importa `tauriData` y agrega `data: tauriData(),` al objeto.

En `src/infrastructure/tauri/contract.test.ts`, importa los 5 esquemas nuevos y agrega al `it.each`:

```ts
    ["importSummary", importSummarySchema],
    ["packageExportResult", packageExportResultSchema],
    ["backup", backupSchema],
    ["restoreResult", restoreResultSchema],
    ["startupStatus", startupStatusSchema],
    ["corruptError", appErrorSchema],
```

- [ ] **Step 3: Adaptador en memoria**

En `src/infrastructure/memory/export.ts`, agrega a `MemoryExporter` el método `remember(path: string): void;` («un paquete exportado en esta sesión también se puede mostrar en su carpeta») y en el objeto `exporter`:

```ts
    remember(path) {
      exported.add(path);
    },
```

Crea `src/infrastructure/memory/data.ts`:

```ts
import type { Backup, BackupKind, ImportedProject, ImportSummary, StartupStatus } from "../../domain/data";
import type { DataTransfer } from "../../ports/data";
import { AppError } from "../../ports/errors";

export interface MemoryDataDeps {
  /** Crea el proyecto que «trae» la importación. */
  importProject(name: string): Promise<ImportedProject>;
  /** Registra el paquete como exportado en la sesión (para «Abrir carpeta»). */
  remember(path: string): void;
}

/** ok: importa/exporta · cancel: el usuario cierra el diálogo · invalid: Rust rechaza el archivo. */
export type MemoryDataMode = "ok" | "cancel" | "invalid";

export interface MemoryData extends DataTransfer {
  mode: MemoryDataMode;
  invalidMessage: string;
  status: StartupStatus;
  backups: Backup[];
  /** Operaciones pedidas, en orden (`importLegacy`, `restore:<archivo>`, `recover:vacía`…). */
  calls: string[];
}

export const MEMORY_INVALID_IMPORT =
  "No se importó nada. Revisa estos problemas:\n• Proyecto «Aplicación de recibos», EJ-001: Completa caso y entorno.\n• Proyecto «Aplicación de recibos», EJ-002: Fecha inválida.";

const backup = (fileName: string, kind: BackupKind, createdAt: string): Backup => ({ fileName, kind, createdAt, sizeBytes: 98304 });
const pad = (n: number) => String(n).padStart(2, "0");

/** Replica en memoria lo que devuelven los comandos de `commands/data.rs`. */
export function createMemoryData(deps: MemoryDataDeps): MemoryData {
  let manualSeq = 0;
  const pick = async (call: string, name: string, counts: { cases: number; evidence: number }): Promise<ImportSummary | null> => {
    data.calls.push(call);
    if (data.mode === "cancel") return null;
    if (data.mode === "invalid") throw new AppError("validation", data.invalidMessage);
    return { projects: [await deps.importProject(name)], ...counts };
  };
  const data: MemoryData = {
    mode: "ok",
    invalidMessage: MEMORY_INVALID_IMPORT,
    status: { recovery: false, detail: "" },
    backups: [backup("qapture-diario-20261016.db", "daily", "2026-10-16T09:00:00"), backup("qapture-diario-20261015.db", "daily", "2026-10-15T09:00:00")],
    calls: [],
    async startupStatus() {
      return { ...data.status };
    },
    importLegacy: () => pick("importLegacy", "Aplicación de recibos", { cases: 1, evidence: 2 }),
    importPackage: () => pick("importPackage", "Recibos (importado)", { cases: 3, evidence: 4 }),
    async exportPackage(projectId) {
      data.calls.push(`exportPackage:${projectId}`);
      if (data.mode === "cancel") return null;
      const fileName = "T4S_ACME_Recibos_20261016.qapture";
      const path = `C:\\Users\\ana\\Documents\\${fileName}`;
      deps.remember(path);
      return { path, fileName, cases: 2, evidence: 2, missingImages: 0 };
    },
    async listBackups() {
      return structuredClone(data.backups);
    },
    async createBackup() {
      data.calls.push("createBackup");
      manualSeq += 1;
      const created = backup(`qapture-manual-20261016-1000${pad(manualSeq)}.db`, "manual", `2026-10-16T10:00:${pad(manualSeq)}`);
      data.backups = [created, ...data.backups];
      return structuredClone(created);
    },
    async restoreBackup(fileName) {
      data.calls.push(`restore:${fileName}`);
      const restored = data.backups.find((b) => b.fileName === fileName);
      if (!restored) throw new AppError("not_found", "El respaldo no existe.");
      const safety = backup("qapture-antes-de-restaurar-20261016-110000.db", "before-restore", "2026-10-16T11:00:00");
      data.backups = [safety, ...data.backups];
      return { restored: structuredClone(restored), safetyCopy: safety.fileName, missingImages: 0, setAsideFiles: 0 };
    },
    async recover(fileName) {
      data.calls.push(`recover:${fileName ?? "vacía"}`);
      if (!data.status.recovery) throw new AppError("validation", "Qapture no está en modo recuperación.");
      data.status = { recovery: false, detail: "" };
    },
  };
  return data;
}
```

En `src/infrastructure/memory/repositories.ts`:
- importa `createMemoryData` y `MemoryData` de `./data`, y `emptyMeta` de `../../domain/project`;
- agrega `data: MemoryData;` a `MemoryRepositories`;
- cambia el `return { … };` final por `const repos: MemoryRepositories = { … };` seguido de `return repos;`, y agrega dentro del objeto:

```ts
    // `repos` se usa dentro de funciones que corren después de inicializarse.
    data: createMemoryData({
      importProject: async (name) => {
        const project = await repos.projects.create({ meta: { ...emptyMeta(), name }, templateId: "tpl-funcional" });
        return { id: project.id, name: project.name };
      },
      remember: (path) => repos.exporter.remember(path),
    }),
```

En `src/ui/test-utils.tsx`, cambia el tipo de `setup` a `(repos: MemoryRepositories) => Promise<void>` (importa `MemoryRepositories` de `../infrastructure/memory/repositories`). Los callers existentes siguen compilando porque `MemoryRepositories` extiende `Repositories`.

Run: `pnpm typecheck && pnpm test src/infrastructure`
Expected: PASS (incluye el contrato con las 6 claves nuevas).

- [ ] **Step 4: Tests del store (fallan)**

Crea `src/ui/store/data-slice.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { emptyMeta } from "../../domain/project";
import { createMemoryRepositories, type MemoryRepositories } from "../../infrastructure/memory/repositories";
import { memoryPreferences } from "./preferences";
import { createWorkspaceStore } from "./workspace";

async function setup(configure?: (repos: MemoryRepositories) => Promise<void> | void) {
  const repos = createMemoryRepositories();
  await configure?.(repos);
  const store = createWorkspaceStore(repos, memoryPreferences(null, ""));
  await store.getState().init();
  return { repos, store };
}

const lastToast = (store: Awaited<ReturnType<typeof setup>>["store"]) => {
  const { toasts } = store.getState();
  return toasts[toasts.length - 1];
};

const seed = (repos: MemoryRepositories) =>
  repos.projects.create({ meta: { ...emptyMeta(), name: "Recibos", client: "ACME" }, templateId: "tpl-funcional" });

describe("importar", () => {
  it("v1: abre el proyecto importado y muestra el resumen", async () => {
    const { repos, store } = await setup();
    await store.getState().importLegacy();
    const s = store.getState();
    expect(s.project?.name).toBe("Aplicación de recibos");
    expect(s.projects.map((p) => p.name)).toEqual(["Aplicación de recibos"]);
    expect(lastToast(store)).toMatchObject({ kind: "success", message: "Importado: 1 proyecto, 1 caso y 2 capturas." });
    expect(repos.data.calls).toEqual(["importLegacy"]);
    expect(s.dataBusy).toBe(false);
  });

  it("cerrar el diálogo de archivo no cambia nada", async () => {
    const { repos, store } = await setup((r) => void (r.data.mode = "cancel"));
    await store.getState().importPackage();
    expect(store.getState().projects).toEqual([]);
    expect(store.getState().toasts).toEqual([]);
    expect(repos.data.calls).toEqual(["importPackage"]);
  });

  it("un archivo rechazado deja la lista de problemas y no importa nada", async () => {
    const { store } = await setup((r) => void (r.data.mode = "invalid"));
    await store.getState().importLegacy();
    expect(store.getState().importIssues).toEqual({
      title: "No se importó nada. Revisa estos problemas:",
      items: ["Proyecto «Aplicación de recibos», EJ-001: Completa caso y entorno.", "Proyecto «Aplicación de recibos», EJ-002: Fecha inválida."],
    });
    expect(store.getState().projects).toEqual([]);
    expect(store.getState().toasts).toEqual([]);
    store.getState().dismissImportIssues();
    expect(store.getState().importIssues).toBeNull();
  });

  it("no corre dos operaciones de datos a la vez", async () => {
    const { repos, store } = await setup();
    const s = store.getState();
    await Promise.all([s.importLegacy(), s.importPackage()]);
    expect(repos.data.calls).toEqual(["importLegacy"]);
  });
});

describe("exportar paquete", () => {
  it("sin proyecto abierto no hace nada", async () => {
    const { repos, store } = await setup();
    await store.getState().exportPackage();
    expect(repos.data.calls).toEqual([]);
  });

  it("avisa con «Abrir carpeta», que muestra el paquete", async () => {
    const { repos, store } = await setup(async (r) => void (await seed(r)));
    await store.getState().exportPackage();
    const toast = lastToast(store);
    expect(toast.message).toBe("Paquete guardado: T4S_ACME_Recibos_20261016.qapture");
    toast.actions?.[0].run();
    await Promise.resolve();
    expect(repos.exporter.revealed).toEqual(["C:\\Users\\ana\\Documents\\T4S_ACME_Recibos_20261016.qapture"]);
  });
});

describe("respaldos", () => {
  it("lista, crea y restaura", async () => {
    const { repos, store } = await setup(async (r) => void (await seed(r)));
    await store.getState().loadBackups();
    expect(store.getState().backups).toHaveLength(2);
    await store.getState().createBackup();
    expect(store.getState().backups[0].fileName).toBe("qapture-manual-20261016-100001.db");
    expect(lastToast(store).message).toBe("Respaldo creado: qapture-manual-20261016-100001.db");
    expect(await store.getState().restoreBackup("qapture-diario-20261015.db")).toBe(true);
    expect(lastToast(store).message).toBe("Se restauró el respaldo del 15/10/2026 09:00.");
    expect(store.getState().backups[0].kind).toBe("before-restore");
    expect(store.getState().project?.name).toBe("Recibos");
    expect(repos.data.calls).toEqual(["createBackup", "restore:qapture-diario-20261015.db"]);
  });

  it("un respaldo que no existe avisa y devuelve false", async () => {
    const { store } = await setup();
    expect(await store.getState().restoreBackup("qapture-manual-20991231-000000.db")).toBe(false);
    expect(lastToast(store)).toMatchObject({ kind: "error", message: "El respaldo no existe." });
  });
});

describe("modo recuperación", () => {
  const damaged = (r: MemoryRepositories) => void (r.data.status = { recovery: true, detail: "base dañada" });

  it("al iniciar no carga proyectos y trae los respaldos", async () => {
    const { store } = await setup(damaged);
    const s = store.getState();
    expect(s.recovery).toEqual({ recovery: true, detail: "base dañada" });
    expect(s.ready).toBe(true);
    expect(s.backups).toHaveLength(2);
  });

  it("recuperar con un respaldo vuelve al uso normal", async () => {
    const { repos, store } = await setup(damaged);
    await store.getState().recover("qapture-diario-20261016.db");
    expect(store.getState().recovery).toBeNull();
    expect(lastToast(store).message).toBe("Base recuperada desde el respaldo.");
    expect(repos.data.calls).toEqual(["recover:qapture-diario-20261016.db"]);
  });

  it("empezar vacía e importar un paquete abre la importación a continuación", async () => {
    const { repos, store } = await setup(damaged);
    await store.getState().recover(null, true);
    expect(repos.data.calls).toEqual(["recover:vacía", "importPackage"]);
    expect(store.getState().project?.name).toBe("Recibos (importado)");
    expect(lastToast(store).message).toBe("Importado: 1 proyecto, 3 casos y 4 capturas.");
  });
});
```

Run: `pnpm test src/ui/store/data-slice.test.ts`
Expected: FAIL (`importLegacy` no es una función).

- [ ] **Step 5: Implementar el slice e integrarlo**

En `src/ui/store/evidence-slice.ts`, agrega a `SliceHelpers`:

```ts
  /** Recarga plantillas y proyectos y abre `projectId` (o el actual, o el primero). */
  reload(projectId?: string | null): Promise<void>;
```

Crea `src/ui/store/data-slice.ts`:

```ts
import type { StoreApi } from "zustand/vanilla";
import { importSummaryMessage, parseImportIssues, restoreMessage, type Backup, type ImportIssues, type ImportSummary, type StartupStatus } from "../../domain/data";
import { isValidationError } from "../../ports/errors";
import type { Repositories } from "../../ports/repositories";
import type { SliceHelpers } from "./evidence-slice";
import type { WorkspaceState } from "./workspace";

export interface DataSlice {
  /** Con la base dañada, el estado del arranque; `null` en uso normal. */
  recovery: StartupStatus | null;
  backups: Backup[];
  /** Problemas de la última importación rechazada, para el diálogo «No se pudo importar». */
  importIssues: ImportIssues | null;
  /** Hay una importación, exportación, respaldo o restauración en curso. */
  dataBusy: boolean;
  loadBackups(): Promise<void>;
  importLegacy(): Promise<void>;
  importPackage(): Promise<void>;
  exportPackage(): Promise<void>;
  createBackup(): Promise<void>;
  /** `true` si se restauró (para cerrar el diálogo). */
  restoreBackup(fileName: string): Promise<boolean>;
  recover(fileName: string | null, thenImportPackage?: boolean): Promise<void>;
  dismissImportIssues(): void;
}

type Set = StoreApi<WorkspaceState>["setState"];
type Get = StoreApi<WorkspaceState>["getState"];

const missingMessage = (n: number) =>
  n === 1 ? "1 captura no se incluyó porque falta su imagen." : `${n} capturas no se incluyeron porque falta su imagen.`;

export function createDataSlice(repos: Repositories, set: Set, get: Get, h: SliceHelpers): DataSlice {
  /** Una operación de datos a la vez; `false` si ya había otra en curso. */
  const exclusive = async (run: () => Promise<void>): Promise<boolean> => {
    if (get().dataBusy) return false;
    set({ dataBusy: true });
    try {
      await run();
      return true;
    } finally {
      set({ dataBusy: false });
    }
  };

  const runImport = async (pick: () => Promise<ImportSummary | null>) => {
    await exclusive(async () => {
      try {
        const summary = await pick();
        if (!summary) return;
        await h.reload(summary.projects[0]?.id ?? null);
        get().pushToast("success", importSummaryMessage(summary));
      } catch (error) {
        if (isValidationError(error)) set({ importIssues: parseImportIssues(error.message) });
        else h.report(error);
      }
    });
  };

  return {
    recovery: null,
    backups: [],
    importIssues: null,
    dataBusy: false,

    async loadBackups() {
      try {
        set({ backups: await repos.data.listBackups() });
      } catch (error) {
        h.report(error);
      }
    },

    importLegacy: () => runImport(() => repos.data.importLegacy()),
    importPackage: () => runImport(() => repos.data.importPackage()),

    async exportPackage() {
      const current = get().project;
      if (!current) return;
      await exclusive(async () => {
        try {
          const result = await repos.data.exportPackage(current.id);
          if (!result) return;
          get().pushToast("success", `Paquete guardado: ${result.fileName}`, [
            { label: "Abrir carpeta", run: () => void repos.exporter.revealFile(result.path).catch(h.report) },
          ]);
          if (result.missingImages > 0) get().pushToast("error", missingMessage(result.missingImages));
        } catch (error) {
          h.report(error);
        }
      });
    },

    async createBackup() {
      await exclusive(async () => {
        try {
          const created = await repos.data.createBackup();
          get().pushToast("success", `Respaldo creado: ${created.fileName}`);
        } catch (error) {
          h.report(error);
          return;
        }
        await get().loadBackups();
      });
    },

    async restoreBackup(fileName) {
      let restored = false;
      await exclusive(async () => {
        try {
          const result = await repos.data.restoreBackup(fileName);
          restored = true;
          await h.reload(null);
          await get().loadBackups();
          get().pushToast("success", restoreMessage(result));
        } catch (error) {
          h.report(error);
        }
      });
      return restored;
    },

    async recover(fileName, thenImportPackage = false) {
      const ran = await exclusive(async () => {
        try {
          await repos.data.recover(fileName);
          set({ recovery: null });
          await h.reload(null);
          get().pushToast("success", fileName ? "Base recuperada desde el respaldo." : "Qapture empezó con una base vacía.");
        } catch (error) {
          h.report(error);
        }
      });
      if (ran && thenImportPackage && !get().recovery) await get().importPackage();
    },

    dismissImportIssues: () => set({ importIssues: null }),
  };
}
```

En `src/ui/store/workspace.ts`:
- importa `createDataSlice` y `DataSlice` de `./data-slice`;
- `export interface WorkspaceState extends EvidenceSlice, ExportSlice, DataSlice {`;
- después de `refreshCases`, agrega:

```ts
    const reload = async (projectId: string | null = null) => {
      await refreshLists();
      const { projects, project } = get();
      const exists = (id: string | null | undefined): id is string => !!id && projects.some((p) => p.id === id);
      const currentId = project?.id;
      const target = exists(projectId) ? projectId : exists(currentId) ? currentId : (projects[0]?.id ?? null);
      await loadProject(target);
      await get().loadEvidence();
    };
    const helpers = { report, refreshCases, form, reload };
```

- al inicio de `init()`, dentro del `try` y antes de `await refreshLists();`:

```ts
          const status = await repos.data.startupStatus();
          if (status.recovery) {
            // Base dañada: no se cargan proyectos; la pantalla de recuperación ofrece los respaldos.
            set({ recovery: status });
            await get().loadBackups();
            return;
          }
```

- reemplaza los dos spreads del final por:

```ts
      ...createEvidenceSlice(repos, set, get, helpers),
      ...createExportSlice(repos, prefs, set, get, helpers),
      ...createDataSlice(repos, set, get, helpers),
```

- [ ] **Step 6: Ejecutar los tests**

Run: `pnpm typecheck && pnpm test`
Expected: PASS (los tests de F1–F4 siguen en verde: `init` en uso normal no cambia).

- [ ] **Step 7: Commit**

```bash
git add src
git commit -m "feat(f5): datos en el front — puerto DataTransfer, adaptadores, store de importación, respaldos y recuperación

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---
### Task 10: UI — menú Datos, diálogo de respaldos, problemas de importación y pantalla de recuperación

**Files:**
- Create: `src/ui/screens/DataMenu.tsx`, `src/ui/screens/BackupsDialog.tsx`, `src/ui/screens/ImportIssuesDialog.tsx`, `src/ui/screens/RecoveryScreen.tsx`, `src/ui/screens/Data.test.tsx`, `src/ui/screens/Recovery.test.tsx`
- Modify: `src/ui/screens/TopBar.tsx`, `src/ui/screens/AppShell.tsx`

**Interfaces:**
- Consumes: `DataSlice` (Task 9), `ConfirmDialog`, `Dialog`, `btn`, `BACKUP_KIND_LABEL`, `formatBackupDate`, `formatSize`.
- Produces:
  - `<DataMenu onBackups />`: botón «Datos» con `role="menu"` y cuatro `menuitem`: «Importar Bitácora v1…», «Importar paquete .qapture…», «Exportar paquete .qapture…» (deshabilitado sin proyecto) y «Respaldos…».
  - `<BackupsDialog onClose />` y `<BackupTable backups disabled onRestore />`, que también usa la pantalla de recuperación.
  - `<ImportIssuesDialog />`: se muestra solo si `importIssues` no es `null`.
  - `<RecoveryScreen />`: reemplaza toda la app mientras `recovery` no es `null`.
  - `TopBar` recibe `onBackups?: () => void`.

- [ ] **Step 1: Tests de la UI (fallan)**

Crea `src/ui/screens/Data.test.tsx`:

```tsx
import { screen, waitFor, within } from "@testing-library/react";
import type { UserEvent } from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { renderApp, seedProject } from "../test-utils";

async function openMenu(user: UserEvent) {
  await user.click(screen.getByRole("button", { name: /Datos/ }));
  return screen.getByRole("menu", { name: "Datos" });
}

async function choose(user: UserEvent, item: string) {
  await user.click(within(await openMenu(user)).getByRole("menuitem", { name: item }));
}

describe("menú Datos", () => {
  it("importa la Bitácora v1, abre el proyecto importado y cierra el menú", async () => {
    const { user } = await renderApp();
    await choose(user, "Importar Bitácora v1…");
    expect(await screen.findByText("Importado: 1 proyecto, 1 caso y 2 capturas.")).toBeInTheDocument();
    expect(screen.getByRole("combobox", { name: "Proyecto" })).toHaveDisplayValue("Aplicación de recibos");
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
  });

  it("muestra los problemas de un archivo rechazado", async () => {
    const { user, store } = await renderApp({ setup: async (r) => void (r.data.mode = "invalid") });
    await choose(user, "Importar Bitácora v1…");
    const dialog = await screen.findByRole("dialog", { name: "No se pudo importar" });
    expect(within(dialog).getByText("No se importó nada. Revisa estos problemas:")).toBeInTheDocument();
    expect(within(dialog).getAllByRole("listitem")).toHaveLength(2);
    await user.click(within(dialog).getByRole("button", { name: "Entendido" }));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(store.getState().projects).toEqual([]);
  });

  it("exportar paquete solo se habilita con un proyecto abierto", async () => {
    const { user } = await renderApp();
    expect(within(await openMenu(user)).getByRole("menuitem", { name: "Exportar paquete .qapture…" })).toBeDisabled();
  });

  it("exporta el paquete y ofrece abrir la carpeta", async () => {
    const { user, repos } = await renderApp({ setup: async (r) => void (await seedProject(r)) });
    await choose(user, "Exportar paquete .qapture…");
    expect(await screen.findByText("Paquete guardado: T4S_ACME_Recibos_20261016.qapture")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Abrir carpeta" }));
    await waitFor(() => expect(repos.exporter.revealed).toHaveLength(1));
  });

  it("Escape y un clic fuera cierran el menú", async () => {
    const { user } = await renderApp();
    await openMenu(user);
    await user.keyboard("{Escape}");
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    await openMenu(user);
    await user.click(document.body);
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
  });
});

describe("respaldos", () => {
  it("lista, crea y restaura con confirmación", async () => {
    const { user, repos } = await renderApp({ setup: async (r) => void (await seedProject(r)) });
    await choose(user, "Respaldos…");
    const dialog = await screen.findByRole("dialog", { name: "Respaldos" });
    await waitFor(() => expect(within(dialog).getAllByRole("row")).toHaveLength(3)); // encabezado + 2
    expect(within(dialog).getAllByText("Diario")).toHaveLength(2);

    await user.click(within(dialog).getByRole("button", { name: /Respaldar ahora/ }));
    expect(await screen.findByText("Respaldo creado: qapture-manual-20261016-100001.db")).toBeInTheDocument();
    await waitFor(() => expect(within(dialog).getAllByRole("row")).toHaveLength(4));

    const row = within(dialog).getByRole("row", { name: /15\/10\/2026 09:00/ });
    await user.click(within(row).getByRole("button", { name: "Restaurar" }));
    const confirm = screen.getByRole("dialog", { name: "Restaurar respaldo" });
    expect(within(confirm).getByText(/volverá al estado del 15\/10\/2026 09:00/)).toBeInTheDocument();
    await user.click(within(confirm).getByRole("button", { name: "Restaurar" }));

    expect(await screen.findByText("Se restauró el respaldo del 15/10/2026 09:00.")).toBeInTheDocument();
    expect(repos.data.calls).toContain("restore:qapture-diario-20261015.db");
    expect(screen.queryByRole("dialog", { name: "Respaldos" })).not.toBeInTheDocument();
  });

  it("cancelar la confirmación no restaura", async () => {
    const { user, repos } = await renderApp();
    await choose(user, "Respaldos…");
    const dialog = await screen.findByRole("dialog", { name: "Respaldos" });
    const [first] = await within(dialog).findAllByRole("button", { name: "Restaurar" });
    await user.click(first);
    await user.click(within(screen.getByRole("dialog", { name: "Restaurar respaldo" })).getByRole("button", { name: "Cancelar" }));
    expect(repos.data.calls).toEqual([]);
    expect(screen.getByRole("dialog", { name: "Respaldos" })).toBeInTheDocument();
  });
});
```

Crea `src/ui/screens/Recovery.test.tsx`:

```tsx
import { screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { MemoryRepositories } from "../../infrastructure/memory/repositories";
import { renderApp } from "../test-utils";

const damaged = async (r: MemoryRepositories) => {
  r.data.status = { recovery: true, detail: "base dañada: database disk image is malformed" };
};

describe("pantalla de recuperación", () => {
  it("reemplaza la app y ofrece los respaldos", async () => {
    await renderApp({ setup: damaged });
    expect(screen.getByRole("heading", { name: "Qapture no pudo abrir tus datos" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Nueva matriz/ })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Datos/ })).not.toBeInTheDocument();
    expect(await screen.findAllByRole("button", { name: "Restaurar" })).toHaveLength(2);
    expect(screen.getByText("base dañada: database disk image is malformed")).toBeInTheDocument();
  });

  it("restaura un respaldo y vuelve a la app", async () => {
    const { user, repos } = await renderApp({ setup: damaged });
    const [newest] = await screen.findAllByRole("button", { name: "Restaurar" });
    await user.click(newest);
    await user.click(within(screen.getByRole("dialog", { name: "Restaurar respaldo" })).getByRole("button", { name: "Restaurar" }));
    expect(await screen.findByText("Base recuperada desde el respaldo.")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Datos/ })).toBeInTheDocument(); // la barra superior volvió
    expect(repos.data.calls).toEqual(["recover:qapture-diario-20261016.db"]);
  });

  it("empezar vacía e importar un paquete abre la importación", async () => {
    const { user, repos } = await renderApp({ setup: damaged });
    await user.click(screen.getByRole("button", { name: "Empezar vacía e importar un paquete" }));
    await user.click(within(screen.getByRole("dialog", { name: "Empezar con una base vacía" })).getByRole("button", { name: "Empezar vacía" }));
    expect(await screen.findByText("Importado: 1 proyecto, 3 casos y 4 capturas.")).toBeInTheDocument();
    expect(repos.data.calls).toEqual(["recover:vacía", "importPackage"]);
    expect(screen.getByRole("combobox", { name: "Proyecto" })).toHaveDisplayValue("Recibos (importado)");
  });
});
```

Run: `pnpm test src/ui/screens/Data.test.tsx src/ui/screens/Recovery.test.tsx`
Expected: FAIL (no hay botón «Datos» ni pantalla de recuperación).

- [ ] **Step 2: Diálogos de respaldos y de problemas**

Crea `src/ui/screens/BackupsDialog.tsx`:

```tsx
import { DatabaseBackup } from "lucide-react";
import { useEffect, useState } from "react";
import { BACKUP_KIND_LABEL, formatBackupDate, formatSize, type Backup } from "../../domain/data";
import { ConfirmDialog } from "../components/ConfirmDialog";
import { Dialog } from "../components/Dialog";
import { btn } from "../components/styles";
import { useWorkspace } from "../store/context";

export function BackupTable({ backups, disabled, onRestore }: { backups: Backup[]; disabled: boolean; onRestore: (backup: Backup) => void }) {
  if (backups.length === 0) return <p className="text-sm text-t4s-gray">Todavía no hay respaldos en esta computadora.</p>;
  return (
    <table className="w-full text-sm">
      <thead>
        <tr className="text-left text-xs uppercase tracking-wide text-t4s-gray">
          <th className="py-2 font-medium">Fecha</th>
          <th className="font-medium">Tipo</th>
          <th className="font-medium">Tamaño</th>
          <th>
            <span className="sr-only">Acciones</span>
          </th>
        </tr>
      </thead>
      <tbody>
        {backups.map((b) => (
          <tr key={b.fileName} className="border-t border-t4s-line">
            <td className="py-2">{formatBackupDate(b.createdAt)}</td>
            <td>{BACKUP_KIND_LABEL[b.kind]}</td>
            <td>{formatSize(b.sizeBytes)}</td>
            <td className="text-right">
              <button type="button" className={btn.ghost} disabled={disabled} onClick={() => onRestore(b)}>
                Restaurar
              </button>
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

const restoreWarning = (b: Backup) =>
  `Qapture volverá al estado del ${formatBackupDate(b.createdAt)}. Antes guarda una copia del estado actual en Respaldos. ` +
  "Las capturas que no estén en el respaldo se mueven a la carpeta «evidence-sin-uso»; no se borran.";

export function BackupsDialog({ onClose }: { onClose: () => void }) {
  const backups = useWorkspace((s) => s.backups);
  const busy = useWorkspace((s) => s.dataBusy);
  const loadBackups = useWorkspace((s) => s.loadBackups);
  const createBackup = useWorkspace((s) => s.createBackup);
  const restoreBackup = useWorkspace((s) => s.restoreBackup);
  const [pending, setPending] = useState<Backup | null>(null);

  useEffect(() => {
    void loadBackups();
  }, [loadBackups]);

  return (
    <>
      <Dialog title="Respaldos" kicker="Datos" onClose={onClose}>
        <p className="text-sm text-t4s-gray">
          Qapture copia la base de datos una vez al día y conserva las últimas 7 copias diarias. Las capturas no se copian: siguen en la carpeta de datos.
        </p>
        <div className="my-4">
          <button type="button" className={btn.secondary} disabled={busy} onClick={() => void createBackup()}>
            <DatabaseBackup size={16} /> Respaldar ahora
          </button>
        </div>
        <BackupTable backups={backups} disabled={busy} onRestore={setPending} />
      </Dialog>
      {pending && (
        <ConfirmDialog
          title="Restaurar respaldo"
          message={restoreWarning(pending)}
          confirmLabel="Restaurar"
          onCancel={() => setPending(null)}
          onConfirm={async () => {
            const ok = await restoreBackup(pending.fileName);
            setPending(null);
            if (ok) onClose();
          }}
        />
      )}
    </>
  );
}
```

Crea `src/ui/screens/ImportIssuesDialog.tsx`:

```tsx
import { Dialog, DialogFooter } from "../components/Dialog";
import { btn } from "../components/styles";
import { useWorkspace } from "../store/context";

export function ImportIssuesDialog() {
  const issues = useWorkspace((s) => s.importIssues);
  const dismiss = useWorkspace((s) => s.dismissImportIssues);
  if (!issues) return null;
  return (
    <Dialog title="No se pudo importar" onClose={dismiss}>
      <p className="text-sm">{issues.title}</p>
      {issues.items.length > 0 && (
        <ul className="mt-3 list-disc space-y-1 pl-5 text-sm">
          {issues.items.map((item, index) => (
            <li key={index}>{item}</li>
          ))}
        </ul>
      )}
      <DialogFooter>
        <button type="button" className={btn.primary} onClick={dismiss}>
          Entendido
        </button>
      </DialogFooter>
    </Dialog>
  );
}
```

- [ ] **Step 3: Menú Datos y barra superior**

Crea `src/ui/screens/DataMenu.tsx`:

```tsx
import { Database } from "lucide-react";
import { useEffect, useRef, useState, type ReactNode } from "react";
import { btn } from "../components/styles";
import { useWorkspace } from "../store/context";

function MenuItem({ children, disabled = false, onClick }: { children: ReactNode; disabled?: boolean; onClick: () => void }) {
  return (
    <button
      type="button"
      role="menuitem"
      disabled={disabled}
      onClick={onClick}
      className="block w-full px-4 py-2 text-left text-sm hover:bg-t4s-cream disabled:cursor-not-allowed disabled:opacity-50"
    >
      {children}
    </button>
  );
}

/** UC-18 / UC-19 / UC-20. F6 lleva estas acciones también a Settings › Datos. */
export function DataMenu({ onBackups }: { onBackups: () => void }) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  const project = useWorkspace((s) => s.project);
  const busy = useWorkspace((s) => s.dataBusy);
  const importLegacy = useWorkspace((s) => s.importLegacy);
  const importPackage = useWorkspace((s) => s.importPackage);
  const exportPackage = useWorkspace((s) => s.exportPackage);

  useEffect(() => {
    if (!open) return;
    const onPointer = (e: MouseEvent) => {
      if (!ref.current?.contains(e.target as Node)) setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    document.addEventListener("mousedown", onPointer);
    window.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onPointer);
      window.removeEventListener("keydown", onKey);
    };
  }, [open]);

  const run = (action: () => void) => () => {
    setOpen(false);
    action();
  };

  return (
    <div ref={ref} className="relative">
      <button type="button" className={btn.secondary} aria-haspopup="menu" aria-expanded={open} disabled={busy} onClick={() => setOpen((o) => !o)}>
        <Database size={16} /> Datos
      </button>
      {open && (
        <div role="menu" aria-label="Datos" className="absolute right-0 z-30 mt-1 w-64 rounded-lg border border-t4s-line bg-white py-1 shadow-lg">
          <MenuItem onClick={run(() => void importLegacy())}>Importar Bitácora v1…</MenuItem>
          <MenuItem onClick={run(() => void importPackage())}>Importar paquete .qapture…</MenuItem>
          <MenuItem disabled={!project} onClick={run(() => void exportPackage())}>
            Exportar paquete .qapture…
          </MenuItem>
          <hr className="my-1 border-t4s-line" />
          <MenuItem onClick={run(onBackups)}>Respaldos…</MenuItem>
        </div>
      )}
    </div>
  );
}
```

En `src/ui/screens/TopBar.tsx`:
- importa `DataMenu` de `./DataMenu`;
- agrega `onBackups?: () => void` a las props (`{ onNewProject, onSettings, onExport, onBackups }`);
- dentro de `<div className="ml-auto flex gap-2">`, como primer hijo: `{onBackups && <DataMenu onBackups={onBackups} />}`.

- [ ] **Step 4: Pantalla de recuperación y AppShell**

Crea `src/ui/screens/RecoveryScreen.tsx`:

```tsx
import { TriangleAlert } from "lucide-react";
import { useState } from "react";
import { formatBackupDate, type Backup } from "../../domain/data";
import { ConfirmDialog } from "../components/ConfirmDialog";
import { btn } from "../components/styles";
import { useWorkspace } from "../store/context";
import { BackupTable } from "./BackupsDialog";

type Choice = { backup: Backup | null; thenImport: boolean };

/** Spec §8: la base no abre. Volver a un respaldo o empezar con una base vacía (e importar un paquete). */
export function RecoveryScreen() {
  const recovery = useWorkspace((s) => s.recovery);
  const backups = useWorkspace((s) => s.backups);
  const busy = useWorkspace((s) => s.dataBusy);
  const recover = useWorkspace((s) => s.recover);
  const [pending, setPending] = useState<Choice | null>(null);

  return (
    <main className="mx-auto flex w-full max-w-3xl flex-col gap-6 p-8">
      <header className="flex items-start gap-3">
        <TriangleAlert className="mt-1 shrink-0 text-red-700" size={28} aria-hidden />
        <div>
          <h1 className="text-xl font-semibold">Qapture no pudo abrir tus datos</h1>
          <p className="text-sm text-t4s-gray">
            La base de datos parece dañada. No se borra: queda en la carpeta de datos como «qapture-danada-…db» por si hace falta revisarla.
          </p>
        </div>
      </header>
      <section className="rounded-xl border border-t4s-line bg-white p-5">
        <h2 className="mb-3 font-semibold">Volver a un respaldo</h2>
        <BackupTable backups={backups} disabled={busy} onRestore={(backup) => setPending({ backup, thenImport: false })} />
      </section>
      <section className="rounded-xl border border-t4s-line bg-white p-5">
        <h2 className="mb-1 font-semibold">Empezar de nuevo</h2>
        <p className="mb-3 text-sm text-t4s-gray">Crea una base vacía. Si un compañero te compartió el proyecto, impórtalo desde su paquete .qapture.</p>
        <div className="flex flex-wrap gap-2">
          <button type="button" className={btn.primary} disabled={busy} onClick={() => setPending({ backup: null, thenImport: true })}>
            Empezar vacía e importar un paquete
          </button>
          <button type="button" className={btn.secondary} disabled={busy} onClick={() => setPending({ backup: null, thenImport: false })}>
            Empezar con una base vacía
          </button>
        </div>
      </section>
      {recovery?.detail && (
        <details className="text-xs text-t4s-gray">
          <summary>Detalle técnico</summary>
          <p className="mt-1 break-all">{recovery.detail}</p>
        </details>
      )}
      {pending && (
        <ConfirmDialog
          title={pending.backup ? "Restaurar respaldo" : "Empezar con una base vacía"}
          message={
            pending.backup
              ? `Qapture usará el respaldo del ${formatBackupDate(pending.backup.createdAt)}. Las capturas que no estén en ese respaldo se mueven a la carpeta «evidence-sin-uso».`
              : "Qapture creará una base vacía. Las capturas que había se mueven a la carpeta «evidence-sin-uso»; no se borran."
          }
          confirmLabel={pending.backup ? "Restaurar" : "Empezar vacía"}
          onCancel={() => setPending(null)}
          onConfirm={async () => {
            const choice = pending;
            setPending(null);
            await recover(choice.backup?.fileName ?? null, choice.thenImport);
          }}
        />
      )}
    </main>
  );
}
```

En `src/ui/screens/AppShell.tsx`:
- importa `BackupsDialog`, `ImportIssuesDialog` y `RecoveryScreen`;
- agrega `const recovery = useWorkspace((s) => s.recovery);` junto a los demás selectores (antes de cualquier `return`);
- amplía el estado: `useState<"new-project" | "settings" | "export" | "backups" | null>(null)`;
- después de `if (!ready) return …;`:

```tsx
  if (recovery) {
    return (
      <div data-testid="app-ready" className="flex h-full flex-col overflow-y-auto bg-t4s-cream">
        <RecoveryScreen />
        <ImportIssuesDialog />
        <Toasts />
      </div>
    );
  }
```

- en el `<TopBar …>`, agrega `onBackups={() => setDialog("backups")}`;
- antes de `<Toasts />`:

```tsx
      {dialog === "backups" && <BackupsDialog onClose={() => setDialog(null)} />}
      <ImportIssuesDialog />
```

- [ ] **Step 5: Ejecutar los tests**

Run: `pnpm typecheck && pnpm test && pnpm build`
Expected: PASS. Los tests de F1–F4 que buscan botones de la barra superior siguen funcionando: «Datos» es un botón nuevo con otro nombre.

- [ ] **Step 6: Smoke en desarrollo**

Run: `pnpm tauri dev`
Recorre a mano:
1. **Datos › Importar Bitácora v1…** con `src-tauri/fixtures/bitacora-v1.json` → toast `Importado: 2 proyectos, 3 casos y 3 capturas.` y se abre «Aplicación de recibos» con EJ-001, EJ-002 y EJ-004.
2. Repite → aparece «Aplicación de recibos (importado)».
3. **Exportar paquete .qapture…** → «Guardar como» con nombre `T4S_Aplicación-de-recibos_<hoy>.qapture` → toast con «Abrir carpeta», que abre el Explorador en el archivo.
4. **Importar paquete .qapture…** con ese archivo → copia `(importado 2)` con las mismas capturas.
5. **Respaldos…** → «Respaldar ahora» agrega una fila «Manual». Restaura el diario de hoy: vuelve al estado del arranque y aparece la fila «Antes de restaurar».
6. Cierra la app desde la bandeja. Copia `qapture.db` aparte, reemplázalo por un archivo de texto y abre la app → pantalla de recuperación con los respaldos. Restaura uno → vuelve la app. Luego restituye tu `qapture.db` original.

- [ ] **Step 7: Commit**

```bash
git add src
git commit -m "feat(f5): menú Datos, diálogo de respaldos, problemas de importación y pantalla de recuperación

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 11: Verificación integral, versión `0.5.0` y cierre técnico del contrato

**Files:**
- Modify: `package.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `src-tauri/tauri.conf.json`
- Modify: `docs/superpowers/contracts/F5-importacion-paquetes.md`, `docs/superpowers/contracts/README.md`

- [ ] **Step 1: Versión**

Cambia `"version": "0.4.0"` → `"0.5.0"` en `package.json` y en `src-tauri/tauri.conf.json`, y `version = "0.4.0"` → `"0.5.0"` en `src-tauri/Cargo.toml`. Luego `cd src-tauri && cargo check` para actualizar `Cargo.lock`.

- [ ] **Step 2: Verificación completa**

```bash
cd src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test && cd ..
pnpm typecheck && pnpm test && pnpm build
pnpm tauri build
```

Expected: todo en verde. El instalador queda en `src-tauri/target/release/bundle/nsis/Qapture_0.5.0_x64-setup.exe`. Instálalo y repite el smoke de la Task 10 (pasos 1, 3 y 5) con el build de release: la ACL de producción debe permitir los 8 comandos nuevos, y los diálogos de archivo deben abrirse sobre la ventana principal.

- [ ] **Step 3: Criterios de aceptación (automáticos)**

En `docs/superpowers/contracts/F5-importacion-paquetes.md`, marca los tres criterios con los tests que los cubren:
- Importar el fixture v1 conserva EJ-###, `next_sequence`, los saltos de línea y las imágenes (mismo hash) → `transfer::legacy::tests::imports_the_fixture_keeping_codes_sequence_text_and_images`.
- La importación es todo o nada → `transfer::legacy::tests::a_truncated_png_in_the_second_project_rolls_back_everything`, `transfer::legacy::tests::a_rejected_file_writes_nothing`, `transfer::package::tests::foreign_newer_or_damaged_packages_import_nothing` y `transfer::bundle::tests::a_failed_insert_leaves_no_rows_and_no_files`.
- Exportar → importar → exportar da el mismo `project.json` → `transfer::package::tests::export_import_export_gives_the_same_project_json` (sin ids en el archivo, así que la igualdad es byte a byte).

En la tabla **Tasks** del contrato, agrega una columna `Estado` con `✅` en F5-T1…F5-T6. En `README.md`, cambia el estado de F5 a `🧪 En pruebas del equipo`.

- [ ] **Step 4: Commit**

```bash
git add package.json src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/tauri.conf.json docs
git commit -m "chore(f5): versión 0.5.0 y cierre técnico del contrato F5

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

- [ ] **Step 5: Entrega al equipo**

Pide a la persona usuaria que ejecute el **Checklist de pruebas locales** del contrato F5 (pruebas 1–5) con `Qapture_0.5.0_x64-setup.exe`:
- **Prueba 1:** importar el `matrices.json` **real** de la v1 (`app-local/data/matrices.json`, fuera del repo; nunca se copia al repo). Debe aparecer «Aplicación de recibos» con EJ-001 y sus 2 capturas, y el texto de «Datos de entrada» con sus saltos de línea.
- **Prueba 2:** importar un respaldo v1 dañado (por ejemplo, el `matrices.json` real con una fecha `2026-02-30` editada a mano en una copia).
- **Prueba 3:** exportar un `.qapture` y que un compañero lo importe en su equipo.
- **Prueba 4:** importar dos veces el mismo paquete.
- **Prueba 5:** restaurar un respaldo con confirmación.

Los hallazgos se corrigen en esta misma rama. La fase se cierra cuando el checklist está firmado. Push y PR, solo cuando lo pida.

---

## Probar un build

```bash
git pull && pnpm install
pnpm tauri build
# src-tauri/target/release/bundle/nsis/Qapture_0.5.0_x64-setup.exe
```

Los datos de prueba están en `%APPDATA%\com.teams4soft.qapture\`:
- `qapture.db` y la carpeta `evidence\`;
- `backups\` con `qapture-diario-*.db` (rotan 7), `qapture-manual-*.db` y `qapture-antes-de-restaurar-*.db`;
- `evidence-sin-uso\` aparece solo si una restauración apartó capturas;
- `qapture-danada-*.db` aparece solo tras usar el modo recuperación.

El fixture ficticio para probar la importación v1 sin datos reales está en `src-tauri/fixtures/bitacora-v1.json`. F5 no agrega migraciones de esquema, así que el primer arranque no deja un respaldo `qapture-v3-*`.

## Ejecución

Recomendado: **subagent-driven**.
- **Tasks 1–3 (importación v1) y 4–5 (paquete):** son Rust puro y comparten el núcleo de la Task 1. Por eso conviene un revisor por task que verifique que el todo o nada se cumple en cada camino de error.
- **Tasks 6–7 (respaldos y recuperación):** tocan el arranque y el estado global. Un error ahí puede borrar capturas reales (Review Focus 1 y 4), así que merecen revisión aparte.
- **Tasks 8–10:** dependen de nombres exactos del contrato (`fileName`, `null` al cancelar, `kind` en kebab-case).
- **Codex:** las Tasks 2–5 se pueden delegar a Codex (`/codex:rescue`) indicando la task y este plan; Claude revisa cada diff contra su sección.
