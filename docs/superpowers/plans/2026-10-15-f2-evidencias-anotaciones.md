# F2 — Evidencias y editor de anotaciones · Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Que un tester adjunte capturas a sus casos (Ctrl+V, arrastrar o elegir archivo), las anote con flechas, números, texto, recuadros, resaltado, desenfoque y recorte, y las gestione (ver, reordenar, reasignar, re-anotar, eliminar), conservando siempre la imagen original.

**Architecture:** Rust sigue siendo la autoridad de datos: normaliza cada imagen a PNG (máx. 2400 px de lado mayor) en `evidence/_staging/`, y al guardar mueve el original y escribe el render en `evidence/<proyecto>/` dentro de una operación "archivos + transacción" con reversión. Las imágenes viajan por IPC binario (`tauri::ipc::Request` con cuerpo crudo y un *frame* `[u32 LE largo][JSON meta][PNG]`) y las miniaturas se sirven por el protocolo `asset:` limitado a `evidence/`. En el front, un modelo de anotaciones puro y versionado (TS + zod) alimenta un editor Konva que exporta el PNG a resolución nativa; el editor, la galería y el flujo de registro se conectan al store Zustand mediante un *slice* de evidencias.

**Tech Stack:** Tauri 2 (`protocol-asset`), Rust 2024, rusqlite 0.40, image 0.25 (png, jpeg, webp) · React 19, TypeScript 6, Konva 10 + react-konva 19, Zustand 5, zod 4, lucide-react · Vitest 5, Testing Library, jsdom.

**Spec:** [`docs/superpowers/specs/2026-10-15-qapture-design.md`](../specs/2026-10-15-qapture-design.md) (§5 modelo `EVIDENCE`, §6.3, §6.4, §8, §9) · **Contrato:** [`docs/superpowers/contracts/F2-evidencias-anotaciones.md`](../contracts/F2-evidencias-anotaciones.md)

## Global Constraints

- Rama `feat/f2-evidencias-anotaciones` creada desde `main` actualizado. Un commit por task y todos los mensajes terminan con una línea en blanco y `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>` (o la línea de la herramienta que haga el commit). **No hacer push ni merge sin pedirlo.**
- Plataforma Windows; comandos en Git Bash desde la raíz `qapture/` (los de Rust, desde `qapture/src-tauri/`).
- Textos visibles **en español**, con comillas angulares en los mensajes de campo: `«Descripción» es obligatorio.`
- Paleta T4S: `#2B629E`, `#BAC82D`, `#292B2D`, `#F6F4F1`, `#596069`. Color por defecto de las anotaciones: rojo `#E02424`.
- Formatos aceptados: **PNG, JPEG y WebP** (validados por *magic bytes*, no por extensión). Entrada máx. **64 MB** y **12 000 px por lado**. El original se guarda en PNG con **lado mayor ≤ 2400 px** (sin agrandar los pequeños). El render debe ser PNG con lados ≤ 2400 px.
- Descripción de la evidencia: obligatoria, ≤ **2000** caracteres Unicode. Anotaciones: JSON `{ "version": 1, "crop": …, "shapes": [...] }`, ≤ 512 KB y ≤ 500 formas.
- Rutas en la base de datos **relativas** al directorio de datos (`evidence/<proyecto>/<id>.orig.png`); los DTO devuelven rutas absolutas.
- **Nunca nombrar un directorio `evidence/`** dentro del repo: el `.gitignore` lo ignora. Usar `src-tauri/src/media/`, `src/ui/gallery/` y `src/ui/annotator/`. Los *archivos* `evidence.rs` / `evidence.ts` sí se pueden usar.
- Capabilities sin cambios (`["core:default"]`): todo el I/O pasa por comandos propios con rutas validadas (spec §9). El protocolo `asset:` queda limitado a `$APPDATA/evidence/**`.
- Fixtures y tests con datos ficticios ("ACME", "Recibos", "Pólizas"). Nunca datos ni capturas reales de clientes.
- El fixture `src/infrastructure/tauri/__fixtures__/contracts.json` **sí cambia en esta fase**, y solo se regenera con `UPDATE_CONTRACTS=1 cargo test contract`.
- Cero warnings: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `pnpm typecheck`, `pnpm test` y `pnpm build` en verde al cerrar cada task.

## Review Focus

1. **Testers con datos de F1 que actualizan a F2:** la migración `0002` corre sobre una base con proyectos y casos. Debe dejar antes una copia en `backups/`, aplicar la migración y conservar los datos → test en Task 1.
2. **Reintentar "Guardar" después de un fallo** (disco lleno, base bloqueada): no deben quedar archivos huérfanos y la imagen debe seguir en `_staging/` para que el segundo intento funcione → test en Task 4.
3. **Ruta manipulada desde el webview** (`..\..\qapture.db` o un archivo fuera de `evidence/`) en `read_evidence_image`, `discard_staged_image` o `stagedPath`: se rechaza con "La ruta de la imagen no es válida." → tests en Task 3 y Task 4.
4. **`Ctrl+V` cuando no corresponde:** con un diálogo abierto, o con texto en el portapapeles, la app **no** intercepta el pegado, así que pegar texto en los formularios sigue funcionando → test en Task 11.
5. **Arrastrar una mezcla** de 3 imágenes y un PDF: se encolan las 3 imágenes y un toast avisa que se omitió 1 archivo → test en Task 11.

## File Structure

```
qapture/
├─ package.json                                       (T9 konva, T13 versión)
├─ src/
│  ├─ test/setup.ts                                   (T10) mock global de AnnotatorCanvas
│  ├─ domain/
│  │  ├─ cases.ts                                     (T1) CaseRecord.evidenceCount
│  │  ├─ evidence.ts  evidence.test.ts                (T6)
│  │  └─ annotations.ts  annotations.test.ts          (T6)
│  ├─ ports/repositories.ts                           (T7) EvidenceStore
│  ├─ application/use-cases.ts                        (T11) attachEvidence, reannotateEvidence, renameEvidence
│  ├─ infrastructure/
│  │  ├─ tauri/dto.ts  contract.test.ts               (T1, T7)
│  │  ├─ tauri/client.ts  client.test.ts              (T7) InvokeArgs + callBytes
│  │  ├─ tauri/frame.ts  frame.test.ts                (T7)
│  │  ├─ tauri/repositories.ts                        (T7)
│  │  └─ memory/evidence.ts  evidence.test.ts  repositories.ts   (T1, T7)
│  └─ ui/
│     ├─ annotator/
│     │  ├─ history.ts  geometry.ts  shape-factory.ts  *.test.ts (T8)
│     │  ├─ types.ts  hooks.ts  ShapeNode.tsx  CropMask.tsx  AnnotatorCanvas.tsx  Toolbar.tsx   (T9)
│     │  ├─ __mocks__/AnnotatorCanvas.tsx            (T10)
│     │  └─ AnnotationEditor.tsx  AnnotationEditor.test.tsx      (T10)
│     ├─ gallery/
│     │  ├─ files.ts  files.test.ts  useEvidenceIntake.ts        (T11)
│     │  └─ EvidenceGallery.tsx  EvidenceCard.tsx  EvidenceViewer.tsx  DescriptionDialog.tsx (T12)
│     ├─ store/evidence-slice.ts  evidence-slice.test.ts  workspace.ts   (T11)
│     └─ screens/
│        ├─ EvidenceEditorHost.tsx  Workspace.tsx  Evidence.test.tsx   (T11, T12)
│        └─ CaseDetail.tsx  CaseListPane.tsx          (T11, T12)
└─ src-tauri/
   ├─ Cargo.toml  tauri.conf.json                     (T2, T5, T13)
   └─ src/
      ├─ db/mod.rs  db/migrations/0002_evidence.sql   (T1)
      ├─ dto.rs                                       (T1, T4, T5)
      ├─ media/mod.rs  media/raster.rs  media/storage.rs   (T2, T3)
      ├─ repo/evidence.rs  repo/cases.rs  repo/mod.rs (T1, T4)
      ├─ commands/frame.rs  commands/evidence.rs  commands/cases.rs  commands/projects.rs   (T5)
      ├─ state.rs  lib.rs                             (T5)
```

---

### Task 1: Migración `0002_evidence`, respaldo previo y contador de evidencias por caso

**Files:**
- Create: `src-tauri/src/db/migrations/0002_evidence.sql`
- Modify: `src-tauri/src/db/mod.rs`, `src-tauri/src/dto.rs` (`CaseDto` + contrato), `src-tauri/src/repo/cases.rs`
- Modify: `src/domain/cases.ts`, `src/domain/cases.test.ts`, `src/infrastructure/tauri/dto.ts`, `src/infrastructure/memory/repositories.ts`
- Modify: `docs/superpowers/contracts/README.md`; Create: `docs/superpowers/plans/2026-10-15-f2-evidencias-anotaciones.md` (copia de este plan)

**Interfaces:**
- Produces: tabla `evidence`; `CaseDto.evidence_count: i64` (JSON `evidenceCount`); `CaseRecord.evidenceCount: number`; respaldo automático `backups/qapture-v<N>-<AAAAMMDD-HHMMSS>.db` antes de migrar.

- [ ] **Step 1: Rama y copia del plan**

```bash
git checkout main && git pull
git checkout -b feat/f2-evidencias-anotaciones
cp "$HOME/.claude/plans/cached-sauteeing-peacock.md" docs/superpowers/plans/2026-10-15-f2-evidencias-anotaciones.md
```

En `docs/superpowers/contracts/README.md`, en la tabla **Estado**, cambia la fila F1 a `✅ Cerrada` y la fila F2 a:

```markdown
| F2 | [Evidencias + editor de anotaciones](F2-evidencias-anotaciones.md) | [plan](../plans/2026-10-15-f2-evidencias-anotaciones.md) | 🟡 En curso | `v0.2.0` |
```

- [ ] **Step 2: Tests que fallan (Rust)**

En `src-tauri/src/db/mod.rs`, dentro de `mod tests`, agrega:

```rust
    #[test]
    fn backs_up_an_existing_db_before_migrating() {
        let dir = std::env::temp_dir().join(format!("qapture-test-{}", new_id()));
        let path = dir.join("qapture.db");
        std::fs::create_dir_all(&dir).unwrap();
        {
            let old = Connection::open(&path).unwrap();
            old.execute_batch(MIGRATIONS[0]).unwrap();
            old.pragma_update(None, "user_version", 1).unwrap();
            old.execute(
                "INSERT INTO template (id, name, test_type, builtin, created_at) VALUES ('t-old', 'Vieja', '', 0, '')",
                [],
            )
            .unwrap();
        }
        let conn = open(&path).unwrap();
        assert_eq!(schema_version(&conn).unwrap(), MIGRATIONS.len() as i64);
        let kept: i64 = conn
            .query_row("SELECT COUNT(*) FROM template WHERE id = 't-old'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(kept, 1);
        let backups = || std::fs::read_dir(dir.join("backups")).unwrap().count();
        assert_eq!(backups(), 1);
        drop(conn);
        drop(open(&path).unwrap());
        assert_eq!(backups(), 1, "sin migraciones pendientes no se respalda de nuevo");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_new_db_is_not_backed_up() {
        let dir = std::env::temp_dir().join(format!("qapture-test-{}", new_id()));
        drop(open(&dir.join("qapture.db")).unwrap());
        assert!(!dir.join("backups").exists());
        let _ = std::fs::remove_dir_all(dir);
    }
```

En `src-tauri/src/repo/cases.rs`, dentro de `mod tests`, agrega:

```rust
    #[test]
    fn cases_report_their_evidence_count() {
        let conn = conn();
        let p = project(&conn);
        let c = create(&conn, &p.id, &input(&p, &[("entorno", "QA")])).unwrap();
        assert_eq!(c.evidence_count, 0);
        for position in 0..2 {
            conn.execute(
                "INSERT INTO evidence (id, case_id, position, description, original_path, rendered_path,
                                       annotations, width, height, captured_at, updated_at)
                 VALUES (?1, ?2, ?3, 'd', 'o', 'r', '{}', 1, 1, '', '')",
                params![new_id(), c.id, position],
            )
            .unwrap();
        }
        assert_eq!(get(&conn, &c.id).unwrap().evidence_count, 2);
        assert_eq!(list(&conn, &p.id).unwrap()[0].evidence_count, 2);
        assert_eq!(duplicate(&conn, &c.id).unwrap().evidence_count, 0, "duplicar no copia evidencias (D14)");
        delete(&conn, &c.id).unwrap();
        let rows: i64 = conn.query_row("SELECT COUNT(*) FROM evidence", [], |r| r.get(0)).unwrap();
        assert_eq!(rows, 0, "borrar el caso borra sus filas de evidencia");
    }
```

- [ ] **Step 3: Verificar que fallan**

Run: `cd src-tauri && cargo test db::tests repo::cases::tests 2>&1 | tail -20`
Expected: error de compilación `no field evidence_count on type CaseDto` y fallo de `backs_up_an_existing_db_before_migrating`.

- [ ] **Step 4: Migración**

`src-tauri/src/db/migrations/0002_evidence.sql`:

```sql
CREATE TABLE evidence (
  id TEXT PRIMARY KEY,
  case_id TEXT NOT NULL REFERENCES test_case(id) ON DELETE CASCADE,
  position INTEGER NOT NULL,
  description TEXT NOT NULL,
  original_path TEXT NOT NULL,
  rendered_path TEXT NOT NULL,
  annotations TEXT NOT NULL,
  width INTEGER NOT NULL,
  height INTEGER NOT NULL,
  captured_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX idx_evidence_case ON evidence(case_id, position);
```

- [ ] **Step 5: Respaldo previo a migrar (`src-tauri/src/db/mod.rs`)**

Reemplaza los `use`, `MIGRATIONS` y `open` por:

```rust
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use rusqlite::Connection;

use crate::error::AppResult;

pub mod seed;

const MIGRATIONS: &[&str] = &[
    include_str!("migrations/0001_init.sql"),
    include_str!("migrations/0002_evidence.sql"),
];

pub fn open(path: &Path) -> AppResult<Connection> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let conn = Connection::open(path)?;
    conn.busy_timeout(Duration::from_secs(5))?;
    conn.query_row("PRAGMA journal_mode = WAL", [], |_| Ok(()))?;
    backup_before_migrate(&conn, path)?;
    init(conn)
}

/// Spec §8: antes de migrar una base que ya tiene datos, deja una copia en `backups/`.
fn backup_before_migrate(conn: &Connection, path: &Path) -> AppResult<Option<PathBuf>> {
    let current = schema_version(conn)?;
    if current == 0 || current >= MIGRATIONS.len() as i64 {
        return Ok(None);
    }
    let dir = path
        .parent()
        .map_or_else(|| PathBuf::from("backups"), |p| p.join("backups"));
    std::fs::create_dir_all(&dir)?;
    let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
    let target = dir.join(format!("qapture-v{current}-{stamp}.db"));
    conn.execute("VACUUM INTO ?1", [target.to_string_lossy().into_owned()])?;
    Ok(Some(target))
}
```

- [ ] **Step 6: `evidence_count` en `CaseDto` y en el repositorio de casos**

En `src-tauri/src/dto.rs`, en `CaseDto`, agrega el campo justo después de `values`:

```rust
    pub values: BTreeMap<String, String>,
    pub evidence_count: i64,
```

En el `sample()` del módulo `contract`, en el `CaseDto` agrega `evidence_count: 2,` después de `values: …,`.

En `src-tauri/src/repo/cases.rs`:

```rust
const CASE_COLUMNS: &str = "id, project_id, code, title, status, created_at, updated_at,
    (SELECT COUNT(*) FROM evidence e WHERE e.case_id = test_case.id) AS evidence_count";
```

y en `case_from_row`, después de `values: BTreeMap::new(),`:

```rust
        evidence_count: r.get("evidence_count")?,
```

- [ ] **Step 7: Regenerar el contrato y verificar Rust**

```bash
cd src-tauri
UPDATE_CONTRACTS=1 cargo test contract
cargo test
```
Expected: todos los tests pasan (41 previos + 3 nuevos). `git diff ../src/infrastructure/tauri/__fixtures__/contracts.json` muestra solo `"evidenceCount": 2` en `case`.

- [ ] **Step 8: Lado TS**

`src/domain/cases.ts`, en `CaseRecord`, después de `values`:

```ts
  values: Record<string, string>;
  evidenceCount: number;
```

`src/domain/cases.test.ts`, en el helper `record`, después de `values: {},` agrega `evidenceCount: 0,`.

`src/infrastructure/tauri/dto.ts`, en `caseSchema`, después de `values`:

```ts
  values: z.record(z.string(), z.string()),
  evidenceCount: z.number().int(),
```

`src/infrastructure/memory/repositories.ts`: en `cases.create` el objeto queda `{ id: id("case"), projectId, code: formatCode(p.nextSequence++), ...clean, evidenceCount: 0, createdAt: ts, updatedAt: ts }`, y en `cases.duplicate` agrega `evidenceCount: 0,` después de `values,` (Task 7 calcula el valor real).

- [ ] **Step 9: Verificar TS y commit**

```bash
pnpm typecheck && pnpm test
git add -A docs/superpowers src-tauri/src src/domain src/infrastructure
git commit -m "feat(f2): migración de evidencias, respaldo previo y contador por caso

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: Normalización y validación de imágenes (`media/raster.rs`)

**Files:**
- Modify: `src-tauri/Cargo.toml`, `src-tauri/src/lib.rs` (`mod media;`)
- Create: `src-tauri/src/media/mod.rs`, `src-tauri/src/media/raster.rs`

**Interfaces:**
- Produces:
  - `pub struct NormalizedImage { pub png: Vec<u8>, pub width: u32, pub height: u32 }`
  - `pub fn normalize(bytes: &[u8]) -> AppResult<NormalizedImage>`: valida magic bytes (PNG/JPEG/WebP), ≤ 64 MB y ≤ 12 000 px por lado, aplica la orientación EXIF, reduce a lado mayor 2400 px y codifica a PNG.
  - `pub fn validate_render(bytes: &[u8]) -> AppResult<(u32, u32)>`: exige PNG con lados entre 1 y 2400 px.
  - `pub fn dimensions(bytes: &[u8]) -> AppResult<(u32, u32)>`
  - Constantes `MAX_INPUT_BYTES`, `MAX_INPUT_SIDE = 12_000`, `MAX_STORED_SIDE = 2_400`.
  - `#[cfg(test)] pub(crate) fn sample_png(w: u32, h: u32) -> Vec<u8>` (para tests de otras tasks).

- [ ] **Step 1: Dependencia**

```bash
cd src-tauri
cargo add image@0.25 --no-default-features --features png,jpeg,webp
```

`src-tauri/src/media/mod.rs`:

```rust
pub mod raster;
```

En `src-tauri/src/lib.rs`, agrega `mod media;` después de `mod error;`. Mientras `normalize` no tenga llamadores fuera de los tests (los tiene desde Task 4), agrega `#![allow(dead_code)]` como primera línea de `media/raster.rs` y quítalo en Task 4.

- [ ] **Step 2: Tests que fallan**

`src-tauri/src/media/raster.rs` (por ahora solo los tests y las firmas vacías con `todo!()`):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn encode(img: DynamicImage, format: ImageFormat) -> Vec<u8> {
        let mut out = Vec::new();
        img.write_to(&mut Cursor::new(&mut out), format).unwrap();
        out
    }

    fn rgb(w: u32, h: u32) -> DynamicImage {
        DynamicImage::ImageRgb8(image::RgbImage::from_pixel(w, h, image::Rgb([200, 30, 30])))
    }

    #[test]
    fn keeps_small_images_and_outputs_png() {
        let out = normalize(&sample_png(800, 600)).unwrap();
        assert_eq!((out.width, out.height), (800, 600));
        assert_eq!(image::guess_format(&out.png).unwrap(), ImageFormat::Png);
    }

    #[test]
    fn downscales_the_longest_side_to_2400() {
        let wide = normalize(&sample_png(3000, 1500)).unwrap();
        assert_eq!((wide.width, wide.height), (2400, 1200));
        let tall = normalize(&sample_png(1000, 3000)).unwrap();
        assert_eq!((tall.width, tall.height), (800, 2400));
    }

    #[test]
    fn accepts_jpeg_and_webp() {
        for format in [ImageFormat::Jpeg, ImageFormat::WebP] {
            let out = normalize(&encode(rgb(320, 200), format)).unwrap();
            assert_eq!((out.width, out.height), (320, 200), "{format:?}");
        }
    }

    #[test]
    fn keeps_transparency() {
        let rgba = DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(10, 10, image::Rgba([0, 0, 0, 0])));
        let out = normalize(&encode(rgba, ImageFormat::Png)).unwrap();
        assert!(image::load_from_memory(&out.png).unwrap().color().has_alpha());
    }

    #[test]
    fn rejects_non_images_and_other_formats() {
        let gif = encode(rgb(4, 4), ImageFormat::Gif);
        for bytes in [b"".to_vec(), b"hola mundo".to_vec(), b"%PDF-1.7".to_vec(), gif] {
            assert_eq!(
                normalize(&bytes).err().unwrap().to_string(),
                "El archivo no es una imagen PNG, JPEG o WebP."
            );
        }
    }

    #[test]
    fn rejects_more_than_12000_px_per_side() {
        let err = normalize(&sample_png(12_001, 1)).err().unwrap().to_string();
        assert_eq!(err, "La imagen mide 12001×1 px; el máximo es 12000 px por lado.");
    }

    #[test]
    fn validate_render_requires_png_up_to_2400() {
        assert_eq!(validate_render(&sample_png(2400, 10)).unwrap(), (2400, 10));
        assert!(validate_render(&sample_png(2401, 10)).is_err());
        assert!(validate_render(&encode(rgb(10, 10), ImageFormat::Jpeg)).is_err());
        assert!(validate_render(b"").is_err());
    }
}
```

Para que el test del GIF compile, el encoder GIF se necesita solo en tests. En `Cargo.toml` agrega:

```toml
[dev-dependencies]
image = { version = "0.25", default-features = false, features = ["png", "jpeg", "webp", "gif"] }
```

- [ ] **Step 3: Verificar que fallan**

Run: `cargo test media::raster 2>&1 | tail -15`
Expected: FAIL (`not yet implemented`).

- [ ] **Step 4: Implementación**

Contenido de `src-tauri/src/media/raster.rs`, arriba del módulo de tests:

```rust
use std::io::Cursor;

use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader, Limits, imageops::FilterType};

use crate::error::{AppError, AppResult, validation};

pub const MAX_INPUT_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_INPUT_SIDE: u32 = 12_000;
pub const MAX_STORED_SIDE: u32 = 2_400;

const UNSUPPORTED: &str = "El archivo no es una imagen PNG, JPEG o WebP.";

pub struct NormalizedImage {
    pub png: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

fn reader(bytes: &[u8]) -> AppResult<ImageReader<Cursor<&[u8]>>> {
    if bytes.len() > MAX_INPUT_BYTES {
        return Err(validation("La imagen supera 64 MB."));
    }
    let format = image::guess_format(bytes).map_err(|_| validation(UNSUPPORTED))?;
    if !matches!(format, ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::WebP) {
        return Err(validation(UNSUPPORTED));
    }
    let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
    let mut limits = Limits::default();
    limits.max_alloc = Some(1024 * 1024 * 1024);
    reader.limits(limits);
    Ok(reader)
}

pub fn dimensions(bytes: &[u8]) -> AppResult<(u32, u32)> {
    reader(bytes)?
        .into_dimensions()
        .map_err(|_| validation(UNSUPPORTED))
}

fn check_side(width: u32, height: u32) -> AppResult<()> {
    if width == 0 || height == 0 {
        return Err(validation(UNSUPPORTED));
    }
    if width > MAX_INPUT_SIDE || height > MAX_INPUT_SIDE {
        return Err(validation(format!(
            "La imagen mide {width}×{height} px; el máximo es {MAX_INPUT_SIDE} px por lado."
        )));
    }
    Ok(())
}

fn encode_png(img: &DynamicImage) -> AppResult<Vec<u8>> {
    let mut png = Vec::new();
    img.write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
        .map_err(|e| AppError::Io(std::io::Error::other(e.to_string())))?;
    Ok(png)
}

pub fn normalize(bytes: &[u8]) -> AppResult<NormalizedImage> {
    let (width, height) = dimensions(bytes)?;
    check_side(width, height)?;
    let mut decoder = reader(bytes)?
        .into_decoder()
        .map_err(|_| validation(UNSUPPORTED))?;
    let orientation = decoder
        .orientation()
        .unwrap_or(image::metadata::Orientation::NoTransforms);
    let mut img = DynamicImage::from_decoder(decoder)
        .map_err(|_| validation("La imagen está dañada o incompleta."))?;
    // Las fotos de celular traen la rotación en EXIF: se aplica para guardarlas derechas.
    img.apply_orientation(orientation);
    if img.width().max(img.height()) > MAX_STORED_SIDE {
        img = img.resize(MAX_STORED_SIDE, MAX_STORED_SIDE, FilterType::Lanczos3);
    }
    let img = if img.color().has_alpha() {
        DynamicImage::ImageRgba8(img.to_rgba8())
    } else {
        DynamicImage::ImageRgb8(img.to_rgb8())
    };
    Ok(NormalizedImage {
        png: encode_png(&img)?,
        width: img.width(),
        height: img.height(),
    })
}

pub fn validate_render(bytes: &[u8]) -> AppResult<(u32, u32)> {
    let invalid = || validation("El render de la evidencia debe ser un PNG de hasta 2400 px por lado.");
    if image::guess_format(bytes).ok() != Some(ImageFormat::Png) {
        return Err(invalid());
    }
    let (w, h) = dimensions(bytes).map_err(|_| invalid())?;
    if w == 0 || h == 0 || w > MAX_STORED_SIDE || h > MAX_STORED_SIDE {
        return Err(invalid());
    }
    Ok((w, h))
}

#[cfg(test)]
pub(crate) fn sample_png(width: u32, height: u32) -> Vec<u8> {
    let img = DynamicImage::ImageRgb8(image::RgbImage::from_pixel(width, height, image::Rgb([40, 98, 158])));
    encode_png(&img).unwrap()
}
```

Si `cargo` informa que `into_decoder`, `orientation` o `apply_orientation` no existen, la versión resuelta de `image` es anterior a 0.25.4: ejecuta `cargo update -p image`.

- [ ] **Step 5: Verificar y commit**

```bash
cargo test media::raster
cargo clippy --all-targets -- -D warnings && cargo fmt --check
git add Cargo.toml Cargo.lock src/lib.rs src/media
git commit -m "feat(f2): normalización y validación de imágenes de evidencia

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: Almacenamiento de archivos (`media/storage.rs`)

**Files:**
- Create: `src-tauri/src/media/storage.rs`; Modify: `src-tauri/src/media/mod.rs`

**Interfaces:**
- Produces:
  - `pub struct EvidenceFiles` con `new(data_dir)`, `root()`, `staging_dir()`, `project_dir(project_id)`, `original_rel(project_id, id) -> String`, `render_rel(project_id, id) -> String`, `absolute(rel) -> PathBuf`, `display(rel) -> String`, `stage(png) -> AppResult<PathBuf>`, `resolve_inside(raw, staged_only) -> AppResult<PathBuf>`, `discard_staged(raw)`, `clear_staging()`, `remove_files(&[String])`, `remove_project(project_id)` y `sweep(&referenced, &projects) -> AppResult<usize>`.
  - `pub fn write_atomic(path: &Path, bytes: &[u8]) -> AppResult<()>`
  - `#[cfg(test)] pub(crate) struct TempDir` con `new()`, `path()` y `files()`.

- [ ] **Step 1: Tests que fallan**

`src-tauri/src/media/mod.rs`:

```rust
pub mod raster;
pub mod storage;
```

Al final de `src-tauri/src/media/storage.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn put(files: &EvidenceFiles, rel: &str) {
        let path = files.absolute(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b"x").unwrap();
    }

    #[test]
    fn stage_writes_inside_staging_and_can_be_discarded() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let staged = files.stage(b"png").unwrap();
        assert!(staged.starts_with(files.staging_dir()));
        let raw = staged.to_string_lossy().into_owned();
        assert!(files.resolve_inside(&raw, true).is_ok());
        assert!(files.resolve_inside(&raw, false).is_ok());
        files.discard_staged(&raw).unwrap();
        assert!(!staged.exists());
    }

    #[test]
    fn resolve_rejects_paths_outside_evidence() {
        let tmp = TempDir::new();
        let files = tmp.files();
        files.stage(b"x").unwrap();
        let db = tmp.path().join("qapture.db");
        fs::write(&db, b"db").unwrap();
        let sneaky = files.staging_dir().join("..").join("..").join("qapture.db");
        for raw in [
            db.to_string_lossy().into_owned(),
            sneaky.to_string_lossy().into_owned(),
            "no-existe.png".to_string(),
            files.root().to_string_lossy().into_owned(),
        ] {
            assert_eq!(
                files.resolve_inside(&raw, false).unwrap_err().to_string(),
                "La ruta de la imagen no es válida.",
                "{raw}"
            );
        }
        let rel = EvidenceFiles::original_rel("p1", "e1");
        put(&files, &rel);
        let inside = files.display(&rel);
        assert!(files.resolve_inside(&inside, false).is_ok());
        assert!(files.resolve_inside(&inside, true).is_err(), "solo _staging");
    }

    #[test]
    fn write_atomic_replaces_without_leftovers() {
        let tmp = TempDir::new();
        let path = tmp.path().join("a.png");
        write_atomic(&path, b"uno").unwrap();
        write_atomic(&path, b"dos").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"dos");
        assert_eq!(fs::read_dir(tmp.path()).unwrap().count(), 1);
    }

    #[test]
    fn clear_staging_and_sweep_remove_unreferenced_files() {
        let tmp = TempDir::new();
        let files = tmp.files();
        files.stage(b"x").unwrap();
        let keep = EvidenceFiles::original_rel("p1", "e1");
        let orphan = EvidenceFiles::render_rel("p1", "e2");
        let ghost = EvidenceFiles::original_rel("p2", "e3");
        for rel in [&keep, &orphan, &ghost] {
            put(&files, rel);
        }
        files.clear_staging().unwrap();
        assert!(!files.staging_dir().exists());
        let removed = files
            .sweep(&HashSet::from([keep.clone()]), &HashSet::from(["p1".to_string()]))
            .unwrap();
        assert_eq!(removed, 2);
        assert!(files.absolute(&keep).exists());
        assert!(!files.absolute(&orphan).exists());
        assert!(!files.project_dir("p2").exists());
    }

    #[test]
    fn remove_project_ignores_unsafe_ids() {
        let tmp = TempDir::new();
        let files = tmp.files();
        files.stage(b"x").unwrap();
        for id in ["", "..", "a/b", "a\\b"] {
            files.remove_project(id);
        }
        assert!(files.staging_dir().exists());
    }
}
```

- [ ] **Step 2: Verificar que fallan**

Run: `cargo test media::storage 2>&1 | tail -5`
Expected: error de compilación (`EvidenceFiles` no existe).

- [ ] **Step 3: Implementación** (arriba de los tests en `storage.rs`)

```rust
use std::{
    collections::HashSet,
    fs,
    path::{MAIN_SEPARATOR_STR, Path, PathBuf},
};

use crate::{
    db::new_id,
    error::{AppResult, validation},
};

pub const EVIDENCE_DIR: &str = "evidence";
pub const STAGING_DIR: &str = "_staging";

/// `<datos>/evidence/<proyecto>/<id>.orig.png` y `.render.png`, más `evidence/_staging/`
/// para las imágenes que todavía están abiertas en el editor.
#[derive(Debug, Clone)]
pub struct EvidenceFiles {
    data_dir: PathBuf,
}

impl EvidenceFiles {
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        Self { data_dir: data_dir.into() }
    }

    pub fn root(&self) -> PathBuf {
        self.data_dir.join(EVIDENCE_DIR)
    }

    pub fn staging_dir(&self) -> PathBuf {
        self.root().join(STAGING_DIR)
    }

    pub fn project_dir(&self, project_id: &str) -> PathBuf {
        self.root().join(project_id)
    }

    pub fn original_rel(project_id: &str, id: &str) -> String {
        format!("{EVIDENCE_DIR}/{project_id}/{id}.orig.png")
    }

    pub fn render_rel(project_id: &str, id: &str) -> String {
        format!("{EVIDENCE_DIR}/{project_id}/{id}.render.png")
    }

    pub fn absolute(&self, rel: &str) -> PathBuf {
        self.data_dir.join(rel.replace('/', MAIN_SEPARATOR_STR))
    }

    pub fn display(&self, rel: &str) -> String {
        self.absolute(rel).to_string_lossy().into_owned()
    }

    pub fn stage(&self, png: &[u8]) -> AppResult<PathBuf> {
        let dir = self.staging_dir();
        fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{}.png", new_id()));
        write_atomic(&path, png)?;
        Ok(path)
    }

    /// Valida una ruta que llega del webview: debe ser un archivo existente dentro de
    /// `evidence/` (o de `_staging/` si `staged_only`), resolviendo `..` y enlaces.
    pub fn resolve_inside(&self, raw: &str, staged_only: bool) -> AppResult<PathBuf> {
        let invalid = || validation("La ruta de la imagen no es válida.");
        let base = if staged_only { self.staging_dir() } else { self.root() };
        let base = fs::canonicalize(base).map_err(|_| invalid())?;
        let path = fs::canonicalize(raw).map_err(|_| invalid())?;
        if path.starts_with(&base) && path.is_file() {
            Ok(path)
        } else {
            Err(invalid())
        }
    }

    pub fn discard_staged(&self, raw: &str) -> AppResult<()> {
        fs::remove_file(self.resolve_inside(raw, true)?)?;
        Ok(())
    }

    pub fn clear_staging(&self) -> AppResult<()> {
        let dir = self.staging_dir();
        if dir.exists() {
            fs::remove_dir_all(dir)?;
        }
        Ok(())
    }

    /// Borrado de mejor esfuerzo: la base ya no referencia estos archivos y `sweep` limpia lo que quede.
    pub fn remove_files(&self, rels: &[String]) {
        for rel in rels {
            let _ = fs::remove_file(self.absolute(rel));
        }
    }

    pub fn remove_project(&self, project_id: &str) {
        if project_id.is_empty() || project_id.contains(['/', '\\', '.']) {
            return;
        }
        let _ = fs::remove_dir_all(self.project_dir(project_id));
    }

    /// Borra lo que la base de datos no referencia: restos de fallos, cierres bruscos
    /// o proyectos eliminados. `referenced` son rutas relativas (`evidence/<p>/<archivo>`).
    pub fn sweep(&self, referenced: &HashSet<String>, projects: &HashSet<String>) -> AppResult<usize> {
        let root = self.root();
        if !root.exists() {
            return Ok(0);
        }
        let mut removed = 0;
        for entry in fs::read_dir(&root)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let path = entry.path();
            if name == STAGING_DIR {
                continue;
            }
            if !path.is_dir() || !projects.contains(&name) {
                let _ = if path.is_dir() { fs::remove_dir_all(&path) } else { fs::remove_file(&path) };
                removed += 1;
                continue;
            }
            for file in fs::read_dir(&path)? {
                let file = file?;
                let rel = format!("{EVIDENCE_DIR}/{name}/{}", file.file_name().to_string_lossy());
                if !referenced.contains(&rel) {
                    let _ = fs::remove_file(file.path());
                    removed += 1;
                }
            }
        }
        Ok(removed)
    }
}

/// Escribe en un temporal del mismo directorio y renombra: nunca deja un PNG a medias.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> AppResult<()> {
    let tmp = path.with_extension(format!("{}.tmp", new_id()));
    fs::write(&tmp, bytes)?;
    if let Err(error) = fs::rename(&tmp, path) {
        let _ = fs::remove_file(&tmp);
        return Err(error.into());
    }
    Ok(())
}

#[cfg(test)]
pub(crate) struct TempDir(PathBuf);

#[cfg(test)]
impl TempDir {
    pub fn new() -> Self {
        let path = std::env::temp_dir().join(format!("qapture-test-{}", new_id()));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }

    pub fn files(&self) -> EvidenceFiles {
        EvidenceFiles::new(&self.0)
    }
}

#[cfg(test)]
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
```

Si clippy marca `dead_code` en `storage.rs` antes de Task 4, agrega `#![allow(dead_code)]` como primera línea y quítalo en Task 4.

- [ ] **Step 4: Verificar y commit**

```bash
cargo test media::storage
cargo clippy --all-targets -- -D warnings && cargo fmt --check
git add src/media
git commit -m "feat(f2): almacenamiento de evidencias con rutas validadas y limpieza de huérfanos

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: Repositorio de evidencias (archivos + transacción, borrado en cascada)

**Files:**
- Create: `src-tauri/src/repo/evidence.rs`
- Modify: `src-tauri/src/repo/mod.rs` (`pub mod evidence;` + helper `case`), `src-tauri/src/dto.rs` (DTO de evidencia), `src-tauri/src/media/*.rs` (quitar `#![allow(dead_code)]`)

**Interfaces:**
- Consumes: `raster::{normalize, dimensions, validate_render}`, `EvidenceFiles`, `write_atomic`, `cases::delete`, `projects::{delete, touch}`.
- Produces (DTO en `dto.rs`, `#[serde(rename_all = "camelCase")]`):
  - `EvidenceDto { id, case_id, position: i64, description, original_path, rendered_path, annotations: String, width: i64, height: i64, captured_at, updated_at }` (rutas absolutas).
  - `StagedImageDto { path, width: i64, height: i64 }`
  - `NewEvidenceMeta { case_id, staged_path, description, annotations }` y `EvidenceUpdateMeta { evidence_id, description, annotations }` (`Deserialize`, `deny_unknown_fields`).
  - Funciones `repo::evidence::{stage, get, list, add, update, update_description, move_to_case, reorder, delete, delete_case, delete_project, sweep_orphans}`, con las firmas de Step 4.

- [ ] **Step 1: DTO y helper de tests**

En `src-tauri/src/dto.rs`, después de `CaseInput`:

```rust
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceDto {
    pub id: String,
    pub case_id: String,
    pub position: i64,
    pub description: String,
    pub original_path: String,
    pub rendered_path: String,
    pub annotations: String,
    pub width: i64,
    pub height: i64,
    pub captured_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StagedImageDto {
    pub path: String,
    pub width: i64,
    pub height: i64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NewEvidenceMeta {
    pub case_id: String,
    pub staged_path: String,
    pub description: String,
    pub annotations: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EvidenceUpdateMeta {
    pub evidence_id: String,
    pub description: String,
    pub annotations: String,
}
```

En `src-tauri/src/repo/mod.rs`: agrega `pub mod evidence;` y, dentro de `test_support`, ajusta los imports y agrega el helper:

```rust
    use std::collections::BTreeMap;

    use rusqlite::{Connection, params};

    use crate::{
        db,
        dto::{CaseDto, CaseInput, NewProjectInput, ProjectDto, ProjectMetaInput},
    };

    // …funciones existentes…

    pub fn case(conn: &Connection, project: &ProjectDto) -> CaseDto {
        let values = BTreeMap::from([(field_id(project, "entorno"), "QA".to_string())]);
        super::cases::create(
            conn,
            &project.id,
            &CaseInput {
                title: "Aplicación de recibos".into(),
                status: "No ejecutado".into(),
                values,
            },
        )
        .expect("caso")
    }

    pub fn other_project(conn: &Connection) -> ProjectDto {
        super::projects::create(
            conn,
            NewProjectInput {
                meta: meta("Pólizas"),
                template_id: "tpl-funcional".into(),
            },
        )
        .expect("proyecto")
    }
```

- [ ] **Step 2: Tests que fallan**

Al final de `src-tauri/src/repo/evidence.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        dto::{CaseDto, ProjectDto},
        media::{raster::sample_png, storage::TempDir},
        repo::{
            cases as case_repo,
            test_support::{case, conn, other_project, project},
        },
    };

    const ANN: &str = r#"{"version":1,"crop":null,"shapes":[]}"#;

    struct Env {
        _tmp: TempDir,
        files: EvidenceFiles,
        conn: Connection,
        project: ProjectDto,
        case: CaseDto,
    }

    fn env() -> Env {
        let tmp = TempDir::new();
        let files = tmp.files();
        let conn = conn();
        let project = project(&conn);
        let case = case(&conn, &project);
        Env { _tmp: tmp, files, conn, project, case }
    }

    fn staged(e: &Env) -> StagedImageDto {
        stage(&e.files, &sample_png(800, 600)).unwrap()
    }

    fn meta(case_id: &str, staged: &StagedImageDto, description: &str) -> NewEvidenceMeta {
        NewEvidenceMeta {
            case_id: case_id.into(),
            staged_path: staged.path.clone(),
            description: description.into(),
            annotations: ANN.into(),
        }
    }

    fn add_one(e: &Env, case_id: &str, description: &str) -> EvidenceDto {
        let s = staged(e);
        add(&e.conn, &e.files, &meta(case_id, &s, description), &sample_png(800, 600)).unwrap()
    }

    fn project_files(e: &Env) -> usize {
        fs::read_dir(e.files.project_dir(&e.project.id)).map(|d| d.count()).unwrap_or(0)
    }

    fn descriptions(e: &Env, case_id: &str) -> Vec<String> {
        list(&e.conn, &e.files, case_id).unwrap().into_iter().map(|x| x.description).collect()
    }

    #[test]
    fn add_moves_the_original_writes_the_render_and_counts() {
        let e = env();
        let s = staged(&e);
        let first = add(&e.conn, &e.files, &meta(&e.case.id, &s, "  Pantalla de recibos  "), &sample_png(400, 300)).unwrap();
        assert_eq!((first.position, first.width, first.height), (0, 800, 600));
        assert_eq!(first.description, "Pantalla de recibos");
        assert!(std::path::Path::new(&first.original_path).exists());
        assert!(std::path::Path::new(&first.rendered_path).exists());
        assert!(!std::path::Path::new(&s.path).exists(), "el staging se consume");
        let second = add_one(&e, &e.case.id, "Segunda");
        assert_eq!(second.position, 1);
        assert_eq!(descriptions(&e, &e.case.id), ["Pantalla de recibos", "Segunda"]);
        assert_eq!(case_repo::get(&e.conn, &e.case.id).unwrap().evidence_count, 2);
    }

    #[test]
    fn add_validates_without_touching_files() {
        let e = env();
        let s = staged(&e);
        let render = sample_png(10, 10);
        let blank = meta(&e.case.id, &s, "  ");
        assert_eq!(add(&e.conn, &e.files, &blank, &render).unwrap_err().to_string(), "«Descripción» es obligatorio.");
        let mut bad_ann = meta(&e.case.id, &s, "ok");
        bad_ann.annotations = r#"{"version":2,"shapes":[]}"#.into();
        assert_eq!(add(&e.conn, &e.files, &bad_ann, &render).unwrap_err().to_string(), "Las anotaciones no tienen un formato válido.");
        let mut outside = meta(&e.case.id, &s, "ok");
        let db = e._tmp.path().join("qapture.db");
        fs::write(&db, b"db").unwrap();
        outside.staged_path = db.to_string_lossy().into_owned();
        assert_eq!(add(&e.conn, &e.files, &outside, &render).unwrap_err().to_string(), "La ruta de la imagen no es válida.");
        assert!(add(&e.conn, &e.files, &meta(&e.case.id, &s, "ok"), b"no-png").is_err());
        assert_eq!(add(&e.conn, &e.files, &meta("no-existe", &s, "ok"), &render).unwrap_err().code(), "not_found");
        assert!(std::path::Path::new(&s.path).exists());
        assert_eq!(project_files(&e), 0);
    }

    #[test]
    fn add_rolls_back_files_when_the_db_fails_and_can_retry() {
        let e = env();
        let s = staged(&e);
        e.conn
            .execute_batch("CREATE TRIGGER fail_insert BEFORE INSERT ON evidence BEGIN SELECT RAISE(ABORT, 'falla simulada'); END;")
            .unwrap();
        assert!(add(&e.conn, &e.files, &meta(&e.case.id, &s, "ok"), &sample_png(10, 10)).is_err());
        assert_eq!(project_files(&e), 0, "sin archivos huérfanos");
        assert!(std::path::Path::new(&s.path).exists(), "el staging vuelve para reintentar");
        e.conn.execute_batch("DROP TRIGGER fail_insert;").unwrap();
        add(&e.conn, &e.files, &meta(&e.case.id, &s, "ok"), &sample_png(10, 10)).unwrap();
        assert_eq!(project_files(&e), 2);
    }

    #[test]
    fn add_leaves_no_row_when_files_fail() {
        let e = env();
        let s = staged(&e);
        fs::create_dir_all(e.files.root()).unwrap();
        fs::write(e.files.project_dir(&e.project.id), b"soy un archivo").unwrap();
        assert!(add(&e.conn, &e.files, &meta(&e.case.id, &s, "ok"), &sample_png(10, 10)).is_err());
        assert_eq!(list(&e.conn, &e.files, &e.case.id).unwrap().len(), 0);
        assert!(std::path::Path::new(&s.path).exists());
    }

    #[test]
    fn update_replaces_the_render_and_keeps_the_original() {
        let e = env();
        let ev = add_one(&e, &e.case.id, "Antes");
        let original = fs::read(&ev.original_path).unwrap();
        let new_render = sample_png(20, 20);
        let ann = r#"{"version":1,"crop":null,"shapes":[{"id":"s1"}]}"#;
        let up = EvidenceUpdateMeta { evidence_id: ev.id.clone(), description: "Después".into(), annotations: ann.into() };
        let saved = update(&e.conn, &e.files, &up, &new_render).unwrap();
        assert_eq!((saved.description.as_str(), saved.annotations.as_str()), ("Después", ann));
        assert_eq!(fs::read(&ev.rendered_path).unwrap(), new_render);
        assert_eq!(fs::read(&ev.original_path).unwrap(), original);
        assert_eq!(project_files(&e), 2, "sin .bak ni .tmp");

        e.conn
            .execute_batch("CREATE TRIGGER fail_update BEFORE UPDATE ON evidence BEGIN SELECT RAISE(ABORT, 'falla simulada'); END;")
            .unwrap();
        assert!(update(&e.conn, &e.files, &up, &sample_png(30, 30)).is_err());
        assert_eq!(fs::read(&ev.rendered_path).unwrap(), new_render, "se restaura el render previo");
        assert_eq!(project_files(&e), 2);
    }

    #[test]
    fn update_description_trims_and_validates() {
        let e = env();
        let ev = add_one(&e, &e.case.id, "Antes");
        assert_eq!(update_description(&e.conn, &e.files, &ev.id, "  Nueva ").unwrap().description, "Nueva");
        assert!(update_description(&e.conn, &e.files, &ev.id, "").is_err());
        assert_eq!(update_description(&e.conn, &e.files, "x", "a").unwrap_err().code(), "not_found");
    }

    #[test]
    fn move_and_reorder_keep_positions_compact() {
        let e = env();
        let other = case(&e.conn, &e.project);
        let a = add_one(&e, &e.case.id, "A");
        let b = add_one(&e, &e.case.id, "B");
        let c = add_one(&e, &e.case.id, "C");
        let moved = move_to_case(&e.conn, &e.files, &b.id, &other.id).unwrap();
        assert_eq!((moved.case_id.as_str(), moved.position), (other.id.as_str(), 0));
        assert_eq!(descriptions(&e, &e.case.id), ["A", "C"]);
        assert_eq!(list(&e.conn, &e.files, &e.case.id).unwrap()[1].position, 1);

        let foreign = case(&e.conn, &other_project(&e.conn));
        assert_eq!(
            move_to_case(&e.conn, &e.files, &a.id, &foreign.id).unwrap_err().to_string(),
            "Solo puedes mover evidencias entre casos de la misma matriz."
        );

        let reordered = reorder(&e.conn, &e.files, &e.case.id, &[c.id.clone(), a.id.clone()]).unwrap();
        assert_eq!(reordered.iter().map(|x| x.description.as_str()).collect::<Vec<_>>(), ["C", "A"]);
        for stale in [vec![c.id.clone()], vec![c.id.clone(), c.id.clone()], vec![c.id.clone(), b.id.clone()]] {
            assert_eq!(
                reorder(&e.conn, &e.files, &e.case.id, &stale).unwrap_err().to_string(),
                "La lista de evidencias cambió. Recarga el caso."
            );
        }
    }

    #[test]
    fn delete_removes_files_and_compacts() {
        let e = env();
        let a = add_one(&e, &e.case.id, "A");
        add_one(&e, &e.case.id, "B");
        delete(&e.conn, &e.files, &a.id).unwrap();
        assert!(!std::path::Path::new(&a.original_path).exists());
        assert!(!std::path::Path::new(&a.rendered_path).exists());
        let rest = list(&e.conn, &e.files, &e.case.id).unwrap();
        assert_eq!((rest.len(), rest[0].position), (1, 0));
        assert_eq!(delete(&e.conn, &e.files, &a.id).unwrap_err().code(), "not_found");
    }

    #[test]
    fn deleting_a_case_or_project_removes_its_files() {
        let e = env();
        let ev = add_one(&e, &e.case.id, "A");
        let keep_case = case(&e.conn, &e.project);
        let kept = add_one(&e, &keep_case.id, "B");
        delete_case(&e.conn, &e.files, &e.case.id).unwrap();
        assert!(!std::path::Path::new(&ev.original_path).exists());
        assert!(std::path::Path::new(&kept.original_path).exists());
        delete_project(&e.conn, &e.files, &e.project.id).unwrap();
        assert!(!e.files.project_dir(&e.project.id).exists());
    }

    #[test]
    fn sweep_orphans_keeps_referenced_files() {
        let e = env();
        let ev = add_one(&e, &e.case.id, "A");
        let stray = e.files.project_dir(&e.project.id).join("perdido.orig.png");
        fs::write(&stray, b"x").unwrap();
        fs::create_dir_all(e.files.project_dir("proyecto-borrado")).unwrap();
        assert_eq!(sweep_orphans(&e.conn, &e.files).unwrap(), 2);
        assert!(std::path::Path::new(&ev.original_path).exists());
        assert!(std::path::Path::new(&ev.rendered_path).exists());
        assert!(!stray.exists());
    }
}
```

- [ ] **Step 3: Verificar que fallan**

Run: `cargo test repo::evidence 2>&1 | tail -5`
Expected: error de compilación (funciones inexistentes).

- [ ] **Step 4: Implementación** (arriba de los tests en `repo/evidence.rs`)

```rust
use std::{collections::HashSet, fs};

use rusqlite::{Connection, OptionalExtension, Row, params};

use super::{cases, projects};
use crate::{
    db::{new_id, now},
    domain::text::required_text,
    dto::{EvidenceDto, EvidenceUpdateMeta, NewEvidenceMeta, StagedImageDto},
    error::{AppError, AppResult, validation},
    media::{
        raster,
        storage::{EvidenceFiles, write_atomic},
    },
};

pub const MAX_DESCRIPTION: usize = 2000;
pub const MAX_ANNOTATIONS_BYTES: usize = 512 * 1024;

const COLUMNS: &str = "id, case_id, position, description, original_path, rendered_path,
    annotations, width, height, captured_at, updated_at";

fn not_found() -> AppError {
    AppError::NotFound("La evidencia no existe.".into())
}

fn from_row(files: &EvidenceFiles, r: &Row) -> rusqlite::Result<EvidenceDto> {
    Ok(EvidenceDto {
        id: r.get("id")?,
        case_id: r.get("case_id")?,
        position: r.get("position")?,
        description: r.get("description")?,
        original_path: files.display(&r.get::<_, String>("original_path")?),
        rendered_path: files.display(&r.get::<_, String>("rendered_path")?),
        annotations: r.get("annotations")?,
        width: r.get("width")?,
        height: r.get("height")?,
        captured_at: r.get("captured_at")?,
        updated_at: r.get("updated_at")?,
    })
}

fn validate_description(raw: &str) -> AppResult<String> {
    required_text("Descripción", raw, MAX_DESCRIPTION)
}

/// Rust solo exige la envoltura versionada; la forma de cada anotación la valida el dominio TS.
fn validate_annotations(raw: &str) -> AppResult<String> {
    if raw.len() > MAX_ANNOTATIONS_BYTES {
        return Err(validation("Las anotaciones superan 512 KB."));
    }
    let invalid = || validation("Las anotaciones no tienen un formato válido.");
    let value: serde_json::Value = serde_json::from_str(raw).map_err(|_| invalid())?;
    let ok = value.get("version").and_then(serde_json::Value::as_i64) == Some(1)
        && value.get("shapes").is_some_and(serde_json::Value::is_array);
    if ok { Ok(raw.to_string()) } else { Err(invalid()) }
}

fn case_project(conn: &Connection, case_id: &str) -> AppResult<String> {
    conn.query_row("SELECT project_id FROM test_case WHERE id = ?1", [case_id], |r| r.get(0))
        .optional()?
        .ok_or_else(|| AppError::NotFound("El caso no existe.".into()))
}

struct Stored {
    case_id: String,
    project_id: String,
    original_rel: String,
    render_rel: String,
}

fn stored(conn: &Connection, id: &str) -> AppResult<Stored> {
    conn.query_row(
        "SELECT e.case_id, c.project_id, e.original_path, e.rendered_path
         FROM evidence e JOIN test_case c ON c.id = e.case_id WHERE e.id = ?1",
        [id],
        |r| {
            Ok(Stored {
                case_id: r.get(0)?,
                project_id: r.get(1)?,
                original_rel: r.get(2)?,
                render_rel: r.get(3)?,
            })
        },
    )
    .optional()?
    .ok_or_else(not_found)
}

fn ids_in(conn: &Connection, case_id: &str) -> AppResult<Vec<String>> {
    let mut stmt = conn.prepare("SELECT id FROM evidence WHERE case_id = ?1 ORDER BY position, captured_at")?;
    let ids = stmt.query_map([case_id], |r| r.get(0))?.collect::<Result<Vec<String>, _>>()?;
    Ok(ids)
}

fn set_positions(conn: &Connection, ids: &[String]) -> AppResult<()> {
    let mut stmt = conn.prepare("UPDATE evidence SET position = ?1 WHERE id = ?2")?;
    for (position, id) in ids.iter().enumerate() {
        stmt.execute(params![position as i64, id])?;
    }
    Ok(())
}

fn compact(conn: &Connection, case_id: &str) -> AppResult<()> {
    set_positions(conn, &ids_in(conn, case_id)?)
}

pub fn get(conn: &Connection, files: &EvidenceFiles, id: &str) -> AppResult<EvidenceDto> {
    conn.query_row(&format!("SELECT {COLUMNS} FROM evidence WHERE id = ?1"), [id], |r| from_row(files, r))
        .optional()?
        .ok_or_else(not_found)
}

pub fn list(conn: &Connection, files: &EvidenceFiles, case_id: &str) -> AppResult<Vec<EvidenceDto>> {
    case_project(conn, case_id)?;
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLUMNS} FROM evidence WHERE case_id = ?1 ORDER BY position, captured_at"
    ))?;
    let rows = stmt
        .query_map([case_id], |r| from_row(files, r))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn stage(files: &EvidenceFiles, bytes: &[u8]) -> AppResult<StagedImageDto> {
    let image = raster::normalize(bytes)?;
    let path = files.stage(&image.png)?;
    Ok(StagedImageDto {
        path: path.to_string_lossy().into_owned(),
        width: image.width.into(),
        height: image.height.into(),
    })
}

pub fn add(conn: &Connection, files: &EvidenceFiles, meta: &NewEvidenceMeta, render: &[u8]) -> AppResult<EvidenceDto> {
    let description = validate_description(&meta.description)?;
    let annotations = validate_annotations(&meta.annotations)?;
    let project_id = case_project(conn, &meta.case_id)?;
    let staged = files.resolve_inside(&meta.staged_path, true)?;
    let (width, height) = raster::dimensions(&fs::read(&staged)?)?;
    raster::validate_render(render)?;

    let id = new_id();
    let original_rel = EvidenceFiles::original_rel(&project_id, &id);
    let render_rel = EvidenceFiles::render_rel(&project_id, &id);
    let original = files.absolute(&original_rel);
    let rendered = files.absolute(&render_rel);
    fs::create_dir_all(files.project_dir(&project_id))?;
    write_atomic(&rendered, render)?;
    if let Err(error) = fs::rename(&staged, &original) {
        let _ = fs::remove_file(&rendered);
        return Err(error.into());
    }
    let saved = (|| -> AppResult<()> {
        let tx = conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO evidence (id, case_id, position, description, original_path, rendered_path,
                                   annotations, width, height, captured_at, updated_at)
             VALUES (?1, ?2, (SELECT COUNT(*) FROM evidence WHERE case_id = ?2), ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
            params![id, meta.case_id, description, original_rel, render_rel, annotations, width, height, now()],
        )?;
        projects::touch(&tx, &project_id)?;
        tx.commit()?;
        Ok(())
    })();
    if let Err(error) = saved {
        // Devolver la imagen a _staging permite reintentar «Guardar» desde el editor.
        let _ = fs::rename(&original, &staged);
        let _ = fs::remove_file(&rendered);
        return Err(error);
    }
    get(conn, files, &id)
}

pub fn update(conn: &Connection, files: &EvidenceFiles, meta: &EvidenceUpdateMeta, render: &[u8]) -> AppResult<EvidenceDto> {
    let s = stored(conn, &meta.evidence_id)?;
    let description = validate_description(&meta.description)?;
    let annotations = validate_annotations(&meta.annotations)?;
    raster::validate_render(render)?;
    let rendered = files.absolute(&s.render_rel);
    let backup = rendered.with_extension("png.bak");
    let had_render = rendered.exists();
    if had_render {
        fs::rename(&rendered, &backup)?;
    }
    let restore = || {
        if had_render {
            let _ = fs::rename(&backup, &rendered);
        } else {
            let _ = fs::remove_file(&rendered);
        }
    };
    if let Err(error) = write_atomic(&rendered, render) {
        restore();
        return Err(error);
    }
    let saved = (|| -> AppResult<()> {
        let tx = conn.unchecked_transaction()?;
        tx.execute(
            "UPDATE evidence SET description = ?1, annotations = ?2, updated_at = ?3 WHERE id = ?4",
            params![description, annotations, now(), meta.evidence_id],
        )?;
        projects::touch(&tx, &s.project_id)?;
        tx.commit()?;
        Ok(())
    })();
    match saved {
        Ok(()) => {
            let _ = fs::remove_file(&backup);
        }
        Err(error) => {
            restore();
            return Err(error);
        }
    }
    get(conn, files, &meta.evidence_id)
}

pub fn update_description(conn: &Connection, files: &EvidenceFiles, id: &str, description: &str) -> AppResult<EvidenceDto> {
    let s = stored(conn, id)?;
    let description = validate_description(description)?;
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "UPDATE evidence SET description = ?1, updated_at = ?2 WHERE id = ?3",
        params![description, now(), id],
    )?;
    projects::touch(&tx, &s.project_id)?;
    tx.commit()?;
    get(conn, files, id)
}

pub fn move_to_case(conn: &Connection, files: &EvidenceFiles, id: &str, target_case_id: &str) -> AppResult<EvidenceDto> {
    let s = stored(conn, id)?;
    if case_project(conn, target_case_id)? != s.project_id {
        return Err(validation("Solo puedes mover evidencias entre casos de la misma matriz."));
    }
    if target_case_id != s.case_id {
        let tx = conn.unchecked_transaction()?;
        tx.execute(
            "UPDATE evidence SET case_id = ?1, updated_at = ?2,
                    position = (SELECT COUNT(*) FROM evidence WHERE case_id = ?1)
             WHERE id = ?3",
            params![target_case_id, now(), id],
        )?;
        compact(&tx, &s.case_id)?;
        projects::touch(&tx, &s.project_id)?;
        tx.commit()?;
    }
    get(conn, files, id)
}

pub fn reorder(conn: &Connection, files: &EvidenceFiles, case_id: &str, ordered: &[String]) -> AppResult<Vec<EvidenceDto>> {
    let project_id = case_project(conn, case_id)?;
    let mut current = ids_in(conn, case_id)?;
    current.sort();
    let mut requested = ordered.to_vec();
    requested.sort();
    if current != requested {
        return Err(validation("La lista de evidencias cambió. Recarga el caso."));
    }
    let tx = conn.unchecked_transaction()?;
    set_positions(&tx, ordered)?;
    projects::touch(&tx, &project_id)?;
    tx.commit()?;
    list(conn, files, case_id)
}

pub fn delete(conn: &Connection, files: &EvidenceFiles, id: &str) -> AppResult<()> {
    let s = stored(conn, id)?;
    let tx = conn.unchecked_transaction()?;
    tx.execute("DELETE FROM evidence WHERE id = ?1", [id])?;
    compact(&tx, &s.case_id)?;
    projects::touch(&tx, &s.project_id)?;
    tx.commit()?;
    files.remove_files(&[s.original_rel, s.render_rel]);
    Ok(())
}

fn case_files(conn: &Connection, case_id: &str) -> AppResult<Vec<String>> {
    let mut stmt = conn.prepare("SELECT original_path, rendered_path FROM evidence WHERE case_id = ?1")?;
    let mut out = Vec::new();
    for row in stmt.query_map([case_id], |r| Ok([r.get::<_, String>(0)?, r.get::<_, String>(1)?]))? {
        out.extend(row?);
    }
    Ok(out)
}

pub fn delete_case(conn: &Connection, files: &EvidenceFiles, case_id: &str) -> AppResult<()> {
    let paths = case_files(conn, case_id)?;
    cases::delete(conn, case_id)?;
    files.remove_files(&paths);
    Ok(())
}

pub fn delete_project(conn: &Connection, files: &EvidenceFiles, project_id: &str) -> AppResult<()> {
    projects::delete(conn, project_id)?;
    files.remove_project(project_id);
    Ok(())
}

/// Al arrancar: borra archivos de `evidence/` que la base no referencia.
pub fn sweep_orphans(conn: &Connection, files: &EvidenceFiles) -> AppResult<usize> {
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
    files.sweep(&referenced, &projects)
}
```

Quita los `#![allow(dead_code)]` provisionales de `media/raster.rs` y `media/storage.rs`. Si clippy aún marca código sin usar porque los comandos llegan en Task 5, agrega `#[allow(dead_code)]` solo en el `mod evidence;` y quítalo en Task 5.

- [ ] **Step 5: Verificar y commit**

```bash
cargo test
cargo clippy --all-targets -- -D warnings && cargo fmt --check
git add src
git commit -m "feat(f2): repositorio de evidencias atómico con borrado en cascada de archivos

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: Comandos, IPC binario, protocolo `asset:` y contrato

**Files:**
- Create: `src-tauri/src/commands/frame.rs`, `src-tauri/src/commands/evidence.rs`
- Modify: `src-tauri/src/commands/mod.rs`, `commands/cases.rs`, `commands/projects.rs`, `src-tauri/src/state.rs`, `src-tauri/src/lib.rs`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `src-tauri/src/dto.rs` (contrato)

**Interfaces:**
- Consumes: `repo::evidence::*`, `EvidenceFiles`.
- Produces (comandos Tauri; argumentos JSON en camelCase):
  - `stage_evidence_image` (cuerpo crudo = bytes de la imagen) → `StagedImageDto`
  - `discard_staged_image { path }` → `null`
  - `read_evidence_image { path }` → bytes (`ArrayBuffer` en JS)
  - `list_evidence { caseId }` → `EvidenceDto[]`
  - `add_evidence` (cuerpo crudo = frame con `NewEvidenceMeta` + PNG del render) → `EvidenceDto`
  - `update_evidence` (frame con `EvidenceUpdateMeta` + PNG) → `EvidenceDto`
  - `update_evidence_description { evidenceId, description }` → `EvidenceDto`
  - `move_evidence { evidenceId, targetCaseId }` → `EvidenceDto`
  - `reorder_evidence { caseId, orderedIds }` → `EvidenceDto[]`
  - `delete_evidence { evidenceId }` → `null`
  - Frame binario: `[u32 little-endian = largo del JSON][JSON utf-8][bytes]`.
  - `AppState::files() -> &EvidenceFiles`.

- [ ] **Step 1: Tests del frame (fallan)**

`src-tauri/src/commands/frame.rs`, al final:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::NewEvidenceMeta;

    fn frame(meta: &str, payload: &[u8]) -> Vec<u8> {
        let mut out = (meta.len() as u32).to_le_bytes().to_vec();
        out.extend_from_slice(meta.as_bytes());
        out.extend_from_slice(payload);
        out
    }

    #[test]
    fn splits_meta_and_payload() {
        let body = frame(
            r#"{"caseId":"c","stagedPath":"p","description":"Descripción ✅","annotations":"{}"}"#,
            &[1, 2, 3],
        );
        let (meta, payload) = split::<NewEvidenceMeta>(&body).unwrap();
        assert_eq!(meta.description, "Descripción ✅");
        assert_eq!(payload, &[1, 2, 3]);
    }

    #[test]
    fn rejects_truncated_invalid_or_unknown_frames() {
        let mut too_long = frame("{}", b"");
        too_long[0] = 200;
        for body in [vec![], vec![1, 0], frame("{", b""), too_long] {
            assert!(split::<serde_json::Value>(&body).is_err());
        }
        let extra = frame(r#"{"caseId":"c","stagedPath":"p","description":"d","annotations":"{}","x":1}"#, b"");
        assert!(split::<NewEvidenceMeta>(&extra).is_err());
    }
}
```

- [ ] **Step 2: Verificar que fallan**

Run: `cargo test commands::frame 2>&1 | tail -5`
Expected: error de compilación (`split` no existe).

- [ ] **Step 3: Frame**

Arriba de los tests en `frame.rs`:

```rust
use serde::de::DeserializeOwned;
use tauri::ipc::{InvokeBody, Request};

use crate::error::{AppResult, validation};

/// Cuerpo crudo de un `invoke` binario (las imágenes no viajan como JSON).
pub fn raw<'a>(request: &'a Request<'_>) -> AppResult<&'a [u8]> {
    match request.body() {
        InvokeBody::Raw(bytes) => Ok(bytes),
        InvokeBody::Json(_) => Err(validation("Se esperaba contenido binario.")),
    }
}

/// `[u32 LE largo][JSON][bytes]`: metadatos tipados más un PNG en una sola llamada.
pub fn split<T: DeserializeOwned>(body: &[u8]) -> AppResult<(T, &[u8])> {
    let bad = || validation("La solicitud binaria no tiene el formato esperado.");
    let header: [u8; 4] = body.get(..4).ok_or_else(bad)?.try_into().map_err(|_| bad())?;
    let len = u32::from_le_bytes(header) as usize;
    let end = 4usize.checked_add(len).ok_or_else(bad)?;
    let json = body.get(4..end).ok_or_else(bad)?;
    let meta = serde_json::from_slice(json)
        .map_err(|e| validation(format!("La solicitud binaria no tiene el formato esperado: {e}")))?;
    Ok((meta, &body[end..]))
}
```

- [ ] **Step 4: Estado, comandos y cascadas**

`src-tauri/src/state.rs`:

```rust
use std::sync::{Mutex, MutexGuard};

use rusqlite::Connection;

use crate::media::storage::EvidenceFiles;

pub struct AppState {
    db: Mutex<Connection>,
    files: EvidenceFiles,
}

impl AppState {
    pub fn new(conn: Connection, files: EvidenceFiles) -> Self {
        Self {
            db: Mutex::new(conn),
            files,
        }
    }

    /// Una sola conexión serializada: SQLite + WAL es suficiente para una app de escritorio de un usuario.
    pub fn db(&self) -> MutexGuard<'_, Connection> {
        self.db
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn files(&self) -> &EvidenceFiles {
        &self.files
    }
}
```

`src-tauri/src/commands/evidence.rs`:

```rust
use std::fs;

use tauri::{
    State,
    ipc::{Request, Response},
};

use super::frame;
use crate::{
    dto::{EvidenceDto, EvidenceUpdateMeta, NewEvidenceMeta, StagedImageDto},
    error::AppResult,
    repo::evidence,
    state::AppState,
};

#[tauri::command(async)]
pub fn stage_evidence_image(state: State<'_, AppState>, request: Request<'_>) -> AppResult<StagedImageDto> {
    evidence::stage(state.files(), frame::raw(&request)?)
}

#[tauri::command(async)]
pub fn discard_staged_image(state: State<'_, AppState>, path: String) -> AppResult<()> {
    state.files().discard_staged(&path)
}

#[tauri::command(async)]
pub fn read_evidence_image(state: State<'_, AppState>, path: String) -> AppResult<Response> {
    let path = state.files().resolve_inside(&path, false)?;
    Ok(Response::new(fs::read(path)?))
}

#[tauri::command(async)]
pub fn list_evidence(state: State<'_, AppState>, case_id: String) -> AppResult<Vec<EvidenceDto>> {
    evidence::list(&state.db(), state.files(), &case_id)
}

#[tauri::command(async)]
pub fn add_evidence(state: State<'_, AppState>, request: Request<'_>) -> AppResult<EvidenceDto> {
    let (meta, render) = frame::split::<NewEvidenceMeta>(frame::raw(&request)?)?;
    evidence::add(&state.db(), state.files(), &meta, render)
}

#[tauri::command(async)]
pub fn update_evidence(state: State<'_, AppState>, request: Request<'_>) -> AppResult<EvidenceDto> {
    let (meta, render) = frame::split::<EvidenceUpdateMeta>(frame::raw(&request)?)?;
    evidence::update(&state.db(), state.files(), &meta, render)
}

#[tauri::command(async)]
pub fn update_evidence_description(
    state: State<'_, AppState>,
    evidence_id: String,
    description: String,
) -> AppResult<EvidenceDto> {
    evidence::update_description(&state.db(), state.files(), &evidence_id, &description)
}

#[tauri::command(async)]
pub fn move_evidence(
    state: State<'_, AppState>,
    evidence_id: String,
    target_case_id: String,
) -> AppResult<EvidenceDto> {
    evidence::move_to_case(&state.db(), state.files(), &evidence_id, &target_case_id)
}

#[tauri::command(async)]
pub fn reorder_evidence(
    state: State<'_, AppState>,
    case_id: String,
    ordered_ids: Vec<String>,
) -> AppResult<Vec<EvidenceDto>> {
    evidence::reorder(&state.db(), state.files(), &case_id, &ordered_ids)
}

#[tauri::command(async)]
pub fn delete_evidence(state: State<'_, AppState>, evidence_id: String) -> AppResult<()> {
    evidence::delete(&state.db(), state.files(), &evidence_id)
}
```

`src-tauri/src/commands/mod.rs`:

```rust
pub mod cases;
pub mod evidence;
mod frame;
pub mod projects;
pub mod templates;
```

En `commands/cases.rs`, `delete_case` pasa a borrar también los archivos (agrega `repo::evidence` a los imports):

```rust
#[tauri::command(async)]
pub fn delete_case(state: State<'_, AppState>, case_id: String) -> AppResult<()> {
    evidence::delete_case(&state.db(), state.files(), &case_id)
}
```

En `commands/projects.rs`, igual con `delete_project` (importa `repo::evidence`):

```rust
#[tauri::command(async)]
pub fn delete_project(state: State<'_, AppState>, id: String) -> AppResult<()> {
    evidence::delete_project(&state.db(), state.files(), &id)
}
```

Si `cases::delete` o `projects::delete` dejan de importarse en esos archivos, quita los `use` que sobren para que clippy quede limpio.

- [ ] **Step 5: Wiring en `lib.rs`**

Reemplaza el `.setup` y agrega los comandos:

```rust
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let conn = db::open(&data_dir.join("qapture.db"))?;
            let files = media::storage::EvidenceFiles::new(&data_dir);
            // Restos de sesiones anteriores: capturas sin guardar y archivos sin fila en la db.
            if let Err(error) = files.clear_staging() {
                eprintln!("No se pudo limpiar evidence/_staging: {error}");
            }
            if let Err(error) = repo::evidence::sweep_orphans(&conn, &files) {
                eprintln!("No se pudieron limpiar evidencias huérfanas: {error}");
            }
            app.manage(state::AppState::new(conn, files));
            system::tray::setup_tray(app)?;
            Ok(())
        })
```

Y en `generate_handler!`, después de `commands::cases::delete_case,`:

```rust
            commands::evidence::stage_evidence_image,
            commands::evidence::discard_staged_image,
            commands::evidence::read_evidence_image,
            commands::evidence::list_evidence,
            commands::evidence::add_evidence,
            commands::evidence::update_evidence,
            commands::evidence::update_evidence_description,
            commands::evidence::move_evidence,
            commands::evidence::reorder_evidence,
            commands::evidence::delete_evidence,
```

- [ ] **Step 6: Protocolo `asset:` y arrastrar archivos al webview**

`src-tauri/Cargo.toml`:

```toml
tauri = { version = "2", features = ["tray-icon", "protocol-asset"] }
```

`src-tauri/tauri.conf.json`: en la ventana agrega `"dragDropEnabled": false`, para que el webview reciba los `drop` de archivos HTML5 (en Windows, el manejador nativo de Tauri los intercepta). En `app.security` agrega:

```json
      "assetProtocol": {
        "enable": true,
        "scope": ["$APPDATA/evidence/**"]
      }
```

(La CSP ya permite `asset:` y `http://asset.localhost` en `img-src`. `capabilities/default.json` no cambia.)

- [ ] **Step 7: Contrato**

En `src-tauri/src/dto.rs`, módulo `contract`:

En `inputs_json()` agrega:

```rust
            "newEvidenceMeta": { "caseId": "case-1", "stagedPath": "C:\\datos\\evidence\\_staging\\stg-1.png",
                                 "description": "Pantalla de recibos", "annotations": ANNOTATIONS },
            "evidenceUpdateMeta": { "evidenceId": "ev-1", "description": "Pantalla de recibos", "annotations": ANNOTATIONS }
```

con la constante al inicio del módulo:

```rust
    const ANNOTATIONS: &str = r#"{"version":1,"crop":null,"shapes":[]}"#;
```

En `sample()`, **antes** de la entrada `"case"` (que consume `ts`):

```rust
            "evidence": EvidenceDto { id: "ev-1".into(), case_id: "case-1".into(), position: 0,
                                      description: "Pantalla de recibos aplicados".into(),
                                      original_path: "C:\\datos\\evidence\\prj-1\\ev-1.orig.png".into(),
                                      rendered_path: "C:\\datos\\evidence\\prj-1\\ev-1.render.png".into(),
                                      annotations: ANNOTATIONS.into(), width: 1920, height: 1080,
                                      captured_at: ts.clone(), updated_at: ts.clone() },
            "stagedImage": StagedImageDto { path: "C:\\datos\\evidence\\_staging\\stg-1.png".into(), width: 1920, height: 1080 },
```

En `inputs_deserialize_from_ts_shape` agrega:

```rust
        serde_json::from_value::<NewEvidenceMeta>(i["newEvidenceMeta"].clone()).unwrap();
        serde_json::from_value::<EvidenceUpdateMeta>(i["evidenceUpdateMeta"].clone()).unwrap();
```

- [ ] **Step 8: Verificar y commit**

```bash
UPDATE_CONTRACTS=1 cargo test contract
cargo test
cargo clippy --all-targets -- -D warnings && cargo fmt --check
cd .. && pnpm test
git add src-tauri src/infrastructure/tauri/__fixtures__/contracts.json
git commit -m "feat(f2): comandos de evidencias con IPC binario y protocolo asset acotado

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

Expected: `cargo test` y `pnpm test` en verde. El build de Tauri valida que `assetProtocol` y la feature `protocol-asset` coincidan.

---

### Task 6: Dominio TS — evidencias y modelo de anotaciones

**Files:**
- Create: `src/domain/evidence.ts`, `src/domain/evidence.test.ts`, `src/domain/annotations.ts`, `src/domain/annotations.test.ts`

**Interfaces:**
- Produces:
  - `evidence.ts`: `Evidence`, `StagedImage`, `NewEvidenceInput { caseId, stagedPath, description, annotations }`, `EvidenceUpdateInput { evidenceId, description, annotations }`, `MAX_DESCRIPTION = 2000`, `ACCEPTED_IMAGE_TYPES`, `isAcceptedImage(file: { type: string })`, `validateDescription(raw): string | null`, `evidenceLabel(e)`, `moveItem<T>(items, from, to)`.
  - `annotations.ts`: `Annotations`, `Shape`, `ShapeKind`, `Rect`, `ANNOTATIONS_VERSION`, `ANNOTATION_COLORS`, `DEFAULT_COLOR`, `annotationsSchema`, `emptyAnnotations()`, `serializeAnnotations(a)`, `parseAnnotations(text): Annotations | null`, `markerNumbers(shapes): Map<string, number>`, `nextShapeId(shapes)`, `addShape`, `updateShape(a, id, next)`, `removeShape`, `setCrop`, `findShape`.

- [ ] **Step 1: Tests que fallan**

`src/domain/evidence.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { evidenceLabel, isAcceptedImage, moveItem, validateDescription } from "./evidence";

describe("evidencias", () => {
  it("exige descripción y limita a 2000 caracteres Unicode", () => {
    expect(validateDescription("  ")).toBe("«Descripción» es obligatorio.");
    expect(validateDescription("✅".repeat(2000))).toBeNull();
    expect(validateDescription("a".repeat(2001))).toBe("«Descripción» supera 2000 caracteres.");
  });

  it("acepta solo PNG, JPEG y WebP", () => {
    expect(["image/png", "image/jpeg", "image/webp"].every((type) => isAcceptedImage({ type }))).toBe(true);
    expect(isAcceptedImage({ type: "application/pdf" })).toBe(false);
    expect(isAcceptedImage({ type: "image/gif" })).toBe(false);
  });

  it("numera desde 1 y mueve elementos sin mutar", () => {
    expect(evidenceLabel({ position: 0 })).toBe("Evidencia 1");
    const ids = ["a", "b", "c"];
    expect(moveItem(ids, 2, 0)).toEqual(["c", "a", "b"]);
    expect(moveItem(ids, 0, 1)).toEqual(["b", "a", "c"]);
    expect(moveItem(ids, 0, 9)).toEqual(ids);
    expect(ids).toEqual(["a", "b", "c"]);
  });
});
```

`src/domain/annotations.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import {
  addShape,
  emptyAnnotations,
  markerNumbers,
  nextShapeId,
  parseAnnotations,
  removeShape,
  serializeAnnotations,
  setCrop,
  updateShape,
  type Annotations,
  type Shape,
} from "./annotations";

const marker = (id: string): Shape => ({ id, kind: "marker", x: 10, y: 10, color: "#E02424", radius: 18 });

const full: Annotations = {
  version: 1,
  crop: { x: 5, y: 5, width: 300, height: 200 },
  shapes: [
    { id: "s1", kind: "arrow", points: [0, 0, 50, 60], color: "#E02424", strokeWidth: 6 },
    marker("s2"),
    { id: "s3", kind: "text", x: 1, y: 2, text: "Monto ✅", color: "#2B629E", fontSize: 28 },
    { id: "s4", kind: "rect", x: 1, y: 1, width: 20, height: 10, color: "#BAC82D", strokeWidth: 4 },
    { id: "s5", kind: "ellipse", x: 1, y: 1, width: 20, height: 10, color: "#BAC82D", strokeWidth: 4 },
    { id: "s6", kind: "highlight", x: 1, y: 1, width: 20, height: 10 },
    { id: "s7", kind: "blur", x: 1, y: 1, width: 20, height: 10 },
  ],
};

describe("anotaciones", () => {
  it("serializar y deserializar da el mismo resultado", () => {
    expect(parseAnnotations(serializeAnnotations(full))).toEqual(full);
    expect(parseAnnotations(serializeAnnotations(emptyAnnotations()))).toEqual(emptyAnnotations());
  });

  it("rechaza JSON inválido, otra versión o formas desconocidas", () => {
    expect(parseAnnotations("no es json")).toBeNull();
    expect(parseAnnotations(JSON.stringify({ ...full, version: 2 }))).toBeNull();
    expect(parseAnnotations(JSON.stringify({ ...full, shapes: [{ id: "x", kind: "estrella" }] }))).toBeNull();
    expect(parseAnnotations(JSON.stringify({ ...full, extra: 1 }))).toBeNull();
  });

  it("numera los marcadores en orden y se reordena al borrar uno", () => {
    let a = emptyAnnotations();
    for (const id of ["m1", "m2", "m3"]) a = addShape(a, marker(id));
    a = addShape(a, full.shapes[0]);
    expect([...markerNumbers(a.shapes)]).toEqual([["m1", 1], ["m2", 2], ["m3", 3]]);
    a = removeShape(a, "m2");
    expect([...markerNumbers(a.shapes)]).toEqual([["m1", 1], ["m3", 2]]);
  });

  it("genera ids únicos y actualiza sin mutar", () => {
    expect(nextShapeId([])).toBe("s1");
    expect(nextShapeId(full.shapes)).toBe("s8");
    const moved = updateShape(full, "s2", { ...marker("s2"), x: 99 });
    expect(moved.shapes[1]).toMatchObject({ x: 99 });
    expect(full.shapes[1]).toMatchObject({ x: 10 });
    expect(setCrop(full, null).crop).toBeNull();
    expect(full.crop).not.toBeNull();
  });
});
```

- [ ] **Step 2: Verificar que fallan**

Run: `pnpm vitest run src/domain/evidence.test.ts src/domain/annotations.test.ts`
Expected: FAIL (módulos inexistentes).

- [ ] **Step 3: `src/domain/evidence.ts`**

```ts
import { charLength } from "./fields";

export interface Evidence {
  id: string;
  caseId: string;
  position: number;
  description: string;
  originalPath: string;
  renderedPath: string;
  annotations: string;
  width: number;
  height: number;
  capturedAt: string;
  updatedAt: string;
}

export interface StagedImage {
  path: string;
  width: number;
  height: number;
}

export interface NewEvidenceInput {
  caseId: string;
  stagedPath: string;
  description: string;
  annotations: string;
}

export interface EvidenceUpdateInput {
  evidenceId: string;
  description: string;
  annotations: string;
}

export const MAX_DESCRIPTION = 2000;
export const ACCEPTED_IMAGE_TYPES = ["image/png", "image/jpeg", "image/webp"] as const;

export const isAcceptedImage = (file: { type: string }) => (ACCEPTED_IMAGE_TYPES as readonly string[]).includes(file.type);

export function validateDescription(raw: string): string | null {
  const value = raw.trim();
  if (!value) return "«Descripción» es obligatorio.";
  if (charLength(value) > MAX_DESCRIPTION) return `«Descripción» supera ${MAX_DESCRIPTION} caracteres.`;
  return null;
}

export const evidenceLabel = (e: Pick<Evidence, "position">) => `Evidencia ${e.position + 1}`;

export function moveItem<T>(items: T[], from: number, to: number): T[] {
  if (from < 0 || from >= items.length || to < 0 || to >= items.length || from === to) return [...items];
  const next = [...items];
  const [item] = next.splice(from, 1);
  next.splice(to, 0, item);
  return next;
}
```

- [ ] **Step 4: `src/domain/annotations.ts`**

```ts
import { z } from "zod";

export const ANNOTATIONS_VERSION = 1 as const;
export const DEFAULT_COLOR = "#E02424";
export const ANNOTATION_COLORS = [
  { name: "Rojo", value: "#E02424" },
  { name: "Azul T4S", value: "#2B629E" },
  { name: "Lima T4S", value: "#BAC82D" },
  { name: "Amarillo", value: "#F5B700" },
  { name: "Carbón", value: "#292B2D" },
  { name: "Blanco", value: "#FFFFFF" },
] as const;

const num = z.number();
const size = z.number().positive();
const color = z.string().regex(/^#[0-9A-Fa-f]{6}$/);
const id = z.string().min(1).max(40);
const rect = { x: num, y: num, width: size, height: size };

const shapeSchema = z.discriminatedUnion("kind", [
  z.strictObject({ id, kind: z.literal("arrow"), points: z.tuple([num, num, num, num]), color, strokeWidth: size }),
  z.strictObject({ id, kind: z.literal("marker"), x: num, y: num, color, radius: size }),
  z.strictObject({ id, kind: z.literal("text"), x: num, y: num, text: z.string().max(500), color, fontSize: size }),
  z.strictObject({ id, kind: z.literal("rect"), ...rect, color, strokeWidth: size }),
  z.strictObject({ id, kind: z.literal("ellipse"), ...rect, color, strokeWidth: size }),
  z.strictObject({ id, kind: z.literal("highlight"), ...rect }),
  z.strictObject({ id, kind: z.literal("blur"), ...rect }),
]);

/** Capa editable sobre el PNG original inmutable (spec §6.4). Coordenadas en píxeles del original. */
export const annotationsSchema = z.strictObject({
  version: z.literal(ANNOTATIONS_VERSION),
  crop: z.strictObject(rect).nullable(),
  shapes: z.array(shapeSchema).max(500),
});

export type Shape = z.infer<typeof shapeSchema>;
export type ShapeKind = Shape["kind"];
export type Annotations = z.infer<typeof annotationsSchema>;
export interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export const emptyAnnotations = (): Annotations => ({ version: ANNOTATIONS_VERSION, crop: null, shapes: [] });

export const serializeAnnotations = (a: Annotations): string => JSON.stringify(a);

export function parseAnnotations(text: string): Annotations | null {
  try {
    const result = annotationsSchema.safeParse(JSON.parse(text));
    return result.success ? result.data : null;
  } catch {
    return null;
  }
}

/** Los marcadores ①②③ se numeran por orden de creación; al borrar uno, los siguientes se renumeran. */
export function markerNumbers(shapes: Shape[]): Map<string, number> {
  const numbers = new Map<string, number>();
  for (const s of shapes) if (s.kind === "marker") numbers.set(s.id, numbers.size + 1);
  return numbers;
}

export function nextShapeId(shapes: Shape[]): string {
  const max = shapes.reduce((m, s) => {
    const n = Number(s.id.slice(1));
    return s.id.startsWith("s") && Number.isInteger(n) && n > m ? n : m;
  }, 0);
  return `s${max + 1}`;
}

export const addShape = (a: Annotations, shape: Shape): Annotations => ({ ...a, shapes: [...a.shapes, shape] });
export const updateShape = (a: Annotations, shapeId: string, next: Shape): Annotations => ({
  ...a,
  shapes: a.shapes.map((s) => (s.id === shapeId ? next : s)),
});
export const removeShape = (a: Annotations, shapeId: string): Annotations => ({ ...a, shapes: a.shapes.filter((s) => s.id !== shapeId) });
export const setCrop = (a: Annotations, crop: Rect | null): Annotations => ({ ...a, crop });
export const findShape = (a: Annotations, shapeId: string): Shape | null => a.shapes.find((s) => s.id === shapeId) ?? null;
```

- [ ] **Step 5: Verificar y commit**

```bash
pnpm vitest run src/domain && pnpm typecheck
git add src/domain
git commit -m "feat(f2): dominio de evidencias y modelo de anotaciones versionado

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 7: Puerto `EvidenceStore`, adaptador Tauri (frame + binario) y adaptador en memoria

**Files:**
- Modify: `src/ports/repositories.ts`, `src/infrastructure/tauri/client.ts`, `client.test.ts`, `dto.ts`, `contract.test.ts`, `repositories.ts`, `src/infrastructure/memory/repositories.ts`
- Create: `src/infrastructure/tauri/frame.ts`, `frame.test.ts`, `src/infrastructure/memory/evidence.ts`, `src/infrastructure/memory/evidence.test.ts`

**Interfaces:**
- Consumes: tipos de `domain/evidence.ts`; comandos de Task 5.
- Produces:

```ts
export interface EvidenceStore {
  stage(image: Uint8Array): Promise<StagedImage>;
  discardStaged(path: string): Promise<void>;
  readImage(path: string): Promise<Uint8Array>;
  imageUrl(path: string, version: string): string;
  list(caseId: string): Promise<Evidence[]>;
  add(input: NewEvidenceInput, render: Uint8Array): Promise<Evidence>;
  update(input: EvidenceUpdateInput, render: Uint8Array): Promise<Evidence>;
  updateDescription(evidenceId: string, description: string): Promise<Evidence>;
  move(evidenceId: string, targetCaseId: string): Promise<Evidence>;
  reorder(caseId: string, orderedIds: string[]): Promise<Evidence[]>;
  remove(evidenceId: string): Promise<void>;
}
```

`Repositories` agrega `evidence: EvidenceStore`. También: `encodeFrame(meta, payload): Uint8Array`, `callBytes(command, args): Promise<Uint8Array>`, `evidenceSchema`, `stagedImageSchema` y `createMemoryEvidence(deps)`.

- [ ] **Step 1: Tests que fallan**

`src/infrastructure/tauri/frame.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { encodeFrame } from "./frame";

describe("encodeFrame", () => {
  it("antepone el largo del JSON en u32 little-endian", () => {
    const frame = encodeFrame({ description: "Recibo ✅" }, new Uint8Array([9, 8]));
    const len = new DataView(frame.buffer).getUint32(0, true);
    const json = new TextDecoder().decode(frame.slice(4, 4 + len));
    expect(JSON.parse(json)).toEqual({ description: "Recibo ✅" });
    expect([...frame.slice(4 + len)]).toEqual([9, 8]);
  });
});
```

En `src/infrastructure/tauri/client.test.ts`: cambia el import a `const { call, callBytes } = await import("./client");` y agrega:

```ts
  it("pasa cuerpos binarios tal cual", async () => {
    invoke.mockResolvedValue({ ok: 1 });
    const body = new Uint8Array([1, 2]);
    await call("cmd", body, z.strictObject({ ok: z.number() }));
    expect(invoke).toHaveBeenCalledWith("cmd", body);
  });

  it("callBytes devuelve Uint8Array y rechaza respuestas no binarias", async () => {
    invoke.mockResolvedValue(new Uint8Array([7, 7]).buffer);
    await expect(callBytes("img", { path: "x" })).resolves.toEqual(new Uint8Array([7, 7]));
    invoke.mockResolvedValue({ no: "binario" });
    await expect(callBytes("img", {})).rejects.toMatchObject({ code: "contract" });
  });
```

En `src/infrastructure/tauri/contract.test.ts`: importa `evidenceSchema, stagedImageSchema`, agrega `["evidence", evidenceSchema], ["stagedImage", stagedImageSchema],` al `it.each`, y en el test TS→Rust agrega:

```ts
    const newEvidence: NewEvidenceInput = { caseId: "", stagedPath: "", description: "", annotations: "" };
    const evidenceUpdate: EvidenceUpdateInput = { evidenceId: "", description: "", annotations: "" };
    expect(sortedKeys(newEvidence)).toEqual(sortedKeys(fixture.inputs.newEvidenceMeta));
    expect(sortedKeys(evidenceUpdate)).toEqual(sortedKeys(fixture.inputs.evidenceUpdateMeta));
```

(con `import type { EvidenceUpdateInput, NewEvidenceInput } from "../../domain/evidence";`).

`src/infrastructure/memory/evidence.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { emptyAnnotations, serializeAnnotations } from "../../domain/annotations";
import { emptyMeta } from "../../domain/project";
import { createMemoryRepositories } from "./repositories";

const PNG = new Uint8Array([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
const ANN = serializeAnnotations(emptyAnnotations());

async function setup() {
  const repos = createMemoryRepositories();
  const p = await repos.projects.create({ meta: { ...emptyMeta(), name: "Recibos" }, templateId: "tpl-funcional" });
  const entorno = p.fields.find((f) => f.key === "entorno")!.id;
  const newCase = (title: string) => repos.cases.create(p.id, { title, status: "No ejecutado", values: { [entorno]: "QA" } });
  const add = async (caseId: string, description: string) => {
    const staged = await repos.evidence.stage(PNG);
    return repos.evidence.add({ caseId, stagedPath: staged.path, description, annotations: ANN }, PNG);
  };
  return { repos, p, newCase, add };
}

describe("evidencias en memoria", () => {
  it("valida como Rust y cuenta por caso", async () => {
    const { repos, p, newCase, add } = await setup();
    const c = await newCase("Recibos");
    await expect(repos.evidence.stage(new Uint8Array([1, 2, 3]))).rejects.toMatchObject({ message: "El archivo no es una imagen PNG, JPEG o WebP." });
    const staged = await repos.evidence.stage(PNG);
    await expect(repos.evidence.add({ caseId: c.id, stagedPath: staged.path, description: " ", annotations: ANN }, PNG)).rejects.toMatchObject({
      message: "«Descripción» es obligatorio.",
    });
    const e = await add(c.id, "Pantalla");
    expect(e.position).toBe(0);
    expect((await repos.cases.list(p.id))[0].evidenceCount).toBe(1);
    await expect(repos.evidence.readImage(e.renderedPath)).resolves.toEqual(PNG);
  });

  it("mueve dentro del proyecto, reordena y borra en cascada", async () => {
    const { repos, p, newCase, add } = await setup();
    const a = await newCase("A");
    const b = await newCase("B");
    const e1 = await add(a.id, "uno");
    const e2 = await add(a.id, "dos");
    await repos.evidence.move(e1.id, b.id);
    expect((await repos.evidence.list(a.id)).map((e) => [e.description, e.position])).toEqual([["dos", 0]]);
    const other = await repos.projects.create({ meta: { ...emptyMeta(), name: "Otra" }, templateId: "tpl-funcional" });
    const foreign = await repos.cases.create(other.id, { title: "X", status: "No ejecutado", values: { [other.fields.find((f) => f.key === "entorno")!.id]: "QA" } });
    await expect(repos.evidence.move(e2.id, foreign.id)).rejects.toMatchObject({ message: "Solo puedes mover evidencias entre casos de la misma matriz." });
    await expect(repos.evidence.reorder(a.id, [])).rejects.toMatchObject({ message: "La lista de evidencias cambió. Recarga el caso." });
    await repos.cases.remove(b.id);
    await expect(repos.evidence.readImage(e1.renderedPath)).rejects.toMatchObject({ code: "not_found" });
    await repos.projects.remove(p.id);
    await expect(repos.evidence.readImage(e2.renderedPath)).rejects.toMatchObject({ code: "not_found" });
  });
});
```

- [ ] **Step 2: Verificar que fallan**

Run: `pnpm vitest run src/infrastructure`
Expected: FAIL (módulos y exports inexistentes).

- [ ] **Step 3: Puerto y frame**

`src/ports/repositories.ts`: agrega los imports de `../domain/evidence` y la interfaz `EvidenceStore` de **Interfaces**. A `Repositories` agrega `evidence: EvidenceStore;`.

`src/infrastructure/tauri/frame.ts`:

```ts
/** `[u32 LE largo][JSON utf-8][bytes]`: el formato que espera `commands/frame.rs`. */
export function encodeFrame(meta: unknown, payload: Uint8Array): Uint8Array {
  const json = new TextEncoder().encode(JSON.stringify(meta));
  const out = new Uint8Array(4 + json.length + payload.length);
  new DataView(out.buffer).setUint32(0, json.length, true);
  out.set(json, 4);
  out.set(payload, 4 + json.length);
  return out;
}
```

- [ ] **Step 4: Cliente binario (`client.ts`, reemplazar completo)**

```ts
import { invoke, type InvokeArgs } from "@tauri-apps/api/core";
import type { z } from "zod";
import { AppError } from "../../ports/errors";
import { appErrorSchema } from "./dto";

async function invokeCore(command: string, args: InvokeArgs): Promise<unknown> {
  try {
    return await invoke(command, args);
  } catch (error) {
    const parsed = appErrorSchema.safeParse(error);
    if (parsed.success) throw new AppError(parsed.data.code, parsed.data.message, parsed.data.detail);
    throw new AppError("internal", "No se pudo comunicar con el núcleo de Qapture.", String(error));
  }
}

export async function call<S extends z.ZodType>(command: string, args: InvokeArgs, schema: S): Promise<z.infer<S>> {
  const raw = await invokeCore(command, args);
  const result = schema.safeParse(raw);
  if (!result.success) {
    throw new AppError("contract", "La respuesta del núcleo no tiene el formato esperado.", result.error.message);
  }
  return result.data;
}

/** Para comandos que devuelven `tauri::ipc::Response` (bytes crudos). */
export async function callBytes(command: string, args: InvokeArgs): Promise<Uint8Array> {
  const raw = await invokeCore(command, args);
  if (raw instanceof ArrayBuffer) return new Uint8Array(raw);
  if (raw instanceof Uint8Array) return raw;
  throw new AppError("contract", "La respuesta del núcleo no tiene el formato esperado.", "se esperaba contenido binario");
}
```

- [ ] **Step 5: Esquemas y adaptador Tauri**

`src/infrastructure/tauri/dto.ts`:

```ts
export const evidenceSchema = z.strictObject({
  id: z.string(),
  caseId: z.string(),
  position: z.number().int(),
  description: z.string(),
  originalPath: z.string(),
  renderedPath: z.string(),
  annotations: z.string(),
  width: z.number().int(),
  height: z.number().int(),
  capturedAt: z.string(),
  updatedAt: z.string(),
});

export const stagedImageSchema = z.strictObject({
  path: z.string(),
  width: z.number().int(),
  height: z.number().int(),
});
```

`src/infrastructure/tauri/repositories.ts`: importa `convertFileSrc` de `@tauri-apps/api/core`, `callBytes`, `encodeFrame` y los dos esquemas, y agrega al objeto devuelto:

```ts
    evidence: {
      stage: (image) => call("stage_evidence_image", image, stagedImageSchema),
      discardStaged: async (path) => {
        await call("discard_staged_image", { path }, nothing);
      },
      readImage: (path) => callBytes("read_evidence_image", { path }),
      // `?v=` evita que el webview muestre un render viejo tras re-anotar.
      imageUrl: (path, version) => `${convertFileSrc(path)}?v=${encodeURIComponent(version)}`,
      list: (caseId) => call("list_evidence", { caseId }, z.array(evidenceSchema)),
      add: (input, render) => call("add_evidence", encodeFrame(input, render), evidenceSchema),
      update: (input, render) => call("update_evidence", encodeFrame(input, render), evidenceSchema),
      updateDescription: (evidenceId, description) => call("update_evidence_description", { evidenceId, description }, evidenceSchema),
      move: (evidenceId, targetCaseId) => call("move_evidence", { evidenceId, targetCaseId }, evidenceSchema),
      reorder: (caseId, orderedIds) => call("reorder_evidence", { caseId, orderedIds }, z.array(evidenceSchema)),
      remove: async (evidenceId) => {
        await call("delete_evidence", { evidenceId }, nothing);
      },
    },
```

- [ ] **Step 6: Adaptador en memoria**

`src/infrastructure/memory/evidence.ts`:

```ts
import { parseAnnotations } from "../../domain/annotations";
import { validateDescription, type Evidence } from "../../domain/evidence";
import { AppError } from "../../ports/errors";
import type { EvidenceStore } from "../../ports/repositories";

const startsWith = (b: Uint8Array, sig: number[], offset = 0) => sig.every((v, i) => b[offset + i] === v);
const isPng = (b: Uint8Array) => startsWith(b, [0x89, 0x50, 0x4e, 0x47]);
const isImage = (b: Uint8Array) =>
  isPng(b) || startsWith(b, [0xff, 0xd8, 0xff]) || (startsWith(b, [0x52, 0x49, 0x46, 0x46]) && startsWith(b, [0x57, 0x45, 0x42, 0x50], 8));

export interface MemoryEvidenceDeps {
  caseProject(caseId: string): string;
  touch(projectId: string): void;
  id(prefix: string): string;
  now(): string;
}

export interface MemoryEvidence extends EvidenceStore {
  countFor(caseId: string): number;
  removeForCase(caseId: string): void;
}

/** Replica en memoria las reglas de `repo/evidence.rs` para tests de aplicación y de UI. */
export function createMemoryEvidence(deps: MemoryEvidenceDeps): MemoryEvidence {
  const staged = new Map<string, Uint8Array>();
  const files = new Map<string, Uint8Array>();
  const items = new Map<string, Evidence>();
  const fail = (message: string): never => {
    throw new AppError("validation", message);
  };
  const get = (evidenceId: string) => items.get(evidenceId) ?? fail("La evidencia no existe.");
  const inCase = (caseId: string) => [...items.values()].filter((e) => e.caseId === caseId).sort((a, b) => a.position - b.position);
  const compact = (caseId: string) => inCase(caseId).forEach((e, position) => (e.position = position));
  const check = (description: string, annotations: string, render: Uint8Array) => {
    const message = validateDescription(description);
    if (message) fail(message);
    if (!parseAnnotations(annotations)) fail("Las anotaciones no tienen un formato válido.");
    if (!isPng(render)) fail("El render de la evidencia debe ser un PNG de hasta 2400 px por lado.");
  };
  const copy = (e: Evidence): Evidence => ({ ...e });
  const removeOne = (e: Evidence) => {
    files.delete(e.originalPath);
    files.delete(e.renderedPath);
    items.delete(e.id);
  };

  return {
    async stage(image) {
      if (!isImage(image)) fail("El archivo no es una imagen PNG, JPEG o WebP.");
      const path = `mem/evidence/_staging/${deps.id("stg")}.png`;
      staged.set(path, image);
      return { path, width: 1280, height: 720 };
    },
    async discardStaged(path) {
      staged.delete(path);
    },
    async readImage(path) {
      const bytes = staged.get(path) ?? files.get(path);
      if (!bytes) throw new AppError("not_found", "La imagen no existe.");
      return bytes;
    },
    imageUrl: (path, version) => `memory://${path}?v=${encodeURIComponent(version)}`,
    async list(caseId) {
      deps.caseProject(caseId);
      return inCase(caseId).map(copy);
    },
    async add(input, render) {
      check(input.description, input.annotations, render);
      const projectId = deps.caseProject(input.caseId);
      const original = staged.get(input.stagedPath) ?? fail("La ruta de la imagen no es válida.");
      const evidenceId = deps.id("ev");
      const ts = deps.now();
      const e: Evidence = {
        id: evidenceId,
        caseId: input.caseId,
        position: inCase(input.caseId).length,
        description: input.description.trim(),
        originalPath: `mem/evidence/${projectId}/${evidenceId}.orig.png`,
        renderedPath: `mem/evidence/${projectId}/${evidenceId}.render.png`,
        annotations: input.annotations,
        width: 1280,
        height: 720,
        capturedAt: ts,
        updatedAt: ts,
      };
      files.set(e.originalPath, original);
      files.set(e.renderedPath, render);
      staged.delete(input.stagedPath);
      items.set(e.id, e);
      deps.touch(projectId);
      return copy(e);
    },
    async update(input, render) {
      const e = get(input.evidenceId);
      check(input.description, input.annotations, render);
      files.set(e.renderedPath, render);
      Object.assign(e, { description: input.description.trim(), annotations: input.annotations, updatedAt: deps.now() });
      return copy(e);
    },
    async updateDescription(evidenceId, description) {
      const e = get(evidenceId);
      const message = validateDescription(description);
      if (message) fail(message);
      Object.assign(e, { description: description.trim(), updatedAt: deps.now() });
      return copy(e);
    },
    async move(evidenceId, targetCaseId) {
      const e = get(evidenceId);
      if (deps.caseProject(targetCaseId) !== deps.caseProject(e.caseId)) fail("Solo puedes mover evidencias entre casos de la misma matriz.");
      if (targetCaseId !== e.caseId) {
        const source = e.caseId;
        Object.assign(e, { caseId: targetCaseId, position: inCase(targetCaseId).length, updatedAt: deps.now() });
        compact(source);
      }
      return copy(e);
    },
    async reorder(caseId, orderedIds) {
      deps.caseProject(caseId);
      const current = inCase(caseId).map((e) => e.id).sort();
      if (JSON.stringify(current) !== JSON.stringify([...orderedIds].sort())) fail("La lista de evidencias cambió. Recarga el caso.");
      orderedIds.forEach((evidenceId, position) => (get(evidenceId).position = position));
      return inCase(caseId).map(copy);
    },
    async remove(evidenceId) {
      const e = get(evidenceId);
      removeOne(e);
      compact(e.caseId);
    },
    countFor: (caseId) => inCase(caseId).length,
    removeForCase: (caseId) => inCase(caseId).forEach(removeOne),
  };
}
```

`src/infrastructure/memory/repositories.ts`:
- Importa `createMemoryEvidence`.
- Después de declarar `getCase` y `touch`, crea:

```ts
  const evidence = createMemoryEvidence({
    caseProject: (caseId) => getCase(caseId).projectId,
    touch: (projectId) => touch(getProject(projectId)),
    id,
    now,
  });
  const withCount = (c: CaseRecord): CaseRecord => ({ ...copy(c), evidenceCount: evidence.countFor(c.id) });
```

- En `cases.list` devuelve `projectCases(projectId).map(withCount)`. En `create`, `update` y `duplicate` devuelve `withCount(record)` / `withCount(c)` en lugar de `copy(...)`.
- En `cases.remove`, antes de `cases.delete(caseId)`, llama `evidence.removeForCase(caseId);`. En `projects.remove`, dentro del `for`, llama `evidence.removeForCase(c.id);` antes de `cases.delete(c.id)`.
- Agrega `evidence,` al objeto devuelto.

- [ ] **Step 7: Verificar y commit**

```bash
pnpm test && pnpm typecheck
git add src/ports src/infrastructure
git commit -m "feat(f2): puerto EvidenceStore con adaptadores Tauri (binario) y en memoria

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 8: Núcleo puro del anotador — historial, geometría y fábrica de formas

**Files:**
- Create: `src/ui/annotator/history.ts`, `history.test.ts`, `geometry.ts`, `geometry.test.ts`, `shape-factory.ts`, `shape-factory.test.ts`

**Interfaces:**
- Consumes: `Shape`, `Rect` de `domain/annotations.ts`.
- Produces:
  - `history.ts`: `History<T>`, `createHistory(present)`, `commit(h, next, limit = 100)`, `undo(h)`, `redo(h)`, `canUndo(h)` y `canRedo(h)`.
  - `geometry.ts`: `Point`, `MIN_SIZE = 4`, `clampPoint(p, w, h)`, `rectFromPoints(a, b)`, `clampRect(r, w, h)`, `isMeaningful(r)`, `fitScale(imageW, imageH, boxW, boxH)`, `exportRegion(crop, w, h)` y `pixelSizeFor(r)`.
  - `shape-factory.ts`: `Tool`, `DragTool`, `SizeLevel = "s" | "m" | "l"`, `ToolStyle`, `isDragTool(t)`, `styleFor(w, h, color, level)`, `placeShape("marker" | "text", id, p, style)`, `dragShape(tool, id, start, end, style)`, `isShapeMeaningful(s)`, `shapeOrigin(s)`, `moveShape(s, dx, dy)` y `transformShape(s, { x, y, scaleX, scaleY })`.

- [ ] **Step 1: Tests que fallan**

`src/ui/annotator/history.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { canRedo, canUndo, commit, createHistory, redo, undo } from "./history";

describe("historial", () => {
  it("deshace y rehace; un cambio nuevo descarta el futuro", () => {
    let h = createHistory(0);
    expect(canUndo(h)).toBe(false);
    h = commit(commit(h, 1), 2);
    h = undo(h);
    expect([h.present, canRedo(h)]).toEqual([1, true]);
    h = redo(h);
    expect(h.present).toBe(2);
    h = commit(undo(h), 9);
    expect([h.present, canRedo(h)]).toEqual([9, false]);
    expect(undo(undo(undo(h))).present).toBe(0);
  });

  it("ignora cambios idénticos y respeta el límite", () => {
    const h = createHistory("a");
    expect(commit(h, "a")).toBe(h);
    let long = createHistory(0);
    for (let i = 1; i <= 5; i++) long = commit(long, i, 3);
    expect(long.past).toEqual([2, 3, 4]);
  });
});
```

`src/ui/annotator/geometry.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { clampRect, exportRegion, fitScale, isMeaningful, pixelSizeFor, rectFromPoints } from "./geometry";

describe("geometría", () => {
  it("normaliza un arrastre en cualquier dirección y lo recorta a la imagen", () => {
    expect(rectFromPoints({ x: 50, y: 40 }, { x: 10, y: 0 })).toEqual({ x: 10, y: 0, width: 40, height: 40 });
    expect(clampRect({ x: -10, y: 90, width: 50, height: 50 }, 100, 100)).toEqual({ x: 0, y: 90, width: 40, height: 10 });
    expect(isMeaningful({ x: 0, y: 0, width: 3, height: 30 })).toBe(false);
  });

  it("ajusta la imagen al espacio sin agrandarla", () => {
    expect(fitScale(2400, 1200, 1200, 1000)).toBe(0.5);
    expect(fitScale(400, 300, 1200, 1000)).toBe(1);
    expect(fitScale(400, 300, 0, 0)).toBe(1);
  });

  it("exporta a resolución nativa la imagen completa o el recorte", () => {
    expect(exportRegion(null, 1920, 1080)).toEqual({ x: 0, y: 0, width: 1920, height: 1080 });
    expect(exportRegion({ x: 10.4, y: 20.6, width: 100.2, height: 50.5 }, 1920, 1080)).toEqual({ x: 10, y: 21, width: 100, height: 51 });
    expect(exportRegion({ x: 1900, y: 1000, width: 500, height: 500 }, 1920, 1080)).toEqual({ x: 1900, y: 1000, width: 20, height: 80 });
  });

  it("pixela con bloques grandes para que el texto no se lea", () => {
    expect(pixelSizeFor({ x: 0, y: 0, width: 200, height: 24 })).toBe(10);
    expect(pixelSizeFor({ x: 0, y: 0, width: 300, height: 90 })).toBe(30);
  });
});
```

`src/ui/annotator/shape-factory.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { dragShape, isShapeMeaningful, moveShape, placeShape, shapeOrigin, styleFor, transformShape } from "./shape-factory";

const style = styleFor(2400, 1200, "#E02424", "m");

describe("fábrica de formas", () => {
  it("escala el estilo con el tamaño de la imagen", () => {
    expect(style).toEqual({ color: "#E02424", strokeWidth: 6, fontSize: 40, markerRadius: 27 });
    expect(styleFor(300, 200, "#E02424", "s")).toEqual({ color: "#E02424", strokeWidth: 2, fontSize: 14, markerRadius: 10 });
  });

  it("crea formas por clic y por arrastre", () => {
    expect(placeShape("marker", "s1", { x: 5, y: 6 }, style)).toEqual({ id: "s1", kind: "marker", x: 5, y: 6, color: "#E02424", radius: 27 });
    expect(placeShape("text", "s2", { x: 5, y: 6 }, style)).toMatchObject({ kind: "text", text: "Texto", fontSize: 40 });
    expect(dragShape("arrow", "s3", { x: 1, y: 2 }, { x: 30, y: 40 }, style)).toMatchObject({ points: [1, 2, 30, 40], strokeWidth: 6 });
    expect(dragShape("rect", "s4", { x: 50, y: 50 }, { x: 10, y: 20 }, style)).toMatchObject({ x: 10, y: 20, width: 40, height: 30 });
    expect(dragShape("highlight", "s5", { x: 0, y: 0 }, { x: 9, y: 9 }, style)).toEqual({ id: "s5", kind: "highlight", x: 0, y: 0, width: 9, height: 9 });
  });

  it("descarta formas diminutas", () => {
    expect(isShapeMeaningful(dragShape("rect", "x", { x: 0, y: 0 }, { x: 2, y: 2 }, style))).toBe(false);
    expect(isShapeMeaningful(dragShape("arrow", "x", { x: 0, y: 0 }, { x: 1, y: 1 }, style))).toBe(false);
    expect(isShapeMeaningful(placeShape("marker", "x", { x: 0, y: 0 }, style))).toBe(true);
  });

  it("mueve y transforma según el origen de cada forma", () => {
    const arrow = dragShape("arrow", "a", { x: 0, y: 0 }, { x: 10, y: 10 }, style);
    expect(shapeOrigin(arrow)).toEqual({ x: 0, y: 0 });
    expect(moveShape(arrow, 5, -5)).toMatchObject({ points: [5, -5, 15, 5] });
    const ellipse = dragShape("ellipse", "e", { x: 10, y: 10 }, { x: 50, y: 30 }, style);
    expect(shapeOrigin(ellipse)).toEqual({ x: 30, y: 20 });
    expect(transformShape(ellipse, { x: 30, y: 20, scaleX: 2, scaleY: 1 })).toMatchObject({ x: -10, y: 10, width: 80, height: 20 });
    const rect = dragShape("rect", "r", { x: 0, y: 0 }, { x: 10, y: 10 }, style);
    expect(transformShape(rect, { x: 3, y: 4, scaleX: 3, scaleY: 0.1 })).toMatchObject({ x: 3, y: 4, width: 30, height: 4 });
    const text = placeShape("text", "t", { x: 0, y: 0 }, style);
    expect(transformShape(text, { x: 1, y: 1, scaleX: 9, scaleY: 9 })).toMatchObject({ fontSize: 200 });
  });
});
```

- [ ] **Step 2: Verificar que fallan**

Run: `pnpm vitest run src/ui/annotator`
Expected: FAIL (módulos inexistentes).

- [ ] **Step 3: `history.ts`**

```ts
export interface History<T> {
  past: T[];
  present: T;
  future: T[];
}

export const createHistory = <T>(present: T): History<T> => ({ past: [], present, future: [] });

export function commit<T>(h: History<T>, next: T, limit = 100): History<T> {
  if (Object.is(next, h.present)) return h;
  return { past: [...h.past, h.present].slice(-limit), present: next, future: [] };
}

export function undo<T>(h: History<T>): History<T> {
  if (!h.past.length) return h;
  return { past: h.past.slice(0, -1), present: h.past[h.past.length - 1], future: [h.present, ...h.future] };
}

export function redo<T>(h: History<T>): History<T> {
  if (!h.future.length) return h;
  return { past: [...h.past, h.present], present: h.future[0], future: h.future.slice(1) };
}

export const canUndo = <T>(h: History<T>) => h.past.length > 0;
export const canRedo = <T>(h: History<T>) => h.future.length > 0;
```

- [ ] **Step 4: `geometry.ts`**

```ts
import type { Rect } from "../../domain/annotations";

export interface Point {
  x: number;
  y: number;
}

export const MIN_SIZE = 4;

const clamp = (value: number, min: number, max: number) => Math.min(Math.max(value, min), max);

export const clampPoint = (p: Point, width: number, height: number): Point => ({ x: clamp(p.x, 0, width), y: clamp(p.y, 0, height) });

export const rectFromPoints = (a: Point, b: Point): Rect => ({
  x: Math.min(a.x, b.x),
  y: Math.min(a.y, b.y),
  width: Math.abs(b.x - a.x),
  height: Math.abs(b.y - a.y),
});

export function clampRect(r: Rect, width: number, height: number): Rect {
  const x = clamp(r.x, 0, width);
  const y = clamp(r.y, 0, height);
  return { x, y, width: clamp(r.x + r.width, 0, width) - x, height: clamp(r.y + r.height, 0, height) - y };
}

export const isMeaningful = (r: Rect) => r.width >= MIN_SIZE && r.height >= MIN_SIZE;

/** Escala para mostrar la imagen completa en el espacio disponible, sin agrandarla. */
export function fitScale(imageW: number, imageH: number, boxW: number, boxH: number): number {
  if (imageW <= 0 || imageH <= 0 || boxW <= 0 || boxH <= 0) return 1;
  return Math.min(1, boxW / imageW, boxH / imageH);
}

/** Región a exportar en píxeles enteros del original: todo, o el recorte no destructivo. */
export function exportRegion(crop: Rect | null, width: number, height: number): Rect {
  if (!crop) return { x: 0, y: 0, width, height };
  const c = clampRect(crop, width, height);
  const x = Math.min(Math.round(c.x), width - 1);
  const y = Math.min(Math.round(c.y), height - 1);
  return {
    x,
    y,
    width: Math.max(1, Math.min(width - x, Math.round(c.width))),
    height: Math.max(1, Math.min(height - y, Math.round(c.height))),
  };
}

/** Bloques de al menos 10 px y de un tercio del lado menor: un número de póliza no se puede leer. */
export const pixelSizeFor = (r: Rect) => Math.max(10, Math.round(Math.min(r.width, r.height) / 3));
```

- [ ] **Step 5: `shape-factory.ts`**

```ts
import type { Shape } from "../../domain/annotations";
import { isMeaningful, MIN_SIZE, rectFromPoints, type Point } from "./geometry";

export type Tool = "select" | "arrow" | "marker" | "text" | "rect" | "ellipse" | "highlight" | "blur" | "crop";
export type DragTool = "arrow" | "rect" | "ellipse" | "highlight" | "blur";
export type SizeLevel = "s" | "m" | "l";

export interface ToolStyle {
  color: string;
  strokeWidth: number;
  fontSize: number;
  markerRadius: number;
}

const DRAG_TOOLS: readonly Tool[] = ["arrow", "rect", "ellipse", "highlight", "blur"];
const FACTOR: Record<SizeLevel, number> = { s: 0.6, m: 1, l: 1.6 };
const clamp = (value: number, min: number, max: number) => Math.min(Math.max(value, min), max);

export const isDragTool = (tool: Tool): tool is DragTool => DRAG_TOOLS.includes(tool);

/** Grosores relativos al tamaño de la imagen, para que se vean igual en una captura chica o en una 4K. */
export function styleFor(imageW: number, imageH: number, color: string, level: SizeLevel): ToolStyle {
  const base = Math.max(imageW, imageH, 1);
  const f = FACTOR[level];
  return {
    color,
    strokeWidth: Math.max(2, Math.round((base / 400) * f)),
    fontSize: Math.max(14, Math.round((base / 60) * f)),
    markerRadius: Math.max(10, Math.round((base / 90) * f)),
  };
}

export function placeShape(tool: "marker" | "text", id: string, p: Point, style: ToolStyle): Shape {
  if (tool === "marker") return { id, kind: "marker", x: p.x, y: p.y, color: style.color, radius: style.markerRadius };
  return { id, kind: "text", x: p.x, y: p.y, text: "Texto", color: style.color, fontSize: style.fontSize };
}

export function dragShape(tool: DragTool, id: string, start: Point, end: Point, style: ToolStyle): Shape {
  if (tool === "arrow") return { id, kind: "arrow", points: [start.x, start.y, end.x, end.y], color: style.color, strokeWidth: style.strokeWidth };
  const r = rectFromPoints(start, end);
  if (tool === "highlight" || tool === "blur") return { id, kind: tool, ...r };
  return { id, kind: tool, ...r, color: style.color, strokeWidth: style.strokeWidth };
}

export function isShapeMeaningful(s: Shape): boolean {
  if (s.kind === "arrow") return Math.hypot(s.points[2] - s.points[0], s.points[3] - s.points[1]) >= MIN_SIZE;
  if (s.kind === "marker" || s.kind === "text") return true;
  return isMeaningful(s);
}

/** Posición del nodo Konva de la forma: las flechas usan sus puntos y las elipses su centro. */
export function shapeOrigin(s: Shape): Point {
  if (s.kind === "arrow") return { x: 0, y: 0 };
  if (s.kind === "ellipse") return { x: s.x + s.width / 2, y: s.y + s.height / 2 };
  return { x: s.x, y: s.y };
}

export function moveShape(s: Shape, dx: number, dy: number): Shape {
  if (s.kind === "arrow") {
    const [x1, y1, x2, y2] = s.points;
    return { ...s, points: [x1 + dx, y1 + dy, x2 + dx, y2 + dy] };
  }
  return { ...s, x: s.x + dx, y: s.y + dy };
}

export function transformShape(s: Shape, t: { x: number; y: number; scaleX: number; scaleY: number }): Shape {
  switch (s.kind) {
    case "arrow":
      return s;
    case "marker":
      return { ...s, x: t.x, y: t.y, radius: clamp(s.radius * Math.max(t.scaleX, t.scaleY), 8, 120) };
    case "text":
      return { ...s, x: t.x, y: t.y, fontSize: clamp(Math.round(s.fontSize * t.scaleY), 10, 200) };
    case "ellipse": {
      const width = Math.max(MIN_SIZE, s.width * t.scaleX);
      const height = Math.max(MIN_SIZE, s.height * t.scaleY);
      return { ...s, x: t.x - width / 2, y: t.y - height / 2, width, height };
    }
    default:
      return { ...s, x: t.x, y: t.y, width: Math.max(MIN_SIZE, s.width * t.scaleX), height: Math.max(MIN_SIZE, s.height * t.scaleY) };
  }
}
```

- [ ] **Step 6: Verificar y commit**

```bash
pnpm vitest run src/ui/annotator && pnpm typecheck
git add src/ui/annotator
git commit -m "feat(f2): núcleo del anotador — historial, geometría y fábrica de formas

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 9: Lienzo Konva — herramientas, selección, desenfoque, recorte y exportación nativa

**Files:**
- Modify: `package.json` / `pnpm-lock.yaml` (konva, react-konva)
- Create: `src/ui/annotator/types.ts`, `hooks.ts`, `ShapeNode.tsx`, `CropMask.tsx`, `AnnotatorCanvas.tsx`, `Toolbar.tsx`

**Interfaces:**
- Consumes: dominio de anotaciones (T6) y núcleo del anotador (T8).
- Produces:

```ts
// types.ts
export interface AnnotatorHandle { exportPng(): Promise<Uint8Array> }
export interface AnnotatorCanvasProps {
  ref?: Ref<AnnotatorHandle>;
  imagePath: string;
  loadImage(path: string): Promise<Uint8Array>;
  annotations: Annotations;
  tool: Tool;
  color: string;
  sizeLevel: SizeLevel;
  selectedId: string | null;
  onSelect(id: string | null): void;
  onCommit(next: Annotations): void;
  onToolDone?(): void;
}
```

- `AnnotatorCanvas(props: AnnotatorCanvasProps)` es el único módulo que importa `konva` / `react-konva` (junto con `ShapeNode` y `CropMask`). Sus props usan siempre coordenadas del original.
- `Toolbar` con las props de Step 6 y `TOOL_BY_KEY: Record<string, Tool>` para los atajos de teclado.

Esta task no tiene tests unitarios: Konva necesita un `<canvas>` real y jsdom no lo tiene. Su lógica pura ya quedó probada en Task 8, y el comportamiento se verifica con `pnpm typecheck`, `pnpm build` y la prueba manual de Step 7.

- [ ] **Step 1: Dependencias**

```bash
pnpm add konva@^10 react-konva@^19
```

- [ ] **Step 2: `types.ts` y `hooks.ts`**

`src/ui/annotator/types.ts`:

```ts
import type { Ref } from "react";
import type { Annotations } from "../../domain/annotations";
import type { SizeLevel, Tool } from "./shape-factory";

export interface AnnotatorHandle {
  /** PNG de la región exportada (todo o el recorte) a resolución nativa del original. */
  exportPng(): Promise<Uint8Array>;
}

export interface AnnotatorCanvasProps {
  ref?: Ref<AnnotatorHandle>;
  imagePath: string;
  loadImage(path: string): Promise<Uint8Array>;
  annotations: Annotations;
  tool: Tool;
  color: string;
  sizeLevel: SizeLevel;
  selectedId: string | null;
  onSelect(id: string | null): void;
  onCommit(next: Annotations): void;
  onToolDone?(): void;
}
```

`src/ui/annotator/hooks.ts`:

```ts
import { useEffect, useState, type RefObject } from "react";

export function useBoxSize(ref: RefObject<HTMLElement | null>) {
  const [size, setSize] = useState({ width: 0, height: 0 });
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const update = () => setSize({ width: el.clientWidth, height: el.clientHeight });
    update();
    const observer = new ResizeObserver(update);
    observer.observe(el);
    return () => observer.disconnect();
  }, [ref]);
  return size;
}

/** Carga el original por IPC y lo expone como `blob:` (mismo origen: el canvas no queda "tainted" al exportar). */
export function useLoadedImage(path: string, load: (path: string) => Promise<Uint8Array>) {
  const [state, setState] = useState<{ image: HTMLImageElement | null; error: string | null }>({ image: null, error: null });
  useEffect(() => {
    let cancelled = false;
    let url: string | null = null;
    setState({ image: null, error: null });
    load(path)
      .then((bytes) => {
        if (cancelled) return;
        url = URL.createObjectURL(new Blob([bytes as BlobPart], { type: "image/png" }));
        const img = new window.Image();
        img.onload = () => !cancelled && setState({ image: img, error: null });
        img.onerror = () => !cancelled && setState({ image: null, error: "No se pudo abrir la imagen." });
        img.src = url;
      })
      .catch(() => !cancelled && setState({ image: null, error: "No se pudo abrir la imagen." }));
    return () => {
      cancelled = true;
      if (url) URL.revokeObjectURL(url);
    };
  }, [path, load]);
  return state;
}
```

- [ ] **Step 3: `ShapeNode.tsx` (incluye el desenfoque pixelado)**

```tsx
import Konva from "konva";
import { useEffect, useRef } from "react";
import { Arrow, Circle, Ellipse, Group, Image as KImage, Label, Rect, Tag, Text } from "react-konva";
import type { Shape } from "../../domain/annotations";
import { pixelSizeFor } from "./geometry";
import { shapeOrigin } from "./shape-factory";

export interface Transform {
  x: number;
  y: number;
  scaleX: number;
  scaleY: number;
}

export interface ShapeNodeProps {
  shape: Shape;
  image: HTMLImageElement;
  number: number;
  draggable: boolean;
  preview?: boolean;
  onSelect?(): void;
  onMoved?(dx: number, dy: number): void;
  onTransformed?(t: Transform): void;
}

export function ShapeNode({ shape, image, number, draggable, preview = false, onSelect, onMoved, onTransformed }: ShapeNodeProps) {
  const origin = shapeOrigin(shape);
  const common = {
    id: preview ? undefined : shape.id,
    draggable,
    listening: !preview,
    onMouseDown: () => onSelect?.(),
    onDragEnd: (e: Konva.KonvaEventObject<DragEvent>) => {
      const node = e.target;
      const dx = node.x() - origin.x;
      const dy = node.y() - origin.y;
      node.position(origin);
      onMoved?.(dx, dy);
    },
    onTransformEnd: (e: Konva.KonvaEventObject<Event>) => {
      const node = e.target;
      const t = { x: node.x(), y: node.y(), scaleX: node.scaleX(), scaleY: node.scaleY() };
      node.scale({ x: 1, y: 1 });
      node.position(origin);
      onTransformed?.(t);
    },
  };

  switch (shape.kind) {
    case "arrow": {
      const head = shape.strokeWidth * 4;
      return (
        <Arrow {...common} x={0} y={0} points={shape.points} stroke={shape.color} fill={shape.color} strokeWidth={shape.strokeWidth} pointerLength={head} pointerWidth={head} hitStrokeWidth={Math.max(20, shape.strokeWidth * 3)} />
      );
    }
    case "marker":
      return (
        <Group {...common} x={shape.x} y={shape.y}>
          <Circle radius={shape.radius} fill={shape.color} stroke="#FFFFFF" strokeWidth={Math.max(2, shape.radius / 8)} />
          <Text text={String(number)} width={shape.radius * 2} height={shape.radius * 2} offsetX={shape.radius} offsetY={shape.radius} align="center" verticalAlign="middle" fontSize={shape.radius * 1.15} fontStyle="bold" fontFamily="Arial" fill="#FFFFFF" />
        </Group>
      );
    case "text":
      return (
        <Label {...common} x={shape.x} y={shape.y}>
          <Tag fill="rgba(255,255,255,0.85)" cornerRadius={shape.fontSize / 6} />
          <Text text={shape.text} fontSize={shape.fontSize} fontFamily="Arial" fill={shape.color} padding={shape.fontSize / 4} />
        </Label>
      );
    case "rect":
      return <Rect {...common} x={shape.x} y={shape.y} width={shape.width} height={shape.height} stroke={shape.color} strokeWidth={shape.strokeWidth} />;
    case "ellipse":
      return <Ellipse {...common} x={origin.x} y={origin.y} radiusX={shape.width / 2} radiusY={shape.height / 2} stroke={shape.color} strokeWidth={shape.strokeWidth} />;
    case "highlight":
      return <Rect {...common} x={shape.x} y={shape.y} width={shape.width} height={shape.height} fill="rgba(255,221,0,0.38)" />;
    case "blur":
      return preview ? (
        <Rect x={shape.x} y={shape.y} width={shape.width} height={shape.height} stroke="#FFFFFF" strokeWidth={2} dash={[6, 4]} strokeScaleEnabled={false} listening={false} />
      ) : (
        <PixelatedRegion shape={shape} image={image} common={common} />
      );
  }
}

/** Pixela la región del ORIGINAL que cubre la forma. El render final queda aplanado y no se puede revertir. */
function PixelatedRegion({ shape, image, common }: { shape: Extract<Shape, { kind: "blur" }>; image: HTMLImageElement; common: object }) {
  const ref = useRef<Konva.Image>(null);
  useEffect(() => {
    const node = ref.current;
    if (!node) return;
    node.cache();
    node.getLayer()?.batchDraw();
    return () => {
      node.clearCache();
    };
  }, [shape.x, shape.y, shape.width, shape.height, image]);
  return (
    <KImage
      ref={ref}
      {...common}
      x={shape.x}
      y={shape.y}
      width={shape.width}
      height={shape.height}
      image={image}
      crop={{ x: shape.x, y: shape.y, width: shape.width, height: shape.height }}
      filters={[Konva.Filters.Pixelate]}
      pixelSize={pixelSizeFor(shape)}
    />
  );
}
```

- [ ] **Step 4: `CropMask.tsx`**

```tsx
import { Rect } from "react-konva";
import type { Rect as Area } from "../../domain/annotations";
import { clampRect } from "./geometry";

const SHADE = "rgba(0,0,0,0.55)";

/** Oscurece lo que queda fuera del recorte. Solo se ve en el editor: no forma parte del render. */
export function CropMask({ crop, width, height }: { crop: Area | null; width: number; height: number }) {
  if (!crop) return null;
  const c = clampRect(crop, width, height);
  return (
    <>
      <Rect x={0} y={0} width={width} height={c.y} fill={SHADE} />
      <Rect x={0} y={c.y + c.height} width={width} height={height - c.y - c.height} fill={SHADE} />
      <Rect x={0} y={c.y} width={c.x} height={c.height} fill={SHADE} />
      <Rect x={c.x + c.width} y={c.y} width={width - c.x - c.width} height={c.height} fill={SHADE} />
      <Rect x={c.x} y={c.y} width={c.width} height={c.height} stroke="#FFFFFF" strokeWidth={2} dash={[8, 6]} strokeScaleEnabled={false} />
    </>
  );
}
```

- [ ] **Step 5: `AnnotatorCanvas.tsx`**

```tsx
import Konva from "konva";
import { useEffect, useImperativeHandle, useMemo, useRef, useState } from "react";
import { Group, Image as KImage, Layer, Stage, Transformer } from "react-konva";
import { addShape, markerNumbers, nextShapeId, setCrop, updateShape, type Rect, type Shape } from "../../domain/annotations";
import { CropMask } from "./CropMask";
import { clampPoint, clampRect, exportRegion, fitScale, isMeaningful, rectFromPoints, type Point } from "./geometry";
import { useBoxSize, useLoadedImage } from "./hooks";
import { dragShape, isDragTool, isShapeMeaningful, moveShape, placeShape, styleFor, transformShape } from "./shape-factory";
import { ShapeNode } from "./ShapeNode";
import type { AnnotatorCanvasProps } from "./types";

const TRANSFORMABLE = new Set<Shape["kind"]>(["rect", "ellipse", "highlight", "blur", "text", "marker"]);

interface Draft {
  start: Point;
  shape: Shape | null;
  crop: Rect | null;
}

export function AnnotatorCanvas({ ref, imagePath, loadImage, annotations, tool, color, sizeLevel, selectedId, onSelect, onCommit, onToolDone }: AnnotatorCanvasProps) {
  const box = useRef<HTMLDivElement>(null);
  const size = useBoxSize(box);
  const { image, error } = useLoadedImage(imagePath, loadImage);
  const content = useRef<Konva.Layer>(null);
  const transformer = useRef<Konva.Transformer>(null);
  const [draft, setDraft] = useState<Draft | null>(null);
  const scale = image ? fitScale(image.width, image.height, size.width, size.height) : 1;
  const style = image ? styleFor(image.width, image.height, color, sizeLevel) : null;
  const numbers = useMemo(() => markerNumbers(annotations.shapes), [annotations.shapes]);

  useImperativeHandle(
    ref,
    () => ({
      async exportPng() {
        const layer = content.current;
        if (!layer || !image) throw new Error("La imagen aún no está lista.");
        const region = exportRegion(annotations.crop, image.width, image.height);
        // Se exporta solo la capa de contenido (sin selección ni máscara de recorte) a escala 1:1 del original.
        const raw = layer.toCanvas({
          x: region.x * scale,
          y: region.y * scale,
          width: region.width * scale,
          height: region.height * scale,
          pixelRatio: 1 / scale,
        });
        const out = document.createElement("canvas");
        out.width = region.width;
        out.height = region.height;
        const ctx = out.getContext("2d");
        if (!ctx) throw new Error("No se pudo preparar el render.");
        ctx.drawImage(raw, 0, 0, region.width, region.height);
        const blob = await new Promise<Blob>((resolve, reject) =>
          out.toBlob((b) => (b ? resolve(b) : reject(new Error("No se pudo generar el PNG."))), "image/png"),
        );
        return new Uint8Array(await blob.arrayBuffer());
      },
    }),
    [annotations.crop, image, scale],
  );

  useEffect(() => {
    const tr = transformer.current;
    const layer = content.current;
    if (!tr || !layer) return;
    const shape = annotations.shapes.find((s) => s.id === selectedId);
    const node = shape && tool === "select" && TRANSFORMABLE.has(shape.kind) ? layer.findOne(`#${shape.id}`) : undefined;
    tr.keepRatio(shape?.kind === "text" || shape?.kind === "marker");
    tr.nodes(node ? [node] : []);
    tr.getLayer()?.batchDraw();
  }, [annotations, selectedId, tool]);

  const pointer = (e: Konva.KonvaEventObject<MouseEvent>): Point | null => {
    const p = e.target.getStage()?.getPointerPosition();
    if (!p || !image) return null;
    return clampPoint({ x: p.x / scale, y: p.y / scale }, image.width, image.height);
  };

  const onDown = (e: Konva.KonvaEventObject<MouseEvent>) => {
    if (!image || !style) return;
    if (tool === "select") {
      if (e.target === e.target.getStage() || e.target.name() === "background") onSelect(null);
      return;
    }
    const p = pointer(e);
    if (!p) return;
    if (tool === "marker" || tool === "text") {
      const shape = placeShape(tool, nextShapeId(annotations.shapes), p, style);
      onCommit(addShape(annotations, shape));
      if (tool === "text") {
        onSelect(shape.id);
        onToolDone?.();
      }
      return;
    }
    setDraft({ start: p, shape: null, crop: null });
  };

  const onMove = (e: Konva.KonvaEventObject<MouseEvent>) => {
    if (!draft || !style) return;
    const p = pointer(e);
    if (!p) return;
    if (tool === "crop") setDraft({ ...draft, crop: rectFromPoints(draft.start, p) });
    else if (isDragTool(tool)) setDraft({ ...draft, shape: dragShape(tool, "draft", draft.start, p, style) });
  };

  const onUp = () => {
    if (!draft || !image) return;
    const d = draft;
    setDraft(null);
    if (tool === "crop" && d.crop && isMeaningful(d.crop)) {
      onCommit(setCrop(annotations, clampRect(d.crop, image.width, image.height)));
    } else if (d.shape && isShapeMeaningful(d.shape)) {
      onCommit(addShape(annotations, { ...d.shape, id: nextShapeId(annotations.shapes) }));
    }
  };

  return (
    <div ref={box} className="relative h-full w-full overflow-hidden bg-t4s-charcoal/90">
      {error && (
        <p role="alert" className="absolute inset-0 grid place-items-center text-sm text-white">
          {error}
        </p>
      )}
      {!image && !error && <p className="absolute inset-0 grid place-items-center text-sm text-white/80">Cargando imagen…</p>}
      {image && size.width > 0 && (
        <Stage
          width={Math.round(image.width * scale)}
          height={Math.round(image.height * scale)}
          onMouseDown={onDown}
          onMouseMove={onMove}
          onMouseUp={onUp}
          onMouseLeave={onUp}
          className="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 shadow-lg"
          style={{ cursor: tool === "select" ? "default" : "crosshair" }}
        >
          <Layer ref={content} scaleX={scale} scaleY={scale}>
            <KImage image={image} name="background" />
            {annotations.shapes.map((s) => (
              <ShapeNode
                key={s.id}
                shape={s}
                image={image}
                number={numbers.get(s.id) ?? 0}
                draggable={tool === "select"}
                onSelect={() => tool === "select" && onSelect(s.id)}
                onMoved={(dx, dy) => onCommit(updateShape(annotations, s.id, moveShape(s, dx, dy)))}
                onTransformed={(t) => onCommit(updateShape(annotations, s.id, transformShape(s, t)))}
              />
            ))}
          </Layer>
          <Layer>
            <Group scaleX={scale} scaleY={scale} listening={false}>
              {draft?.shape && <ShapeNode shape={draft.shape} image={image} number={0} draggable={false} preview />}
              <CropMask crop={draft?.crop ?? annotations.crop} width={image.width} height={image.height} />
            </Group>
            <Transformer ref={transformer} rotateEnabled={false} flipEnabled={false} ignoreStroke />
          </Layer>
        </Stage>
      )}
    </div>
  );
}
```

- [ ] **Step 6: `Toolbar.tsx`**

```tsx
import { Circle, Crop, Grid3x3, Hash, Highlighter, MousePointer2, MoveUpRight, Redo2, Square, Trash2, Type, Undo2, X, type LucideIcon } from "lucide-react";
import { ANNOTATION_COLORS } from "../../domain/annotations";
import { iconButton, inputCls } from "../components/styles";
import type { SizeLevel, Tool } from "./shape-factory";

const TOOLS: { id: Tool; label: string; key: string; icon: LucideIcon }[] = [
  { id: "select", label: "Seleccionar", key: "V", icon: MousePointer2 },
  { id: "arrow", label: "Flecha", key: "A", icon: MoveUpRight },
  { id: "marker", label: "Número", key: "N", icon: Hash },
  { id: "text", label: "Texto", key: "T", icon: Type },
  { id: "rect", label: "Recuadro", key: "R", icon: Square },
  { id: "ellipse", label: "Elipse", key: "E", icon: Circle },
  { id: "highlight", label: "Resaltado", key: "H", icon: Highlighter },
  { id: "blur", label: "Desenfoque", key: "B", icon: Grid3x3 },
  { id: "crop", label: "Recortar", key: "C", icon: Crop },
];

export const TOOL_BY_KEY: Record<string, Tool> = Object.fromEntries(TOOLS.map((t) => [t.key.toLowerCase(), t.id]));

export interface ToolbarProps {
  tool: Tool;
  onTool(tool: Tool): void;
  color: string;
  onColor(color: string): void;
  sizeLevel: SizeLevel;
  onSizeLevel(level: SizeLevel): void;
  canUndo: boolean;
  canRedo: boolean;
  onUndo(): void;
  onRedo(): void;
  hasSelection: boolean;
  onDelete(): void;
  hasCrop: boolean;
  onClearCrop(): void;
}

export function Toolbar(p: ToolbarProps) {
  const pressed = (on: boolean) => `${iconButton} ${on ? "bg-t4s-blue/15 text-t4s-blue" : ""}`;
  return (
    <div role="toolbar" aria-label="Herramientas de anotación" className="flex flex-wrap items-center gap-1 border-b border-t4s-line bg-white px-3 py-2">
      {TOOLS.map((t) => (
        <button key={t.id} type="button" aria-label={t.label} title={`${t.label} (${t.key})`} aria-pressed={p.tool === t.id} className={pressed(p.tool === t.id)} onClick={() => p.onTool(t.id)}>
          <t.icon size={18} />
        </button>
      ))}
      <span className="mx-2 h-6 w-px bg-t4s-line" />
      {ANNOTATION_COLORS.map((c) => (
        <button
          key={c.value}
          type="button"
          aria-label={`Color ${c.name}`}
          aria-pressed={p.color === c.value}
          className={`h-6 w-6 rounded-full border-2 ${p.color === c.value ? "border-t4s-charcoal" : "border-t4s-line"}`}
          style={{ backgroundColor: c.value }}
          onClick={() => p.onColor(c.value)}
        />
      ))}
      <select aria-label="Grosor" className={`${inputCls} ml-2 w-28 py-1`} value={p.sizeLevel} onChange={(e) => p.onSizeLevel(e.target.value as SizeLevel)}>
        <option value="s">Fino</option>
        <option value="m">Medio</option>
        <option value="l">Grueso</option>
      </select>
      <span className="mx-2 h-6 w-px bg-t4s-line" />
      <button type="button" aria-label="Deshacer" title="Deshacer (Ctrl+Z)" className={iconButton} disabled={!p.canUndo} onClick={p.onUndo}>
        <Undo2 size={18} />
      </button>
      <button type="button" aria-label="Rehacer" title="Rehacer (Ctrl+Y)" className={iconButton} disabled={!p.canRedo} onClick={p.onRedo}>
        <Redo2 size={18} />
      </button>
      <button type="button" aria-label="Eliminar anotación" title="Eliminar anotación (Supr)" className={iconButton} disabled={!p.hasSelection} onClick={p.onDelete}>
        <Trash2 size={18} />
      </button>
      {p.hasCrop && (
        <button type="button" className={`${iconButton} flex items-center gap-1 text-sm`} onClick={p.onClearCrop}>
          <X size={16} /> Quitar recorte
        </button>
      )}
    </div>
  );
}
```

- [ ] **Step 7: Verificar y commit**

```bash
pnpm typecheck && pnpm test && pnpm build
git add package.json pnpm-lock.yaml src/ui/annotator
git commit -m "feat(f2): lienzo Konva con herramientas, desenfoque, recorte y exportación nativa

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

La prueba visual del lienzo se hace al final de Task 11, cuando el editor ya está conectado a la app (`pnpm tauri dev`).

---

### Task 10: Editor de evidencia (diálogo con caso, descripción, deshacer y atajos)

**Files:**
- Create: `src/ui/annotator/AnnotationEditor.tsx`, `src/ui/annotator/AnnotationEditor.test.tsx`, `src/ui/annotator/__mocks__/AnnotatorCanvas.tsx`
- Modify: `src/test/setup.ts`

**Interfaces:**
- Consumes: `AnnotatorCanvas` (T9), `Toolbar`, `history`, `validateDescription`, `FormResult` (de `ui/store/workspace.ts`).
- Produces:

```ts
export interface EditorCase { id: string; code: string; title: string }
export interface EditorSaveInput { caseId: string; description: string; annotations: Annotations; render: Uint8Array }
export interface AnnotationEditorProps {
  title: string;
  imagePath: string;
  loadImage(path: string): Promise<Uint8Array>;
  cases: EditorCase[];
  initialCaseId: string | null;
  caseLocked: boolean;
  initialDescription: string;
  initialAnnotations: Annotations;
  pending: number;
  onSave(input: EditorSaveInput): Promise<FormResult>;
  onCancel(): void;
}
export function AnnotationEditor(props: AnnotationEditorProps): JSX.Element  // role="dialog", aria-label="Editor de evidencia"
```

- El mock `MOCK_RENDER` (PNG mínimo) y los botones de prueba `dibujar marcador` / `dibujar texto` existen solo en tests.

- [ ] **Step 1: Mock global del lienzo**

`src/ui/annotator/__mocks__/AnnotatorCanvas.tsx`:

```tsx
import { useImperativeHandle } from "react";
import { addShape, nextShapeId } from "../../../domain/annotations";
import type { AnnotatorCanvasProps } from "../types";

export const MOCK_RENDER = new Uint8Array([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);

/** jsdom no tiene <canvas>: en los tests el lienzo es un doble que dibuja con botones. */
export function AnnotatorCanvas({ ref, annotations, onCommit, onSelect }: AnnotatorCanvasProps) {
  useImperativeHandle(ref, () => ({ exportPng: async () => MOCK_RENDER }), []);
  const texts = annotations.shapes.flatMap((s) => (s.kind === "text" ? [s.text] : [])).join("|");
  return (
    <div data-testid="annotator-canvas" data-shapes={annotations.shapes.length} data-texts={texts} data-crop={annotations.crop ? "si" : "no"}>
      <button type="button" onClick={() => onCommit(addShape(annotations, { id: nextShapeId(annotations.shapes), kind: "marker", x: 10, y: 10, color: "#E02424", radius: 18 }))}>
        dibujar marcador
      </button>
      <button
        type="button"
        onClick={() => {
          const id = nextShapeId(annotations.shapes);
          onCommit(addShape(annotations, { id, kind: "text", x: 5, y: 5, text: "Texto", color: "#E02424", fontSize: 28 }));
          onSelect(id);
        }}
      >
        dibujar texto
      </button>
    </div>
  );
}
```

`src/test/setup.ts`, agrega al final:

```ts
import { vi } from "vitest";

// Konva necesita un <canvas> real; todos los tests de UI usan el doble de __mocks__.
vi.mock("../ui/annotator/AnnotatorCanvas");
```

(Mueve el `import { vi }` junto a los demás imports. Si en esta versión de Vitest el mock declarado en `setupFiles` no se aplica a los demás archivos, agrega `vi.mock("../annotator/AnnotatorCanvas")` al inicio de cada test que renderice el editor, con la ruta relativa a ese archivo.)

- [ ] **Step 2: Tests que fallan**

`src/ui/annotator/AnnotationEditor.test.tsx`:

```tsx
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { emptyAnnotations } from "../../domain/annotations";
import type { FormResult } from "../store/workspace";
import { AnnotationEditor, type AnnotationEditorProps } from "./AnnotationEditor";
import { MOCK_RENDER } from "./__mocks__/AnnotatorCanvas";

const cases = [
  { id: "c1", code: "EJ-001", title: "Recibos" },
  { id: "c2", code: "EJ-002", title: "Pólizas" },
];

function setup(over: Partial<AnnotationEditorProps> = {}) {
  const onSave = vi.fn(async (): Promise<FormResult> => ({ ok: true }));
  const onCancel = vi.fn();
  render(
    <AnnotationEditor
      title="Nueva evidencia"
      imagePath="img.png"
      loadImage={async () => new Uint8Array()}
      cases={cases}
      initialCaseId="c2"
      caseLocked={false}
      initialDescription=""
      initialAnnotations={emptyAnnotations()}
      pending={0}
      onSave={onSave}
      onCancel={onCancel}
      {...over}
    />,
  );
  const dialog = screen.getByRole("dialog", { name: "Editor de evidencia" });
  return { user: userEvent.setup(), onSave, onCancel, dialog, canvas: within(dialog).getByTestId("annotator-canvas") };
}

describe("editor de evidencia", () => {
  it("preselecciona el caso y exige la descripción", async () => {
    const { user, onSave, dialog } = setup();
    expect(within(dialog).getByLabelText("Caso")).toHaveValue("c2");
    await user.click(within(dialog).getByRole("button", { name: "Guardar evidencia" }));
    expect(within(dialog).getByText("«Descripción» es obligatorio.")).toBeInTheDocument();
    expect(onSave).not.toHaveBeenCalled();
  });

  it("guarda caso, descripción, anotaciones y render", async () => {
    const { user, onSave, dialog } = setup();
    await user.click(within(dialog).getByRole("button", { name: "dibujar marcador" }));
    await user.selectOptions(within(dialog).getByLabelText("Caso"), "c1");
    await user.type(within(dialog).getByLabelText("Descripción"), "① Recibo aplicado");
    await user.click(within(dialog).getByRole("button", { name: "Guardar evidencia" }));
    expect(onSave).toHaveBeenCalledWith({
      caseId: "c1",
      description: "① Recibo aplicado",
      annotations: expect.objectContaining({ shapes: [expect.objectContaining({ kind: "marker" })] }),
      render: MOCK_RENDER,
    });
  });

  it("muestra el error del guardado", async () => {
    const { user, dialog } = setup({ onSave: async () => ({ ok: false, errors: { _form: "El caso no existe." } }) });
    await user.type(within(dialog).getByLabelText("Descripción"), "x");
    await user.click(within(dialog).getByRole("button", { name: "Guardar evidencia" }));
    expect(await within(dialog).findByText("El caso no existe.")).toBeInTheDocument();
  });

  it("deshace y rehace con botones y teclado", async () => {
    const { user, dialog, canvas } = setup();
    await user.click(within(dialog).getByRole("button", { name: "dibujar marcador" }));
    expect(canvas).toHaveAttribute("data-shapes", "1");
    await user.keyboard("{Control>}z{/Control}");
    expect(canvas).toHaveAttribute("data-shapes", "0");
    await user.keyboard("{Control>}y{/Control}");
    expect(canvas).toHaveAttribute("data-shapes", "1");
    await user.click(within(dialog).getByRole("button", { name: "Deshacer" }));
    expect(canvas).toHaveAttribute("data-shapes", "0");
  });

  it("edita el texto seleccionado y lo borra con Supr", async () => {
    const { user, dialog, canvas } = setup();
    await user.click(within(dialog).getByRole("button", { name: "dibujar texto" }));
    const input = within(dialog).getByLabelText("Texto");
    await user.clear(input);
    await user.type(input, "Monto");
    expect(canvas).toHaveAttribute("data-texts", "Monto");
    await user.click(within(dialog).getByRole("button", { name: "Seleccionar" }));
    await user.keyboard("{Delete}");
    expect(canvas).toHaveAttribute("data-shapes", "0");
  });

  it("cambia de herramienta con su tecla, bloquea el caso al re-anotar y cancela con Esc", async () => {
    const { user, dialog, onCancel } = setup({ caseLocked: true, pending: 2 });
    await user.keyboard("r");
    expect(within(dialog).getByRole("button", { name: "Recuadro" })).toHaveAttribute("aria-pressed", "true");
    expect(within(dialog).getByLabelText("Caso")).toBeDisabled();
    expect(within(dialog).getByText("2 en cola")).toBeInTheDocument();
    await user.keyboard("{Escape}");
    expect(onCancel).toHaveBeenCalled();
  });
});
```

- [ ] **Step 3: Verificar que fallan**

Run: `pnpm vitest run src/ui/annotator/AnnotationEditor.test.tsx`
Expected: FAIL (`AnnotationEditor` no existe).

- [ ] **Step 4: `AnnotationEditor.tsx`**

```tsx
import { useEffect, useRef, useState } from "react";
import { DEFAULT_COLOR, findShape, removeShape, setCrop, updateShape, type Annotations } from "../../domain/annotations";
import type { FormErrors } from "../../domain/cases";
import { validateDescription } from "../../domain/evidence";
import { FormField } from "../components/FormField";
import { btn, inputCls } from "../components/styles";
import type { FormResult } from "../store/workspace";
import { AnnotatorCanvas } from "./AnnotatorCanvas";
import { canRedo, canUndo, commit, createHistory, redo, undo } from "./history";
import type { SizeLevel, Tool } from "./shape-factory";
import { TOOL_BY_KEY, Toolbar } from "./Toolbar";
import type { AnnotatorHandle } from "./types";

export interface EditorCase {
  id: string;
  code: string;
  title: string;
}

export interface EditorSaveInput {
  caseId: string;
  description: string;
  annotations: Annotations;
  render: Uint8Array;
}

export interface AnnotationEditorProps {
  title: string;
  imagePath: string;
  loadImage(path: string): Promise<Uint8Array>;
  cases: EditorCase[];
  initialCaseId: string | null;
  caseLocked: boolean;
  initialDescription: string;
  initialAnnotations: Annotations;
  pending: number;
  onSave(input: EditorSaveInput): Promise<FormResult>;
  onCancel(): void;
}

const isTyping = (target: EventTarget | null) =>
  target instanceof HTMLElement && (target.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName));

export function AnnotationEditor(props: AnnotationEditorProps) {
  const { onCancel } = props;
  const canvas = useRef<AnnotatorHandle>(null);
  const [history, setHistory] = useState(() => createHistory(props.initialAnnotations));
  const [tool, setTool] = useState<Tool>("arrow");
  const [color, setColor] = useState<string>(DEFAULT_COLOR);
  const [sizeLevel, setSizeLevel] = useState<SizeLevel>("m");
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [caseId, setCaseId] = useState(props.initialCaseId ?? "");
  const [description, setDescription] = useState(props.initialDescription);
  const [errors, setErrors] = useState<FormErrors>({});
  const [saving, setSaving] = useState(false);
  const annotations = history.present;
  const selected = selectedId ? findShape(annotations, selectedId) : null;
  const apply = (next: Annotations) => setHistory((h) => commit(h, next));

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const typing = isTyping(e.target);
      const key = e.key.toLowerCase();
      if ((e.ctrlKey || e.metaKey) && !typing) {
        if (key === "z" && !e.shiftKey) {
          e.preventDefault();
          setHistory(undo);
          setSelectedId(null);
        } else if (key === "y" || (key === "z" && e.shiftKey)) {
          e.preventDefault();
          setHistory(redo);
        }
        return;
      }
      if (typing || e.altKey || e.ctrlKey || e.metaKey) return;
      if (e.key === "Escape") onCancel();
      else if ((e.key === "Delete" || e.key === "Backspace") && selectedId) {
        setHistory((h) => commit(h, removeShape(h.present, selectedId)));
        setSelectedId(null);
      } else if (TOOL_BY_KEY[key]) setTool(TOOL_BY_KEY[key]);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onCancel, selectedId]);

  const changeColor = (value: string) => {
    setColor(value);
    if (selected && "color" in selected) apply(updateShape(annotations, selected.id, { ...selected, color: value }));
  };

  const save = async () => {
    const next: FormErrors = {};
    if (!caseId) next.caseId = "Elige el caso de la evidencia.";
    const message = validateDescription(description);
    if (message) next.description = message;
    setErrors(next);
    if (Object.keys(next).length || !canvas.current) return;
    setSaving(true);
    try {
      const render = await canvas.current.exportPng();
      const result = await props.onSave({ caseId, description, annotations, render });
      if (!result.ok) setErrors(result.errors);
    } catch {
      setErrors({ _form: "No se pudo generar la imagen anotada." });
    } finally {
      setSaving(false);
    }
  };

  return (
    <div role="dialog" aria-modal="true" aria-label="Editor de evidencia" className="fixed inset-0 z-40 flex flex-col bg-t4s-cream">
      <header className="flex items-center justify-between border-b border-t4s-line bg-white px-5 py-3">
        <h2 className="text-lg font-semibold">{props.title}</h2>
        {props.pending > 0 && <span className="rounded-full bg-t4s-lime/30 px-3 py-1 text-xs font-medium">{props.pending} en cola</span>}
      </header>
      <Toolbar
        tool={tool}
        onTool={setTool}
        color={color}
        onColor={changeColor}
        sizeLevel={sizeLevel}
        onSizeLevel={setSizeLevel}
        canUndo={canUndo(history)}
        canRedo={canRedo(history)}
        onUndo={() => {
          setHistory(undo);
          setSelectedId(null);
        }}
        onRedo={() => setHistory(redo)}
        hasSelection={!!selected}
        onDelete={() => {
          if (!selectedId) return;
          apply(removeShape(annotations, selectedId));
          setSelectedId(null);
        }}
        hasCrop={!!annotations.crop}
        onClearCrop={() => apply(setCrop(annotations, null))}
      />
      <div className="flex min-h-0 flex-1">
        <div className="min-w-0 flex-1">
          <AnnotatorCanvas
            ref={canvas}
            imagePath={props.imagePath}
            loadImage={props.loadImage}
            annotations={annotations}
            tool={tool}
            color={color}
            sizeLevel={sizeLevel}
            selectedId={selectedId}
            onSelect={setSelectedId}
            onCommit={apply}
            onToolDone={() => setTool("select")}
          />
        </div>
        <aside className="flex w-80 shrink-0 flex-col gap-4 overflow-y-auto border-l border-t4s-line bg-white p-5">
          <FormField label="Caso" htmlFor="evidence-case" error={errors.caseId}>
            <select id="evidence-case" className={inputCls} value={caseId} disabled={props.caseLocked} onChange={(e) => setCaseId(e.target.value)}>
              <option value="">Elige un caso…</option>
              {props.cases.map((c) => (
                <option key={c.id} value={c.id}>
                  {c.code} · {c.title}
                </option>
              ))}
            </select>
          </FormField>
          <FormField label="Descripción" htmlFor="evidence-description" error={errors.description}>
            <textarea id="evidence-description" rows={6} className={inputCls} value={description} onChange={(e) => setDescription(e.target.value)} />
          </FormField>
          <p className="text-xs text-t4s-gray">Explica aquí qué señala cada número ①②③.</p>
          {selected?.kind === "text" && (
            <FormField label="Texto" htmlFor="annotation-text">
              <input id="annotation-text" className={inputCls} value={selected.text} onChange={(e) => apply(updateShape(annotations, selected.id, { ...selected, text: e.target.value }))} />
            </FormField>
          )}
          {errors._form && (
            <p role="alert" className="text-sm text-red-700">
              {errors._form}
            </p>
          )}
          <div className="mt-auto flex justify-end gap-2 border-t border-t4s-line pt-4">
            <button type="button" className={btn.secondary} onClick={onCancel}>
              Cancelar
            </button>
            <button type="button" className={btn.primary} disabled={saving} onClick={() => void save()}>
              {saving ? "Guardando…" : "Guardar evidencia"}
            </button>
          </div>
        </aside>
      </div>
    </div>
  );
}
```

`FormField` (`src/ui/components/FormField.tsx`) ya acepta `label`, `htmlFor`, `error` y `children`. No le pases `required`: agrega " *" a la etiqueta y los tests buscan las etiquetas exactas "Caso" y "Descripción".

- [ ] **Step 5: Verificar y commit**

```bash
pnpm test && pnpm typecheck
git add src/ui/annotator src/test/setup.ts
git commit -m "feat(f2): editor de evidencia con caso, descripción obligatoria y deshacer/rehacer

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 11: Flujo de registro — pegar, arrastrar o elegir → editor → guardar

**Files:**
- Create: `src/ui/store/evidence-slice.ts`, `src/ui/store/evidence-slice.test.ts`, `src/ui/gallery/files.ts`, `src/ui/gallery/files.test.ts`, `src/ui/gallery/useEvidenceIntake.ts`, `src/ui/screens/EvidenceEditorHost.tsx`, `src/ui/screens/Evidence.test.tsx`
- Modify: `src/application/use-cases.ts`, `src/ui/store/workspace.ts`, `src/ui/screens/Workspace.tsx`, `src/ui/screens/CaseDetail.tsx` (zona de soltar)

**Interfaces:**
- Consumes: `EvidenceStore` (T7), `AnnotationEditor` (T10), `serializeAnnotations` / `parseAnnotations` (T6).
- Produces:
  - Use cases: `attachEvidence(repos, input: NewEvidenceInput, render)`, `reannotateEvidence(repos, input: EvidenceUpdateInput, render)` y `renameEvidence(repos, evidenceId, description)`, todos `Promise<Outcome<Evidence>>` (errores en `description` o `_form`).
  - `EvidenceSlice`, que se agrega a `WorkspaceState`:

```ts
export type EditorSession = { kind: "new"; imagePath: string; caseId: string | null } | { kind: "edit"; evidence: Evidence };
export interface EditorRequest { imagePath: string; caseId?: string | null }
export interface EvidenceSlice {
  evidence: Evidence[];                 // del caso seleccionado
  editor: EditorSession | null;
  editorQueue: EditorRequest[];
  loadEvidence(): Promise<void>;
  intakeImages(images: Uint8Array[]): Promise<void>;
  openEvidenceEditor(request: EditorRequest): void;   // interfaz que reutilizan F3 y F6
  editEvidence(evidenceId: string): void;
  cancelEditor(): Promise<void>;
  saveEditor(input: EditorSaveInput): Promise<FormResult>;
  readImage(path: string): Promise<Uint8Array>;
  imageUrl(path: string, version: string): string;
  updateEvidenceDescription(evidenceId: string, description: string): Promise<FormResult>;
  moveEvidence(evidenceId: string, targetCaseId: string): Promise<void>;
  reorderEvidence(orderedIds: string[]): Promise<void>;
  deleteEvidence(evidenceId: string): Promise<void>;
}
```

  - `files.ts`: `filesFrom(data: DataTransfer | null): File[]`, `hasFiles(data)`, `readBytes(file): Promise<Uint8Array>` y `splitAccepted(files): { images: File[]; rejected: number }`.
  - `useEvidenceIntake(): (files: File[]) => Promise<void>`

- [ ] **Step 1: Tests que fallan**

`src/ui/gallery/files.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { filesFrom, hasFiles, readBytes, splitAccepted } from "./files";

const file = (type: string, bytes = [1, 2]) => new File([new Uint8Array(bytes)], "x", { type });

describe("archivos entrantes", () => {
  it("separa imágenes aceptadas de otros archivos", () => {
    const { images, rejected } = splitAccepted([file("image/png"), file("application/pdf"), file("image/webp")]);
    expect([images.length, rejected]).toEqual([2, 1]);
  });

  it("lee archivos desde files o desde items y detecta arrastres de archivos", () => {
    const png = file("image/png");
    expect(filesFrom({ files: [png], items: [], types: ["Files"] } as unknown as DataTransfer)).toEqual([png]);
    const fromItems = { files: [], items: [{ kind: "file", getAsFile: () => png }, { kind: "string", getAsFile: () => null }], types: ["Files"] };
    expect(filesFrom(fromItems as unknown as DataTransfer)).toEqual([png]);
    expect(filesFrom(null)).toEqual([]);
    expect(hasFiles({ types: ["text/plain"] } as unknown as DataTransfer)).toBe(false);
  });

  it("lee los bytes", async () => {
    expect([...(await readBytes(file("image/png", [7, 8, 9])))]).toEqual([7, 8, 9]);
  });
});
```

`src/ui/store/evidence-slice.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { emptyAnnotations } from "../../domain/annotations";
import { createMemoryRepositories } from "../../infrastructure/memory/repositories";
import { seedProject } from "../test-utils";
import { memoryPreferences } from "./preferences";
import { createWorkspaceStore } from "./workspace";

const PNG = new Uint8Array([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);

async function setup() {
  const repos = createMemoryRepositories();
  const p = await seedProject(repos);
  const entorno = p.fields.find((f) => f.key === "entorno")!.id;
  const c1 = await repos.cases.create(p.id, { title: "Recibos", status: "No ejecutado", values: { [entorno]: "QA" } });
  const c2 = await repos.cases.create(p.id, { title: "Pólizas", status: "No ejecutado", values: { [entorno]: "QA" } });
  const store = createWorkspaceStore(repos, memoryPreferences(p.id));
  await store.getState().init();
  const save = (caseId = c1.id, description = "Pantalla") =>
    store.getState().saveEditor({ caseId, description, annotations: emptyAnnotations(), render: PNG });
  return { repos, store, c1, c2, save };
}

describe("slice de evidencias", () => {
  it("abre el editor con el caso activo, guarda y actualiza el contador", async () => {
    const { store, c1, save } = await setup();
    await store.getState().intakeImages([PNG]);
    expect(store.getState().editor).toMatchObject({ kind: "new", caseId: c1.id });
    expect(await save()).toEqual({ ok: true });
    const s = store.getState();
    expect(s.editor).toBeNull();
    expect(s.evidence.map((e) => e.description)).toEqual(["Pantalla"]);
    expect(s.cases.find((c) => c.id === c1.id)?.evidenceCount).toBe(1);
    expect(s.toasts.at(-1)?.message).toBe("Evidencia 1 agregada a EJ-001.");
  });

  it("devuelve el error de descripción sin cerrar el editor", async () => {
    const { store, save } = await setup();
    await store.getState().intakeImages([PNG]);
    expect(await save(undefined, " ")).toEqual({ ok: false, errors: { description: "«Descripción» es obligatorio." } });
    expect(store.getState().editor).not.toBeNull();
  });

  it("encola varias imágenes y las abre en secuencia; cancelar descarta el staging", async () => {
    const { repos, store, save } = await setup();
    await store.getState().intakeImages([PNG, PNG, PNG]);
    expect(store.getState().editorQueue).toHaveLength(2);
    await save();
    expect(store.getState().editorQueue).toHaveLength(1);
    const current = store.getState().editor;
    await store.getState().cancelEditor();
    if (current?.kind === "new") await expect(repos.evidence.readImage(current.imagePath)).rejects.toBeTruthy();
    expect(store.getState().editorQueue).toHaveLength(0);
    expect(store.getState().editor?.kind).toBe("new");
  });

  it("sin caso seleccionado avisa y no abre el editor", async () => {
    const { store } = await setup();
    store.getState().selectCase(null);
    await store.getState().intakeImages([PNG]);
    expect(store.getState().editor).toBeNull();
    expect(store.getState().toasts.at(-1)?.message).toBe("Crea o selecciona un caso antes de agregar evidencias.");
  });

  it("re-anota, mueve, reordena y elimina", async () => {
    const { store, c1, c2, save } = await setup();
    for (const d of ["uno", "dos"]) {
      await store.getState().intakeImages([PNG]);
      await save(c1.id, d);
    }
    const [first, second] = store.getState().evidence;
    store.getState().editEvidence(first.id);
    expect(store.getState().editor).toMatchObject({ kind: "edit" });
    await save(c1.id, "uno editada");
    expect(store.getState().evidence[0].description).toBe("uno editada");

    await store.getState().reorderEvidence([second.id, first.id]);
    expect(store.getState().evidence.map((e) => e.description)).toEqual(["dos", "uno editada"]);

    await store.getState().moveEvidence(second.id, c2.id);
    expect(store.getState().evidence.map((e) => e.description)).toEqual(["uno editada"]);
    expect(store.getState().cases.find((c) => c.id === c2.id)?.evidenceCount).toBe(1);

    await store.getState().deleteEvidence(first.id);
    expect(store.getState().evidence).toEqual([]);
  });
});
```

`src/ui/screens/Evidence.test.tsx` (flujo de registro; Task 12 agrega aquí los tests de la galería):

```tsx
import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { Repositories } from "../../ports/repositories";
import { renderApp, seedProject } from "../test-utils";

export const PNG_BYTES = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
export const png = (name = "captura.png") => new File([new Uint8Array(PNG_BYTES)], name, { type: "image/png" });

export async function seedCases(repos: Repositories) {
  const p = await seedProject(repos);
  const entorno = p.fields.find((f) => f.key === "entorno")!.id;
  const a = await repos.cases.create(p.id, { title: "Recibos", status: "No ejecutado", values: { [entorno]: "QA" } });
  const b = await repos.cases.create(p.id, { title: "Pólizas", status: "No ejecutado", values: { [entorno]: "QA" } });
  return { p, a, b };
}

const paste = (files: File[], types = ["Files"]) => fireEvent.paste(window, { clipboardData: { files, items: [], types } });
const editor = () => screen.findByRole("dialog", { name: "Editor de evidencia" });

describe("registro de evidencias", () => {
  it("Ctrl+V abre el editor con el caso activo, exige descripción y guarda", async () => {
    const { user, store } = await renderApp({ setup: async (repos) => void (await seedCases(repos)) });
    paste([png()]);
    const dialog = await editor();
    expect(within(dialog).getByLabelText("Caso")).toHaveDisplayValue("EJ-001 · Recibos");
    await user.click(within(dialog).getByRole("button", { name: "Guardar evidencia" }));
    expect(within(dialog).getByText("«Descripción» es obligatorio.")).toBeInTheDocument();
    await user.type(within(dialog).getByLabelText("Descripción"), "Recibo aplicado");
    await user.click(within(dialog).getByRole("button", { name: "Guardar evidencia" }));
    await waitFor(() => expect(screen.queryByRole("dialog", { name: "Editor de evidencia" })).not.toBeInTheDocument());
    expect(screen.getByText("Evidencia 1 agregada a EJ-001.")).toBeInTheDocument();
    expect(store.getState().evidence).toHaveLength(1);
  });

  it("no intercepta el pegado con un diálogo abierto ni sin imagen", async () => {
    const { user } = await renderApp({ setup: async (repos) => void (await seedCases(repos)) });
    paste([], ["text/plain"]);
    await user.click(screen.getByRole("button", { name: /Nuevo caso/ }));
    paste([png()]);
    expect(screen.queryByRole("dialog", { name: "Editor de evidencia" })).not.toBeInTheDocument();
    expect(screen.getByRole("dialog", { name: "Nuevo caso" })).toBeInTheDocument();
  });

  it("arrastrar 3 imágenes y un PDF encola las imágenes y avisa del omitido", async () => {
    await renderApp({ setup: async (repos) => void (await seedCases(repos)) });
    const pdf = new File([new Uint8Array([0x25, 0x50, 0x44, 0x46])], "informe.pdf", { type: "application/pdf" });
    fireEvent.drop(screen.getByRole("region", { name: "Detalle del caso" }), {
      dataTransfer: { files: [png("a.png"), png("b.png"), pdf, png("c.png")], items: [], types: ["Files"] },
    });
    const dialog = await editor();
    expect(within(dialog).getByText("2 en cola")).toBeInTheDocument();
    expect(screen.getByText("Se omitió 1 archivo: solo se aceptan imágenes PNG, JPEG o WebP.")).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Verificar que fallan**

Run: `pnpm vitest run src/ui/gallery src/ui/store/evidence-slice.test.ts src/ui/screens/Evidence.test.tsx`
Expected: FAIL (módulos y acciones inexistentes).

- [ ] **Step 3: Casos de uso (`src/application/use-cases.ts`)**

Agrega los imports de `validateDescription`, `Evidence`, `EvidenceUpdateInput` y `NewEvidenceInput` desde `../domain/evidence`, y al final:

```ts
const RENDER_FAILED = "No se pudo generar la imagen anotada.";

export async function attachEvidence(repos: Repositories, input: NewEvidenceInput, render: Uint8Array): Promise<Outcome<Evidence>> {
  const message = validateDescription(input.description);
  if (message) return { ok: false, errors: { description: message } };
  if (!render.length) return { ok: false, errors: { _form: RENDER_FAILED } };
  return guard(() => repos.evidence.add({ ...input, description: input.description.trim() }, render));
}

export async function reannotateEvidence(repos: Repositories, input: EvidenceUpdateInput, render: Uint8Array): Promise<Outcome<Evidence>> {
  const message = validateDescription(input.description);
  if (message) return { ok: false, errors: { description: message } };
  if (!render.length) return { ok: false, errors: { _form: RENDER_FAILED } };
  return guard(() => repos.evidence.update({ ...input, description: input.description.trim() }, render));
}

export async function renameEvidence(repos: Repositories, evidenceId: string, description: string): Promise<Outcome<Evidence>> {
  const message = validateDescription(description);
  if (message) return { ok: false, errors: { description: message } };
  return guard(() => repos.evidence.updateDescription(evidenceId, description.trim()));
}
```

- [ ] **Step 4: Archivos entrantes**

`src/ui/gallery/files.ts`:

```ts
import { isAcceptedImage } from "../../domain/evidence";

export function filesFrom(data: DataTransfer | null): File[] {
  if (!data) return [];
  const files = Array.from(data.files ?? []);
  if (files.length) return files;
  return Array.from(data.items ?? [])
    .filter((item) => item.kind === "file")
    .map((item) => item.getAsFile())
    .filter((f): f is File => f !== null);
}

export const hasFiles = (data: DataTransfer | null) => !!data && Array.from(data.types ?? []).includes("Files");

export function splitAccepted(files: File[]) {
  const images = files.filter(isAcceptedImage);
  return { images, rejected: files.length - images.length };
}

export async function readBytes(file: Blob): Promise<Uint8Array> {
  if (typeof file.arrayBuffer === "function") return new Uint8Array(await file.arrayBuffer());
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(new Uint8Array(reader.result as ArrayBuffer));
    reader.onerror = () => reject(reader.error);
    reader.readAsArrayBuffer(file);
  });
}
```

`src/ui/gallery/useEvidenceIntake.ts`:

```ts
import { useCallback } from "react";
import { useWorkspace } from "../store/context";
import { readBytes, splitAccepted } from "./files";

export function useEvidenceIntake() {
  const intakeImages = useWorkspace((s) => s.intakeImages);
  const pushToast = useWorkspace((s) => s.pushToast);
  return useCallback(
    async (files: File[]) => {
      const { images, rejected } = splitAccepted(files);
      if (rejected) {
        pushToast(
          "error",
          rejected === 1
            ? "Se omitió 1 archivo: solo se aceptan imágenes PNG, JPEG o WebP."
            : `Se omitieron ${rejected} archivos: solo se aceptan imágenes PNG, JPEG o WebP.`,
        );
      }
      if (images.length) await intakeImages(await Promise.all(images.map(readBytes)));
    },
    [intakeImages, pushToast],
  );
}
```

- [ ] **Step 5: Slice de evidencias (`src/ui/store/evidence-slice.ts`)**

```ts
import type { StoreApi } from "zustand/vanilla";
import * as useCases from "../../application/use-cases";
import { serializeAnnotations } from "../../domain/annotations";
import type { Evidence } from "../../domain/evidence";
import type { Repositories } from "../../ports/repositories";
import type { EditorSaveInput } from "../annotator/AnnotationEditor";
import type { FormResult, WorkspaceState } from "./workspace";

export type EditorSession = { kind: "new"; imagePath: string; caseId: string | null } | { kind: "edit"; evidence: Evidence };

export interface EditorRequest {
  imagePath: string;
  caseId?: string | null;
}

export interface EvidenceSlice {
  evidence: Evidence[];
  editor: EditorSession | null;
  editorQueue: EditorRequest[];
  loadEvidence(): Promise<void>;
  intakeImages(images: Uint8Array[]): Promise<void>;
  openEvidenceEditor(request: EditorRequest): void;
  editEvidence(evidenceId: string): void;
  cancelEditor(): Promise<void>;
  saveEditor(input: EditorSaveInput): Promise<FormResult>;
  readImage(path: string): Promise<Uint8Array>;
  imageUrl(path: string, version: string): string;
  updateEvidenceDescription(evidenceId: string, description: string): Promise<FormResult>;
  moveEvidence(evidenceId: string, targetCaseId: string): Promise<void>;
  reorderEvidence(orderedIds: string[]): Promise<void>;
  deleteEvidence(evidenceId: string): Promise<void>;
}

type Set = StoreApi<WorkspaceState>["setState"];
type Get = StoreApi<WorkspaceState>["getState"];

export interface SliceHelpers {
  report(error: unknown): void;
  refreshCases(): Promise<void>;
  form<T>(run: () => Promise<useCases.Outcome<T>>, onOk: (value: T) => Promise<void> | void): Promise<FormResult>;
}

export function createEvidenceSlice(repos: Repositories, set: Set, get: Get, h: SliceHelpers): EvidenceSlice {
  const session = (request: EditorRequest): EditorSession => ({
    kind: "new",
    imagePath: request.imagePath,
    caseId: request.caseId ?? get().selectedCaseId,
  });
  const openNext = () => {
    const [head, ...rest] = get().editorQueue;
    set({ editor: head ? session(head) : null, editorQueue: rest });
  };
  const afterChange = async () => {
    await h.refreshCases();
    await get().loadEvidence();
  };

  return {
    evidence: [],
    editor: null,
    editorQueue: [],

    async loadEvidence() {
      const caseId = get().selectedCaseId;
      if (!caseId) {
        set({ evidence: [] });
        return;
      }
      try {
        const evidence = await repos.evidence.list(caseId);
        if (get().selectedCaseId === caseId) set({ evidence });
      } catch (error) {
        h.report(error);
      }
    },

    async intakeImages(images) {
      const caseId = get().selectedCaseId;
      if (!caseId) {
        get().pushToast("error", "Crea o selecciona un caso antes de agregar evidencias.");
        return;
      }
      for (const image of images) {
        try {
          const staged = await repos.evidence.stage(image);
          get().openEvidenceEditor({ imagePath: staged.path, caseId });
        } catch (error) {
          h.report(error);
        }
      }
    },

    openEvidenceEditor(request) {
      if (get().editor) set((s) => ({ editorQueue: [...s.editorQueue, request] }));
      else set({ editor: session(request) });
    },

    editEvidence(evidenceId) {
      const evidence = get().evidence.find((e) => e.id === evidenceId);
      if (evidence && !get().editor) set({ editor: { kind: "edit", evidence } });
    },

    async cancelEditor() {
      const editor = get().editor;
      if (editor?.kind === "new") {
        try {
          await repos.evidence.discardStaged(editor.imagePath);
        } catch {
          // Si falla, el próximo arranque vacía evidence/_staging.
        }
      }
      openNext();
    },

    async saveEditor(input) {
      const editor = get().editor;
      if (!editor) return { ok: false, errors: { _form: "El editor no está abierto." } };
      const annotations = serializeAnnotations(input.annotations);
      if (editor.kind === "new") {
        return h.form(
          () => useCases.attachEvidence(repos, { caseId: input.caseId, stagedPath: editor.imagePath, description: input.description, annotations }, input.render),
          async (saved) => {
            const code = get().cases.find((c) => c.id === saved.caseId)?.code ?? "el caso";
            openNext();
            await afterChange();
            get().pushToast("success", `Evidencia ${saved.position + 1} agregada a ${code}.`);
          },
        );
      }
      return h.form(
        () => useCases.reannotateEvidence(repos, { evidenceId: editor.evidence.id, description: input.description, annotations }, input.render),
        async (saved) => {
          openNext();
          await afterChange();
          get().pushToast("success", `Evidencia ${saved.position + 1} actualizada.`);
        },
      );
    },

    readImage: (path) => repos.evidence.readImage(path),
    imageUrl: (path, version) => repos.evidence.imageUrl(path, version),

    updateEvidenceDescription: (evidenceId, description) =>
      h.form(
        () => useCases.renameEvidence(repos, evidenceId, description),
        async () => {
          await get().loadEvidence();
          get().pushToast("success", "Descripción guardada.");
        },
      ),

    async moveEvidence(evidenceId, targetCaseId) {
      try {
        await repos.evidence.move(evidenceId, targetCaseId);
        await afterChange();
        const code = get().cases.find((c) => c.id === targetCaseId)?.code ?? "otro caso";
        get().pushToast("success", `Evidencia movida a ${code}.`);
      } catch (error) {
        h.report(error);
      }
    },

    async reorderEvidence(orderedIds) {
      const caseId = get().selectedCaseId;
      if (!caseId) return;
      try {
        set({ evidence: await repos.evidence.reorder(caseId, orderedIds) });
      } catch (error) {
        h.report(error);
        await get().loadEvidence();
      }
    },

    async deleteEvidence(evidenceId) {
      const removed = get().evidence.find((e) => e.id === evidenceId);
      try {
        await repos.evidence.remove(evidenceId);
        await afterChange();
        if (removed) get().pushToast("success", `Evidencia ${removed.position + 1} eliminada.`);
      } catch (error) {
        h.report(error);
      }
    },
  };
}
```

- [ ] **Step 6: Conectar el slice al store (`src/ui/store/workspace.ts`)**

1. Importa `createEvidenceSlice, type EvidenceSlice` desde `./evidence-slice`.
2. `export interface WorkspaceState extends EvidenceSlice {` (el resto de la interfaz no cambia).
3. Envuelve la creación del store para suscribirte a la selección:

```ts
export function createWorkspaceStore(repos: Repositories, prefs: Preferences = browserPreferences()): StoreApi<WorkspaceState> {
  let toastSeq = 0;

  const store = createStore<WorkspaceState>()((set, get) => {
    // …report, refreshLists, loadProject y form sin cambios…

    const refreshCases = async () => {
      const current = get().project;
      if (current) set({ cases: await repos.cases.list(current.id) });
    };

    return {
      // …todos los campos y acciones existentes sin cambios…
      ...createEvidenceSlice(repos, set, get, { report, refreshCases, form }),
    };
  });

  // Las evidencias siguen al caso seleccionado, sin importar qué acción cambió la selección.
  store.subscribe((state, previous) => {
    if (state.selectedCaseId !== previous.selectedCaseId) void state.loadEvidence();
  });
  return store;
}
```

- [ ] **Step 7: Host del editor, Ctrl+V y zona de soltar**

`src/ui/screens/EvidenceEditorHost.tsx`:

```tsx
import { emptyAnnotations, parseAnnotations } from "../../domain/annotations";
import { AnnotationEditor } from "../annotator/AnnotationEditor";
import { useWorkspace } from "../store/context";

export function EvidenceEditorHost() {
  const editor = useWorkspace((s) => s.editor);
  const queue = useWorkspace((s) => s.editorQueue);
  const cases = useWorkspace((s) => s.cases);
  const saveEditor = useWorkspace((s) => s.saveEditor);
  const cancelEditor = useWorkspace((s) => s.cancelEditor);
  const readImage = useWorkspace((s) => s.readImage);
  if (!editor) return null;

  const evidence = editor.kind === "edit" ? editor.evidence : null;
  return (
    <AnnotationEditor
      key={evidence ? `${evidence.id}:${evidence.updatedAt}` : editor.kind === "new" ? editor.imagePath : ""}
      title={evidence ? `Anotar evidencia ${evidence.position + 1}` : "Nueva evidencia"}
      imagePath={evidence ? evidence.originalPath : editor.kind === "new" ? editor.imagePath : ""}
      loadImage={readImage}
      cases={cases.map((c) => ({ id: c.id, code: c.code, title: c.title }))}
      initialCaseId={evidence ? evidence.caseId : editor.kind === "new" ? editor.caseId : null}
      caseLocked={!!evidence}
      initialDescription={evidence?.description ?? ""}
      initialAnnotations={(evidence && parseAnnotations(evidence.annotations)) || emptyAnnotations()}
      pending={queue.length}
      onSave={saveEditor}
      onCancel={() => void cancelEditor()}
    />
  );
}
```

`src/ui/screens/Workspace.tsx` (reemplazar completo):

```tsx
import { useEffect, useState } from "react";
import { isAcceptedImage } from "../../domain/evidence";
import { filesFrom } from "../gallery/files";
import { useEvidenceIntake } from "../gallery/useEvidenceIntake";
import { CaseDetail } from "./CaseDetail";
import { CaseFormDialog } from "./CaseFormDialog";
import { CaseListPane } from "./CaseListPane";
import { EvidenceEditorHost } from "./EvidenceEditorHost";

export function Workspace() {
  const [editing, setEditing] = useState<{ caseId?: string } | null>(null);
  const intake = useEvidenceIntake();

  useEffect(() => {
    // Ctrl+V local (como en la v1): solo si hay una imagen y no hay ningún diálogo abierto.
    const onPaste = (e: ClipboardEvent) => {
      if (document.querySelector('[aria-modal="true"]')) return;
      const images = filesFrom(e.clipboardData).filter(isAcceptedImage);
      if (!images.length) return;
      e.preventDefault();
      void intake(images);
    };
    window.addEventListener("paste", onPaste);
    return () => window.removeEventListener("paste", onPaste);
  }, [intake]);

  return (
    <main className="flex min-h-0 flex-1">
      <CaseListPane onNewCase={() => setEditing({})} />
      <CaseDetail onEdit={(caseId) => setEditing({ caseId })} />
      {editing && <CaseFormDialog caseId={editing.caseId} onClose={() => setEditing(null)} />}
      <EvidenceEditorHost />
    </main>
  );
}
```

Verifica que `Dialog` (`src/ui/components/Dialog.tsx`) ya renderiza `aria-modal="true"`: lo hace, y es la señal que usa `onPaste`.

`src/ui/screens/CaseDetail.tsx`: convierte la `<section aria-label="Detalle del caso">` del caso seleccionado en zona de soltar:

```tsx
import { filesFrom, hasFiles } from "../gallery/files";
import { useEvidenceIntake } from "../gallery/useEvidenceIntake";
// …
  const intake = useEvidenceIntake();
  const [dropping, setDropping] = useState(false);
// …
    <section
      aria-label="Detalle del caso"
      className={`flex-1 overflow-y-auto p-6 ${dropping ? "bg-t4s-blue/5 outline-2 outline-dashed outline-t4s-blue" : ""}`}
      onDragOver={(e) => {
        if (!hasFiles(e.dataTransfer)) return;
        e.preventDefault();
        setDropping(true);
      }}
      onDragLeave={() => setDropping(false)}
      onDrop={(e) => {
        if (!hasFiles(e.dataTransfer)) return;
        e.preventDefault();
        setDropping(false);
        void intake(filesFrom(e.dataTransfer));
      }}
    >
```

(Los hooks deben ir antes de los `return` tempranos del componente.)

- [ ] **Step 8: Verificar, probar en la app y commit**

```bash
pnpm test && pnpm typecheck && pnpm build
pnpm tauri dev
```

Prueba manual rápida en `pnpm tauri dev`:
1. `Win+Shift+S` y luego `Ctrl+V` → se abre el editor con la captura.
2. Dibuja una flecha, un ①, un texto, un recuadro, una elipse, un resaltado, un desenfoque y un recorte. Mueve y redimensiona con la herramienta Seleccionar, y usa `Ctrl+Z` / `Ctrl+Y`.
3. Guarda con descripción. En `%APPDATA%\com.teams4soft.qapture\evidence\<proyecto>\` quedan `.orig.png` y `.render.png`. El render muestra exactamente lo dibujado, a resolución nativa y con el recorte aplicado.

```bash
git add src
git commit -m "feat(f2): flujo de registro de evidencias (pegar, arrastrar, cola y editor)

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 12: Galería en el detalle — miniaturas, visor, reordenar, reasignar, re-anotar y eliminar

**Files:**
- Create: `src/ui/gallery/EvidenceGallery.tsx`, `EvidenceCard.tsx`, `EvidenceViewer.tsx`, `DescriptionDialog.tsx`
- Modify: `src/ui/screens/CaseDetail.tsx` (galería en lugar del placeholder y confirmación con el número de evidencias), `src/ui/screens/CaseListPane.tsx` (contador), `src/ui/screens/Evidence.test.tsx`

**Interfaces:**
- Consumes: `EvidenceSlice` (T11), `moveItem`, `evidenceLabel`, `ACCEPTED_IMAGE_TYPES` (T6), `useEvidenceIntake`, `ConfirmDialog`, `Dialog`.
- Produces: `<EvidenceGallery caseRecord={CaseRecord} />` y un contador con `aria-label="N evidencia(s)"` en la lista de casos.

- [ ] **Step 1: Tests que fallan** (agregar a `src/ui/screens/Evidence.test.tsx`)

```tsx
import { emptyAnnotations, serializeAnnotations } from "../../domain/annotations";

async function seedWithEvidence(repos: Repositories, descriptions = ["Primera", "Segunda"]) {
  const seeded = await seedCases(repos);
  for (const description of descriptions) {
    const staged = await repos.evidence.stage(new Uint8Array(PNG_BYTES));
    await repos.evidence.add(
      { caseId: seeded.a.id, stagedPath: staged.path, description, annotations: serializeAnnotations(emptyAnnotations()) },
      new Uint8Array(PNG_BYTES),
    );
  }
  return seeded;
}

const gallery = () => screen.getByRole("region", { name: "Evidencias" });
const cards = () => within(screen.getByRole("list", { name: "Lista de evidencias" })).getAllByRole("listitem");

describe("galería de evidencias", () => {
  it("muestra miniaturas, el contador en la lista y el visor", async () => {
    const { user } = await renderApp({ setup: async (repos) => void (await seedWithEvidence(repos)) });
    expect(await within(gallery()).findByRole("heading", { name: "Evidencias (2)" })).toBeInTheDocument();
    const list = screen.getByRole("list", { name: "Lista de casos" });
    expect(within(list).getByLabelText("2 evidencias")).toBeInTheDocument();
    await user.click(within(gallery()).getByRole("button", { name: "Ver evidencia 1" }));
    const viewer = screen.getByRole("dialog", { name: "Evidencia 1 de 2" });
    await user.keyboard("{ArrowRight}");
    expect(within(viewer).getByText("Segunda")).toBeInTheDocument();
    expect(screen.getByRole("dialog", { name: "Evidencia 2 de 2" })).toBeInTheDocument();
  });

  it("reordena, edita la descripción y re-anota sin dejar la miniatura en caché", async () => {
    const { user } = await renderApp({ setup: async (repos) => void (await seedWithEvidence(repos)) });
    await within(gallery()).findByRole("heading", { name: "Evidencias (2)" });
    await user.click(within(gallery()).getByRole("button", { name: "Mover evidencia 2 antes" }));
    await waitFor(() => expect(within(cards()[0]).getByText("Segunda")).toBeInTheDocument());

    await user.click(within(gallery()).getByRole("button", { name: "Editar descripción de evidencia 1" }));
    const dialog = screen.getByRole("dialog", { name: "Editar descripción" });
    await user.clear(within(dialog).getByLabelText("Descripción"));
    await user.type(within(dialog).getByLabelText("Descripción"), "Segunda editada");
    await user.click(within(dialog).getByRole("button", { name: "Guardar" }));
    expect(await within(gallery()).findByText("Segunda editada")).toBeInTheDocument();

    const before = within(gallery()).getByRole("img", { name: "Evidencia 1" }).getAttribute("src");
    await user.click(within(gallery()).getByRole("button", { name: "Anotar evidencia 1" }));
    const editorDialog = await screen.findByRole("dialog", { name: "Editor de evidencia" });
    expect(within(editorDialog).getByRole("heading", { name: "Anotar evidencia 1" })).toBeInTheDocument();
    expect(within(editorDialog).getByLabelText("Descripción")).toHaveValue("Segunda editada");
    await user.click(within(editorDialog).getByRole("button", { name: "Guardar evidencia" }));
    expect(await screen.findByText("Evidencia 1 actualizada.")).toBeInTheDocument();
    expect(within(gallery()).getByRole("img", { name: "Evidencia 1" }).getAttribute("src")).not.toBe(before);
  });

  it("reasigna a otro caso y elimina con confirmación", async () => {
    const { user } = await renderApp({ setup: async (repos) => void (await seedWithEvidence(repos, ["Única"])) });
    await within(gallery()).findByRole("heading", { name: "Evidencias (1)" });
    await user.selectOptions(within(gallery()).getByLabelText("Mover evidencia 1 a otro caso"), "EJ-002 · Pólizas");
    expect(await screen.findByText("Evidencia movida a EJ-002.")).toBeInTheDocument();
    expect(within(gallery()).getByRole("heading", { name: "Evidencias (0)" })).toBeInTheDocument();
    const list = screen.getByRole("list", { name: "Lista de casos" });
    expect(within(list).getByLabelText("1 evidencia")).toBeInTheDocument();

    await user.click(within(list).getByText("Pólizas"));
    await within(gallery()).findByRole("heading", { name: "Evidencias (1)" });
    await user.click(within(gallery()).getByRole("button", { name: "Eliminar evidencia 1" }));
    await user.click(within(screen.getByRole("dialog", { name: "Eliminar evidencia 1" })).getByRole("button", { name: "Eliminar evidencia" }));
    expect(await screen.findByText("Evidencia 1 eliminada.")).toBeInTheDocument();
    expect(within(list).queryByLabelText("1 evidencia")).not.toBeInTheDocument();
  });

  it("elegir archivo abre el editor", async () => {
    const { user } = await renderApp({ setup: async (repos) => void (await seedCases(repos)) });
    await user.upload(screen.getByLabelText("Elegir imágenes de evidencia"), png());
    expect(await screen.findByRole("dialog", { name: "Editor de evidencia" })).toBeInTheDocument();
  });
});
```

(Une los imports con los de Task 11 al inicio del archivo; `waitFor` ya está importado.)

- [ ] **Step 2: Verificar que fallan**

Run: `pnpm vitest run src/ui/screens/Evidence.test.tsx`
Expected: FAIL (no existe la región "Evidencias").

- [ ] **Step 3: `EvidenceCard.tsx`**

```tsx
import { ArrowLeft, ArrowRight, Pencil, PenLine, Trash2 } from "lucide-react";
import type { CaseRecord } from "../../domain/cases";
import { evidenceLabel, type Evidence } from "../../domain/evidence";
import { iconButton, inputCls } from "../components/styles";

export const DRAG_TYPE = "application/x-qapture-evidence";

export interface EvidenceCardProps {
  evidence: Evidence;
  url: string;
  isFirst: boolean;
  isLast: boolean;
  otherCases: CaseRecord[];
  onView(): void;
  onAnnotate(): void;
  onRename(): void;
  onShift(delta: -1 | 1): void;
  onMove(targetCaseId: string): void;
  onDelete(): void;
  onDropEvidence(draggedId: string): void;
}

export function EvidenceCard(p: EvidenceCardProps) {
  const label = evidenceLabel(p.evidence);
  return (
    <li
      draggable
      onDragStart={(e) => {
        e.dataTransfer.setData(DRAG_TYPE, p.evidence.id);
        e.dataTransfer.effectAllowed = "move";
      }}
      onDragOver={(e) => {
        if (Array.from(e.dataTransfer.types).includes(DRAG_TYPE)) e.preventDefault();
      }}
      onDrop={(e) => {
        const dragged = e.dataTransfer.getData(DRAG_TYPE);
        if (!dragged) return;
        e.preventDefault();
        e.stopPropagation();
        if (dragged !== p.evidence.id) p.onDropEvidence(dragged);
      }}
      className="overflow-hidden rounded-xl border border-t4s-line bg-white"
    >
      <button type="button" aria-label={`Ver ${label.toLowerCase()}`} onClick={p.onView} className="block aspect-video w-full bg-t4s-cream">
        <img src={p.url} alt={label} className="h-full w-full object-contain" draggable={false} />
      </button>
      <div className="space-y-2 p-3">
        <p className="text-xs font-semibold text-t4s-blue">{label}</p>
        <p className="line-clamp-3 whitespace-pre-wrap text-sm">{p.evidence.description}</p>
        <div className="flex flex-wrap items-center gap-1">
          <button type="button" aria-label={`Anotar ${label.toLowerCase()}`} title="Anotar" className={iconButton} onClick={p.onAnnotate}>
            <PenLine size={16} />
          </button>
          <button type="button" aria-label={`Editar descripción de ${label.toLowerCase()}`} title="Editar descripción" className={iconButton} onClick={p.onRename}>
            <Pencil size={16} />
          </button>
          <button type="button" aria-label={`Mover ${label.toLowerCase()} antes`} title="Mover antes" className={iconButton} disabled={p.isFirst} onClick={() => p.onShift(-1)}>
            <ArrowLeft size={16} />
          </button>
          <button type="button" aria-label={`Mover ${label.toLowerCase()} después`} title="Mover después" className={iconButton} disabled={p.isLast} onClick={() => p.onShift(1)}>
            <ArrowRight size={16} />
          </button>
          <button type="button" aria-label={`Eliminar ${label.toLowerCase()}`} title="Eliminar" className={iconButton} onClick={p.onDelete}>
            <Trash2 size={16} />
          </button>
        </div>
        {p.otherCases.length > 0 && (
          <select aria-label={`Mover ${label.toLowerCase()} a otro caso`} className={`${inputCls} py-1 text-xs`} value="" onChange={(e) => e.target.value && p.onMove(e.target.value)}>
            <option value="">Mover a otro caso…</option>
            {p.otherCases.map((c) => (
              <option key={c.id} value={c.id}>
                {c.code} · {c.title}
              </option>
            ))}
          </select>
        )}
      </div>
    </li>
  );
}
```

- [ ] **Step 4: `EvidenceViewer.tsx` y `DescriptionDialog.tsx`**

`src/ui/gallery/EvidenceViewer.tsx`:

```tsx
import { ChevronLeft, ChevronRight, PenLine } from "lucide-react";
import { useEffect } from "react";
import type { Evidence } from "../../domain/evidence";
import { Dialog } from "../components/Dialog";
import { btn } from "../components/styles";

export function EvidenceViewer({ items, index, urlFor, onIndex, onAnnotate, onClose }: {
  items: Evidence[];
  index: number;
  urlFor(e: Evidence): string;
  onIndex(index: number): void;
  onAnnotate(e: Evidence): void;
  onClose(): void;
}) {
  const current = items[index];
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "ArrowLeft" && index > 0) onIndex(index - 1);
      if (e.key === "ArrowRight" && index < items.length - 1) onIndex(index + 1);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [index, items.length, onIndex]);
  if (!current) return null;

  return (
    <Dialog title={`Evidencia ${index + 1} de ${items.length}`} wide onClose={onClose}>
      <img src={urlFor(current)} alt={`Evidencia ${index + 1}`} className="max-h-[65vh] w-full rounded-lg bg-t4s-cream object-contain" />
      <p className="mt-3 whitespace-pre-wrap text-sm">{current.description}</p>
      <div className="mt-4 flex justify-between">
        <div className="flex gap-2">
          <button type="button" className={btn.secondary} disabled={index === 0} onClick={() => onIndex(index - 1)}>
            <ChevronLeft size={16} /> Anterior
          </button>
          <button type="button" className={btn.secondary} disabled={index === items.length - 1} onClick={() => onIndex(index + 1)}>
            Siguiente <ChevronRight size={16} />
          </button>
        </div>
        <button type="button" className={btn.primary} onClick={() => onAnnotate(current)}>
          <PenLine size={16} /> Anotar
        </button>
      </div>
    </Dialog>
  );
}
```

`src/ui/gallery/DescriptionDialog.tsx`:

```tsx
import { useState } from "react";
import type { Evidence } from "../../domain/evidence";
import { Dialog, DialogFooter } from "../components/Dialog";
import { FormField } from "../components/FormField";
import { btn, inputCls } from "../components/styles";
import { useWorkspace } from "../store/context";

export function DescriptionDialog({ evidence, onClose }: { evidence: Evidence; onClose: () => void }) {
  const update = useWorkspace((s) => s.updateEvidenceDescription);
  const [value, setValue] = useState(evidence.description);
  const [error, setError] = useState<string | undefined>();
  const [saving, setSaving] = useState(false);
  return (
    <Dialog title="Editar descripción" onClose={onClose}>
      <form
        className="grid gap-4"
        onSubmit={async (e) => {
          e.preventDefault();
          setSaving(true);
          const result = await update(evidence.id, value);
          setSaving(false);
          if (result.ok) onClose();
          else setError(result.errors.description ?? result.errors._form);
        }}
      >
        <FormField label="Descripción" htmlFor="evidence-description-edit" error={error}>
          <textarea id="evidence-description-edit" rows={6} className={inputCls} value={value} onChange={(e) => setValue(e.target.value)} />
        </FormField>
        <DialogFooter>
          <button type="button" className={btn.secondary} onClick={onClose}>
            Cancelar
          </button>
          <button type="submit" className={btn.primary} disabled={saving}>
            Guardar
          </button>
        </DialogFooter>
      </form>
    </Dialog>
  );
}
```

- [ ] **Step 5: `EvidenceGallery.tsx`**

```tsx
import { Upload } from "lucide-react";
import { useRef, useState } from "react";
import type { CaseRecord } from "../../domain/cases";
import { ACCEPTED_IMAGE_TYPES, evidenceLabel, moveItem, type Evidence } from "../../domain/evidence";
import { ConfirmDialog } from "../components/ConfirmDialog";
import { btn } from "../components/styles";
import { useWorkspace } from "../store/context";
import { DescriptionDialog } from "./DescriptionDialog";
import { EvidenceCard } from "./EvidenceCard";
import { EvidenceViewer } from "./EvidenceViewer";
import { useEvidenceIntake } from "./useEvidenceIntake";

export function EvidenceGallery({ caseRecord }: { caseRecord: CaseRecord }) {
  const evidence = useWorkspace((s) => s.evidence);
  const cases = useWorkspace((s) => s.cases);
  const imageUrl = useWorkspace((s) => s.imageUrl);
  const editEvidence = useWorkspace((s) => s.editEvidence);
  const moveEvidence = useWorkspace((s) => s.moveEvidence);
  const reorderEvidence = useWorkspace((s) => s.reorderEvidence);
  const deleteEvidence = useWorkspace((s) => s.deleteEvidence);
  const intake = useEvidenceIntake();
  const picker = useRef<HTMLInputElement>(null);
  const [viewing, setViewing] = useState<number | null>(null);
  const [renaming, setRenaming] = useState<Evidence | null>(null);
  const [deleting, setDeleting] = useState<Evidence | null>(null);
  const ids = evidence.map((e) => e.id);
  const others = cases.filter((c) => c.id !== caseRecord.id);
  const urlFor = (e: Evidence) => imageUrl(e.renderedPath, e.updatedAt);
  const annotate = (e: Evidence) => {
    setViewing(null);
    editEvidence(e.id);
  };

  return (
    <section aria-label="Evidencias" className="mt-6">
      <div className="flex items-center justify-between">
        <h3 className="font-semibold">Evidencias ({evidence.length})</h3>
        <button type="button" className={btn.secondary} onClick={() => picker.current?.click()}>
          <Upload size={16} /> Elegir imagen
        </button>
        <input
          ref={picker}
          type="file"
          hidden
          multiple
          accept={ACCEPTED_IMAGE_TYPES.join(",")}
          aria-label="Elegir imágenes de evidencia"
          onChange={async (e) => {
            const files = Array.from(e.target.files ?? []);
            e.target.value = "";
            await intake(files);
          }}
        />
      </div>
      {evidence.length === 0 ? (
        <p className="mt-3 rounded-xl border border-dashed border-t4s-line p-6 text-center text-sm text-t4s-gray">
          Pega una captura con <kbd className="rounded bg-t4s-cream px-1">Ctrl</kbd>+<kbd className="rounded bg-t4s-cream px-1">V</kbd>, arrástrala aquí o elige un archivo.
        </p>
      ) : (
        <ol aria-label="Lista de evidencias" className="mt-3 grid grid-cols-2 gap-4 xl:grid-cols-3">
          {evidence.map((e, index) => (
            <EvidenceCard
              key={e.id}
              evidence={e}
              url={urlFor(e)}
              isFirst={index === 0}
              isLast={index === evidence.length - 1}
              otherCases={others}
              onView={() => setViewing(index)}
              onAnnotate={() => annotate(e)}
              onRename={() => setRenaming(e)}
              onShift={(delta) => void reorderEvidence(moveItem(ids, index, index + delta))}
              onMove={(target) => void moveEvidence(e.id, target)}
              onDelete={() => setDeleting(e)}
              onDropEvidence={(dragged) => void reorderEvidence(moveItem(ids, ids.indexOf(dragged), index))}
            />
          ))}
        </ol>
      )}
      {viewing !== null && (
        <EvidenceViewer items={evidence} index={viewing} urlFor={urlFor} onIndex={setViewing} onAnnotate={annotate} onClose={() => setViewing(null)} />
      )}
      {renaming && <DescriptionDialog evidence={renaming} onClose={() => setRenaming(null)} />}
      {deleting && (
        <ConfirmDialog
          title={`Eliminar ${evidenceLabel(deleting).toLowerCase()}`}
          message="La captura original, su versión anotada y su descripción se eliminarán de forma permanente."
          confirmLabel="Eliminar evidencia"
          onCancel={() => setDeleting(null)}
          onConfirm={async () => {
            const target = deleting;
            setDeleting(null);
            await deleteEvidence(target.id);
          }}
        />
      )}
    </section>
  );
}
```

El título del `ConfirmDialog` queda como "Eliminar evidencia 1", que es lo que buscan los tests.

- [ ] **Step 6: Detalle y lista de casos**

`src/ui/screens/CaseDetail.tsx`:
- Reemplaza el bloque placeholder ("Las evidencias … llegan en la fase F2.") por `<EvidenceGallery caseRecord={current} />` (import desde `../gallery/EvidenceGallery`).
- En el `ConfirmDialog` de eliminar caso:

```tsx
          message={
            current.evidenceCount
              ? `El caso y sus ${current.evidenceCount} evidencias se eliminarán de forma permanente. Su código no se reutilizará.`
              : "El caso se eliminará de forma permanente. Su código no se reutilizará."
          }
```

`src/ui/screens/CaseListPane.tsx`: importa `Images` de `lucide-react` y, en el botón de cada caso, reemplaza la línea del entorno por:

```tsx
              <span className="flex items-center justify-between gap-2">
                <span className="truncate text-xs text-t4s-gray">{envField ? c.values[envField.id] ?? "" : ""}</span>
                {c.evidenceCount > 0 && (
                  <span aria-label={`${c.evidenceCount} ${c.evidenceCount === 1 ? "evidencia" : "evidencias"}`} className="inline-flex shrink-0 items-center gap-1 rounded-full bg-t4s-cream px-2 text-xs text-t4s-gray">
                    <Images size={12} /> {c.evidenceCount}
                  </span>
                )}
              </span>
```

- [ ] **Step 7: Verificar y commit**

```bash
pnpm test && pnpm typecheck && pnpm build
git add src
git commit -m "feat(f2): galería de evidencias con visor, reordenar, reasignar y eliminar

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 13: Verificación integral, versión `0.2.0` y cierre del contrato

**Files:**
- Modify: `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json` (versión `0.2.0`), `src-tauri/Cargo.lock`
- Modify: `docs/superpowers/contracts/F2-evidencias-anotaciones.md`, `docs/superpowers/contracts/README.md`

- [ ] **Step 1: Versión**

Cambia `"version": "0.1.0"` → `"0.2.0"` en `package.json` y `src-tauri/tauri.conf.json`, y `version = "0.1.0"` → `"0.2.0"` en `src-tauri/Cargo.toml`. Luego `cd src-tauri && cargo check` para actualizar `Cargo.lock`.

- [ ] **Step 2: Verificación completa**

```bash
cd src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test && cd ..
pnpm typecheck && pnpm test && pnpm build
pnpm tauri build
```

Expected: todo en verde. Instalador en `src-tauri/target/release/bundle/nsis/Qapture_0.2.0_x64-setup.exe`.

- [ ] **Step 3: Revisión de criterios de aceptación (automáticos)**

Marca en `docs/superpowers/contracts/F2-evidencias-anotaciones.md` los cuatro criterios, cada uno con el test que lo cubre:
- Guardar es atómico → `repo::evidence::tests::add_rolls_back_files_when_the_db_fails_and_can_retry`, `add_leaves_no_row_when_files_fail` y `update_replaces_the_render_and_keeps_the_original`.
- Eliminar caso o proyecto elimina archivos → `deleting_a_case_or_project_removes_its_files`.
- Serialización ida y vuelta y renumeración → `src/domain/annotations.test.ts`.
- Imágenes inválidas o de más de 12 000 px → `media::raster::tests::rejects_*`.

En la tabla **Tasks** del contrato, agrega una columna o marca `✅` a F2-T1…F2-T9. En `README.md`, cambia el estado de F2 a `🧪 En pruebas del equipo`.

- [ ] **Step 4: Commit**

```bash
git add -A package.json src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/tauri.conf.json docs
git commit -m "chore(f2): versión 0.2.0 y cierre técnico del contrato F2

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

- [ ] **Step 5: Entrega al equipo**

Pide a la persona usuaria que ejecute el **Checklist de pruebas locales** del contrato F2 (pruebas 1–9) con el instalador `Qapture_0.2.0_x64-setup.exe`, sobre una instalación que ya tenga datos de F1 (así se valida también el respaldo en `%APPDATA%\com.teams4soft.qapture\backups\`). La fase se cierra cuando el checklist está firmado. Push y PR, solo cuando lo pida.

---

## Probar un build

```bash
git pull && pnpm install
pnpm tauri build
# src-tauri/target/release/bundle/nsis/Qapture_0.2.0_x64-setup.exe
```

Datos de prueba: `%APPDATA%\com.teams4soft.qapture\` (`qapture.db`, `evidence\`, `backups\`). Para empezar de cero, sal desde la bandeja y borra esa carpeta.

## Ejecución

Recomendado: **subagent-driven**. Hay 13 tasks con interfaces encadenadas (DTO → puerto → slice → UI), y un error en el contrato de una task rompe las siguientes, así que conviene un revisor por task. Las tasks Rust 1–5 y las TS 6–8 son independientes entre sí, y se pueden delegar a Codex (`/codex:rescue`) indicando la task y este plan. Claude revisa cada diff contra su sección.
