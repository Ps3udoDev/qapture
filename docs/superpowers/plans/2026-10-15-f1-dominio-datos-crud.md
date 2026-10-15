# F1 — Dominio, SQLite y CRUD · Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Que un tester cree proyectos desde plantillas, personalice columnas y registre, edite, duplique, filtre y elimine casos de prueba, con persistencia en SQLite y la identidad visual Qapture/T4S.

**Architecture:** Rust (Tauri 2) es la autoridad de datos: SQLite con migraciones `user_version`, repositorios por agregado, validación de dominio y comandos `invoke`. React 19 + TS refleja la validación para dar respuesta inmediata, consume los comandos a través de puertos (`ports/`) implementados por un adaptador Tauri y por uno en memoria (para tests), y orquesta el estado con Zustand. Un test de contrato (fixture JSON generado por Rust y validado con `zod`) evita que TS y Rust se desincronicen.

**Tech Stack:** Tauri 2, Rust 2024, rusqlite 0.40 (bundled), chrono, uuid, thiserror · React 19, TypeScript 6, Vite 8, Tailwind CSS 4, Zustand 5, zod 4, lucide-react · Vitest 5, Testing Library, jsdom.

**Spec:** [`docs/superpowers/specs/2026-10-15-qapture-design.md`](../specs/2026-10-15-qapture-design.md) · **Contrato:** [`docs/superpowers/contracts/F1-dominio-datos-crud.md`](../contracts/F1-dominio-datos-crud.md)

## Global Constraints

- Trabajo en la rama `feat/f1-dominio-datos-crud` creada desde `main`. Un commit por task y todos los mensajes terminan con `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Plataforma Windows; comandos en Git Bash desde la raíz del repo `qapture/` (los comandos Rust, desde `qapture/src-tauri/`).
- Identificador `com.teams4soft.qapture`; nombre de producto `Qapture`; ventana "Qapture · Teams4Soft".
- Textos visibles al usuario **en español**. Comillas angulares en los mensajes de campo: `«Entorno» es obligatorio.`
- Paleta: `t4s-blue #2B629E`, `t4s-lime #BAC82D`, `t4s-charcoal #292B2D`, `t4s-cream #F6F4F1`, `t4s-gray #596069`.
- Estados fijos (en este orden): `No ejecutado`, `En ejecución`, `Por documentar`, `Aprobado`, `Fallido`, `Bloqueado`. Valor por defecto: `No ejecutado`.
- Códigos de ejecución: `EJ-` + secuencia de al menos 3 dígitos (`EJ-001`, `EJ-1000`); nunca se reutilizan.
- Tipos de campo: `text` (máx. 500), `longtext` (máx. 16000), `list` (máx. 100), `date` (`AAAA-MM-DD`), `number` (acepta coma decimal `1,5` → `1.5`), `boolean` (`"true"`/`"false"`). La longitud se cuenta en caracteres Unicode (`chars().count()` en Rust y `[...s].length` en TS).
- Límites: título del caso ≤ 300; nombre de proyecto ≤ 150; cliente ≤ 150; tipo de prueba ≤ 100; versión ≤ 100; columnas por proyecto ≤ 60; opciones por lista 1–50 (≤ 100 caracteres cada una); ancho de columna 8–100.
- Repo público: **nunca** se versionan datos reales de clientes. Fixtures y tests usan datos ficticios ("ACME", "Recibos").
- Rust es la fuente de verdad de las plantillas incluidas (`tpl-funcional`, `tpl-regresion`, `tpl-api`, `tpl-uat`).
- Sin `tauri-specta`: los tipos se protegen con el test de contrato (spec D12).

## Review Focus

1. **Columnas desactualizadas en un formulario abierto:** si las columnas cambiaron después de abrir el formulario, guardar debe mostrar "Recarga el proyecto", recargar las columnas en silencio y aceptar el segundo intento → test en Task 11.
2. **El último proyecto recordado ya no existe** (se borró o la db se reinició): la app debe abrir el primero disponible, sin error → test en Task 11.
3. **Emoji y acentos en textos:** el límite de longitud debe dar el mismo resultado en TS y en Rust (un emoji cuenta 1) → tests en Task 4 y Task 9.
4. **Coma decimal en campos numéricos** (`1,5`), habitual en usuarios de habla hispana: se acepta y se guarda como `1.5`; `1.234,5` se rechaza → tests en Task 4 y Task 9.
5. **Dos instancias abiertas a la vez** (no hay instancia única hasta F6): la segunda no debe fallar con "database is locked" → `busy_timeout` de 5 s y test en Task 3.

## File Structure

```
qapture/
├─ .gitattributes                                  (T1) normaliza EOL a LF
├─ vite.config.ts                                  (T1) Tailwind + Vitest
├─ src/
│  ├─ main.tsx                                     (T1, T12) monta store + App
│  ├─ App.tsx                                      (T1, T12)
│  ├─ test/setup.ts                                (T1)
│  ├─ assets/brand/qapture-mark.svg                (T2)
│  ├─ domain/
│  │  ├─ status.ts  fields.ts  cases.ts  summary.ts  project.ts   (T9) + *.test.ts
│  ├─ ports/
│  │  ├─ errors.ts  repositories.ts                (T10)
│  ├─ infrastructure/
│  │  ├─ tauri/dto.ts  client.ts  repositories.ts  (T10)
│  │  ├─ tauri/__fixtures__/contracts.json         (T8, generado por Rust)
│  │  └─ memory/fixtures.ts  repositories.ts       (T10)
│  ├─ application/use-cases.ts                     (T11)
│  └─ ui/
│     ├─ theme/tokens.css                          (T1)
│     ├─ store/preferences.ts  workspace.ts  context.tsx   (T11)
│     ├─ components/styles.ts  Dialog.tsx  FormField.tsx  Toasts.tsx  StatusBadge.tsx  ConfirmDialog.tsx  FieldControl.tsx
│     ├─ screens/AppShell.tsx  TopBar.tsx  EmptyState.tsx  NewProjectDialog.tsx  ProjectMetaFields.tsx   (T12)
│     ├─ screens/Workspace.tsx  CaseListPane.tsx  CaseDetail.tsx  CaseFormDialog.tsx                     (T13)
│     ├─ screens/ProjectSettingsDialog.tsx  ColumnEditor.tsx  TemplatesPanel.tsx                          (T14)
│     └─ test-utils.tsx                            (T12)
└─ src-tauri/
   ├─ app-icon.svg                                 (T2)
   ├─ Cargo.toml                                   (T3)
   └─ src/
      ├─ lib.rs                                    (T3–T8)
      ├─ error.rs  state.rs  dto.rs                (T3, T8, T5)
      ├─ db/mod.rs  db/seed.rs  db/migrations/0001_init.sql   (T3)
      ├─ domain/mod.rs  status.rs  fields.rs  text.rs         (T4)
      ├─ repo/mod.rs  templates.rs  projects.rs  fields.rs  cases.rs   (T5–T7)
      └─ commands/mod.rs  templates.rs  projects.rs  cases.rs           (T8)
```

---

### Task 1: Tooling base — limpieza de la plantilla, Tailwind con tokens T4S, Vitest y CSP

**Files:**
- Delete: `src/App.css`, `src/assets/react.svg`, `public/tauri.svg`, `public/vite.svg`
- Create: `.gitattributes`, `src/test/setup.ts`, `src/ui/theme/tokens.css`, `src/App.test.tsx`
- Modify: `vite.config.ts` (reemplazar completo), `package.json` (scripts), `tsconfig.json` (target/lib), `src/App.tsx` (reemplazar completo), `src/main.tsx`, `src-tauri/tauri.conf.json` (`app.security`, ventana)

**Interfaces:**
- Produces: clases Tailwind `bg-t4s-*`, `text-t4s-*` y `border-t4s-*` (tokens `--color-t4s-*`); scripts `pnpm test`, `pnpm typecheck`.

- [ ] **Step 1: Crear la rama e instalar dependencias**

```bash
git checkout -b feat/f1-dominio-datos-crud
git rm -q src/App.css src/assets/react.svg public/tauri.svg public/vite.svg
pnpm add zustand zod lucide-react
pnpm add -D vitest jsdom @testing-library/react @testing-library/user-event @testing-library/jest-dom tailwindcss @tailwindcss/vite @types/node
```

- [ ] **Step 2: `.gitattributes`**

```gitattributes
* text=auto eol=lf
*.png binary
*.ico binary
*.icns binary
```

Luego normaliza: `git add --renormalize .`

- [ ] **Step 3: `vite.config.ts` (reemplazar completo)**

```ts
/// <reference types="vitest/config" />
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import process from "node:process";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig(() => ({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  test: {
    environment: "jsdom",
    setupFiles: ["./src/test/setup.ts"],
    include: ["src/**/*.test.{ts,tsx}"],
  },
}));
```

- [ ] **Step 4: Setup de tests, tokens y scripts**

`src/test/setup.ts`:

```ts
import "@testing-library/jest-dom/vitest";
import { afterEach } from "vitest";
import { cleanup } from "@testing-library/react";

afterEach(() => cleanup());
```

`src/ui/theme/tokens.css`:

```css
@import "tailwindcss";

@theme {
  --color-t4s-blue: #2b629e;
  --color-t4s-blue-dark: #1f4a78;
  --color-t4s-lime: #bac82d;
  --color-t4s-charcoal: #292b2d;
  --color-t4s-cream: #f6f4f1;
  --color-t4s-gray: #596069;
  --color-t4s-line: #e3e0da;
  --font-sans: "Segoe UI", Arial, system-ui, sans-serif;
}

html,
body,
#root {
  height: 100%;
}

body {
  @apply bg-t4s-cream font-sans text-t4s-charcoal antialiased;
}
```

En `package.json`, la sección `scripts` queda así:

```json
"scripts": {
  "dev": "vite",
  "build": "tsc && vite build",
  "preview": "vite preview",
  "tauri": "tauri",
  "test": "vitest run",
  "test:watch": "vitest",
  "typecheck": "tsc --noEmit"
}
```

En `tsconfig.json` cambia `"target": "ES2020"` por `"target": "ES2022"` y `"lib": ["ES2020", "DOM"]` por `"lib": ["ES2022", "DOM", "DOM.Iterable"]`.

- [ ] **Step 5: Escribir el test de humo (falla)**

`src/App.test.tsx`:

```tsx
import { render, screen } from "@testing-library/react";
import { expect, it } from "vitest";
import App from "./App";

it("muestra la marca Qapture", () => {
  render(<App />);
  expect(screen.getByRole("heading", { name: "Qapture" })).toBeInTheDocument();
});
```

Run: `pnpm test`
Expected: FAIL (el `App.tsx` de la plantilla importa `./App.css`, que ya no existe).

- [ ] **Step 6: `App.tsx` y `main.tsx` mínimos**

`src/App.tsx` (reemplazar completo):

```tsx
import logo from "./assets/brand/t4s-logo.png";

export default function App() {
  return (
    <main className="grid h-full place-items-center">
      <div className="text-center">
        <img src={logo} alt="Teams4Soft" className="mx-auto h-28" />
        <h1 className="mt-4 text-2xl font-semibold">Qapture</h1>
        <p className="text-t4s-gray">Evidencias QA · Teams4Soft</p>
      </div>
    </main>
  );
}
```

`src/main.tsx`: agrega `import "./ui/theme/tokens.css";` como primera línea.

- [ ] **Step 7: CSP y ventana**

En `src-tauri/tauri.conf.json` reemplaza el bloque `"security": { "csp": null }` por:

```json
"security": {
  "csp": "default-src 'self'; img-src 'self' data: blob: asset: http://asset.localhost; style-src 'self' 'unsafe-inline'; script-src 'self'; connect-src 'self' ipc: http://ipc.localhost; object-src 'none'",
  "devCsp": null
}
```

y en el objeto de la ventana agrega `"minWidth": 960, "minHeight": 600, "center": true`.

- [ ] **Step 8: Verificar**

Run: `pnpm test && pnpm typecheck && pnpm build`
Expected: 1 test PASS; typecheck sin errores; `dist/` generado.

- [ ] **Step 9: Commit**

```bash
git add -A
git commit -m "chore(f1): tooling base con Tailwind T4S, Vitest y CSP"
```

---

### Task 2: Ícono propio de Qapture

**Files:**
- Create: `src-tauri/app-icon.svg`, `src/assets/brand/qapture-mark.svg` (copia)
- Modify (generados): `src-tauri/icons/*`

**Interfaces:**
- Produces: `src/assets/brand/qapture-mark.svg`, que la UI usa en `TopBar` (Task 12).

- [ ] **Step 1: Diseño del ícono** (Q con anillo blanco, obturador lima y marcas de visor, sobre azul T4S)

`src-tauri/app-icon.svg`:

```svg
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024">
  <rect width="1024" height="1024" rx="224" fill="#2B629E"/>
  <g fill="none" stroke="#FFFFFF" stroke-opacity="0.35" stroke-width="40" stroke-linecap="round" stroke-linejoin="round">
    <path d="M176 320V176h144"/>
    <path d="M704 176h144v144"/>
    <path d="M176 704v144h144"/>
  </g>
  <circle cx="488" cy="488" r="232" fill="none" stroke="#FFFFFF" stroke-width="96"/>
  <path d="M640 640 L824 824" stroke="#BAC82D" stroke-width="112" stroke-linecap="round"/>
  <circle cx="488" cy="488" r="72" fill="#BAC82D"/>
</svg>
```

- [ ] **Step 2: Generar los íconos y copiar la marca**

```bash
pnpm tauri icon src-tauri/app-icon.svg
rm -rf src-tauri/icons/android src-tauri/icons/ios
cp src-tauri/app-icon.svg src/assets/brand/qapture-mark.svg
```

Expected: `src-tauri/icons/icon.ico`, `32x32.png`, `128x128.png`, `128x128@2x.png` e `icon.icns` regenerados.

- [ ] **Step 3: Verificar**

Run: `cd src-tauri && cargo check && cd ..`
Expected: compila. Abre `src-tauri/icons/128x128.png` y confirma visualmente la Q blanca con el obturador lima.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat(f1): ícono propio de Qapture"
```

---
### Task 3: SQLite — errores, apertura, migraciones y siembra de plantillas

**Files:**
- Modify: `src-tauri/Cargo.toml`, `src-tauri/src/lib.rs` (agregar módulos)
- Create: `src-tauri/src/error.rs`, `src-tauri/src/db/mod.rs`, `src-tauri/src/db/seed.rs`, `src-tauri/src/db/migrations/0001_init.sql`

**Interfaces:**
- Produces: `error::{AppError, AppResult, validation(msg)}`; `db::{open(&Path), open_in_memory(), now(), new_id(), schema_version(&Connection)}`; las tablas `template`, `template_field`, `project`, `project_field`, `test_case` y `field_value`; las plantillas `tpl-funcional`, `tpl-regresion`, `tpl-api` y `tpl-uat`, con ids de campo `<tpl>-<key>`.

- [ ] **Step 1: Dependencias**

```bash
cd src-tauri
cargo add rusqlite@0.40 --features bundled
cargo add uuid --features v4
cargo add thiserror@2
cargo add chrono --no-default-features --features clock,std
```

- [ ] **Step 2: Escribir los tests de db (fallan)**

`src-tauri/src/db/mod.rs` (de momento solo los tests; la implementación va en el Step 4):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_are_idempotent() {
        let conn = open_in_memory().unwrap();
        assert_eq!(schema_version(&conn).unwrap(), MIGRATIONS.len() as i64);
        migrate(&conn).unwrap();
        assert_eq!(schema_version(&conn).unwrap(), MIGRATIONS.len() as i64);
    }

    #[test]
    fn seeds_four_builtin_templates_once() {
        let conn = open_in_memory().unwrap();
        seed::ensure_builtin_templates(&conn).unwrap();
        let templates: i64 = conn
            .query_row("SELECT COUNT(*) FROM template WHERE builtin = 1", [], |r| r.get(0))
            .unwrap();
        let funcional: i64 = conn
            .query_row("SELECT COUNT(*) FROM template_field WHERE template_id = 'tpl-funcional'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(templates, 4);
        assert_eq!(funcional, 11);
    }

    #[test]
    fn foreign_keys_are_enabled() {
        let conn = open_in_memory().unwrap();
        let fk: i64 = conn.query_row("PRAGMA foreign_keys", [], |r| r.get(0)).unwrap();
        assert_eq!(fk, 1);
    }

    #[test]
    fn two_connections_to_the_same_file_can_write() {
        let dir = std::env::temp_dir().join(format!("qapture-test-{}", new_id()));
        let path = dir.join("nested").join("qapture.db");
        let a = open(&path).unwrap();
        let b = open(&path).unwrap();
        let mode: String = a.query_row("PRAGMA journal_mode", [], |r| r.get(0)).unwrap();
        assert_eq!(mode, "wal");
        for (conn, name) in [(&a, "A"), (&b, "B")] {
            conn.execute(
                "INSERT INTO template (id, name, test_type, builtin, created_at) VALUES (?1, ?2, '', 0, ?3)",
                rusqlite::params![new_id(), name, now()],
            )
            .unwrap();
        }
        let total: i64 = b.query_row("SELECT COUNT(*) FROM template", [], |r| r.get(0)).unwrap();
        assert_eq!(total, 6);
        drop(a);
        drop(b);
        let _ = std::fs::remove_dir_all(dir);
    }
}
```

En `src-tauri/src/lib.rs` agrega al inicio del archivo:

```rust
mod db;
mod error;
```

Run: `cargo test db::`
Expected: FAIL de compilación (`open_in_memory`, `MIGRATIONS` y `seed` no existen).

- [ ] **Step 3: Errores**

`src-tauri/src/error.rs`:

```rust
use serde::{ser::SerializeStruct, Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    Validation(String),
    #[error("{0}")]
    NotFound(String),
    #[error("base de datos: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("archivos: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

pub type AppResult<T> = Result<T, AppError>;

pub fn validation(message: impl Into<String>) -> AppError {
    AppError::Validation(message.into())
}

impl AppError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Validation(_) => "validation",
            Self::NotFound(_) => "not_found",
            Self::Db(_) => "db",
            Self::Io(_) => "io",
            Self::Json(_) => "internal",
        }
    }

    pub fn user_message(&self) -> String {
        match self {
            Self::Validation(m) | Self::NotFound(m) => m.clone(),
            _ => "No se pudo completar la operación. Revisa el espacio disponible e inténtalo nuevamente.".into(),
        }
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("AppError", 3)?;
        s.serialize_field("code", self.code())?;
        s.serialize_field("message", &self.user_message())?;
        s.serialize_field("detail", &self.to_string())?;
        s.end()
    }
}
```

- [ ] **Step 4: Migración, apertura y siembra**

`src-tauri/src/db/migrations/0001_init.sql`:

```sql
CREATE TABLE template (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL UNIQUE COLLATE NOCASE,
  test_type TEXT NOT NULL DEFAULT '',
  builtin INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL
);

CREATE TABLE template_field (
  id TEXT PRIMARY KEY,
  template_id TEXT NOT NULL REFERENCES template(id) ON DELETE CASCADE,
  key TEXT NOT NULL,
  label TEXT NOT NULL,
  type TEXT NOT NULL CHECK (type IN ('text','longtext','list','date','number','boolean')),
  options TEXT NOT NULL DEFAULT '[]',
  required INTEGER NOT NULL DEFAULT 0,
  clear_on_duplicate INTEGER NOT NULL DEFAULT 0,
  width INTEGER NOT NULL DEFAULT 24,
  position INTEGER NOT NULL,
  UNIQUE (template_id, key)
);

CREATE TABLE project (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  client TEXT NOT NULL DEFAULT '',
  test_type TEXT NOT NULL DEFAULT '',
  release_version TEXT NOT NULL DEFAULT '',
  start_date TEXT NOT NULL DEFAULT '',
  end_date TEXT NOT NULL DEFAULT '',
  next_sequence INTEGER NOT NULL DEFAULT 1,
  source_template_id TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE project_field (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES project(id) ON DELETE CASCADE,
  key TEXT NOT NULL,
  label TEXT NOT NULL,
  type TEXT NOT NULL CHECK (type IN ('text','longtext','list','date','number','boolean')),
  options TEXT NOT NULL DEFAULT '[]',
  required INTEGER NOT NULL DEFAULT 0,
  clear_on_duplicate INTEGER NOT NULL DEFAULT 0,
  width INTEGER NOT NULL DEFAULT 24,
  position INTEGER NOT NULL,
  archived INTEGER NOT NULL DEFAULT 0,
  UNIQUE (project_id, key)
);

CREATE TABLE test_case (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES project(id) ON DELETE CASCADE,
  code TEXT NOT NULL,
  title TEXT NOT NULL,
  status TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  UNIQUE (project_id, code)
);

CREATE TABLE field_value (
  case_id TEXT NOT NULL REFERENCES test_case(id) ON DELETE CASCADE,
  field_id TEXT NOT NULL REFERENCES project_field(id) ON DELETE CASCADE,
  value TEXT NOT NULL,
  PRIMARY KEY (case_id, field_id)
);

CREATE INDEX idx_project_field_project ON project_field(project_id);
CREATE INDEX idx_test_case_project ON test_case(project_id);
CREATE INDEX idx_field_value_field ON field_value(field_id);
```

`src-tauri/src/db/mod.rs`: coloca esto **encima** del módulo de tests del Step 2:

```rust
use std::{path::Path, time::Duration};

use rusqlite::Connection;

use crate::error::AppResult;

pub mod seed;

const MIGRATIONS: &[&str] = &[include_str!("migrations/0001_init.sql")];

pub fn open(path: &Path) -> AppResult<Connection> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let conn = Connection::open(path)?;
    conn.busy_timeout(Duration::from_secs(5))?;
    conn.query_row("PRAGMA journal_mode = WAL", [], |_| Ok(()))?;
    init(conn)
}

pub fn open_in_memory() -> AppResult<Connection> {
    init(Connection::open_in_memory()?)
}

fn init(conn: Connection) -> AppResult<Connection> {
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrate(&conn)?;
    seed::ensure_builtin_templates(&conn)?;
    Ok(conn)
}

pub fn schema_version(conn: &Connection) -> AppResult<i64> {
    Ok(conn.query_row("PRAGMA user_version", [], |r| r.get(0))?)
}

fn migrate(conn: &Connection) -> AppResult<()> {
    let current = schema_version(conn)? as usize;
    for (index, sql) in MIGRATIONS.iter().enumerate().skip(current) {
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", (index + 1) as i64)?;
        tx.commit()?;
    }
    Ok(())
}

pub fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}
```

`src-tauri/src/db/seed.rs`:

```rust
use rusqlite::{params, Connection};

use crate::{db::now, error::AppResult};

struct SeedField {
    key: &'static str,
    label: &'static str,
    ty: &'static str,
    options: &'static [&'static str],
    required: bool,
    clear: bool,
    width: i64,
}

fn field(key: &'static str, label: &'static str, ty: &'static str, width: i64) -> SeedField {
    SeedField { key, label, ty, options: &[], required: false, clear: false, width }
}

impl SeedField {
    fn required(mut self) -> Self {
        self.required = true;
        self
    }
    fn clear(mut self) -> Self {
        self.clear = true;
        self
    }
    fn options(mut self, options: &'static [&'static str]) -> Self {
        self.options = options;
        self
    }
}

struct SeedTemplate {
    id: &'static str,
    name: &'static str,
    test_type: &'static str,
    fields: Vec<SeedField>,
}

fn builtins() -> Vec<SeedTemplate> {
    vec![
        SeedTemplate {
            id: "tpl-funcional",
            name: "Funcional",
            test_type: "Funcional",
            fields: vec![
                field("entorno", "Entorno", "text", 21).required(),
                field("modulo", "Módulo", "text", 22),
                field("menu", "Menú", "text", 24),
                field("submenu", "Submenú", "text", 24),
                field("funcionalidad", "Funcionalidad", "text", 28),
                field("datos_entrada", "Datos de entrada", "longtext", 48),
                field("resultado_esperado", "Resultado esperado", "longtext", 44),
                field("resultado_obtenido", "Resultado obtenido", "longtext", 44).clear(),
                field("fecha_ejecucion", "Fecha de ejecución", "date", 15).clear(),
                field("responsable", "Responsable", "text", 24),
                field("observaciones", "Observaciones", "longtext", 40).clear(),
            ],
        },
        SeedTemplate {
            id: "tpl-regresion",
            name: "Regresión",
            test_type: "Regresión",
            fields: vec![
                field("entorno", "Entorno", "text", 21).required(),
                field("modulo", "Módulo", "text", 22),
                field("version_probada", "Versión probada", "text", 18),
                field("caso_origen", "Caso origen", "text", 20),
                field("resultado_esperado", "Resultado esperado", "longtext", 44),
                field("resultado_obtenido", "Resultado obtenido", "longtext", 44).clear(),
                field("fecha_ejecucion", "Fecha de ejecución", "date", 15).clear(),
                field("responsable", "Responsable", "text", 24),
                field("observaciones", "Observaciones", "longtext", 40).clear(),
            ],
        },
        SeedTemplate {
            id: "tpl-api",
            name: "API",
            test_type: "API",
            fields: vec![
                field("entorno", "Entorno", "text", 21).required(),
                field("endpoint", "Endpoint", "text", 40),
                field("metodo", "Método", "list", 12).options(&["GET", "POST", "PUT", "PATCH", "DELETE"]),
                field("request", "Request", "longtext", 48),
                field("response_esperado", "Response esperado", "longtext", 44),
                field("response_obtenido", "Response obtenido", "longtext", 44).clear(),
                field("codigo_http", "Código HTTP", "number", 12).clear(),
                field("fecha_ejecucion", "Fecha de ejecución", "date", 15).clear(),
                field("responsable", "Responsable", "text", 24),
                field("observaciones", "Observaciones", "longtext", 40).clear(),
            ],
        },
        SeedTemplate {
            id: "tpl-uat",
            name: "UAT",
            test_type: "UAT",
            fields: vec![
                field("entorno", "Entorno", "text", 21).required(),
                field("proceso_negocio", "Proceso de negocio", "text", 30),
                field("usuario_clave", "Usuario clave", "text", 24),
                field("pasos", "Pasos", "longtext", 48),
                field("resultado_esperado", "Resultado esperado", "longtext", 44),
                field("resultado_obtenido", "Resultado obtenido", "longtext", 44).clear(),
                field("aceptado", "Aceptado", "boolean", 12).clear(),
                field("fecha_ejecucion", "Fecha de ejecución", "date", 15).clear(),
                field("observaciones", "Observaciones", "longtext", 40).clear(),
            ],
        },
    ]
}

/// Inserta las plantillas incluidas que falten. Es idempotente: las versiones nuevas
/// de la app pueden agregar plantillas sin tocar las existentes.
pub fn ensure_builtin_templates(conn: &Connection) -> AppResult<()> {
    let tx = conn.unchecked_transaction()?;
    for template in builtins() {
        tx.execute(
            "INSERT OR IGNORE INTO template (id, name, test_type, builtin, created_at) VALUES (?1, ?2, ?3, 1, ?4)",
            params![template.id, template.name, template.test_type, now()],
        )?;
        for (position, f) in template.fields.iter().enumerate() {
            tx.execute(
                "INSERT OR IGNORE INTO template_field
                   (id, template_id, key, label, type, options, required, clear_on_duplicate, width, position)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    format!("{}-{}", template.id, f.key),
                    template.id,
                    f.key,
                    f.label,
                    f.ty,
                    serde_json::to_string(f.options)?,
                    f.required,
                    f.clear,
                    f.width,
                    position as i64
                ],
            )?;
        }
    }
    tx.commit()?;
    Ok(())
}
```

- [ ] **Step 5: Ejecutar los tests**

Run: `cargo test db::`
Expected: 4 tests PASS (puede haber warnings de código sin usar; se resuelven en Task 8).

- [ ] **Step 6: Commit**

```bash
cd ..
git add -A
git commit -m "feat(f1): SQLite con migraciones user_version y plantillas incluidas"
```

---

### Task 4: Validación Rust — estados, tipos de campo, textos y slugs

**Files:**
- Create: `src-tauri/src/domain/mod.rs`, `src-tauri/src/domain/status.rs`, `src-tauri/src/domain/fields.rs`, `src-tauri/src/domain/text.rs`
- Modify: `src-tauri/src/lib.rs` (agregar `mod domain;`)

**Interfaces:**
- Consumes: `error::{AppResult, validation}`.
- Produces:
  - `domain::status::{STATUSES: [&str; 6], DEFAULT_STATUS, is_status(&str) -> bool}`
  - `domain::fields::{FieldType (Text|Longtext|List|Date|Number|Boolean; serde lowercase), FieldType::parse(&str) -> AppResult<FieldType>, FieldType::as_str(self), FieldType::is_textual(self), FieldType::max_len(self), FieldRule<'a> { label, field_type, options: &[String], required }, normalize_value(FieldType, &str) -> String, validate_value(&FieldRule, &str) -> Result<(), String>, is_iso_date(&str) -> bool}`
  - `domain::text::{required_text(label, value, max) -> AppResult<String>, optional_text(label, value, max) -> AppResult<String>, slugify(&str) -> String}`

- [ ] **Step 1: Escribir los tests (fallan)**

`src-tauri/src/domain/mod.rs`:

```rust
pub mod fields;
pub mod status;
pub mod text;
```

En `src-tauri/src/lib.rs` agrega `mod domain;` debajo de `mod db;`.

`src-tauri/src/domain/fields.rs`, por ahora solo los tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn rule<'a>(field_type: FieldType, options: &'a [String], required: bool) -> FieldRule<'a> {
        FieldRule { label: "Campo", field_type, options, required }
    }

    #[test]
    fn required_ignores_whitespace() {
        assert_eq!(validate_value(&rule(FieldType::Text, &[], true), "   "), Err("«Campo» es obligatorio.".into()));
        assert_eq!(validate_value(&rule(FieldType::Text, &[], false), ""), Ok(()));
    }

    #[test]
    fn list_values_must_be_options() {
        let options = vec!["Alta".to_string(), "Media".to_string()];
        assert!(validate_value(&rule(FieldType::List, &options, false), "Media").is_ok());
        assert!(validate_value(&rule(FieldType::List, &options, false), "Baja").is_err());
    }

    #[test]
    fn dates_must_exist() {
        assert!(validate_value(&rule(FieldType::Date, &[], false), "2026-02-28").is_ok());
        assert!(validate_value(&rule(FieldType::Date, &[], false), "2026-02-30").is_err());
        assert!(validate_value(&rule(FieldType::Date, &[], false), "2026-2-1").is_err());
    }

    #[test]
    fn numbers_accept_comma_decimal() {
        let r = rule(FieldType::Number, &[], false);
        assert!(validate_value(&r, "1,5").is_ok());
        assert_eq!(normalize_value(FieldType::Number, " 1,5 "), "1.5");
        assert!(validate_value(&r, "-3.25e2").is_ok());
        for bad in ["1.234,5", "0x10", "inf", "NaN", "abc"] {
            assert!(validate_value(&r, bad).is_err(), "{bad} debería rechazarse");
        }
    }

    #[test]
    fn booleans_are_true_or_false() {
        let r = rule(FieldType::Boolean, &[], false);
        assert!(validate_value(&r, "true").is_ok());
        assert!(validate_value(&r, "sí").is_err());
    }

    #[test]
    fn length_counts_unicode_chars() {
        let r = rule(FieldType::Text, &[], false);
        assert!(validate_value(&r, &"🚀".repeat(500)).is_ok());
        assert_eq!(validate_value(&r, &"🚀".repeat(501)), Err("«Campo» supera 500 caracteres.".into()));
    }

    #[test]
    fn longtext_keeps_line_breaks() {
        assert_eq!(normalize_value(FieldType::Longtext, "línea 1\nlínea 2\n"), "línea 1\nlínea 2\n");
        assert_eq!(normalize_value(FieldType::Longtext, " \n "), "");
    }

    #[test]
    fn parse_round_trips() {
        for ty in ["text", "longtext", "list", "date", "number", "boolean"] {
            assert_eq!(FieldType::parse(ty).unwrap().as_str(), ty);
        }
        assert!(FieldType::parse("color").is_err());
    }
}
```

`src-tauri/src/domain/text.rs`, por ahora solo los tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_strips_accents_and_symbols() {
        assert_eq!(slugify("Fecha de ejecución"), "fecha_de_ejecucion");
        assert_eq!(slugify("  ¿Código HTTP?  "), "codigo_http");
        assert_eq!(slugify("Año / Ñandú"), "ano_nandu");
        assert_eq!(slugify("!!!"), "campo");
        assert_eq!(slugify(&"a".repeat(80)).len(), 40);
    }

    #[test]
    fn required_and_optional_text() {
        assert_eq!(required_text("Nombre", "  Hola ", 10).unwrap(), "Hola");
        assert!(required_text("Nombre", " ", 10).is_err());
        assert!(optional_text("Cliente", "abcdef", 5).is_err());
        assert_eq!(optional_text("Cliente", "", 5).unwrap(), "");
    }
}
```

`src-tauri/src/domain/status.rs`, por ahora solo los tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_statuses() {
        assert!(is_status("Aprobado"));
        assert!(!is_status("aprobado"));
        assert_eq!(DEFAULT_STATUS, STATUSES[0]);
    }
}
```

Run: `cd src-tauri && cargo test domain::`
Expected: FAIL de compilación (los símbolos no existen).

- [ ] **Step 2: Implementar `status.rs`** (encima de los tests)

```rust
pub const STATUSES: [&str; 6] = ["No ejecutado", "En ejecución", "Por documentar", "Aprobado", "Fallido", "Bloqueado"];
pub const DEFAULT_STATUS: &str = "No ejecutado";

pub fn is_status(value: &str) -> bool {
    STATUSES.contains(&value)
}
```

- [ ] **Step 3: Implementar `text.rs`** (encima de los tests)

```rust
use crate::error::{validation, AppResult};

pub fn required_text(label: &str, value: &str, max: usize) -> AppResult<String> {
    let v = value.trim();
    if v.is_empty() {
        return Err(validation(format!("«{label}» es obligatorio.")));
    }
    check_max(label, v, max)?;
    Ok(v.to_string())
}

pub fn optional_text(label: &str, value: &str, max: usize) -> AppResult<String> {
    let v = value.trim();
    check_max(label, v, max)?;
    Ok(v.to_string())
}

fn check_max(label: &str, value: &str, max: usize) -> AppResult<()> {
    if value.chars().count() > max {
        return Err(validation(format!("«{label}» supera {max} caracteres.")));
    }
    Ok(())
}

/// Clave estable para una columna: minúsculas ASCII, sin acentos, separada por `_`, máximo 40.
pub fn slugify(label: &str) -> String {
    let mut out = String::new();
    for ch in label.trim().to_lowercase().chars() {
        let c = match ch {
            'á' | 'à' | 'ä' | 'â' => 'a',
            'é' | 'è' | 'ë' | 'ê' => 'e',
            'í' | 'ì' | 'ï' | 'î' => 'i',
            'ó' | 'ò' | 'ö' | 'ô' => 'o',
            'ú' | 'ù' | 'ü' | 'û' => 'u',
            'ñ' => 'n',
            other => other,
        };
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.is_empty() && !out.ends_with('_') {
            out.push('_');
        }
    }
    let limited: String = out.trim_end_matches('_').chars().take(40).collect();
    let limited = limited.trim_end_matches('_');
    if limited.is_empty() { "campo".into() } else { limited.to_string() }
}
```

- [ ] **Step 4: Implementar `fields.rs`** (encima de los tests)

```rust
use serde::{Deserialize, Serialize};

use crate::error::{validation, AppResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    Text,
    Longtext,
    List,
    Date,
    Number,
    Boolean,
}

impl FieldType {
    pub fn parse(value: &str) -> AppResult<Self> {
        Ok(match value {
            "text" => Self::Text,
            "longtext" => Self::Longtext,
            "list" => Self::List,
            "date" => Self::Date,
            "number" => Self::Number,
            "boolean" => Self::Boolean,
            other => return Err(validation(format!("Tipo de campo desconocido: «{other}»."))),
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Longtext => "longtext",
            Self::List => "list",
            Self::Date => "date",
            Self::Number => "number",
            Self::Boolean => "boolean",
        }
    }

    pub fn is_textual(self) -> bool {
        matches!(self, Self::Text | Self::Longtext)
    }

    pub fn max_len(self) -> usize {
        match self {
            Self::Text => 500,
            Self::Longtext => 16000,
            Self::List => 100,
            Self::Date => 10,
            Self::Number => 40,
            Self::Boolean => 5,
        }
    }
}

pub struct FieldRule<'a> {
    pub label: &'a str,
    pub field_type: FieldType,
    pub options: &'a [String],
    pub required: bool,
}

/// Texto: se conserva tal cual (vacío si solo hay espacios). Resto: recortado;
/// los números con coma decimal simple (`1,5`) pasan a punto.
pub fn normalize_value(field_type: FieldType, raw: &str) -> String {
    if field_type.is_textual() {
        return if raw.trim().is_empty() { String::new() } else { raw.to_string() };
    }
    let v = raw.trim();
    if field_type == FieldType::Number && is_comma_decimal(v) {
        return v.replacen(',', ".", 1);
    }
    v.to_string()
}

fn is_comma_decimal(v: &str) -> bool {
    let body = v.strip_prefix(['+', '-']).unwrap_or(v);
    match body.split_once(',') {
        Some((int, frac)) => {
            !int.is_empty()
                && !frac.is_empty()
                && int.bytes().all(|b| b.is_ascii_digit())
                && frac.bytes().all(|b| b.is_ascii_digit())
        }
        None => false,
    }
}

fn is_number(v: &str) -> bool {
    v.bytes().all(|b| b.is_ascii_digit() || matches!(b, b'+' | b'-' | b'.' | b'e' | b'E'))
        && v.parse::<f64>().map(f64::is_finite).unwrap_or(false)
}

pub fn is_iso_date(v: &str) -> bool {
    let b = v.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter().enumerate().all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
        && chrono::NaiveDate::parse_from_str(v, "%Y-%m-%d").is_ok()
}

pub fn validate_value(rule: &FieldRule, raw: &str) -> Result<(), String> {
    let v = normalize_value(rule.field_type, raw);
    let label = rule.label;
    if v.trim().is_empty() {
        return if rule.required { Err(format!("«{label}» es obligatorio.")) } else { Ok(()) };
    }
    let max = rule.field_type.max_len();
    if v.chars().count() > max {
        return Err(format!("«{label}» supera {max} caracteres."));
    }
    match rule.field_type {
        FieldType::Text | FieldType::Longtext => Ok(()),
        FieldType::List if rule.options.iter().any(|o| *o == v) => Ok(()),
        FieldType::List => Err(format!("«{label}» debe ser una de las opciones de la lista.")),
        FieldType::Date if is_iso_date(&v) => Ok(()),
        FieldType::Date => Err(format!("«{label}» debe ser una fecha válida (AAAA-MM-DD).")),
        FieldType::Number if is_number(&v) => Ok(()),
        FieldType::Number => Err(format!("«{label}» debe ser un número.")),
        FieldType::Boolean if v == "true" || v == "false" => Ok(()),
        FieldType::Boolean => Err(format!("«{label}» debe ser Sí o No.")),
    }
}
```

- [ ] **Step 5: Ejecutar los tests**

Run: `cargo test domain::`
Expected: 11 tests PASS.

- [ ] **Step 6: Commit**

```bash
cd ..
git add -A
git commit -m "feat(f1): validación de dominio en Rust (estados, tipos de campo, textos)"
```

---

### Task 5: Repositorios de plantillas y proyectos

**Files:**
- Create: `src-tauri/src/dto.rs`, `src-tauri/src/repo/mod.rs`, `src-tauri/src/repo/fields.rs`, `src-tauri/src/repo/templates.rs`, `src-tauri/src/repo/projects.rs`
- Modify: `src-tauri/src/lib.rs` (agregar `mod dto;` y `mod repo;`)

**Interfaces:**
- Consumes: `db::{new_id, now, open_in_memory}`, `domain::{fields::{FieldType, is_iso_date}, text::{required_text, optional_text}}`, `error::*`.
- Produces:
  - DTO (serde camelCase): `FieldDto { id, key, label, type, options, required, clear_on_duplicate, width, position, archived }`, `TemplateDto { id, name, test_type, builtin, fields }`, `ProjectSummaryDto { id, name, client, case_count, updated_at }`, `ProjectDto { id, name, client, test_type, release_version, start_date, end_date, next_sequence, source_template_id: Option<String>, created_at, updated_at, fields }`.
  - Inputs (`deny_unknown_fields`): `ProjectMetaInput { name, client, test_type, release_version, start_date, end_date }`, `NewProjectInput { meta, template_id }`, `FieldInput { id: Option<String>, label, type, options, required, clear_on_duplicate, width, archived }`.
  - `repo::templates::{list, get, save_from_project(conn, project_id, name, test_type), delete}`
  - `repo::projects::{list, get, ensure_exists, create(conn, NewProjectInput), update_meta(conn, id, &ProjectMetaInput), delete, touch}`
  - `repo::fields::{list_for_template, list_for_project, field_from_row, insert_template_field, insert_project_field}`
  - `repo::test_support::{conn(), meta(name), project(&conn), field_id(&ProjectDto, key), insert_value(&conn, project_id, field_id, value)}` (solo en tests)

- [ ] **Step 1: DTO**

`src-tauri/src/dto.rs`:

```rust
use serde::{Deserialize, Serialize};

use crate::domain::fields::FieldType;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldDto {
    pub id: String,
    pub key: String,
    pub label: String,
    #[serde(rename = "type")]
    pub field_type: FieldType,
    pub options: Vec<String>,
    pub required: bool,
    pub clear_on_duplicate: bool,
    pub width: i64,
    pub position: i64,
    pub archived: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateDto {
    pub id: String,
    pub name: String,
    pub test_type: String,
    pub builtin: bool,
    pub fields: Vec<FieldDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSummaryDto {
    pub id: String,
    pub name: String,
    pub client: String,
    pub case_count: i64,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDto {
    pub id: String,
    pub name: String,
    pub client: String,
    pub test_type: String,
    pub release_version: String,
    pub start_date: String,
    pub end_date: String,
    pub next_sequence: i64,
    pub source_template_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub fields: Vec<FieldDto>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectMetaInput {
    pub name: String,
    pub client: String,
    pub test_type: String,
    pub release_version: String,
    pub start_date: String,
    pub end_date: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NewProjectInput {
    pub meta: ProjectMetaInput,
    pub template_id: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FieldInput {
    pub id: Option<String>,
    pub label: String,
    #[serde(rename = "type")]
    pub field_type: FieldType,
    pub options: Vec<String>,
    pub required: bool,
    pub clear_on_duplicate: bool,
    pub width: i64,
    pub archived: bool,
}
```

En `src-tauri/src/lib.rs` agrega `mod dto;` y `mod repo;` junto a los demás `mod`.

- [ ] **Step 2: Lectura e inserción de columnas**

`src-tauri/src/repo/fields.rs`:

```rust
use rusqlite::{params, types::Type, Connection, Row};

use crate::{db::new_id, domain::fields::FieldType, dto::FieldDto, error::AppResult};

const FIELD_COLUMNS: &str = "id, key, label, type, options, required, clear_on_duplicate, width, position";

pub(crate) fn field_from_row(r: &Row) -> rusqlite::Result<FieldDto> {
    let ty: String = r.get("type")?;
    let options: String = r.get("options")?;
    Ok(FieldDto {
        id: r.get("id")?,
        key: r.get("key")?,
        label: r.get("label")?,
        field_type: FieldType::parse(&ty)
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(3, Type::Text, Box::new(e)))?,
        options: serde_json::from_str(&options)
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(4, Type::Text, Box::new(e)))?,
        required: r.get("required")?,
        clear_on_duplicate: r.get("clear_on_duplicate")?,
        width: r.get("width")?,
        position: r.get("position")?,
        archived: r.get("archived")?,
    })
}

pub fn list_for_template(conn: &Connection, template_id: &str) -> AppResult<Vec<FieldDto>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {FIELD_COLUMNS}, 0 AS archived FROM template_field WHERE template_id = ?1 ORDER BY position"
    ))?;
    let rows = stmt.query_map([template_id], field_from_row)?.collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Columnas del proyecto: primero las activas y luego las archivadas, cada grupo por posición.
pub fn list_for_project(conn: &Connection, project_id: &str) -> AppResult<Vec<FieldDto>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {FIELD_COLUMNS}, archived FROM project_field WHERE project_id = ?1 ORDER BY archived, position"
    ))?;
    let rows = stmt.query_map([project_id], field_from_row)?.collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub(crate) fn insert_template_field(conn: &Connection, template_id: &str, f: &FieldDto, position: i64) -> AppResult<()> {
    conn.execute(
        "INSERT INTO template_field (id, template_id, key, label, type, options, required, clear_on_duplicate, width, position)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            new_id(),
            template_id,
            f.key,
            f.label,
            f.field_type.as_str(),
            serde_json::to_string(&f.options)?,
            f.required,
            f.clear_on_duplicate,
            f.width,
            position
        ],
    )?;
    Ok(())
}

pub(crate) fn insert_project_field(conn: &Connection, project_id: &str, f: &FieldDto, position: i64) -> AppResult<String> {
    let id = new_id();
    conn.execute(
        "INSERT INTO project_field (id, project_id, key, label, type, options, required, clear_on_duplicate, width, position, archived)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            id,
            project_id,
            f.key,
            f.label,
            f.field_type.as_str(),
            serde_json::to_string(&f.options)?,
            f.required,
            f.clear_on_duplicate,
            f.width,
            position,
            f.archived
        ],
    )?;
    Ok(id)
}
```

- [ ] **Step 3: Escribir los tests de proyectos y plantillas (fallan)**

`src-tauri/src/repo/mod.rs`:

```rust
pub mod fields;
pub mod projects;
pub mod templates;

#[cfg(test)]
pub(crate) mod test_support {
    use rusqlite::{params, Connection};

    use crate::{
        db,
        dto::{NewProjectInput, ProjectDto, ProjectMetaInput},
    };

    pub fn conn() -> Connection {
        db::open_in_memory().expect("db en memoria")
    }

    pub fn meta(name: &str) -> ProjectMetaInput {
        ProjectMetaInput {
            name: name.into(),
            client: "ACME".into(),
            test_type: "Funcional".into(),
            release_version: "1.0".into(),
            start_date: "2026-10-01".into(),
            end_date: "2026-10-31".into(),
        }
    }

    pub fn project(conn: &Connection) -> ProjectDto {
        super::projects::create(conn, NewProjectInput { meta: meta("Recibos"), template_id: "tpl-funcional".into() })
            .expect("proyecto")
    }

    pub fn field_id(project: &ProjectDto, key: &str) -> String {
        project.fields.iter().find(|f| f.key == key).unwrap_or_else(|| panic!("falta la columna {key}")).id.clone()
    }

    /// Inserta un caso mínimo con un valor directamente en SQL, para probar columnas
    /// sin depender del repositorio de casos.
    pub fn insert_value(conn: &Connection, project_id: &str, field_id: &str, value: &str) {
        let case_id = db::new_id();
        conn.execute(
            "INSERT INTO test_case (id, project_id, code, title, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'Caso', 'No ejecutado', '', '')",
            params![case_id, project_id, format!("T-{case_id}")],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO field_value (case_id, field_id, value) VALUES (?1, ?2, ?3)",
            params![case_id, field_id, value],
        )
        .unwrap();
    }
}
```

`src-tauri/src/repo/projects.rs`, por ahora solo los tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        dto::NewProjectInput,
        repo::test_support::{conn, meta, project},
    };

    #[test]
    fn create_copies_template_fields_with_new_ids() {
        let conn = conn();
        let p = project(&conn);
        assert_eq!(p.fields.len(), 11);
        assert_eq!(p.fields[0].key, "entorno");
        assert!(p.fields[0].required);
        assert_ne!(p.fields[0].id, "tpl-funcional-entorno");
        assert_eq!(p.next_sequence, 1);
        assert_eq!(p.source_template_id.as_deref(), Some("tpl-funcional"));
    }

    #[test]
    fn create_validates_meta_and_template() {
        let conn = conn();
        let mut bad = meta("  ");
        let err = create(&conn, NewProjectInput { meta: bad.clone(), template_id: "tpl-funcional".into() }).unwrap_err();
        assert_eq!(err.to_string(), "«Nombre del proyecto» es obligatorio.");
        bad.name = "Ok".into();
        bad.start_date = "2026-11-01".into();
        let err = create(&conn, NewProjectInput { meta: bad.clone(), template_id: "tpl-funcional".into() }).unwrap_err();
        assert_eq!(err.to_string(), "La fecha de fin no puede ser anterior a la de inicio.");
        let err = create(&conn, NewProjectInput { meta: meta("Ok"), template_id: "nope".into() }).unwrap_err();
        assert_eq!(err.code(), "not_found");
    }

    #[test]
    fn update_meta_trims_and_touches() {
        let conn = conn();
        let p = project(&conn);
        std::thread::sleep(std::time::Duration::from_millis(5));
        let mut m = meta("  Recibos v2 ");
        m.client = " GMX ".into();
        let updated = update_meta(&conn, &p.id, &m).unwrap();
        assert_eq!(updated.name, "Recibos v2");
        assert_eq!(updated.client, "GMX");
        assert!(updated.updated_at > p.updated_at);
    }

    #[test]
    fn list_and_delete() {
        let conn = conn();
        let p = project(&conn);
        let summaries = list(&conn).unwrap();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].case_count, 0);
        delete(&conn, &p.id).unwrap();
        assert!(list(&conn).unwrap().is_empty());
        let fields: i64 = conn.query_row("SELECT COUNT(*) FROM project_field", [], |r| r.get(0)).unwrap();
        assert_eq!(fields, 0);
        assert_eq!(delete(&conn, &p.id).unwrap_err().code(), "not_found");
    }
}
```

`src-tauri/src/repo/templates.rs`, por ahora solo los tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::repo::test_support::{conn, project};

    #[test]
    fn lists_builtins_first() {
        let conn = conn();
        let all = list(&conn).unwrap();
        assert_eq!(all.len(), 4);
        assert!(all.iter().all(|t| t.builtin));
        let api = all.iter().find(|t| t.id == "tpl-api").unwrap();
        let metodo = api.fields.iter().find(|f| f.key == "metodo").unwrap();
        assert_eq!(metodo.options, vec!["GET", "POST", "PUT", "PATCH", "DELETE"]);
    }

    #[test]
    fn save_from_project_copies_active_fields() {
        let conn = conn();
        let p = project(&conn);
        conn.execute("UPDATE project_field SET archived = 1 WHERE project_id = ?1 AND key = 'menu'", [&p.id]).unwrap();
        let t = save_from_project(&conn, &p.id, " GMX Cobranzas ", "Funcional").unwrap();
        assert_eq!(t.name, "GMX Cobranzas");
        assert!(!t.builtin);
        assert_eq!(t.fields.len(), 10);
        assert!(t.fields.iter().all(|f| f.key != "menu"));
        assert_eq!(t.fields.iter().map(|f| f.position).collect::<Vec<_>>(), (0..10).collect::<Vec<_>>());
    }

    #[test]
    fn template_names_are_unique_case_insensitive() {
        let conn = conn();
        let p = project(&conn);
        let err = save_from_project(&conn, &p.id, "funcional", "").unwrap_err();
        assert_eq!(err.to_string(), "Ya existe una plantilla con ese nombre.");
    }

    #[test]
    fn builtin_templates_cannot_be_deleted() {
        let conn = conn();
        let p = project(&conn);
        assert_eq!(delete(&conn, "tpl-funcional").unwrap_err().to_string(), "Las plantillas incluidas no se pueden eliminar.");
        let t = save_from_project(&conn, &p.id, "Propia", "").unwrap();
        delete(&conn, &t.id).unwrap();
        assert_eq!(get(&conn, &t.id).unwrap_err().code(), "not_found");
    }
}
```

Run: `cd src-tauri && cargo test repo::`
Expected: FAIL de compilación.

- [ ] **Step 4: Implementar `projects.rs`** (encima de los tests)

```rust
use rusqlite::{params, Connection, OptionalExtension, Row};

use super::{fields, templates};
use crate::{
    db::{new_id, now},
    domain::{
        fields::is_iso_date,
        text::{optional_text, required_text},
    },
    dto::{NewProjectInput, ProjectDto, ProjectMetaInput, ProjectSummaryDto},
    error::{validation, AppError, AppResult},
};

fn not_found() -> AppError {
    AppError::NotFound("El proyecto no existe.".into())
}

pub(crate) fn validate_meta(m: &ProjectMetaInput) -> AppResult<ProjectMetaInput> {
    let name = required_text("Nombre del proyecto", &m.name, 150)?;
    let client = optional_text("Cliente", &m.client, 150)?;
    let test_type = optional_text("Tipo de prueba", &m.test_type, 100)?;
    let release_version = optional_text("Versión / release", &m.release_version, 100)?;
    let start_date = optional_text("Fecha de inicio", &m.start_date, 10)?;
    let end_date = optional_text("Fecha de fin", &m.end_date, 10)?;
    for (label, date) in [("Fecha de inicio", &start_date), ("Fecha de fin", &end_date)] {
        if !date.is_empty() && !is_iso_date(date) {
            return Err(validation(format!("«{label}» debe ser una fecha válida (AAAA-MM-DD).")));
        }
    }
    if !start_date.is_empty() && !end_date.is_empty() && start_date > end_date {
        return Err(validation("La fecha de fin no puede ser anterior a la de inicio."));
    }
    Ok(ProjectMetaInput { name, client, test_type, release_version, start_date, end_date })
}

fn project_from_row(r: &Row) -> rusqlite::Result<ProjectDto> {
    Ok(ProjectDto {
        id: r.get("id")?,
        name: r.get("name")?,
        client: r.get("client")?,
        test_type: r.get("test_type")?,
        release_version: r.get("release_version")?,
        start_date: r.get("start_date")?,
        end_date: r.get("end_date")?,
        next_sequence: r.get("next_sequence")?,
        source_template_id: r.get("source_template_id")?,
        created_at: r.get("created_at")?,
        updated_at: r.get("updated_at")?,
        fields: Vec::new(),
    })
}

pub fn ensure_exists(conn: &Connection, id: &str) -> AppResult<()> {
    let exists: bool = conn.query_row("SELECT EXISTS(SELECT 1 FROM project WHERE id = ?1)", [id], |r| r.get(0))?;
    if exists { Ok(()) } else { Err(not_found()) }
}

pub fn get(conn: &Connection, id: &str) -> AppResult<ProjectDto> {
    let project = conn
        .query_row("SELECT * FROM project WHERE id = ?1", [id], project_from_row)
        .optional()?
        .ok_or_else(not_found)?;
    Ok(ProjectDto { fields: fields::list_for_project(conn, id)?, ..project })
}

pub fn list(conn: &Connection) -> AppResult<Vec<ProjectSummaryDto>> {
    let mut stmt = conn.prepare(
        "SELECT p.id, p.name, p.client, p.updated_at,
                (SELECT COUNT(*) FROM test_case c WHERE c.project_id = p.id)
         FROM project p ORDER BY p.updated_at DESC, p.name COLLATE NOCASE",
    )?;
    let rows = stmt
        .query_map([], |r| {
            Ok(ProjectSummaryDto {
                id: r.get(0)?,
                name: r.get(1)?,
                client: r.get(2)?,
                updated_at: r.get(3)?,
                case_count: r.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn create(conn: &Connection, input: NewProjectInput) -> AppResult<ProjectDto> {
    let meta = validate_meta(&input.meta)?;
    let template = templates::get(conn, &input.template_id)?;
    let id = new_id();
    let ts = now();
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "INSERT INTO project (id, name, client, test_type, release_version, start_date, end_date,
                              next_sequence, source_template_id, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, ?8, ?9, ?9)",
        params![id, meta.name, meta.client, meta.test_type, meta.release_version, meta.start_date, meta.end_date, template.id, ts],
    )?;
    for (position, f) in template.fields.iter().enumerate() {
        fields::insert_project_field(&tx, &id, f, position as i64)?;
    }
    tx.commit()?;
    get(conn, &id)
}

pub fn update_meta(conn: &Connection, id: &str, input: &ProjectMetaInput) -> AppResult<ProjectDto> {
    ensure_exists(conn, id)?;
    let m = validate_meta(input)?;
    conn.execute(
        "UPDATE project SET name = ?1, client = ?2, test_type = ?3, release_version = ?4,
                            start_date = ?5, end_date = ?6, updated_at = ?7
         WHERE id = ?8",
        params![m.name, m.client, m.test_type, m.release_version, m.start_date, m.end_date, now(), id],
    )?;
    get(conn, id)
}

pub fn delete(conn: &Connection, id: &str) -> AppResult<()> {
    let removed = conn.execute("DELETE FROM project WHERE id = ?1", [id])?;
    if removed == 0 { Err(not_found()) } else { Ok(()) }
}

pub(crate) fn touch(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute("UPDATE project SET updated_at = ?1 WHERE id = ?2", params![now(), id])?;
    Ok(())
}
```

- [ ] **Step 5: Implementar `templates.rs`** (encima de los tests)

```rust
use rusqlite::{params, Connection, OptionalExtension};

use super::{fields, projects};
use crate::{
    db::{new_id, now},
    domain::text::{optional_text, required_text},
    dto::TemplateDto,
    error::{validation, AppError, AppResult},
};

fn header(r: &rusqlite::Row) -> rusqlite::Result<TemplateDto> {
    Ok(TemplateDto { id: r.get(0)?, name: r.get(1)?, test_type: r.get(2)?, builtin: r.get(3)?, fields: Vec::new() })
}

pub fn list(conn: &Connection) -> AppResult<Vec<TemplateDto>> {
    let mut stmt =
        conn.prepare("SELECT id, name, test_type, builtin FROM template ORDER BY builtin DESC, name COLLATE NOCASE")?;
    let headers = stmt.query_map([], header)?.collect::<Result<Vec<_>, _>>()?;
    headers
        .into_iter()
        .map(|t| Ok(TemplateDto { fields: fields::list_for_template(conn, &t.id)?, ..t }))
        .collect()
}

pub fn get(conn: &Connection, id: &str) -> AppResult<TemplateDto> {
    let t = conn
        .query_row("SELECT id, name, test_type, builtin FROM template WHERE id = ?1", [id], header)
        .optional()?
        .ok_or_else(|| AppError::NotFound("La plantilla no existe.".into()))?;
    Ok(TemplateDto { fields: fields::list_for_template(conn, id)?, ..t })
}

pub fn save_from_project(conn: &Connection, project_id: &str, name: &str, test_type: &str) -> AppResult<TemplateDto> {
    let name = required_text("Nombre de la plantilla", name, 100)?;
    let test_type = optional_text("Tipo de prueba", test_type, 100)?;
    projects::ensure_exists(conn, project_id)?;
    let taken: bool =
        conn.query_row("SELECT EXISTS(SELECT 1 FROM template WHERE name = ?1 COLLATE NOCASE)", [&name], |r| r.get(0))?;
    if taken {
        return Err(validation("Ya existe una plantilla con ese nombre."));
    }
    let source = fields::list_for_project(conn, project_id)?;
    let id = new_id();
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "INSERT INTO template (id, name, test_type, builtin, created_at) VALUES (?1, ?2, ?3, 0, ?4)",
        params![id, name, test_type, now()],
    )?;
    for (position, f) in source.iter().filter(|f| !f.archived).enumerate() {
        fields::insert_template_field(&tx, &id, f, position as i64)?;
    }
    tx.commit()?;
    get(conn, &id)
}

pub fn delete(conn: &Connection, id: &str) -> AppResult<()> {
    if get(conn, id)?.builtin {
        return Err(validation("Las plantillas incluidas no se pueden eliminar."));
    }
    conn.execute("DELETE FROM template WHERE id = ?1", [id])?;
    Ok(())
}
```

- [ ] **Step 6: Ejecutar los tests**

Run: `cargo test repo::`
Expected: 8 tests PASS.

- [ ] **Step 7: Commit**

```bash
cd ..
git add -A
git commit -m "feat(f1): repositorios Rust de plantillas y proyectos"
```

---

### Task 6: Columnas del proyecto — agregar, editar, reordenar y archivar

**Files:**
- Modify: `src-tauri/src/repo/fields.rs` (agregar `save` y sus tests)

**Interfaces:**
- Consumes: `repo::projects::{ensure_exists, touch}`, `domain::text::{required_text, slugify}`, `dto::FieldInput`.
- Produces: `repo::fields::save(conn, project_id, inputs: Vec<FieldInput>) -> AppResult<Vec<FieldDto>>`.

**Reglas (spec §5):**
1. El orden de `inputs` define `position`.
2. Una entrada sin `id` crea una columna, con clave `slugify(label)` única dentro del proyecto (`_2`, `_3`…).
3. Una columna existente que **no** viene en `inputs` se elimina si no tiene datos, o se **archiva** si los tiene.
4. Solo se puede cambiar el tipo de una columna con datos hacia `text` o `longtext`.
5. En una lista con datos no se puede quitar una opción en uso.
6. Los nombres no se repiten entre columnas activas (sin distinguir mayúsculas); el ancho va de 8 a 100; hay máximo 60 columnas; una lista tiene 1–50 opciones, recortadas y sin duplicados.

- [ ] **Step 1: Escribir los tests (fallan)**

Al final de `src-tauri/src/repo/fields.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        dto::FieldInput,
        repo::test_support::{conn, field_id, insert_value, project},
    };

    fn to_input(f: &FieldDto) -> FieldInput {
        FieldInput {
            id: Some(f.id.clone()),
            label: f.label.clone(),
            field_type: f.field_type,
            options: f.options.clone(),
            required: f.required,
            clear_on_duplicate: f.clear_on_duplicate,
            width: f.width,
            archived: f.archived,
        }
    }

    fn new_input(label: &str, field_type: FieldType, options: &[&str]) -> FieldInput {
        FieldInput {
            id: None,
            label: label.into(),
            field_type,
            options: options.iter().map(|o| o.to_string()).collect(),
            required: false,
            clear_on_duplicate: false,
            width: 14,
            archived: false,
        }
    }

    #[test]
    fn adds_and_reorders() {
        let conn = conn();
        let p = project(&conn);
        let mut inputs: Vec<FieldInput> = p.fields.iter().rev().map(to_input).collect();
        inputs.push(new_input("Severidad", FieldType::List, &[" Alta ", "Media", "Media", ""]));
        let saved = save(&conn, &p.id, inputs).unwrap();
        assert_eq!(saved[0].key, "observaciones");
        let sev = saved.iter().find(|f| f.label == "Severidad").unwrap();
        assert_eq!(sev.key, "severidad");
        assert_eq!(sev.options, vec!["Alta", "Media"]);
        assert_eq!(sev.position, 11);
    }

    #[test]
    fn new_keys_are_unique() {
        let conn = conn();
        let p = project(&conn);
        let mut inputs: Vec<FieldInput> = p.fields.iter().map(to_input).collect();
        inputs.push(new_input("Entorno!", FieldType::Text, &[]));
        let saved = save(&conn, &p.id, inputs).unwrap();
        assert!(saved.iter().any(|f| f.key == "entorno_2"));
    }

    #[test]
    fn rejects_invalid_inputs() {
        let conn = conn();
        let p = project(&conn);
        let base: Vec<FieldInput> = p.fields.iter().map(to_input).collect();

        let mut dup = base.clone();
        dup.push(new_input("ENTORNO", FieldType::Text, &[]));
        assert_eq!(save(&conn, &p.id, dup).unwrap_err().to_string(), "La columna «ENTORNO» está repetida.");

        let mut empty_list = base.clone();
        empty_list.push(new_input("Severidad", FieldType::List, &["  "]));
        assert_eq!(
            save(&conn, &p.id, empty_list).unwrap_err().to_string(),
            "La lista «Severidad» necesita al menos una opción."
        );

        let mut narrow = base.clone();
        narrow[0].width = 4;
        assert_eq!(save(&conn, &p.id, narrow).unwrap_err().to_string(), "El ancho de «Entorno» debe estar entre 8 y 100.");

        let mut unknown = base;
        unknown[0].id = Some("no-existe".into());
        assert_eq!(save(&conn, &p.id, unknown).unwrap_err().to_string(), "La columna ya no existe. Recarga el proyecto.");
    }

    #[test]
    fn removed_fields_are_deleted_or_archived() {
        let conn = conn();
        let p = project(&conn);
        insert_value(&conn, &p.id, &field_id(&p, "modulo"), "Cobranzas");
        let inputs: Vec<FieldInput> =
            p.fields.iter().filter(|f| f.key != "menu" && f.key != "modulo").map(to_input).collect();
        let saved = save(&conn, &p.id, inputs).unwrap();
        assert!(saved.iter().all(|f| f.key != "menu"));
        let modulo = saved.iter().find(|f| f.key == "modulo").unwrap();
        assert!(modulo.archived);
        assert_eq!(saved.last().unwrap().key, "modulo");

        let mut restore: Vec<FieldInput> = saved.iter().map(to_input).collect();
        restore.last_mut().unwrap().archived = false;
        let restored = save(&conn, &p.id, restore).unwrap();
        assert!(!restored.iter().find(|f| f.key == "modulo").unwrap().archived);
    }

    #[test]
    fn type_change_with_data_only_to_text() {
        let conn = conn();
        let p = project(&conn);
        insert_value(&conn, &p.id, &field_id(&p, "fecha_ejecucion"), "2026-10-15");
        let idx = p.fields.iter().position(|f| f.key == "fecha_ejecucion").unwrap();

        let mut to_number: Vec<FieldInput> = p.fields.iter().map(to_input).collect();
        to_number[idx].field_type = FieldType::Number;
        assert_eq!(
            save(&conn, &p.id, to_number).unwrap_err().to_string(),
            "No se puede cambiar el tipo de «Fecha de ejecución» porque ya tiene datos. Solo se permite cambiar a Texto o Texto largo."
        );

        let mut to_text: Vec<FieldInput> = p.fields.iter().map(to_input).collect();
        to_text[idx].field_type = FieldType::Longtext;
        assert!(save(&conn, &p.id, to_text).is_ok());
    }

    #[test]
    fn list_options_in_use_cannot_be_removed() {
        let conn = conn();
        let p = project(&conn);
        let mut inputs: Vec<FieldInput> = p.fields.iter().map(to_input).collect();
        inputs.push(new_input("Severidad", FieldType::List, &["Alta", "Media"]));
        let saved = save(&conn, &p.id, inputs).unwrap();
        let sev = saved.iter().find(|f| f.key == "severidad").unwrap().clone();
        insert_value(&conn, &p.id, &sev.id, "Media");

        let mut shrink: Vec<FieldInput> = saved.iter().map(to_input).collect();
        shrink.iter_mut().find(|f| f.id.as_deref() == Some(sev.id.as_str())).unwrap().options = vec!["Alta".into()];
        assert_eq!(
            save(&conn, &p.id, shrink).unwrap_err().to_string(),
            "La opción «Media» de «Severidad» está en uso; no se puede quitar."
        );
    }
}
```

Run: `cd src-tauri && cargo test repo::fields`
Expected: FAIL de compilación (`save` no existe).

- [ ] **Step 2: Implementar `save`**

En `src-tauri/src/repo/fields.rs` cambia los imports del inicio del archivo por:

```rust
use std::collections::HashSet;

use rusqlite::{params, types::Type, Connection, Row};

use super::projects;
use crate::{
    db::new_id,
    domain::{
        fields::FieldType,
        text::{required_text, slugify},
    },
    dto::{FieldDto, FieldInput},
    error::{validation, AppResult},
};
```

y agrega antes del módulo de tests:

```rust
const MAX_FIELDS: usize = 60;

fn has_values(conn: &Connection, field_id: &str) -> AppResult<bool> {
    Ok(conn.query_row("SELECT EXISTS(SELECT 1 FROM field_value WHERE field_id = ?1)", [field_id], |r| r.get(0))?)
}

fn values_in_use(conn: &Connection, field_id: &str) -> AppResult<Vec<String>> {
    let mut stmt = conn.prepare("SELECT DISTINCT value FROM field_value WHERE field_id = ?1 ORDER BY value")?;
    let values = stmt.query_map([field_id], |r| r.get(0))?.collect::<Result<Vec<String>, _>>()?;
    Ok(values)
}

fn normalize_options(label: &str, options: &[String]) -> AppResult<Vec<String>> {
    let mut out: Vec<String> = Vec::new();
    for option in options {
        let o = option.trim();
        if o.is_empty() || out.iter().any(|x| x == o) {
            continue;
        }
        if o.chars().count() > 100 {
            return Err(validation(format!("Una opción de «{label}» supera 100 caracteres.")));
        }
        out.push(o.to_string());
    }
    if out.is_empty() {
        return Err(validation(format!("La lista «{label}» necesita al menos una opción.")));
    }
    if out.len() > 50 {
        return Err(validation(format!("La lista «{label}» admite como máximo 50 opciones.")));
    }
    Ok(out)
}

fn unique_key(base: String, used: &mut HashSet<String>) -> String {
    let mut key = base.clone();
    let mut n = 2;
    while used.contains(&key) {
        key = format!("{base}_{n}");
        n += 1;
    }
    used.insert(key.clone());
    key
}

struct Normalized<'a> {
    input: &'a FieldInput,
    label: String,
    options: Vec<String>,
}

pub fn save(conn: &Connection, project_id: &str, inputs: Vec<FieldInput>) -> AppResult<Vec<FieldDto>> {
    projects::ensure_exists(conn, project_id)?;
    if inputs.len() > MAX_FIELDS {
        return Err(validation(format!("Un proyecto admite como máximo {MAX_FIELDS} columnas.")));
    }
    let existing = list_for_project(conn, project_id)?;

    let mut labels = HashSet::new();
    let mut ids = HashSet::new();
    let mut normalized = Vec::with_capacity(inputs.len());
    for input in &inputs {
        let label = required_text("Nombre de la columna", &input.label, 100)?;
        if !input.archived && !labels.insert(label.to_lowercase()) {
            return Err(validation(format!("La columna «{label}» está repetida.")));
        }
        if !(8..=100).contains(&input.width) {
            return Err(validation(format!("El ancho de «{label}» debe estar entre 8 y 100.")));
        }
        if let Some(id) = &input.id {
            if !ids.insert(id.clone()) {
                return Err(validation(format!("La columna «{label}» está repetida.")));
            }
        }
        let options =
            if input.field_type == FieldType::List { normalize_options(&label, &input.options)? } else { Vec::new() };
        normalized.push(Normalized { input, label, options });
    }

    let tx = conn.unchecked_transaction()?;
    let mut used_keys: HashSet<String> = existing.iter().map(|f| f.key.clone()).collect();
    for (position, n) in normalized.iter().enumerate() {
        let input = n.input;
        let label = &n.label;
        let options_json = serde_json::to_string(&n.options)?;
        match &input.id {
            Some(id) => {
                let current = existing
                    .iter()
                    .find(|f| &f.id == id)
                    .ok_or_else(|| validation("La columna ya no existe. Recarga el proyecto."))?;
                let with_data = has_values(&tx, id)?;
                if with_data && current.field_type != input.field_type && !input.field_type.is_textual() {
                    return Err(validation(format!(
                        "No se puede cambiar el tipo de «{label}» porque ya tiene datos. Solo se permite cambiar a Texto o Texto largo."
                    )));
                }
                if with_data && input.field_type == FieldType::List {
                    if let Some(missing) = values_in_use(&tx, id)?.into_iter().find(|v| !n.options.contains(v)) {
                        return Err(validation(format!(
                            "La opción «{missing}» de «{label}» está en uso; no se puede quitar."
                        )));
                    }
                }
                tx.execute(
                    "UPDATE project_field SET label = ?1, type = ?2, options = ?3, required = ?4,
                            clear_on_duplicate = ?5, width = ?6, position = ?7, archived = ?8
                     WHERE id = ?9",
                    params![
                        label,
                        input.field_type.as_str(),
                        options_json,
                        input.required,
                        input.clear_on_duplicate,
                        input.width,
                        position as i64,
                        input.archived,
                        id
                    ],
                )?;
            }
            None => {
                let key = unique_key(slugify(label), &mut used_keys);
                tx.execute(
                    "INSERT INTO project_field (id, project_id, key, label, type, options, required,
                                                clear_on_duplicate, width, position, archived)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                    params![
                        new_id(),
                        project_id,
                        key,
                        label,
                        input.field_type.as_str(),
                        options_json,
                        input.required,
                        input.clear_on_duplicate,
                        input.width,
                        position as i64,
                        input.archived
                    ],
                )?;
            }
        }
    }

    let kept: HashSet<&str> = inputs.iter().filter_map(|i| i.id.as_deref()).collect();
    for removed in existing.iter().filter(|f| !kept.contains(f.id.as_str())) {
        if has_values(&tx, &removed.id)? {
            tx.execute(
                "UPDATE project_field SET archived = 1, position = ?1 WHERE id = ?2",
                params![(normalized.len() + removed.position as usize) as i64, removed.id],
            )?;
        } else {
            tx.execute("DELETE FROM project_field WHERE id = ?1", [&removed.id])?;
        }
    }
    projects::touch(&tx, project_id)?;
    tx.commit()?;
    list_for_project(conn, project_id)
}
```

- [ ] **Step 3: Ejecutar los tests**

Run: `cargo test repo::`
Expected: 14 tests PASS (8 previos + 6 nuevos).

- [ ] **Step 4: Commit**

```bash
cd ..
git add -A
git commit -m "feat(f1): edición de columnas con reglas de archivado y cambio de tipo"
```

---

### Task 7: Repositorio de casos — EJ-###, validación y duplicado

**Files:**
- Create: `src-tauri/src/repo/cases.rs`
- Modify: `src-tauri/src/repo/mod.rs` (agregar `pub mod cases;`), `src-tauri/src/dto.rs` (agregar `CaseDto` y `CaseInput`)

**Interfaces:**
- Consumes: `repo::{fields::list_for_project, projects::{ensure_exists, touch}}`, `domain::{fields::{FieldRule, normalize_value, validate_value}, status::{is_status, DEFAULT_STATUS}, text::required_text}`.
- Produces:
  - `CaseDto { id, project_id, code, title, status, values: BTreeMap<field_id, value>, created_at, updated_at }` (camelCase)
  - `CaseInput { title, status, values: BTreeMap<String, String> }` (`deny_unknown_fields`)
  - `repo::cases::{list(conn, project_id), get(conn, id), create(conn, project_id, &CaseInput), update(conn, case_id, &CaseInput), duplicate(conn, case_id), delete(conn, case_id)}`

- [ ] **Step 1: DTO de casos**

Agrega al final de `src-tauri/src/dto.rs` (y `use std::collections::BTreeMap;` al inicio):

```rust
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseDto {
    pub id: String,
    pub project_id: String,
    pub code: String,
    pub title: String,
    pub status: String,
    pub values: BTreeMap<String, String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaseInput {
    pub title: String,
    pub status: String,
    pub values: BTreeMap<String, String>,
}
```

- [ ] **Step 2: Escribir los tests (fallan)**

En `src-tauri/src/repo/mod.rs` agrega `pub mod cases;`.

`src-tauri/src/repo/cases.rs`, por ahora solo los tests:

```rust
#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::{
        dto::{CaseInput, FieldInput},
        repo::{
            fields as field_repo, projects as project_repo,
            test_support::{conn, field_id, project},
        },
    };

    fn input(p: &crate::dto::ProjectDto, pairs: &[(&str, &str)]) -> CaseInput {
        let values: BTreeMap<String, String> =
            pairs.iter().map(|(k, v)| (field_id(p, k), v.to_string())).collect();
        CaseInput { title: "Aplicación de recibos".into(), status: "Aprobado".into(), values }
    }

    #[test]
    fn create_assigns_sequential_codes() {
        let conn = conn();
        let p = project(&conn);
        let a = create(&conn, &p.id, &input(&p, &[("entorno", "QA")])).unwrap();
        let b = create(&conn, &p.id, &input(&p, &[("entorno", "QA")])).unwrap();
        assert_eq!((a.code.as_str(), b.code.as_str()), ("EJ-001", "EJ-002"));
        assert_eq!(project_repo::get(&conn, &p.id).unwrap().next_sequence, 3);
        assert_eq!(project_repo::list(&conn).unwrap()[0].case_count, 2);
    }

    #[test]
    fn create_validates() {
        let conn = conn();
        let p = project(&conn);
        assert_eq!(create(&conn, &p.id, &input(&p, &[])).unwrap_err().to_string(), "«Entorno» es obligatorio.");

        let mut bad_status = input(&p, &[("entorno", "QA")]);
        bad_status.status = "Listo".into();
        assert!(create(&conn, &p.id, &bad_status).is_err());

        let mut unknown = input(&p, &[("entorno", "QA")]);
        unknown.values.insert("otro".into(), "x".into());
        assert_eq!(
            create(&conn, &p.id, &unknown).unwrap_err().to_string(),
            "La columna enviada no pertenece al proyecto. Recarga el proyecto."
        );

        let bad_date = input(&p, &[("entorno", "QA"), ("fecha_ejecucion", "2026-13-01")]);
        assert!(create(&conn, &p.id, &bad_date).is_err());

        let mut blank_title = input(&p, &[("entorno", "QA")]);
        blank_title.title = "   ".into();
        assert_eq!(create(&conn, &p.id, &blank_title).unwrap_err().to_string(), "«Caso de prueba» es obligatorio.");
    }

    #[test]
    fn whitespace_values_are_not_stored_and_multiline_is_kept() {
        let conn = conn();
        let p = project(&conn);
        let c = create(
            &conn,
            &p.id,
            &input(&p, &[("entorno", " QA "), ("modulo", "   "), ("datos_entrada", "Recibo: 1\nSucursal: 2 ✅")]),
        )
        .unwrap();
        assert_eq!(c.values.get(&field_id(&p, "entorno")).unwrap(), " QA ");
        assert!(!c.values.contains_key(&field_id(&p, "modulo")));
        assert_eq!(c.values.get(&field_id(&p, "datos_entrada")).unwrap(), "Recibo: 1\nSucursal: 2 ✅");
    }

    #[test]
    fn update_keeps_archived_values() {
        let conn = conn();
        let p = project(&conn);
        let c = create(&conn, &p.id, &input(&p, &[("entorno", "QA"), ("modulo", "Cobranzas")])).unwrap();
        let without_modulo: Vec<FieldInput> = p
            .fields
            .iter()
            .filter(|f| f.key != "modulo")
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
        field_repo::save(&conn, &p.id, without_modulo).unwrap();

        let updated = update(&conn, &c.id, &input(&p, &[("entorno", "Producción")])).unwrap();
        assert_eq!(updated.values.get(&field_id(&p, "entorno")).unwrap(), "Producción");
        assert_eq!(updated.values.get(&field_id(&p, "modulo")).unwrap(), "Cobranzas");

        let stale = input(&p, &[("entorno", "QA"), ("modulo", "Otro")]);
        assert!(update(&conn, &c.id, &stale).is_err());
    }

    #[test]
    fn update_requires_new_required_fields() {
        let conn = conn();
        let p = project(&conn);
        let c = create(&conn, &p.id, &input(&p, &[("entorno", "QA")])).unwrap();
        conn.execute("UPDATE project_field SET required = 1 WHERE id = ?1", [field_id(&p, "responsable")]).unwrap();
        assert_eq!(
            update(&conn, &c.id, &input(&p, &[("entorno", "QA")])).unwrap_err().to_string(),
            "«Responsable» es obligatorio."
        );
    }

    #[test]
    fn duplicate_clears_marked_fields_and_resets_status() {
        let conn = conn();
        let p = project(&conn);
        let src = create(
            &conn,
            &p.id,
            &input(
                &p,
                &[
                    ("entorno", "QA"),
                    ("datos_entrada", "Póliza 1"),
                    ("resultado_obtenido", "OK"),
                    ("fecha_ejecucion", "2026-10-15"),
                    ("observaciones", "nada"),
                ],
            ),
        )
        .unwrap();
        let copy = duplicate(&conn, &src.id).unwrap();
        assert_eq!(copy.code, "EJ-002");
        assert_eq!(copy.status, "No ejecutado");
        assert_eq!(copy.title, src.title);
        assert_eq!(copy.values.get(&field_id(&p, "datos_entrada")).unwrap(), "Póliza 1");
        for key in ["resultado_obtenido", "fecha_ejecucion", "observaciones"] {
            assert!(!copy.values.contains_key(&field_id(&p, key)), "{key} debería limpiarse");
        }
    }

    #[test]
    fn deleted_codes_are_not_reused_and_project_delete_cascades() {
        let conn = conn();
        let p = project(&conn);
        create(&conn, &p.id, &input(&p, &[("entorno", "QA")])).unwrap();
        let b = create(&conn, &p.id, &input(&p, &[("entorno", "QA")])).unwrap();
        delete(&conn, &b.id).unwrap();
        let c = create(&conn, &p.id, &input(&p, &[("entorno", "QA")])).unwrap();
        assert_eq!(c.code, "EJ-003");
        assert_eq!(list(&conn, &p.id).unwrap().len(), 2);
        assert_eq!(delete(&conn, &b.id).unwrap_err().code(), "not_found");

        project_repo::delete(&conn, &p.id).unwrap();
        let values: i64 = conn.query_row("SELECT COUNT(*) FROM field_value", [], |r| r.get(0)).unwrap();
        assert_eq!(values, 0);
    }
}
```

Run: `cd src-tauri && cargo test repo::cases`
Expected: FAIL de compilación.

- [ ] **Step 3: Implementar** (encima de los tests en `cases.rs`)

```rust
use std::collections::{BTreeMap, HashMap};

use rusqlite::{params, Connection, OptionalExtension, Row};

use super::{fields, projects};
use crate::{
    db::{new_id, now},
    domain::{
        fields::{normalize_value, validate_value, FieldRule},
        status::{is_status, DEFAULT_STATUS},
        text::required_text,
    },
    dto::{CaseDto, CaseInput, FieldDto},
    error::{validation, AppError, AppResult},
};

const CASE_COLUMNS: &str = "id, project_id, code, title, status, created_at, updated_at";

fn case_from_row(r: &Row) -> rusqlite::Result<CaseDto> {
    Ok(CaseDto {
        id: r.get("id")?,
        project_id: r.get("project_id")?,
        code: r.get("code")?,
        title: r.get("title")?,
        status: r.get("status")?,
        values: BTreeMap::new(),
        created_at: r.get("created_at")?,
        updated_at: r.get("updated_at")?,
    })
}

pub fn list(conn: &Connection, project_id: &str) -> AppResult<Vec<CaseDto>> {
    projects::ensure_exists(conn, project_id)?;
    let mut stmt = conn.prepare(&format!("SELECT {CASE_COLUMNS} FROM test_case WHERE project_id = ?1 ORDER BY rowid"))?;
    let mut cases = stmt.query_map([project_id], case_from_row)?.collect::<Result<Vec<_>, _>>()?;
    let mut stmt = conn.prepare(
        "SELECT v.case_id, v.field_id, v.value FROM field_value v
         JOIN test_case c ON c.id = v.case_id WHERE c.project_id = ?1",
    )?;
    let mut by_case: HashMap<String, BTreeMap<String, String>> = HashMap::new();
    let rows = stmt.query_map([project_id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?)))?;
    for row in rows {
        let (case_id, field_id, value) = row?;
        by_case.entry(case_id).or_default().insert(field_id, value);
    }
    for case in &mut cases {
        if let Some(values) = by_case.remove(&case.id) {
            case.values = values;
        }
    }
    Ok(cases)
}

pub fn get(conn: &Connection, id: &str) -> AppResult<CaseDto> {
    let mut case = conn
        .query_row(&format!("SELECT {CASE_COLUMNS} FROM test_case WHERE id = ?1"), [id], case_from_row)
        .optional()?
        .ok_or_else(|| AppError::NotFound("El caso no existe.".into()))?;
    let mut stmt = conn.prepare("SELECT field_id, value FROM field_value WHERE case_id = ?1")?;
    case.values = stmt
        .query_map([id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    Ok(case)
}

struct Validated {
    title: String,
    status: String,
    values: Vec<(String, String)>,
}

fn validate_input(project_fields: &[FieldDto], input: &CaseInput) -> AppResult<Validated> {
    let title = required_text("Caso de prueba", &input.title, 300)?;
    if !is_status(&input.status) {
        return Err(validation(format!("Estado inválido: «{}».", input.status)));
    }
    for key in input.values.keys() {
        if !project_fields.iter().any(|f| &f.id == key && !f.archived) {
            return Err(validation("La columna enviada no pertenece al proyecto. Recarga el proyecto."));
        }
    }
    let mut values = Vec::new();
    for f in project_fields.iter().filter(|f| !f.archived) {
        let raw = input.values.get(&f.id).map(String::as_str).unwrap_or("");
        let rule = FieldRule { label: &f.label, field_type: f.field_type, options: &f.options, required: f.required };
        validate_value(&rule, raw).map_err(AppError::Validation)?;
        let value = normalize_value(f.field_type, raw);
        if !value.is_empty() {
            values.push((f.id.clone(), value));
        }
    }
    Ok(Validated { title, status: input.status.clone(), values })
}

fn insert_case(conn: &Connection, project_id: &str, title: &str, status: &str) -> AppResult<String> {
    let seq: i64 = conn.query_row("SELECT next_sequence FROM project WHERE id = ?1", [project_id], |r| r.get(0))?;
    conn.execute("UPDATE project SET next_sequence = next_sequence + 1 WHERE id = ?1", [project_id])?;
    let id = new_id();
    conn.execute(
        "INSERT INTO test_case (id, project_id, code, title, status, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
        params![id, project_id, format!("EJ-{seq:03}"), title, status, now()],
    )?;
    Ok(id)
}

fn insert_values(conn: &Connection, case_id: &str, values: &[(String, String)]) -> AppResult<()> {
    let mut stmt = conn.prepare("INSERT INTO field_value (case_id, field_id, value) VALUES (?1, ?2, ?3)")?;
    for (field_id, value) in values {
        stmt.execute(params![case_id, field_id, value])?;
    }
    Ok(())
}

pub fn create(conn: &Connection, project_id: &str, input: &CaseInput) -> AppResult<CaseDto> {
    projects::ensure_exists(conn, project_id)?;
    let v = validate_input(&fields::list_for_project(conn, project_id)?, input)?;
    let tx = conn.unchecked_transaction()?;
    let id = insert_case(&tx, project_id, &v.title, &v.status)?;
    insert_values(&tx, &id, &v.values)?;
    projects::touch(&tx, project_id)?;
    tx.commit()?;
    get(conn, &id)
}

pub fn update(conn: &Connection, case_id: &str, input: &CaseInput) -> AppResult<CaseDto> {
    let current = get(conn, case_id)?;
    let v = validate_input(&fields::list_for_project(conn, &current.project_id)?, input)?;
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "UPDATE test_case SET title = ?1, status = ?2, updated_at = ?3 WHERE id = ?4",
        params![v.title, v.status, now(), case_id],
    )?;
    tx.execute(
        "DELETE FROM field_value WHERE case_id = ?1
           AND field_id IN (SELECT id FROM project_field WHERE project_id = ?2 AND archived = 0)",
        params![case_id, current.project_id],
    )?;
    insert_values(&tx, case_id, &v.values)?;
    projects::touch(&tx, &current.project_id)?;
    tx.commit()?;
    get(conn, case_id)
}

pub fn duplicate(conn: &Connection, case_id: &str) -> AppResult<CaseDto> {
    let source = get(conn, case_id)?;
    let project_fields = fields::list_for_project(conn, &source.project_id)?;
    let kept: Vec<(String, String)> = source
        .values
        .iter()
        .filter(|(field_id, _)| project_fields.iter().any(|f| &f.id == *field_id && !f.clear_on_duplicate))
        .map(|(f, v)| (f.clone(), v.clone()))
        .collect();
    let tx = conn.unchecked_transaction()?;
    let id = insert_case(&tx, &source.project_id, &source.title, DEFAULT_STATUS)?;
    insert_values(&tx, &id, &kept)?;
    projects::touch(&tx, &source.project_id)?;
    tx.commit()?;
    get(conn, &id)
}

pub fn delete(conn: &Connection, case_id: &str) -> AppResult<()> {
    let case = get(conn, case_id)?;
    let tx = conn.unchecked_transaction()?;
    tx.execute("DELETE FROM test_case WHERE id = ?1", [case_id])?;
    projects::touch(&tx, &case.project_id)?;
    tx.commit()?;
    Ok(())
}
```

- [ ] **Step 4: Ejecutar los tests**

Run: `cargo test repo::`
Expected: 21 tests PASS.

- [ ] **Step 5: Commit**

```bash
cd ..
git add -A
git commit -m "feat(f1): repositorio de casos con códigos EJ-### y duplicado"
```

---

### Task 8: Contrato DTO, comandos Tauri y wiring

**Files:**
- Modify: `src-tauri/src/dto.rs` (módulo de test `contract`), `src-tauri/src/lib.rs` (reemplazar completo)
- Create: `src-tauri/src/state.rs`, `src-tauri/src/commands/mod.rs`, `src-tauri/src/commands/templates.rs`, `src-tauri/src/commands/projects.rs`, `src-tauri/src/commands/cases.rs`, `src/infrastructure/tauri/__fixtures__/contracts.json` (generado)

**Interfaces:**
- Consumes: todos los repositorios de Tasks 5–7.
- Produces: comandos `list_templates`, `save_template_from_project(projectId, name, testType)`, `delete_template(id)`, `list_projects`, `get_project(id)`, `create_project(input)`, `update_project(id, input)`, `delete_project(id)`, `save_project_fields(projectId, fields)`, `list_cases(projectId)`, `create_case(projectId, input)`, `update_case(caseId, input)`, `duplicate_case(caseId)` y `delete_case(caseId)`. Los argumentos van en camelCase desde JS. El fixture `contracts.json` tiene las claves `field`, `template`, `projectSummary`, `project`, `case`, `error` e `inputs.{newProjectInput, projectMetaInput, fieldInput, caseInput}`.

- [ ] **Step 1: Escribir el test de contrato (falla: no existe el fixture)**

Al final de `src-tauri/src/dto.rs`:

```rust
#[cfg(test)]
mod contract {
    use std::{collections::BTreeMap, path::PathBuf};

    use serde_json::{json, Value};

    use super::*;
    use crate::error::AppError;

    fn field() -> FieldDto {
        FieldDto {
            id: "fld-1".into(),
            key: "entorno".into(),
            label: "Entorno".into(),
            field_type: FieldType::List,
            options: vec!["QA".into(), "Producción".into()],
            required: true,
            clear_on_duplicate: false,
            width: 21,
            position: 0,
            archived: false,
        }
    }

    fn meta_json() -> Value {
        json!({ "name": "Recibos", "client": "ACME", "testType": "Funcional", "releaseVersion": "1.0",
                "startDate": "2026-10-01", "endDate": "2026-10-31" })
    }

    fn inputs_json() -> Value {
        json!({
            "newProjectInput": { "meta": meta_json(), "templateId": "tpl-funcional" },
            "projectMetaInput": meta_json(),
            "fieldInput": { "id": null, "label": "Severidad", "type": "list", "options": ["Alta", "Media", "Baja"],
                            "required": false, "clearOnDuplicate": false, "width": 14, "archived": false },
            "caseInput": { "title": "Aplicación de recibos", "status": "Aprobado", "values": { "fld-1": "QA" } }
        })
    }

    fn sample() -> Value {
        let ts = "2026-10-15T12:00:00.000Z".to_string();
        json!({
            "field": field(),
            "template": TemplateDto { id: "tpl-funcional".into(), name: "Funcional".into(), test_type: "Funcional".into(),
                                      builtin: true, fields: vec![field()] },
            "projectSummary": ProjectSummaryDto { id: "prj-1".into(), name: "Recibos".into(), client: "ACME".into(),
                                                  case_count: 3, updated_at: ts.clone() },
            "project": ProjectDto { id: "prj-1".into(), name: "Recibos".into(), client: "ACME".into(),
                                    test_type: "Funcional".into(), release_version: "1.0".into(),
                                    start_date: "2026-10-01".into(), end_date: "2026-10-31".into(), next_sequence: 4,
                                    source_template_id: Some("tpl-funcional".into()), created_at: ts.clone(),
                                    updated_at: ts.clone(), fields: vec![field()] },
            "case": CaseDto { id: "case-1".into(), project_id: "prj-1".into(), code: "EJ-001".into(),
                              title: "Aplicación de recibos".into(), status: "Aprobado".into(),
                              values: BTreeMap::from([("fld-1".to_string(), "QA".to_string())]),
                              created_at: ts.clone(), updated_at: ts },
            "error": AppError::Validation("«Entorno» es obligatorio.".into()),
            "inputs": inputs_json(),
        })
    }

    #[test]
    fn inputs_deserialize_from_ts_shape() {
        let i = inputs_json();
        serde_json::from_value::<NewProjectInput>(i["newProjectInput"].clone()).unwrap();
        serde_json::from_value::<ProjectMetaInput>(i["projectMetaInput"].clone()).unwrap();
        serde_json::from_value::<FieldInput>(i["fieldInput"].clone()).unwrap();
        serde_json::from_value::<CaseInput>(i["caseInput"].clone()).unwrap();
    }

    #[test]
    fn inputs_reject_unknown_fields() {
        let mut case = inputs_json()["caseInput"].clone();
        case["extra"] = json!(1);
        assert!(serde_json::from_value::<CaseInput>(case).is_err());
    }

    /// Regenerar: `UPDATE_CONTRACTS=1 cargo test contract`
    #[test]
    fn dto_contract_matches_fixture() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../src/infrastructure/tauri/__fixtures__/contracts.json");
        let actual = serde_json::to_string_pretty(&sample()).unwrap() + "\n";
        if std::env::var("UPDATE_CONTRACTS").is_ok() {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, &actual).unwrap();
        }
        let expected = std::fs::read_to_string(&path)
            .expect("falta el fixture; ejecuta UPDATE_CONTRACTS=1 cargo test contract")
            .replace("\r\n", "\n");
        assert_eq!(actual, expected, "El contrato cambió: regenera el fixture y actualiza src/infrastructure/tauri/dto.ts");
    }
}
```

Run: `cd src-tauri && cargo test contract`
Expected: `dto_contract_matches_fixture` FAIL ("falta el fixture"); los otros dos PASS.

- [ ] **Step 2: Generar el fixture**

Run: `UPDATE_CONTRACTS=1 cargo test contract && cargo test contract`
Expected: 3 PASS en ambas corridas; existe `../src/infrastructure/tauri/__fixtures__/contracts.json` con `"type": "list"` y `"clearOnDuplicate": false` en camelCase.

- [ ] **Step 3: Estado y comandos**

`src-tauri/src/state.rs`:

```rust
use std::sync::{Mutex, MutexGuard};

use rusqlite::Connection;

pub struct AppState {
    db: Mutex<Connection>,
}

impl AppState {
    pub fn new(conn: Connection) -> Self {
        Self { db: Mutex::new(conn) }
    }

    /// Una sola conexión serializada: SQLite + WAL es suficiente para una app de escritorio de un usuario.
    pub fn db(&self) -> MutexGuard<'_, Connection> {
        self.db.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}
```

`src-tauri/src/commands/mod.rs`:

```rust
pub mod cases;
pub mod projects;
pub mod templates;
```

`src-tauri/src/commands/templates.rs`:

```rust
use tauri::State;

use crate::{dto::TemplateDto, error::AppResult, repo::templates, state::AppState};

#[tauri::command(async)]
pub fn list_templates(state: State<'_, AppState>) -> AppResult<Vec<TemplateDto>> {
    templates::list(&state.db())
}

#[tauri::command(async)]
pub fn save_template_from_project(
    state: State<'_, AppState>,
    project_id: String,
    name: String,
    test_type: String,
) -> AppResult<TemplateDto> {
    templates::save_from_project(&state.db(), &project_id, &name, &test_type)
}

#[tauri::command(async)]
pub fn delete_template(state: State<'_, AppState>, id: String) -> AppResult<()> {
    templates::delete(&state.db(), &id)
}
```

`src-tauri/src/commands/projects.rs`:

```rust
use tauri::State;

use crate::{
    dto::{FieldDto, FieldInput, NewProjectInput, ProjectDto, ProjectMetaInput, ProjectSummaryDto},
    error::AppResult,
    repo::{fields, projects},
    state::AppState,
};

#[tauri::command(async)]
pub fn list_projects(state: State<'_, AppState>) -> AppResult<Vec<ProjectSummaryDto>> {
    projects::list(&state.db())
}

#[tauri::command(async)]
pub fn get_project(state: State<'_, AppState>, id: String) -> AppResult<ProjectDto> {
    projects::get(&state.db(), &id)
}

#[tauri::command(async)]
pub fn create_project(state: State<'_, AppState>, input: NewProjectInput) -> AppResult<ProjectDto> {
    projects::create(&state.db(), input)
}

#[tauri::command(async)]
pub fn update_project(state: State<'_, AppState>, id: String, input: ProjectMetaInput) -> AppResult<ProjectDto> {
    projects::update_meta(&state.db(), &id, &input)
}

#[tauri::command(async)]
pub fn delete_project(state: State<'_, AppState>, id: String) -> AppResult<()> {
    projects::delete(&state.db(), &id)
}

#[tauri::command(async)]
pub fn save_project_fields(
    state: State<'_, AppState>,
    project_id: String,
    fields: Vec<FieldInput>,
) -> AppResult<Vec<FieldDto>> {
    fields::save(&state.db(), &project_id, fields)
}
```

`src-tauri/src/commands/cases.rs`:

```rust
use tauri::State;

use crate::{
    dto::{CaseDto, CaseInput},
    error::AppResult,
    repo::cases,
    state::AppState,
};

#[tauri::command(async)]
pub fn list_cases(state: State<'_, AppState>, project_id: String) -> AppResult<Vec<CaseDto>> {
    cases::list(&state.db(), &project_id)
}

#[tauri::command(async)]
pub fn create_case(state: State<'_, AppState>, project_id: String, input: CaseInput) -> AppResult<CaseDto> {
    cases::create(&state.db(), &project_id, &input)
}

#[tauri::command(async)]
pub fn update_case(state: State<'_, AppState>, case_id: String, input: CaseInput) -> AppResult<CaseDto> {
    cases::update(&state.db(), &case_id, &input)
}

#[tauri::command(async)]
pub fn duplicate_case(state: State<'_, AppState>, case_id: String) -> AppResult<CaseDto> {
    cases::duplicate(&state.db(), &case_id)
}

#[tauri::command(async)]
pub fn delete_case(state: State<'_, AppState>, case_id: String) -> AppResult<()> {
    cases::delete(&state.db(), &case_id)
}
```

- [ ] **Step 4: `lib.rs` (reemplazar completo; elimina el `greet` de la plantilla)**

```rust
mod commands;
mod db;
mod domain;
mod dto;
mod error;
mod repo;
mod state;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let conn = db::open(&data_dir.join("qapture.db"))?;
            app.manage(state::AppState::new(conn));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::templates::list_templates,
            commands::templates::save_template_from_project,
            commands::templates::delete_template,
            commands::projects::list_projects,
            commands::projects::get_project,
            commands::projects::create_project,
            commands::projects::update_project,
            commands::projects::delete_project,
            commands::projects::save_project_fields,
            commands::cases::list_cases,
            commands::cases::create_case,
            commands::cases::update_case,
            commands::cases::duplicate_case,
            commands::cases::delete_case,
        ])
        .run(tauri::generate_context!())
        .expect("no se pudo iniciar Qapture");
}
```

- [ ] **Step 5: Verificar**

Run: `cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check`
Expected: 39 tests PASS (4 db + 11 domain + 21 repo + 3 contract); clippy sin warnings; fmt sin diferencias (si `fmt --check` falla, ejecuta `cargo fmt` y repite).

- [ ] **Step 6: Commit**

```bash
cd ..
git add -A
git commit -m "feat(f1): comandos Tauri y test de contrato DTO"
```

---

### Task 9: Dominio TS — estados, campos, casos, resumen y proyecto

**Files:**
- Create: `src/domain/status.ts`, `src/domain/fields.ts`, `src/domain/cases.ts`, `src/domain/summary.ts`, `src/domain/project.ts`
- Test: `src/domain/fields.test.ts`, `src/domain/cases.test.ts`, `src/domain/summary.test.ts`, `src/domain/project.test.ts`

**Interfaces:**
- Produces:
  - `status.ts`: `STATUSES`, `type Status`, `DEFAULT_STATUS`, `isStatus(v)`
  - `fields.ts`: `FIELD_TYPES`, `type FieldType`, `FIELD_TYPE_LABELS`, `interface FieldDef { id; key; label; type; options; required; clearOnDuplicate; width; position; archived }`, `MAX_LENGTH`, `charLength(s)`, `normalizeValue(type, raw)`, `isIsoDate(v)`, `validateValue(field, raw): string | null`, `activeFields(fields)`, `archivedFields(fields)`
  - `cases.ts`: `interface CaseRecord { id; projectId; code; title; status: Status; values: Record<string,string>; createdAt; updatedAt }`, `interface CaseInput { title; status; values }`, `type FormErrors = Record<string,string>`, `validateCaseInput(fields, input)`, `normalizeCaseInput(fields, input)`, `emptyCaseInput(fields, defaults?)`, `caseToInput(fields, record)`, `filterCases(cases, { search, status })`, `formatCode(n)`
  - `summary.ts`: `countByStatus(cases)`, `successRate(cases): number | null`, `formatRate(rate)`
  - `project.ts`: `interface ProjectMeta`, `Project`, `ProjectSummary`, `Template`, `NewProjectInput { meta; templateId }`, `FieldInput { id: string | null; label; type; options; required; clearOnDuplicate; width; archived }`, `emptyMeta()`, `projectToMeta(p)`, `validateProjectMeta(meta): FormErrors` (claves `name`, `client`, `testType`, `releaseVersion`, `startDate`, `endDate`), `fieldToInput(f)`, `newFieldInput()`, `validateFieldInputs(inputs): FormErrors` (claves = índice de fila, más `_form`), `normalizeFieldInput(input)`

- [ ] **Step 1: Escribir los tests (fallan)**

`src/domain/fields.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { charLength, normalizeValue, validateValue, type FieldDef } from "./fields";

const field = (over: Partial<FieldDef> = {}): FieldDef => ({
  id: "f1",
  key: "campo",
  label: "Campo",
  type: "text",
  options: [],
  required: false,
  clearOnDuplicate: false,
  width: 20,
  position: 0,
  archived: false,
  ...over,
});

describe("validateValue", () => {
  it("exige obligatorios ignorando espacios", () => {
    expect(validateValue(field({ required: true, label: "Entorno" }), "   ")).toBe("«Entorno» es obligatorio.");
    expect(validateValue(field(), "")).toBeNull();
  });

  it("valida listas", () => {
    const f = field({ type: "list", options: ["Alta", "Media"] });
    expect(validateValue(f, "Media")).toBeNull();
    expect(validateValue(f, "Baja")).toMatch(/opciones de la lista/);
  });

  it("valida fechas reales", () => {
    const f = field({ type: "date" });
    expect(validateValue(f, "2026-02-28")).toBeNull();
    expect(validateValue(f, "2026-02-30")).toMatch(/fecha válida/);
    expect(validateValue(f, "2026-2-1")).toMatch(/fecha válida/);
  });

  it("acepta coma decimal y rechaza formatos ambiguos", () => {
    const f = field({ type: "number" });
    expect(validateValue(f, "1,5")).toBeNull();
    expect(normalizeValue("number", " 1,5 ")).toBe("1.5");
    expect(validateValue(f, "-3.25e2")).toBeNull();
    for (const bad of ["1.234,5", "0x10", "Infinity", "NaN", "abc"]) {
      expect(validateValue(f, bad)).toMatch(/número/);
    }
  });

  it("valida sí/no", () => {
    const f = field({ type: "boolean" });
    expect(validateValue(f, "true")).toBeNull();
    expect(validateValue(f, "sí")).toMatch(/Sí o No/);
  });

  it("cuenta caracteres como Rust (un emoji = 1)", () => {
    expect(charLength("✅🚀")).toBe(2);
    expect(validateValue(field(), "🚀".repeat(500))).toBeNull();
    expect(validateValue(field(), "🚀".repeat(501))).toBe("«Campo» supera 500 caracteres.");
  });

  it("conserva saltos de línea en texto largo", () => {
    expect(normalizeValue("longtext", "línea 1\nlínea 2\n")).toBe("línea 1\nlínea 2\n");
    expect(normalizeValue("longtext", " \n ")).toBe("");
  });
});
```

`src/domain/cases.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import {
  caseToInput,
  emptyCaseInput,
  filterCases,
  formatCode,
  normalizeCaseInput,
  validateCaseInput,
  type CaseRecord,
} from "./cases";
import type { FieldDef } from "./fields";

const fields: FieldDef[] = [
  { id: "env", key: "entorno", label: "Entorno", type: "text", options: [], required: true, clearOnDuplicate: false, width: 21, position: 0, archived: false },
  { id: "mod", key: "modulo", label: "Módulo", type: "text", options: [], required: false, clearOnDuplicate: false, width: 22, position: 1, archived: true },
  { id: "obs", key: "observaciones", label: "Observaciones", type: "longtext", options: [], required: false, clearOnDuplicate: true, width: 40, position: 2, archived: false },
];

const record = (over: Partial<CaseRecord>): CaseRecord => ({
  id: "c1",
  projectId: "p1",
  code: "EJ-001",
  title: "Aplicación de recibos",
  status: "Aprobado",
  values: {},
  createdAt: "",
  updatedAt: "",
  ...over,
});

describe("validateCaseInput", () => {
  it("valida título, estado y campos activos", () => {
    const errors = validateCaseInput(fields, { title: " ", status: "Listo" as never, values: { mod: "" } });
    expect(errors).toEqual({
      title: "«Caso de prueba» es obligatorio.",
      status: "Estado inválido.",
      env: "«Entorno» es obligatorio.",
    });
  });

  it("acepta un caso completo", () => {
    expect(validateCaseInput(fields, { title: "Caso", status: "Aprobado", values: { env: "QA" } })).toEqual({});
  });
});

describe("normalizeCaseInput", () => {
  it("recorta el título, descarta vacíos y columnas archivadas", () => {
    expect(
      normalizeCaseInput(fields, { title: "  Caso ", status: "Aprobado", values: { env: "QA", mod: "x", obs: "  " } }),
    ).toEqual({ title: "Caso", status: "Aprobado", values: { env: "QA" } });
  });
});

describe("formularios", () => {
  it("emptyCaseInput usa defaults por clave", () => {
    expect(emptyCaseInput(fields, { entorno: "QA" })).toEqual({
      title: "",
      status: "No ejecutado",
      values: { env: "QA", obs: "" },
    });
  });

  it("caseToInput solo toma columnas activas", () => {
    expect(caseToInput(fields, record({ values: { env: "QA", mod: "Cobranzas" } })).values).toEqual({ env: "QA", obs: "" });
  });
});

describe("filterCases", () => {
  const cases = [
    record({ id: "a", code: "EJ-001", title: "Aplicación de RECIBOS", status: "Aprobado" }),
    record({ id: "b", code: "EJ-002", title: "Emisión", status: "Fallido", values: { env: "Máquina Virtual" } }),
  ];

  it("busca sin distinguir mayúsculas ni acentos, también en valores", () => {
    expect(filterCases(cases, { search: "recíb", status: "" }).map((c) => c.id)).toEqual(["a"]);
    expect(filterCases(cases, { search: "maquina", status: "" }).map((c) => c.id)).toEqual(["b"]);
    expect(filterCases(cases, { search: "ej-002", status: "" }).map((c) => c.id)).toEqual(["b"]);
  });

  it("filtra por estado", () => {
    expect(filterCases(cases, { search: "", status: "Fallido" }).map((c) => c.id)).toEqual(["b"]);
  });
});

it("formatCode rellena a 3 dígitos", () => {
  expect(formatCode(7)).toBe("EJ-007");
  expect(formatCode(1234)).toBe("EJ-1234");
});
```

`src/domain/summary.test.ts`:

```ts
import { expect, it } from "vitest";
import type { Status } from "./status";
import { countByStatus, formatRate, successRate } from "./summary";

const cases = (statuses: Status[]) => statuses.map((status) => ({ status }));

it("cuenta por estado", () => {
  const counts = countByStatus(cases(["Aprobado", "Aprobado", "Fallido"]));
  expect(counts.Aprobado).toBe(2);
  expect(counts.Fallido).toBe(1);
  expect(counts["No ejecutado"]).toBe(0);
});

it("éxito = Aprobado / (Aprobado + Fallido + Bloqueado)", () => {
  const list = cases(["Aprobado", "Aprobado", "Fallido", "Bloqueado", "No ejecutado", "Por documentar"]);
  expect(successRate(list)).toBe(0.5);
  expect(successRate(cases(["No ejecutado"]))).toBeNull();
  expect(formatRate(0.5)).toBe("50 %");
  expect(formatRate(null)).toBe("—");
});
```

`src/domain/project.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { emptyMeta, newFieldInput, normalizeFieldInput, validateFieldInputs, validateProjectMeta } from "./project";

describe("validateProjectMeta", () => {
  it("exige nombre y fechas coherentes", () => {
    expect(validateProjectMeta({ ...emptyMeta(), name: " " })).toEqual({ name: "«Nombre del proyecto» es obligatorio." });
    expect(validateProjectMeta({ ...emptyMeta(), name: "Ok", startDate: "2026-13-01" })).toEqual({
      startDate: "«Fecha de inicio» debe ser una fecha válida (AAAA-MM-DD).",
    });
    expect(validateProjectMeta({ ...emptyMeta(), name: "Ok", startDate: "2026-11-01", endDate: "2026-10-01" })).toEqual({
      endDate: "La fecha de fin no puede ser anterior a la de inicio.",
    });
  });
});

describe("validateFieldInputs", () => {
  it("detecta nombres vacíos, repetidos, listas sin opciones y anchos inválidos", () => {
    const errors = validateFieldInputs([
      { ...newFieldInput(), label: "Entorno" },
      { ...newFieldInput(), label: "entorno " },
      { ...newFieldInput(), label: "" },
      { ...newFieldInput(), label: "Severidad", type: "list", options: [" ", ""] },
      { ...newFieldInput(), label: "Ancho", width: 4 },
      { ...newFieldInput(), label: "Entorno", archived: true },
    ]);
    expect(errors).toEqual({
      "1": "La columna «entorno» está repetida.",
      "2": "La columna 3 necesita un nombre.",
      "3": "La lista «Severidad» necesita al menos una opción.",
      "4": "El ancho de «Ancho» debe estar entre 8 y 100.",
    });
  });

  it("normaliza opciones solo para listas", () => {
    expect(normalizeFieldInput({ ...newFieldInput(), label: " Sev ", type: "list", options: [" Alta", "Alta", "", "Baja"] })).toMatchObject({
      label: "Sev",
      options: ["Alta", "Baja"],
    });
    expect(normalizeFieldInput({ ...newFieldInput(), label: "X", type: "text", options: ["a"] }).options).toEqual([]);
  });
});
```

Run: `pnpm test src/domain`
Expected: FAIL (los módulos no existen).

- [ ] **Step 2: Implementar `status.ts` y `fields.ts`**

`src/domain/status.ts`:

```ts
export const STATUSES = ["No ejecutado", "En ejecución", "Por documentar", "Aprobado", "Fallido", "Bloqueado"] as const;
export type Status = (typeof STATUSES)[number];
export const DEFAULT_STATUS: Status = "No ejecutado";

export function isStatus(value: string): value is Status {
  return (STATUSES as readonly string[]).includes(value);
}
```

`src/domain/fields.ts`:

```ts
export const FIELD_TYPES = ["text", "longtext", "list", "date", "number", "boolean"] as const;
export type FieldType = (typeof FIELD_TYPES)[number];

export const FIELD_TYPE_LABELS: Record<FieldType, string> = {
  text: "Texto",
  longtext: "Texto largo",
  list: "Lista",
  date: "Fecha",
  number: "Número",
  boolean: "Sí / No",
};

export interface FieldDef {
  id: string;
  key: string;
  label: string;
  type: FieldType;
  options: string[];
  required: boolean;
  clearOnDuplicate: boolean;
  width: number;
  position: number;
  archived: boolean;
}

export const MAX_LENGTH: Record<FieldType, number> = {
  text: 500,
  longtext: 16000,
  list: 100,
  date: 10,
  number: 40,
  boolean: 5,
};

const NUMBER_RE = /^[+-]?(\d+(\.\d*)?|\.\d+)([eE][+-]?\d+)?$/;
const COMMA_DECIMAL_RE = /^[+-]?\d+,\d+$/;

/** Igual que `chars().count()` en Rust: cuenta puntos de código, no unidades UTF-16. */
export const charLength = (value: string) => [...value].length;

export function normalizeValue(type: FieldType, raw: string): string {
  if (type === "text" || type === "longtext") return raw.trim() === "" ? "" : raw;
  const v = raw.trim();
  if (type === "number" && COMMA_DECIMAL_RE.test(v)) return v.replace(",", ".");
  return v;
}

export function isIsoDate(value: string): boolean {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(value)) return false;
  const [y, m, d] = value.split("-").map(Number);
  const date = new Date(Date.UTC(y, m - 1, d));
  return date.getUTCFullYear() === y && date.getUTCMonth() === m - 1 && date.getUTCDate() === d;
}

export function validateValue(field: Pick<FieldDef, "label" | "type" | "options" | "required">, raw: string): string | null {
  const v = normalizeValue(field.type, raw);
  const label = field.label;
  if (v.trim() === "") return field.required ? `«${label}» es obligatorio.` : null;
  const max = MAX_LENGTH[field.type];
  if (charLength(v) > max) return `«${label}» supera ${max} caracteres.`;
  switch (field.type) {
    case "text":
    case "longtext":
      return null;
    case "list":
      return field.options.includes(v) ? null : `«${label}» debe ser una de las opciones de la lista.`;
    case "date":
      return isIsoDate(v) ? null : `«${label}» debe ser una fecha válida (AAAA-MM-DD).`;
    case "number":
      return NUMBER_RE.test(v) && Number.isFinite(Number(v)) ? null : `«${label}» debe ser un número.`;
    case "boolean":
      return v === "true" || v === "false" ? null : `«${label}» debe ser Sí o No.`;
  }
}

const byPosition = (a: FieldDef, b: FieldDef) => a.position - b.position;
export const activeFields = (fields: FieldDef[]) => fields.filter((f) => !f.archived).sort(byPosition);
export const archivedFields = (fields: FieldDef[]) => fields.filter((f) => f.archived).sort(byPosition);
```

- [ ] **Step 3: Implementar `cases.ts`, `summary.ts` y `project.ts`**

`src/domain/cases.ts`:

```ts
import { activeFields, charLength, normalizeValue, validateValue, type FieldDef } from "./fields";
import { DEFAULT_STATUS, isStatus, type Status } from "./status";

export interface CaseRecord {
  id: string;
  projectId: string;
  code: string;
  title: string;
  status: Status;
  values: Record<string, string>;
  createdAt: string;
  updatedAt: string;
}

export interface CaseInput {
  title: string;
  status: Status;
  values: Record<string, string>;
}

export type FormErrors = Record<string, string>;

export function validateCaseInput(fields: FieldDef[], input: CaseInput): FormErrors {
  const errors: FormErrors = {};
  const title = input.title.trim();
  if (!title) errors.title = "«Caso de prueba» es obligatorio.";
  else if (charLength(title) > 300) errors.title = "«Caso de prueba» supera 300 caracteres.";
  if (!isStatus(input.status)) errors.status = "Estado inválido.";
  for (const f of activeFields(fields)) {
    const message = validateValue(f, input.values[f.id] ?? "");
    if (message) errors[f.id] = message;
  }
  return errors;
}

export function normalizeCaseInput(fields: FieldDef[], input: CaseInput): CaseInput {
  const values: Record<string, string> = {};
  for (const f of activeFields(fields)) {
    const v = normalizeValue(f.type, input.values[f.id] ?? "");
    if (v !== "") values[f.id] = v;
  }
  return { title: input.title.trim(), status: input.status, values };
}

export function emptyCaseInput(fields: FieldDef[], defaults: Record<string, string> = {}): CaseInput {
  return {
    title: "",
    status: DEFAULT_STATUS,
    values: Object.fromEntries(activeFields(fields).map((f) => [f.id, defaults[f.key] ?? ""])),
  };
}

export function caseToInput(fields: FieldDef[], record: CaseRecord): CaseInput {
  return {
    title: record.title,
    status: record.status,
    values: Object.fromEntries(activeFields(fields).map((f) => [f.id, record.values[f.id] ?? ""])),
  };
}

const fold = (value: string) => value.normalize("NFD").replace(/\p{Diacritic}/gu, "").toLowerCase();

export function filterCases(cases: CaseRecord[], filter: { search: string; status: Status | "" }): CaseRecord[] {
  const query = fold(filter.search.trim());
  return cases.filter(
    (c) =>
      (!filter.status || c.status === filter.status) &&
      (!query || fold([c.code, c.title, ...Object.values(c.values)].join("\n")).includes(query)),
  );
}

export const formatCode = (sequence: number) => `EJ-${String(sequence).padStart(3, "0")}`;
```

`src/domain/summary.ts`:

```ts
import { STATUSES, type Status } from "./status";

export function countByStatus(cases: { status: Status }[]): Record<Status, number> {
  const counts = Object.fromEntries(STATUSES.map((s) => [s, 0])) as Record<Status, number>;
  for (const c of cases) counts[c.status] += 1;
  return counts;
}

/** Aprobado / (Aprobado + Fallido + Bloqueado); null si no hay ejecuciones con veredicto. */
export function successRate(cases: { status: Status }[]): number | null {
  const c = countByStatus(cases);
  const finished = c.Aprobado + c.Fallido + c.Bloqueado;
  return finished === 0 ? null : c.Aprobado / finished;
}

export const formatRate = (rate: number | null) => (rate === null ? "—" : `${Math.round(rate * 100)} %`);
```

`src/domain/project.ts`:

```ts
import type { FormErrors } from "./cases";
import { charLength, isIsoDate, type FieldDef, type FieldType } from "./fields";

export interface ProjectMeta {
  name: string;
  client: string;
  testType: string;
  releaseVersion: string;
  startDate: string;
  endDate: string;
}

export interface Project extends ProjectMeta {
  id: string;
  nextSequence: number;
  sourceTemplateId: string | null;
  createdAt: string;
  updatedAt: string;
  fields: FieldDef[];
}

export interface ProjectSummary {
  id: string;
  name: string;
  client: string;
  caseCount: number;
  updatedAt: string;
}

export interface Template {
  id: string;
  name: string;
  testType: string;
  builtin: boolean;
  fields: FieldDef[];
}

export interface NewProjectInput {
  meta: ProjectMeta;
  templateId: string;
}

export interface FieldInput {
  id: string | null;
  label: string;
  type: FieldType;
  options: string[];
  required: boolean;
  clearOnDuplicate: boolean;
  width: number;
  archived: boolean;
}

export const emptyMeta = (): ProjectMeta => ({ name: "", client: "", testType: "", releaseVersion: "", startDate: "", endDate: "" });

export const projectToMeta = (p: ProjectMeta): ProjectMeta => ({
  name: p.name,
  client: p.client,
  testType: p.testType,
  releaseVersion: p.releaseVersion,
  startDate: p.startDate,
  endDate: p.endDate,
});

const LIMITS: [keyof ProjectMeta, string, number][] = [
  ["name", "Nombre del proyecto", 150],
  ["client", "Cliente", 150],
  ["testType", "Tipo de prueba", 100],
  ["releaseVersion", "Versión / release", 100],
];

export function validateProjectMeta(meta: ProjectMeta): FormErrors {
  const errors: FormErrors = {};
  if (!meta.name.trim()) errors.name = "«Nombre del proyecto» es obligatorio.";
  for (const [key, label, max] of LIMITS) {
    if (!errors[key] && charLength(meta[key].trim()) > max) errors[key] = `«${label}» supera ${max} caracteres.`;
  }
  const start = meta.startDate.trim();
  const end = meta.endDate.trim();
  if (start && !isIsoDate(start)) errors.startDate = "«Fecha de inicio» debe ser una fecha válida (AAAA-MM-DD).";
  if (end && !isIsoDate(end)) errors.endDate = "«Fecha de fin» debe ser una fecha válida (AAAA-MM-DD).";
  if (!errors.startDate && !errors.endDate && start && end && start > end) {
    errors.endDate = "La fecha de fin no puede ser anterior a la de inicio.";
  }
  return errors;
}

export const fieldToInput = (f: FieldDef): FieldInput => ({
  id: f.id,
  label: f.label,
  type: f.type,
  options: [...f.options],
  required: f.required,
  clearOnDuplicate: f.clearOnDuplicate,
  width: f.width,
  archived: f.archived,
});

export const newFieldInput = (): FieldInput => ({
  id: null,
  label: "",
  type: "text",
  options: [],
  required: false,
  clearOnDuplicate: false,
  width: 20,
  archived: false,
});

export function normalizeFieldInput(input: FieldInput): FieldInput {
  const options = input.type === "list" ? [...new Set(input.options.map((o) => o.trim()).filter(Boolean))] : [];
  return { ...input, label: input.label.trim(), options };
}

export function validateFieldInputs(inputs: FieldInput[]): FormErrors {
  const errors: FormErrors = {};
  if (inputs.length > 60) errors._form = "Un proyecto admite como máximo 60 columnas.";
  const seen = new Set<string>();
  inputs.forEach((raw, index) => {
    const input = normalizeFieldInput(raw);
    const key = String(index);
    if (!input.label) {
      errors[key] = `La columna ${index + 1} necesita un nombre.`;
      return;
    }
    if (!input.archived) {
      const folded = input.label.toLowerCase();
      if (seen.has(folded)) {
        errors[key] = `La columna «${input.label}» está repetida.`;
        return;
      }
      seen.add(folded);
    }
    if (input.type === "list" && input.options.length === 0) {
      errors[key] = `La lista «${input.label}» necesita al menos una opción.`;
    } else if (input.width < 8 || input.width > 100) {
      errors[key] = `El ancho de «${input.label}» debe estar entre 8 y 100.`;
    }
  });
  return errors;
}
```

- [ ] **Step 4: Ejecutar los tests**

Run: `pnpm test src/domain && pnpm typecheck`
Expected: todos PASS; typecheck sin errores.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(f1): dominio TS espejo de las reglas de Rust"
```

---

### Task 10: Puertos, adaptador Tauri (zod + contrato) y adaptador en memoria

**Files:**
- Create: `src/ports/errors.ts`, `src/ports/repositories.ts`, `src/infrastructure/tauri/dto.ts`, `src/infrastructure/tauri/client.ts`, `src/infrastructure/tauri/repositories.ts`, `src/infrastructure/memory/fixtures.ts`, `src/infrastructure/memory/repositories.ts`
- Test: `src/infrastructure/tauri/contract.test.ts`, `src/infrastructure/tauri/client.test.ts`, `src/infrastructure/memory/repositories.test.ts`

**Interfaces:**
- Consumes: dominio de Task 9; fixture `src/infrastructure/tauri/__fixtures__/contracts.json` de Task 8.
- Produces:
  - `AppError(code, message, detail?)` con `code: "validation" | "not_found" | "db" | "io" | "internal" | "contract"`; `isValidationError(e)` (true para `validation` y `not_found`).
  - `TemplateRepository { list(); saveFromProject(projectId, name, testType); remove(id) }`
  - `ProjectRepository { list(); get(id); create(input: NewProjectInput); updateMeta(id, meta: ProjectMeta); remove(id); saveFields(projectId, fields: FieldInput[]): Promise<FieldDef[]> }`
  - `CaseRepository { list(projectId); create(projectId, input); update(caseId, input); duplicate(caseId); remove(caseId) }`
  - `Repositories { templates; projects; cases }`
  - `tauriRepositories(): Repositories`; `createMemoryRepositories(): Repositories` (con las plantillas `tpl-funcional` y `tpl-api` y las mismas reglas básicas que Rust).

- [ ] **Step 1: Puertos**

`src/ports/errors.ts`:

```ts
export type AppErrorCode = "validation" | "not_found" | "db" | "io" | "internal" | "contract";

export class AppError extends Error {
  readonly code: AppErrorCode;
  readonly detail: string;

  constructor(code: AppErrorCode, message: string, detail = "") {
    super(message);
    this.name = "AppError";
    this.code = code;
    this.detail = detail;
  }
}

export const isValidationError = (e: unknown): e is AppError =>
  e instanceof AppError && (e.code === "validation" || e.code === "not_found");
```

`src/ports/repositories.ts`:

```ts
import type { CaseInput, CaseRecord } from "../domain/cases";
import type { FieldDef } from "../domain/fields";
import type { FieldInput, NewProjectInput, Project, ProjectMeta, ProjectSummary, Template } from "../domain/project";

export interface TemplateRepository {
  list(): Promise<Template[]>;
  saveFromProject(projectId: string, name: string, testType: string): Promise<Template>;
  remove(id: string): Promise<void>;
}

export interface ProjectRepository {
  list(): Promise<ProjectSummary[]>;
  get(id: string): Promise<Project>;
  create(input: NewProjectInput): Promise<Project>;
  updateMeta(id: string, meta: ProjectMeta): Promise<Project>;
  remove(id: string): Promise<void>;
  saveFields(projectId: string, fields: FieldInput[]): Promise<FieldDef[]>;
}

export interface CaseRepository {
  list(projectId: string): Promise<CaseRecord[]>;
  create(projectId: string, input: CaseInput): Promise<CaseRecord>;
  update(caseId: string, input: CaseInput): Promise<CaseRecord>;
  duplicate(caseId: string): Promise<CaseRecord>;
  remove(caseId: string): Promise<void>;
}

export interface Repositories {
  templates: TemplateRepository;
  projects: ProjectRepository;
  cases: CaseRepository;
}
```

- [ ] **Step 2: Escribir los tests del adaptador Tauri (fallan)**

`src/infrastructure/tauri/contract.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import type { CaseInput } from "../../domain/cases";
import type { FieldInput, NewProjectInput } from "../../domain/project";
import fixture from "./__fixtures__/contracts.json";
import { appErrorSchema, caseSchema, fieldSchema, projectSchema, projectSummarySchema, templateSchema } from "./dto";

describe("contrato Rust → TS", () => {
  it.each([
    ["field", fieldSchema],
    ["template", templateSchema],
    ["projectSummary", projectSummarySchema],
    ["project", projectSchema],
    ["case", caseSchema],
    ["error", appErrorSchema],
  ] as const)("%s cumple el esquema estricto", (key, schema) => {
    expect(() => schema.parse(fixture[key])).not.toThrow();
  });
});

describe("contrato TS → Rust", () => {
  const sortedKeys = (o: object) => Object.keys(o).sort();

  it("los inputs de TS tienen exactamente las claves que Rust acepta", () => {
    const meta = { name: "", client: "", testType: "", releaseVersion: "", startDate: "", endDate: "" };
    const newProject: NewProjectInput = { meta, templateId: "" };
    const field: FieldInput = { id: null, label: "", type: "text", options: [], required: false, clearOnDuplicate: false, width: 8, archived: false };
    const caseInput: CaseInput = { title: "", status: "Aprobado", values: {} };
    expect(sortedKeys(newProject)).toEqual(sortedKeys(fixture.inputs.newProjectInput));
    expect(sortedKeys(meta)).toEqual(sortedKeys(fixture.inputs.projectMetaInput));
    expect(sortedKeys(field)).toEqual(sortedKeys(fixture.inputs.fieldInput));
    expect(sortedKeys(caseInput)).toEqual(sortedKeys(fixture.inputs.caseInput));
  });
});
```

`src/infrastructure/tauri/client.test.ts`:

```ts
import { beforeEach, describe, expect, it, vi } from "vitest";
import { z } from "zod";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => invoke(...args) }));

const { call } = await import("./client");

describe("call", () => {
  beforeEach(() => invoke.mockReset());

  it("devuelve datos validados", async () => {
    invoke.mockResolvedValue({ ok: 1 });
    await expect(call("cmd", { a: 1 }, z.strictObject({ ok: z.number() }))).resolves.toEqual({ ok: 1 });
    expect(invoke).toHaveBeenCalledWith("cmd", { a: 1 });
  });

  it("traduce el error serializado de Rust", async () => {
    invoke.mockRejectedValue({ code: "validation", message: "«Entorno» es obligatorio.", detail: "x" });
    await expect(call("cmd", {}, z.null())).rejects.toMatchObject({ code: "validation", message: "«Entorno» es obligatorio." });
  });

  it("marca como internal un rechazo desconocido", async () => {
    invoke.mockRejectedValue("boom");
    await expect(call("cmd", {}, z.null())).rejects.toMatchObject({ code: "internal" });
  });

  it("marca como contract una respuesta con forma inesperada", async () => {
    invoke.mockResolvedValue({ ok: "no" });
    await expect(call("cmd", {}, z.strictObject({ ok: z.number() }))).rejects.toMatchObject({ code: "contract" });
  });
});
```

Run: `pnpm test src/infrastructure/tauri`
Expected: FAIL (`./dto` y `./client` no existen).

- [ ] **Step 3: Implementar el adaptador Tauri**

`src/infrastructure/tauri/dto.ts`:

```ts
import { z } from "zod";
import { FIELD_TYPES } from "../../domain/fields";
import { STATUSES } from "../../domain/status";

export const fieldSchema = z.strictObject({
  id: z.string(),
  key: z.string(),
  label: z.string(),
  type: z.enum(FIELD_TYPES),
  options: z.array(z.string()),
  required: z.boolean(),
  clearOnDuplicate: z.boolean(),
  width: z.number().int(),
  position: z.number().int(),
  archived: z.boolean(),
});

export const templateSchema = z.strictObject({
  id: z.string(),
  name: z.string(),
  testType: z.string(),
  builtin: z.boolean(),
  fields: z.array(fieldSchema),
});

export const projectSummarySchema = z.strictObject({
  id: z.string(),
  name: z.string(),
  client: z.string(),
  caseCount: z.number().int(),
  updatedAt: z.string(),
});

export const projectSchema = z.strictObject({
  id: z.string(),
  name: z.string(),
  client: z.string(),
  testType: z.string(),
  releaseVersion: z.string(),
  startDate: z.string(),
  endDate: z.string(),
  nextSequence: z.number().int(),
  sourceTemplateId: z.string().nullable(),
  createdAt: z.string(),
  updatedAt: z.string(),
  fields: z.array(fieldSchema),
});

export const caseSchema = z.strictObject({
  id: z.string(),
  projectId: z.string(),
  code: z.string(),
  title: z.string(),
  status: z.enum(STATUSES),
  values: z.record(z.string(), z.string()),
  createdAt: z.string(),
  updatedAt: z.string(),
});

export const appErrorSchema = z.strictObject({
  code: z.enum(["validation", "not_found", "db", "io", "internal"]),
  message: z.string(),
  detail: z.string(),
});
```

`src/infrastructure/tauri/client.ts`:

```ts
import { invoke } from "@tauri-apps/api/core";
import type { z } from "zod";
import { AppError } from "../../ports/errors";
import { appErrorSchema } from "./dto";

export async function call<S extends z.ZodType>(command: string, args: Record<string, unknown>, schema: S): Promise<z.infer<S>> {
  let raw: unknown;
  try {
    raw = await invoke(command, args);
  } catch (error) {
    const parsed = appErrorSchema.safeParse(error);
    if (parsed.success) throw new AppError(parsed.data.code, parsed.data.message, parsed.data.detail);
    throw new AppError("internal", "No se pudo comunicar con el núcleo de Qapture.", String(error));
  }
  const result = schema.safeParse(raw);
  if (!result.success) {
    throw new AppError("contract", "La respuesta del núcleo no tiene el formato esperado.", result.error.message);
  }
  return result.data;
}
```

`src/infrastructure/tauri/repositories.ts`:

```ts
import { z } from "zod";
import type { Repositories } from "../../ports/repositories";
import { call } from "./client";
import { caseSchema, fieldSchema, projectSchema, projectSummarySchema, templateSchema } from "./dto";

const nothing = z.null();

export function tauriRepositories(): Repositories {
  return {
    templates: {
      list: () => call("list_templates", {}, z.array(templateSchema)),
      saveFromProject: (projectId, name, testType) =>
        call("save_template_from_project", { projectId, name, testType }, templateSchema),
      remove: async (id) => {
        await call("delete_template", { id }, nothing);
      },
    },
    projects: {
      list: () => call("list_projects", {}, z.array(projectSummarySchema)),
      get: (id) => call("get_project", { id }, projectSchema),
      create: (input) => call("create_project", { input }, projectSchema),
      updateMeta: (id, input) => call("update_project", { id, input }, projectSchema),
      remove: async (id) => {
        await call("delete_project", { id }, nothing);
      },
      saveFields: (projectId, fields) => call("save_project_fields", { projectId, fields }, z.array(fieldSchema)),
    },
    cases: {
      list: (projectId) => call("list_cases", { projectId }, z.array(caseSchema)),
      create: (projectId, input) => call("create_case", { projectId, input }, caseSchema),
      update: (caseId, input) => call("update_case", { caseId, input }, caseSchema),
      duplicate: (caseId) => call("duplicate_case", { caseId }, caseSchema),
      remove: async (caseId) => {
        await call("delete_case", { caseId }, nothing);
      },
    },
  };
}
```

Run: `pnpm test src/infrastructure/tauri && pnpm typecheck`
Expected: PASS.

- [ ] **Step 4: Escribir los tests del adaptador en memoria (fallan)**

`src/infrastructure/memory/repositories.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { emptyMeta, fieldToInput } from "../../domain/project";
import { createMemoryRepositories } from "./repositories";

async function setup() {
  const repos = createMemoryRepositories();
  const project = await repos.projects.create({ meta: { ...emptyMeta(), name: "Recibos" }, templateId: "tpl-funcional" });
  const id = (key: string) => project.fields.find((f) => f.key === key)!.id;
  return { repos, project, id };
}

describe("memoria", () => {
  it("asigna códigos y no los reutiliza", async () => {
    const { repos, project, id } = await setup();
    const input = { title: "Caso", status: "Aprobado" as const, values: { [id("entorno")]: "QA" } };
    await repos.cases.create(project.id, input);
    const b = await repos.cases.create(project.id, input);
    await repos.cases.remove(b.id);
    expect((await repos.cases.create(project.id, input)).code).toBe("EJ-003");
  });

  it("rechaza casos inválidos y columnas ajenas", async () => {
    const { repos, project, id } = await setup();
    await expect(repos.cases.create(project.id, { title: "Caso", status: "Aprobado", values: {} })).rejects.toMatchObject({
      code: "validation",
      message: "«Entorno» es obligatorio.",
    });
    await expect(
      repos.cases.create(project.id, { title: "Caso", status: "Aprobado", values: { [id("entorno")]: "QA", otro: "x" } }),
    ).rejects.toMatchObject({ message: "La columna enviada no pertenece al proyecto. Recarga el proyecto." });
  });

  it("duplica limpiando las columnas marcadas", async () => {
    const { repos, project, id } = await setup();
    const src = await repos.cases.create(project.id, {
      title: "Caso",
      status: "Aprobado",
      values: { [id("entorno")]: "QA", [id("resultado_obtenido")]: "OK" },
    });
    const copy = await repos.cases.duplicate(src.id);
    expect(copy).toMatchObject({ code: "EJ-002", status: "No ejecutado", values: { [id("entorno")]: "QA" } });
  });

  it("archiva columnas con datos y elimina las vacías", async () => {
    const { repos, project, id } = await setup();
    await repos.cases.create(project.id, { title: "Caso", status: "Aprobado", values: { [id("entorno")]: "QA", [id("modulo")]: "Cobranzas" } });
    const kept = project.fields.filter((f) => f.key !== "modulo" && f.key !== "menu").map(fieldToInput);
    const fields = await repos.projects.saveFields(project.id, kept);
    expect(fields.find((f) => f.key === "menu")).toBeUndefined();
    expect(fields.find((f) => f.key === "modulo")?.archived).toBe(true);
  });

  it("al eliminar un proyecto elimina sus casos", async () => {
    const { repos, project, id } = await setup();
    await repos.cases.create(project.id, { title: "Caso", status: "Aprobado", values: { [id("entorno")]: "QA" } });
    await repos.projects.remove(project.id);
    expect(await repos.projects.list()).toEqual([]);
    await expect(repos.cases.list(project.id)).rejects.toMatchObject({ code: "not_found" });
  });
});
```

Run: `pnpm test src/infrastructure/memory`
Expected: FAIL (el módulo no existe).

- [ ] **Step 5: Implementar el adaptador en memoria**

`src/infrastructure/memory/fixtures.ts` (espejo de un subconjunto de `db/seed.rs`, solo para tests):

```ts
import type { FieldDef, FieldType } from "../../domain/fields";
import type { Template } from "../../domain/project";

type Spec = [key: string, label: string, type: FieldType, width: number, extra?: Partial<FieldDef>];

const build = (templateId: string, specs: Spec[]): FieldDef[] =>
  specs.map(([key, label, type, width, extra], position) => ({
    id: `${templateId}-${key}`,
    key,
    label,
    type,
    options: [],
    required: false,
    clearOnDuplicate: false,
    width,
    position,
    archived: false,
    ...extra,
  }));

export const builtinTemplates: Template[] = [
  {
    id: "tpl-funcional",
    name: "Funcional",
    testType: "Funcional",
    builtin: true,
    fields: build("tpl-funcional", [
      ["entorno", "Entorno", "text", 21, { required: true }],
      ["modulo", "Módulo", "text", 22],
      ["menu", "Menú", "text", 24],
      ["submenu", "Submenú", "text", 24],
      ["funcionalidad", "Funcionalidad", "text", 28],
      ["datos_entrada", "Datos de entrada", "longtext", 48],
      ["resultado_esperado", "Resultado esperado", "longtext", 44],
      ["resultado_obtenido", "Resultado obtenido", "longtext", 44, { clearOnDuplicate: true }],
      ["fecha_ejecucion", "Fecha de ejecución", "date", 15, { clearOnDuplicate: true }],
      ["responsable", "Responsable", "text", 24],
      ["observaciones", "Observaciones", "longtext", 40, { clearOnDuplicate: true }],
    ]),
  },
  {
    id: "tpl-api",
    name: "API",
    testType: "API",
    builtin: true,
    fields: build("tpl-api", [
      ["entorno", "Entorno", "text", 21, { required: true }],
      ["endpoint", "Endpoint", "text", 40],
      ["metodo", "Método", "list", 12, { options: ["GET", "POST", "PUT", "PATCH", "DELETE"] }],
      ["codigo_http", "Código HTTP", "number", 12, { clearOnDuplicate: true }],
    ]),
  },
];
```

`src/infrastructure/memory/repositories.ts`:

```ts
import { formatCode, normalizeCaseInput, validateCaseInput, type CaseInput, type CaseRecord } from "../../domain/cases";
import { activeFields, type FieldDef } from "../../domain/fields";
import { normalizeFieldInput, validateFieldInputs, validateProjectMeta, type Project, type ProjectMeta, type Template } from "../../domain/project";
import { DEFAULT_STATUS } from "../../domain/status";
import { AppError } from "../../ports/errors";
import type { Repositories } from "../../ports/repositories";
import { builtinTemplates } from "./fixtures";

/** Implementación en memoria para tests de la aplicación y de la UI. Replica las reglas principales de Rust. */
export function createMemoryRepositories(): Repositories {
  let seq = 0;
  const id = (prefix: string) => `${prefix}-${++seq}`;
  let tick = 0;
  const now = () => new Date(Date.UTC(2026, 9, 15, 12, 0, 0, tick++)).toISOString();
  const copy = <T>(value: T): T => structuredClone(value);
  const fail = (message: string): never => {
    throw new AppError("validation", message);
  };
  const firstError = (errors: Record<string, string>) => {
    const message = Object.values(errors)[0];
    if (message) fail(message);
  };

  const templates: Template[] = copy(builtinTemplates);
  const projects = new Map<string, Project>();
  const cases = new Map<string, CaseRecord>();

  const getProject = (projectId: string): Project => {
    const p = projects.get(projectId);
    if (!p) throw new AppError("not_found", "El proyecto no existe.");
    return p;
  };
  const getCase = (caseId: string): CaseRecord => {
    const c = cases.get(caseId);
    if (!c) throw new AppError("not_found", "El caso no existe.");
    return c;
  };
  const touch = (p: Project) => {
    p.updatedAt = now();
  };
  const trimMeta = (m: ProjectMeta): ProjectMeta => ({
    name: m.name.trim(),
    client: m.client.trim(),
    testType: m.testType.trim(),
    releaseVersion: m.releaseVersion.trim(),
    startDate: m.startDate.trim(),
    endDate: m.endDate.trim(),
  });
  const checkCase = (p: Project, input: CaseInput) => {
    const active = new Set(activeFields(p.fields).map((f) => f.id));
    if (Object.keys(input.values).some((k) => !active.has(k))) fail("La columna enviada no pertenece al proyecto. Recarga el proyecto.");
    firstError(validateCaseInput(p.fields, input));
    return normalizeCaseInput(p.fields, input);
  };
  const projectCases = (projectId: string) => [...cases.values()].filter((c) => c.projectId === projectId);

  return {
    templates: {
      async list() {
        return copy([...templates].sort((a, b) => Number(b.builtin) - Number(a.builtin) || a.name.localeCompare(b.name)));
      },
      async saveFromProject(projectId, name, testType) {
        const p = getProject(projectId);
        const clean = name.trim();
        if (!clean) fail("«Nombre de la plantilla» es obligatorio.");
        if (templates.some((t) => t.name.toLowerCase() === clean.toLowerCase())) fail("Ya existe una plantilla con ese nombre.");
        const template: Template = {
          id: id("tpl"),
          name: clean,
          testType: testType.trim(),
          builtin: false,
          fields: activeFields(p.fields).map((f, position) => ({ ...f, id: id("tf"), position, archived: false })),
        };
        templates.push(template);
        return copy(template);
      },
      async remove(templateId) {
        const t = templates.find((x) => x.id === templateId);
        if (!t) throw new AppError("not_found", "La plantilla no existe.");
        if (t.builtin) fail("Las plantillas incluidas no se pueden eliminar.");
        templates.splice(templates.indexOf(t), 1);
      },
    },
    projects: {
      async list() {
        return [...projects.values()]
          .sort((a, b) => b.updatedAt.localeCompare(a.updatedAt))
          .map((p) => ({ id: p.id, name: p.name, client: p.client, caseCount: projectCases(p.id).length, updatedAt: p.updatedAt }));
      },
      async get(projectId) {
        return copy(getProject(projectId));
      },
      async create(input) {
        firstError(validateProjectMeta(input.meta));
        const t = templates.find((x) => x.id === input.templateId);
        if (!t) throw new AppError("not_found", "La plantilla no existe.");
        const ts = now();
        const p: Project = {
          id: id("prj"),
          ...trimMeta(input.meta),
          nextSequence: 1,
          sourceTemplateId: t.id,
          createdAt: ts,
          updatedAt: ts,
          fields: t.fields.map((f) => ({ ...f, id: id("fld"), archived: false })),
        };
        projects.set(p.id, p);
        return copy(p);
      },
      async updateMeta(projectId, meta) {
        const p = getProject(projectId);
        firstError(validateProjectMeta(meta));
        Object.assign(p, trimMeta(meta));
        touch(p);
        return copy(p);
      },
      async remove(projectId) {
        getProject(projectId);
        projects.delete(projectId);
        for (const c of projectCases(projectId)) cases.delete(c.id);
      },
      async saveFields(projectId, inputs) {
        const p = getProject(projectId);
        firstError(validateFieldInputs(inputs));
        const hasData = (fieldId: string) => projectCases(projectId).some((c) => (c.values[fieldId] ?? "") !== "");
        const next: FieldDef[] = inputs.map((raw, position) => {
          const input = normalizeFieldInput(raw);
          const existing = input.id ? p.fields.find((f) => f.id === input.id) : undefined;
          if (input.id && !existing) fail("La columna ya no existe. Recarga el proyecto.");
          return {
            id: existing?.id ?? id("fld"),
            key: existing?.key ?? id("campo"),
            label: input.label,
            type: input.type,
            options: input.options,
            required: input.required,
            clearOnDuplicate: input.clearOnDuplicate,
            width: input.width,
            position,
            archived: input.archived,
          };
        });
        for (const f of p.fields) {
          if (!next.some((n) => n.id === f.id) && hasData(f.id)) next.push({ ...f, archived: true, position: next.length });
        }
        p.fields = next;
        touch(p);
        return copy([...activeFields(next), ...next.filter((f) => f.archived)]);
      },
    },
    cases: {
      async list(projectId) {
        getProject(projectId);
        return copy(projectCases(projectId));
      },
      async create(projectId, input) {
        const p = getProject(projectId);
        const clean = checkCase(p, input);
        const ts = now();
        const record: CaseRecord = { id: id("case"), projectId, code: formatCode(p.nextSequence++), ...clean, createdAt: ts, updatedAt: ts };
        cases.set(record.id, record);
        touch(p);
        return copy(record);
      },
      async update(caseId, input) {
        const c = getCase(caseId);
        const p = getProject(c.projectId);
        const clean = checkCase(p, input);
        const archived = Object.fromEntries(Object.entries(c.values).filter(([fid]) => p.fields.find((f) => f.id === fid)?.archived));
        Object.assign(c, { title: clean.title, status: clean.status, values: { ...archived, ...clean.values }, updatedAt: now() });
        touch(p);
        return copy(c);
      },
      async duplicate(caseId) {
        const src = getCase(caseId);
        const p = getProject(src.projectId);
        const values = Object.fromEntries(
          Object.entries(src.values).filter(([fid]) => !p.fields.find((f) => f.id === fid)?.clearOnDuplicate),
        );
        const ts = now();
        const record: CaseRecord = {
          id: id("case"),
          projectId: p.id,
          code: formatCode(p.nextSequence++),
          title: src.title,
          status: DEFAULT_STATUS,
          values,
          createdAt: ts,
          updatedAt: ts,
        };
        cases.set(record.id, record);
        touch(p);
        return copy(record);
      },
      async remove(caseId) {
        const c = getCase(caseId);
        cases.delete(caseId);
        touch(getProject(c.projectId));
      },
    },
  };
}
```

- [ ] **Step 6: Ejecutar los tests**

Run: `pnpm test src/infrastructure && pnpm typecheck`
Expected: todos PASS.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "feat(f1): puertos, adaptador Tauri con contrato zod y adaptador en memoria"
```

---

### Task 11: Casos de uso y store de la app (Zustand)

**Files:**
- Create: `src/application/use-cases.ts`, `src/ui/store/preferences.ts`, `src/ui/store/workspace.ts`, `src/ui/store/context.tsx`
- Test: `src/ui/store/workspace.test.ts`

**Interfaces:**
- Consumes: `Repositories`, `AppError`/`isValidationError`, dominio.
- Produces:
  - `application/use-cases.ts`: `type Outcome<T> = { ok: true; value: T } | { ok: false; errors: FormErrors }`; `createProject(repos, input)`, `updateProjectMeta(repos, projectId, meta)`, `saveProjectFields(repos, projectId, fields)`, `saveCase(repos, project, input, caseId?)`, `saveTemplate(repos, projectId, name, testType)`. Los errores de validación de Rust se devuelven como `{ _form: mensaje }`; los demás se relanzan.
  - `Preferences { getLastProject(); setLastProject(id | null) }`, `browserPreferences()`, `memoryPreferences(initial?)`
  - `createWorkspaceStore(repos, prefs?) : StoreApi<WorkspaceState>`, con estado `templates, projects, project, cases, selectedCaseId, search, statusFilter, ready, toasts` y acciones `init, selectProject, createProject, updateProjectMeta, saveFields, deleteProject, saveTemplateFromProject, deleteTemplate, saveCase, duplicateCase, deleteCase, selectCase, setSearch, setStatusFilter, pushToast, dismissToast`. Las acciones de formulario devuelven `FormResult = { ok: true } | { ok: false; errors: FormErrors }`.
  - `WorkspaceProvider({ store, children })`, `useWorkspace(selector)`

- [ ] **Step 1: Escribir los tests del store (fallan)**

`src/ui/store/workspace.test.ts`:

```ts
import { describe, expect, it, vi } from "vitest";
import { emptyMeta, fieldToInput } from "../../domain/project";
import { createMemoryRepositories } from "../../infrastructure/memory/repositories";
import { AppError } from "../../ports/errors";
import { memoryPreferences } from "./preferences";
import { createWorkspaceStore } from "./workspace";

const meta = (name: string) => ({ ...emptyMeta(), name, client: "ACME" });

async function setup(lastProject: string | null = null) {
  const repos = createMemoryRepositories();
  const prefs = memoryPreferences(lastProject);
  const store = createWorkspaceStore(repos, prefs);
  return { repos, prefs, store, s: () => store.getState() };
}

describe("workspace store", () => {
  it("arranca vacío y listo", async () => {
    const { s } = await setup();
    await s().init();
    expect(s().ready).toBe(true);
    expect(s().project).toBeNull();
    expect(s().templates.map((t) => t.id)).toContain("tpl-funcional");
  });

  it("abre el último proyecto recordado y, si ya no existe, el primero", async () => {
    const a = await setup();
    const p1 = await a.repos.projects.create({ meta: meta("Uno"), templateId: "tpl-funcional" });
    const p2 = await a.repos.projects.create({ meta: meta("Dos"), templateId: "tpl-funcional" });
    a.prefs.setLastProject(p1.id);
    const store = createWorkspaceStore(a.repos, a.prefs);
    await store.getState().init();
    expect(store.getState().project?.id).toBe(p1.id);

    a.prefs.setLastProject("borrado");
    const fallback = createWorkspaceStore(a.repos, a.prefs);
    await fallback.getState().init();
    expect(fallback.getState().project?.id).toBe(p2.id);
    expect(a.prefs.getLastProject()).toBe(p2.id);
  });

  it("valida antes de crear proyecto y selecciona el creado", async () => {
    const { repos, s } = await setup();
    await s().init();
    const spy = vi.spyOn(repos.projects, "create");
    expect(await s().createProject({ meta: meta(" "), templateId: "tpl-funcional" })).toEqual({
      ok: false,
      errors: { name: "«Nombre del proyecto» es obligatorio." },
    });
    expect(spy).not.toHaveBeenCalled();
    expect(await s().createProject({ meta: meta("Recibos"), templateId: "tpl-funcional" })).toEqual({ ok: true });
    expect(s().project?.name).toBe("Recibos");
    expect(s().projects).toHaveLength(1);
  });

  it("crea, duplica y elimina casos moviendo la selección", async () => {
    const { s } = await setup();
    await s().init();
    await s().createProject({ meta: meta("Recibos"), templateId: "tpl-funcional" });
    const env = s().project!.fields.find((f) => f.key === "entorno")!.id;

    const invalid = await s().saveCase({ title: "Caso", status: "Aprobado", values: {} });
    expect(invalid).toEqual({ ok: false, errors: { [env]: "«Entorno» es obligatorio." } });

    expect(await s().saveCase({ title: "Caso", status: "Aprobado", values: { [env]: "QA" } })).toEqual({ ok: true });
    const first = s().cases[0];
    expect(first.code).toBe("EJ-001");
    expect(s().selectedCaseId).toBe(first.id);
    expect(s().projects[0].caseCount).toBe(1);

    await s().duplicateCase(first.id);
    expect(s().cases.map((c) => c.code)).toEqual(["EJ-001", "EJ-002"]);
    expect(s().selectedCaseId).toBe(s().cases[1].id);

    await s().deleteCase(s().cases[1].id);
    expect(s().cases.map((c) => c.code)).toEqual(["EJ-001"]);
    expect(s().selectedCaseId).toBe(first.id);
  });

  it("recupera un formulario con columnas desactualizadas", async () => {
    const { repos, s } = await setup();
    await s().init();
    await s().createProject({ meta: meta("Recibos"), templateId: "tpl-funcional" });
    const project = s().project!;
    const env = project.fields.find((f) => f.key === "entorno")!.id;
    const menu = project.fields.find((f) => f.key === "menu")!.id;
    await repos.projects.saveFields(project.id, project.fields.filter((f) => f.id !== menu).map(fieldToInput));

    const input = { title: "Caso", status: "Aprobado" as const, values: { [env]: "QA", [menu]: "Cobros" } };
    const stale = await s().saveCase(input);
    expect(stale).toEqual({ ok: false, errors: { _form: "La columna enviada no pertenece al proyecto. Recarga el proyecto." } });
    expect(s().project!.fields.some((f) => f.id === menu)).toBe(false);
    expect(await s().saveCase(input)).toEqual({ ok: true });
  });

  it("al eliminar el proyecto abre otro y lo recuerda", async () => {
    const { prefs, s } = await setup();
    await s().init();
    await s().createProject({ meta: meta("Uno"), templateId: "tpl-funcional" });
    const first = s().project!.id;
    await s().createProject({ meta: meta("Dos"), templateId: "tpl-funcional" });
    await s().deleteProject();
    expect(s().project?.id).toBe(first);
    expect(prefs.getLastProject()).toBe(first);
  });

  it("muestra un toast cuando falla algo que no es validación", async () => {
    const { repos, s } = await setup();
    vi.spyOn(repos.projects, "list").mockRejectedValue(new AppError("db", "No se pudo completar la operación."));
    await s().init();
    expect(s().ready).toBe(true);
    expect(s().toasts).toEqual([expect.objectContaining({ kind: "error", message: "No se pudo completar la operación." })]);
  });
});
```

Run: `pnpm test src/ui/store`
Expected: FAIL (los módulos no existen).

- [ ] **Step 2: Casos de uso**

`src/application/use-cases.ts`:

```ts
import { normalizeCaseInput, validateCaseInput, type CaseInput, type CaseRecord, type FormErrors } from "../domain/cases";
import type { FieldDef } from "../domain/fields";
import {
  normalizeFieldInput,
  validateFieldInputs,
  validateProjectMeta,
  type FieldInput,
  type NewProjectInput,
  type Project,
  type ProjectMeta,
  type Template,
} from "../domain/project";
import { isValidationError } from "../ports/errors";
import type { Repositories } from "../ports/repositories";

export type Outcome<T> = { ok: true; value: T } | { ok: false; errors: FormErrors };

const invalid = (errors: FormErrors) => Object.keys(errors).length > 0;

async function guard<T>(run: () => Promise<T>): Promise<Outcome<T>> {
  try {
    return { ok: true, value: await run() };
  } catch (error) {
    if (isValidationError(error)) return { ok: false, errors: { _form: error.message } };
    throw error;
  }
}

export async function createProject(repos: Repositories, input: NewProjectInput): Promise<Outcome<Project>> {
  const errors = validateProjectMeta(input.meta);
  if (invalid(errors)) return { ok: false, errors };
  return guard(() => repos.projects.create(input));
}

export async function updateProjectMeta(repos: Repositories, projectId: string, meta: ProjectMeta): Promise<Outcome<Project>> {
  const errors = validateProjectMeta(meta);
  if (invalid(errors)) return { ok: false, errors };
  return guard(() => repos.projects.updateMeta(projectId, meta));
}

export async function saveProjectFields(repos: Repositories, projectId: string, fields: FieldInput[]): Promise<Outcome<FieldDef[]>> {
  const errors = validateFieldInputs(fields);
  if (invalid(errors)) return { ok: false, errors };
  return guard(() => repos.projects.saveFields(projectId, fields.map(normalizeFieldInput)));
}

export async function saveCase(repos: Repositories, project: Project, input: CaseInput, caseId?: string): Promise<Outcome<CaseRecord>> {
  const errors = validateCaseInput(project.fields, input);
  if (invalid(errors)) return { ok: false, errors };
  const clean = normalizeCaseInput(project.fields, input);
  return guard(() => (caseId ? repos.cases.update(caseId, clean) : repos.cases.create(project.id, clean)));
}

export async function saveTemplate(repos: Repositories, projectId: string, name: string, testType: string): Promise<Outcome<Template>> {
  if (!name.trim()) return { ok: false, errors: { name: "«Nombre de la plantilla» es obligatorio." } };
  return guard(() => repos.templates.saveFromProject(projectId, name, testType));
}
```

> **Nota para quien implementa:** `saveCase` normaliza con las columnas que conoce el store. Si estas quedaron desactualizadas (Review Focus 1), Rust rechaza la columna con "Recarga el proyecto" y el store recarga el proyecto. En el siguiente intento la normalización usa las columnas vigentes y descarta las que ya no existen, así que el guardado pasa.

- [ ] **Step 3: Preferencias, store y contexto**

`src/ui/store/preferences.ts`:

```ts
export interface Preferences {
  getLastProject(): string | null;
  setLastProject(id: string | null): void;
}

const KEY = "qapture.lastProject";

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
  };
}

export function memoryPreferences(initial: string | null = null): Preferences {
  let last = initial;
  return {
    getLastProject: () => last,
    setLastProject: (id) => {
      last = id;
    },
  };
}
```

`src/ui/store/workspace.ts`:

```ts
import { createStore, type StoreApi } from "zustand/vanilla";
import * as useCases from "../../application/use-cases";
import type { CaseInput, CaseRecord, FormErrors } from "../../domain/cases";
import type { FieldInput, NewProjectInput, Project, ProjectMeta, ProjectSummary, Template } from "../../domain/project";
import type { Status } from "../../domain/status";
import { AppError } from "../../ports/errors";
import type { Repositories } from "../../ports/repositories";
import { browserPreferences, type Preferences } from "./preferences";

export type FormResult = { ok: true } | { ok: false; errors: FormErrors };
export interface Toast {
  id: number;
  kind: "success" | "error";
  message: string;
}

export interface WorkspaceState {
  templates: Template[];
  projects: ProjectSummary[];
  project: Project | null;
  cases: CaseRecord[];
  selectedCaseId: string | null;
  search: string;
  statusFilter: Status | "";
  ready: boolean;
  toasts: Toast[];
  init(): Promise<void>;
  selectProject(id: string): Promise<void>;
  createProject(input: NewProjectInput): Promise<FormResult>;
  updateProjectMeta(meta: ProjectMeta): Promise<FormResult>;
  saveFields(fields: FieldInput[]): Promise<FormResult>;
  deleteProject(): Promise<void>;
  saveTemplateFromProject(name: string, testType: string): Promise<FormResult>;
  deleteTemplate(id: string): Promise<void>;
  saveCase(input: CaseInput, caseId?: string): Promise<FormResult>;
  duplicateCase(id: string): Promise<void>;
  deleteCase(id: string): Promise<void>;
  selectCase(id: string | null): void;
  setSearch(search: string): void;
  setStatusFilter(status: Status | ""): void;
  pushToast(kind: Toast["kind"], message: string): void;
  dismissToast(id: number): void;
}

const NO_PROJECT: FormResult = { ok: false, errors: { _form: "No hay un proyecto abierto." } };

export function createWorkspaceStore(repos: Repositories, prefs: Preferences = browserPreferences()): StoreApi<WorkspaceState> {
  let toastSeq = 0;

  return createStore<WorkspaceState>()((set, get) => {
    const report = (error: unknown) => {
      const message = error instanceof AppError ? error.message : "Ocurrió un error inesperado.";
      get().pushToast("error", message);
    };

    const refreshLists = async () => {
      const [templates, projects] = await Promise.all([repos.templates.list(), repos.projects.list()]);
      set({ templates, projects });
    };

    const loadProject = async (id: string | null, keepSelection = false) => {
      if (!id) {
        set({ project: null, cases: [], selectedCaseId: null });
        prefs.setLastProject(null);
        return;
      }
      const [project, cases] = await Promise.all([repos.projects.get(id), repos.cases.list(id)]);
      const previous = get().selectedCaseId;
      const selectedCaseId = keepSelection && cases.some((c) => c.id === previous) ? previous : (cases[0]?.id ?? null);
      set(keepSelection ? { project, cases, selectedCaseId } : { project, cases, selectedCaseId, search: "", statusFilter: "" });
      prefs.setLastProject(id);
    };

    /** Ejecuta una acción de formulario: errores de validación → formulario; el resto → toast. */
    const form = async <T>(run: () => Promise<useCases.Outcome<T>>, onOk: (value: T) => Promise<void> | void): Promise<FormResult> => {
      try {
        const outcome = await run();
        if (!outcome.ok) return outcome;
        await onOk(outcome.value);
        return { ok: true };
      } catch (error) {
        report(error);
        return { ok: false, errors: {} };
      }
    };

    return {
      templates: [],
      projects: [],
      project: null,
      cases: [],
      selectedCaseId: null,
      search: "",
      statusFilter: "",
      ready: false,
      toasts: [],

      async init() {
        try {
          await refreshLists();
          const { projects } = get();
          const last = prefs.getLastProject();
          await loadProject(projects.find((p) => p.id === last)?.id ?? projects[0]?.id ?? null);
        } catch (error) {
          report(error);
        } finally {
          set({ ready: true });
        }
      },

      async selectProject(id) {
        try {
          await loadProject(id);
        } catch (error) {
          report(error);
        }
      },

      createProject: (input) =>
        form(
          () => useCases.createProject(repos, input),
          async (project) => {
            await refreshLists();
            await loadProject(project.id);
            get().pushToast("success", `Matriz «${project.name}» creada.`);
          },
        ),

      async updateProjectMeta(meta) {
        const current = get().project;
        if (!current) return NO_PROJECT;
        return form(
          () => useCases.updateProjectMeta(repos, current.id, meta),
          async (project) => {
            set({ project });
            await refreshLists();
            get().pushToast("success", "Datos del proyecto guardados.");
          },
        );
      },

      async saveFields(fields) {
        const current = get().project;
        if (!current) return NO_PROJECT;
        return form(
          () => useCases.saveProjectFields(repos, current.id, fields),
          async () => {
            await loadProject(current.id, true);
            get().pushToast("success", "Columnas guardadas.");
          },
        );
      },

      async deleteProject() {
        const current = get().project;
        if (!current) return;
        try {
          await repos.projects.remove(current.id);
          await refreshLists();
          await loadProject(get().projects[0]?.id ?? null);
          get().pushToast("success", `Matriz «${current.name}» eliminada.`);
        } catch (error) {
          report(error);
        }
      },

      async saveTemplateFromProject(name, testType) {
        const current = get().project;
        if (!current) return NO_PROJECT;
        return form(
          () => useCases.saveTemplate(repos, current.id, name, testType),
          async (template) => {
            await refreshLists();
            get().pushToast("success", `Plantilla «${template.name}» guardada.`);
          },
        );
      },

      async deleteTemplate(id) {
        try {
          await repos.templates.remove(id);
          await refreshLists();
          get().pushToast("success", "Plantilla eliminada.");
        } catch (error) {
          report(error);
        }
      },

      async saveCase(input, caseId) {
        const current = get().project;
        if (!current) return NO_PROJECT;
        const result = await form(
          () => useCases.saveCase(repos, current, input, caseId),
          async (saved) => {
            set((s) => ({
              cases: caseId ? s.cases.map((c) => (c.id === saved.id ? saved : c)) : [...s.cases, saved],
              selectedCaseId: saved.id,
            }));
            await refreshLists();
            get().pushToast("success", caseId ? `${saved.code} actualizado.` : `${saved.code} creado.`);
          },
        );
        if (!result.ok && result.errors._form) {
          // Las columnas pudieron cambiar mientras el formulario estaba abierto: recargarlas
          // para que el siguiente intento se normalice con las columnas vigentes.
          try {
            set({ project: await repos.projects.get(current.id) });
          } catch (error) {
            report(error);
          }
        }
        return result;
      },

      async duplicateCase(id) {
        try {
          const source = get().cases.find((c) => c.id === id);
          const copy = await repos.cases.duplicate(id);
          set((s) => ({ cases: [...s.cases, copy], selectedCaseId: copy.id }));
          await refreshLists();
          get().pushToast("success", source ? `${copy.code} creado a partir de ${source.code}.` : `${copy.code} creado.`);
        } catch (error) {
          report(error);
        }
      },

      async deleteCase(id) {
        try {
          const { cases } = get();
          const index = cases.findIndex((c) => c.id === id);
          const removed = cases[index];
          await repos.cases.remove(id);
          const rest = cases.filter((c) => c.id !== id);
          set({ cases: rest, selectedCaseId: rest[Math.max(0, index - 1)]?.id ?? null });
          await refreshLists();
          if (removed) get().pushToast("success", `${removed.code} eliminado.`);
        } catch (error) {
          report(error);
        }
      },

      selectCase: (id) => set({ selectedCaseId: id }),
      setSearch: (search) => set({ search }),
      setStatusFilter: (statusFilter) => set({ statusFilter }),
      pushToast: (kind, message) => set((s) => ({ toasts: [...s.toasts, { id: ++toastSeq, kind, message }] })),
      dismissToast: (id) => set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) })),
    };
  });
}
```

`src/ui/store/context.tsx`:

```tsx
import { createContext, useContext, type ReactNode } from "react";
import { useStore } from "zustand";
import type { StoreApi } from "zustand/vanilla";
import type { WorkspaceState } from "./workspace";

const WorkspaceContext = createContext<StoreApi<WorkspaceState> | null>(null);

export function WorkspaceProvider({ store, children }: { store: StoreApi<WorkspaceState>; children: ReactNode }) {
  return <WorkspaceContext.Provider value={store}>{children}</WorkspaceContext.Provider>;
}

/** Los selectores deben devolver valores estables (campos o acciones), nunca objetos nuevos. */
export function useWorkspace<T>(selector: (state: WorkspaceState) => T): T {
  const store = useContext(WorkspaceContext);
  if (!store) throw new Error("useWorkspace requiere <WorkspaceProvider>.");
  return useStore(store, selector);
}
```

- [ ] **Step 4: Ejecutar los tests**

Run: `pnpm test src/ui/store && pnpm typecheck`
Expected: 7 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(f1): casos de uso y store de la app"
```

---

### Task 12: UI — shell, selector de proyecto, estado vacío y nueva matriz

**Files:**
- Create: `src/ui/components/styles.ts`, `src/ui/components/Dialog.tsx`, `src/ui/components/FormField.tsx`, `src/ui/components/Toasts.tsx`, `src/ui/screens/AppShell.tsx`, `src/ui/screens/TopBar.tsx`, `src/ui/screens/EmptyState.tsx`, `src/ui/screens/ProjectMetaFields.tsx`, `src/ui/screens/NewProjectDialog.tsx`, `src/ui/screens/Workspace.tsx` (provisional), `src/ui/test-utils.tsx`
- Modify: `src/App.tsx` (reemplazar), `src/main.tsx` (reemplazar), `src/App.test.tsx` (eliminar: lo reemplaza el test de la shell)
- Test: `src/ui/screens/AppShell.test.tsx`

**Interfaces:**
- Consumes: `useWorkspace`, `WorkspaceProvider`, `createWorkspaceStore`, `tauriRepositories`, `createMemoryRepositories`, `memoryPreferences`.
- Produces:
  - `styles.ts`: `btn.{primary, secondary, danger, ghost}`, `iconButton`, `inputCls`
  - `Dialog({ title, kicker?, wide?, onClose, children })`, `DialogFooter({ children })`
  - `FormField({ label, htmlFor, required?, error?, className?, children })`
  - `Toasts()`, `AppShell()`, `TopBar({ onNewProject, onSettings? })`, `EmptyState({ onCreate })`
  - `ProjectMetaFields({ idPrefix, meta, errors, onChange })`, `NewProjectDialog({ onClose })`
  - `renderApp({ setup?, lastProject? }) → { repos, store, user }`, `seedProject(repos, name?) → Project`

- [ ] **Step 1: Escribir los tests de la shell (fallan)**

`src/ui/test-utils.tsx`:

```tsx
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { emptyMeta } from "../domain/project";
import { createMemoryRepositories } from "../infrastructure/memory/repositories";
import type { Repositories } from "../ports/repositories";
import { AppShell } from "./screens/AppShell";
import { WorkspaceProvider } from "./store/context";
import { memoryPreferences } from "./store/preferences";
import { createWorkspaceStore } from "./store/workspace";

export async function seedProject(repos: Repositories, name = "Recibos") {
  return repos.projects.create({ meta: { ...emptyMeta(), name, client: "ACME" }, templateId: "tpl-funcional" });
}

export async function renderApp(options: { setup?: (repos: Repositories) => Promise<void>; lastProject?: string | null } = {}) {
  const repos = createMemoryRepositories();
  await options.setup?.(repos);
  const store = createWorkspaceStore(repos, memoryPreferences(options.lastProject ?? null));
  const user = userEvent.setup();
  render(
    <WorkspaceProvider store={store}>
      <AppShell />
    </WorkspaceProvider>,
  );
  await screen.findByTestId("app-ready");
  return { repos, store, user };
}
```

`src/ui/screens/AppShell.test.tsx`:

```tsx
import { fireEvent, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { renderApp, seedProject } from "../test-utils";

describe("AppShell", () => {
  it("invita a crear la primera matriz", async () => {
    await renderApp();
    expect(screen.getByRole("heading", { name: "Crea tu primera matriz" })).toBeInTheDocument();
    expect(screen.getByRole("img", { name: "Teams4Soft" })).toBeInTheDocument();
  });

  it("crea un proyecto validando nombre y fechas", async () => {
    const { user } = await renderApp();
    await user.click(within(screen.getByRole("main")).getByRole("button", { name: /Nueva matriz/ }));
    const dialog = screen.getByRole("dialog", { name: "Nueva matriz" });

    await user.click(within(dialog).getByRole("button", { name: "Crear matriz" }));
    expect(within(dialog).getByText("«Nombre del proyecto» es obligatorio.")).toBeInTheDocument();

    await user.type(within(dialog).getByLabelText(/Nombre del proyecto/), "Recibos GMX");
    fireEvent.change(within(dialog).getByLabelText("Fecha de inicio"), { target: { value: "2026-11-01" } });
    fireEvent.change(within(dialog).getByLabelText("Fecha de fin"), { target: { value: "2026-10-01" } });
    await user.click(within(dialog).getByRole("button", { name: "Crear matriz" }));
    expect(within(dialog).getByText("La fecha de fin no puede ser anterior a la de inicio.")).toBeInTheDocument();

    fireEvent.change(within(dialog).getByLabelText("Fecha de fin"), { target: { value: "2026-11-30" } });
    expect(within(dialog).getByRole("radio", { name: /Funcional/ })).toBeChecked();
    await user.click(within(dialog).getByRole("button", { name: "Crear matriz" }));

    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(screen.getByRole("combobox", { name: "Proyecto" })).toHaveDisplayValue("Recibos GMX");
    expect(await screen.findByText("Matriz «Recibos GMX» creada.")).toBeInTheDocument();
  });

  it("cambia de proyecto con el selector", async () => {
    const { user } = await renderApp({
      setup: async (repos) => {
        await seedProject(repos, "Uno");
        await seedProject(repos, "Dos");
      },
    });
    const select = screen.getByRole("combobox", { name: "Proyecto" });
    expect(select).toHaveDisplayValue("ACME · Dos");
    await user.selectOptions(select, screen.getByRole("option", { name: "ACME · Uno" }));
    expect(select).toHaveDisplayValue("ACME · Uno");
  });
});
```

Run: `git rm -q src/App.test.tsx && pnpm test src/ui/screens`
Expected: FAIL (los componentes no existen).

- [ ] **Step 2: Componentes base**

`src/ui/components/styles.ts`:

```ts
const base =
  "inline-flex items-center justify-center gap-1.5 rounded-md px-3 py-2 text-sm font-medium transition-colors disabled:cursor-not-allowed disabled:opacity-50";

export const btn = {
  primary: `${base} bg-t4s-blue text-white hover:bg-t4s-blue-dark`,
  secondary: `${base} border border-t4s-line bg-white text-t4s-charcoal hover:bg-t4s-cream`,
  danger: `${base} bg-red-700 text-white hover:bg-red-800`,
  ghost: `${base} text-t4s-gray hover:bg-t4s-cream hover:text-t4s-charcoal`,
};

export const iconButton = "rounded-md p-1.5 text-t4s-gray hover:bg-t4s-cream hover:text-t4s-charcoal disabled:opacity-40";

export const inputCls =
  "w-full rounded-md border border-t4s-line bg-white px-3 py-2 text-sm focus:border-t4s-blue focus:outline-none focus:ring-2 focus:ring-t4s-blue/20";
```

`src/ui/components/Dialog.tsx`:

```tsx
import { X } from "lucide-react";
import { useEffect, type ReactNode } from "react";
import { iconButton } from "./styles";

export function Dialog({ title, kicker, wide = false, onClose, children }: {
  title: string;
  kicker?: string;
  wide?: boolean;
  onClose: () => void;
  children: ReactNode;
}) {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  return (
    <div className="fixed inset-0 z-40 grid place-items-center bg-t4s-charcoal/40 p-4">
      <div
        role="dialog"
        aria-modal="true"
        aria-label={title}
        className={`flex max-h-[90vh] w-full flex-col rounded-xl bg-white shadow-xl ${wide ? "max-w-4xl" : "max-w-xl"}`}
      >
        <header className="flex items-start justify-between border-b border-t4s-line px-6 py-4">
          <div>
            {kicker && <p className="text-xs font-semibold uppercase tracking-wide text-t4s-blue">{kicker}</p>}
            <h2 className="text-lg font-semibold">{title}</h2>
          </div>
          <button type="button" onClick={onClose} aria-label="Cerrar" className={iconButton}>
            <X size={18} />
          </button>
        </header>
        <div className="overflow-y-auto px-6 py-4">{children}</div>
      </div>
    </div>
  );
}

export function DialogFooter({ children }: { children: ReactNode }) {
  return <div className="col-span-full mt-2 flex justify-end gap-2 border-t border-t4s-line pt-4">{children}</div>;
}
```

`src/ui/components/FormField.tsx`:

```tsx
import type { ReactNode } from "react";

export function FormField({ label, htmlFor, required = false, error, className = "", children }: {
  label: string;
  htmlFor: string;
  required?: boolean;
  error?: string;
  className?: string;
  children: ReactNode;
}) {
  return (
    <div className={className}>
      <label htmlFor={htmlFor} className="mb-1 block text-sm font-medium">
        {label}
        {required && <span className="text-red-700"> *</span>}
      </label>
      {children}
      {error && (
        <p role="alert" className="mt-1 text-xs text-red-700">
          {error}
        </p>
      )}
    </div>
  );
}
```

`src/ui/components/Toasts.tsx`:

```tsx
import { useEffect } from "react";
import { useWorkspace } from "../store/context";

export function Toasts() {
  const toasts = useWorkspace((s) => s.toasts);
  const dismiss = useWorkspace((s) => s.dismissToast);

  useEffect(() => {
    if (toasts.length === 0) return;
    const timer = setTimeout(() => dismiss(toasts[0].id), 4000);
    return () => clearTimeout(timer);
  }, [toasts, dismiss]);

  return (
    <div role="status" aria-live="polite" className="fixed bottom-4 right-4 z-50 flex flex-col gap-2">
      {toasts.map((t) => (
        <div key={t.id} className={`rounded-lg px-4 py-3 text-sm text-white shadow-lg ${t.kind === "error" ? "bg-red-700" : "bg-t4s-charcoal"}`}>
          {t.message}
        </div>
      ))}
    </div>
  );
}
```

- [ ] **Step 3: Pantallas**

`src/ui/screens/ProjectMetaFields.tsx`:

```tsx
import type { ChangeEvent } from "react";
import type { FormErrors } from "../../domain/cases";
import type { ProjectMeta } from "../../domain/project";
import { FormField } from "../components/FormField";
import { inputCls } from "../components/styles";

export function ProjectMetaFields({ idPrefix, meta, errors, onChange }: {
  idPrefix: string;
  meta: ProjectMeta;
  errors: FormErrors;
  onChange: (meta: ProjectMeta) => void;
}) {
  const bind = (key: keyof ProjectMeta) => ({
    id: `${idPrefix}-${key}`,
    value: meta[key],
    className: inputCls,
    onChange: (e: ChangeEvent<HTMLInputElement>) => onChange({ ...meta, [key]: e.target.value }),
  });
  return (
    <>
      <FormField label="Nombre del proyecto o módulo" required htmlFor={`${idPrefix}-name`} error={errors.name} className="col-span-2">
        <input {...bind("name")} placeholder="Ej. Aplicación de recibos" />
      </FormField>
      <FormField label="Cliente" htmlFor={`${idPrefix}-client`} error={errors.client}>
        <input {...bind("client")} placeholder="Ej. GMX" />
      </FormField>
      <FormField label="Tipo de prueba" htmlFor={`${idPrefix}-testType`} error={errors.testType}>
        <input {...bind("testType")} placeholder="Ej. Funcional" />
      </FormField>
      <FormField label="Versión / release" htmlFor={`${idPrefix}-releaseVersion`} error={errors.releaseVersion} className="col-span-2">
        <input {...bind("releaseVersion")} placeholder="Ej. 2.4.0" />
      </FormField>
      <FormField label="Fecha de inicio" htmlFor={`${idPrefix}-startDate`} error={errors.startDate}>
        <input type="date" {...bind("startDate")} />
      </FormField>
      <FormField label="Fecha de fin" htmlFor={`${idPrefix}-endDate`} error={errors.endDate}>
        <input type="date" {...bind("endDate")} />
      </FormField>
    </>
  );
}
```

`src/ui/screens/NewProjectDialog.tsx`:

```tsx
import { useState, type FormEvent } from "react";
import type { FormErrors } from "../../domain/cases";
import { emptyMeta, type ProjectMeta } from "../../domain/project";
import { Dialog, DialogFooter } from "../components/Dialog";
import { btn } from "../components/styles";
import { useWorkspace } from "../store/context";
import { ProjectMetaFields } from "./ProjectMetaFields";

export function NewProjectDialog({ onClose }: { onClose: () => void }) {
  const templates = useWorkspace((s) => s.templates);
  const createProject = useWorkspace((s) => s.createProject);
  const [meta, setMeta] = useState<ProjectMeta>(emptyMeta);
  const [templateId, setTemplateId] = useState(() => templates.find((t) => t.id === "tpl-funcional")?.id ?? templates[0]?.id ?? "");
  const [errors, setErrors] = useState<FormErrors>({});
  const [busy, setBusy] = useState(false);

  async function submit(e: FormEvent) {
    e.preventDefault();
    setBusy(true);
    const result = await createProject({ meta, templateId });
    setBusy(false);
    if (result.ok) onClose();
    else setErrors(result.errors);
  }

  return (
    <Dialog title="Nueva matriz" kicker="Proyecto de pruebas" onClose={onClose}>
      <form onSubmit={submit} noValidate className="grid grid-cols-2 gap-4">
        <ProjectMetaFields idPrefix="np" meta={meta} errors={errors} onChange={setMeta} />
        <fieldset className="col-span-2">
          <legend className="mb-2 text-sm font-medium">Plantilla de columnas</legend>
          <div className="grid grid-cols-2 gap-2">
            {templates.map((t) => (
              <label
                key={t.id}
                className={`cursor-pointer rounded-lg border p-3 text-sm ${templateId === t.id ? "border-t4s-blue bg-t4s-blue/5" : "border-t4s-line"}`}
              >
                <input type="radio" name="template" value={t.id} checked={templateId === t.id} onChange={() => setTemplateId(t.id)} className="sr-only" />
                <span className="font-medium">{t.name}</span>
                <span className="block text-xs text-t4s-gray">
                  {t.fields.length} columnas{t.builtin ? " · incluida" : ""}
                </span>
              </label>
            ))}
          </div>
        </fieldset>
        {errors._form && (
          <p role="alert" className="col-span-2 text-sm text-red-700">
            {errors._form}
          </p>
        )}
        <DialogFooter>
          <button type="button" className={btn.secondary} onClick={onClose}>
            Cancelar
          </button>
          <button type="submit" className={btn.primary} disabled={busy || !templateId}>
            Crear matriz
          </button>
        </DialogFooter>
      </form>
    </Dialog>
  );
}
```

`src/ui/screens/TopBar.tsx`:

```tsx
import { Plus, Settings } from "lucide-react";
import mark from "../../assets/brand/qapture-mark.svg";
import { btn, inputCls } from "../components/styles";
import { useWorkspace } from "../store/context";

export function TopBar({ onNewProject, onSettings }: { onNewProject: () => void; onSettings?: () => void }) {
  const projects = useWorkspace((s) => s.projects);
  const project = useWorkspace((s) => s.project);
  const selectProject = useWorkspace((s) => s.selectProject);

  return (
    <header className="flex items-center gap-4 border-b border-t4s-line bg-white px-5 py-3">
      <div className="flex items-center gap-2">
        <img src={mark} alt="" className="h-8 w-8 rounded-lg" />
        <div className="leading-tight">
          <p className="font-semibold">Qapture</p>
          <p className="text-xs text-t4s-gray">Teams4Soft · Evidencias QA</p>
        </div>
      </div>
      {projects.length > 0 && (
        <label className="ml-6 flex items-center gap-2 text-sm">
          <span className="text-t4s-gray">Proyecto</span>
          <select aria-label="Proyecto" className={`${inputCls} w-72`} value={project?.id ?? ""} onChange={(e) => void selectProject(e.target.value)}>
            {projects.map((p) => (
              <option key={p.id} value={p.id}>
                {p.client ? `${p.client} · ${p.name}` : p.name}
              </option>
            ))}
          </select>
        </label>
      )}
      <div className="ml-auto flex gap-2">
        {onSettings && project && (
          <button type="button" className={btn.secondary} onClick={onSettings}>
            <Settings size={16} /> Configurar
          </button>
        )}
        <button type="button" className={btn.primary} onClick={onNewProject}>
          <Plus size={16} /> Nueva matriz
        </button>
      </div>
    </header>
  );
}
```

`src/ui/screens/EmptyState.tsx`:

```tsx
import { Plus } from "lucide-react";
import logo from "../../assets/brand/t4s-logo.png";
import { btn } from "../components/styles";

export function EmptyState({ onCreate }: { onCreate: () => void }) {
  return (
    <main className="grid flex-1 place-items-center p-8">
      <div className="max-w-md text-center">
        <img src={logo} alt="Teams4Soft" className="mx-auto mb-6 h-28" />
        <h1 className="text-2xl font-semibold">Crea tu primera matriz</h1>
        <p className="mt-2 text-t4s-gray">
          Una matriz agrupa los casos de prueba de un proyecto o módulo. Elige una plantilla de columnas y empieza a registrar ejecuciones.
        </p>
        <button type="button" className={`${btn.primary} mt-6`} onClick={onCreate}>
          <Plus size={16} /> Nueva matriz
        </button>
      </div>
    </main>
  );
}
```

`src/ui/screens/Workspace.tsx` (provisional; Task 13 lo reemplaza):

```tsx
import { useWorkspace } from "../store/context";

export function Workspace() {
  const project = useWorkspace((s) => s.project);
  return <main className="flex-1 p-6">{project?.name}</main>;
}
```

`src/ui/screens/AppShell.tsx`:

```tsx
import { useEffect, useState } from "react";
import { Toasts } from "../components/Toasts";
import { useWorkspace } from "../store/context";
import { EmptyState } from "./EmptyState";
import { NewProjectDialog } from "./NewProjectDialog";
import { TopBar } from "./TopBar";
import { Workspace } from "./Workspace";

export function AppShell() {
  const init = useWorkspace((s) => s.init);
  const ready = useWorkspace((s) => s.ready);
  const project = useWorkspace((s) => s.project);
  const [dialog, setDialog] = useState<"new-project" | null>(null);

  useEffect(() => {
    void init();
  }, [init]);

  if (!ready) return <div className="grid h-full place-items-center text-t4s-gray">Cargando Qapture…</div>;

  return (
    <div data-testid="app-ready" className="flex h-full flex-col">
      <TopBar onNewProject={() => setDialog("new-project")} />
      {project ? <Workspace /> : <EmptyState onCreate={() => setDialog("new-project")} />}
      {dialog === "new-project" && <NewProjectDialog onClose={() => setDialog(null)} />}
      <Toasts />
    </div>
  );
}
```

`src/App.tsx` (reemplazar completo):

```tsx
import type { StoreApi } from "zustand/vanilla";
import { AppShell } from "./ui/screens/AppShell";
import { WorkspaceProvider } from "./ui/store/context";
import type { WorkspaceState } from "./ui/store/workspace";

export default function App({ store }: { store: StoreApi<WorkspaceState> }) {
  return (
    <WorkspaceProvider store={store}>
      <AppShell />
    </WorkspaceProvider>
  );
}
```

`src/main.tsx` (reemplazar completo):

```tsx
import "./ui/theme/tokens.css";
import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { tauriRepositories } from "./infrastructure/tauri/repositories";
import { createWorkspaceStore } from "./ui/store/workspace";

const store = createWorkspaceStore(tauriRepositories());

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App store={store} />
  </React.StrictMode>,
);
```

- [ ] **Step 4: Ejecutar los tests**

Run: `pnpm test && pnpm typecheck`
Expected: todos PASS.

- [ ] **Step 5: Probar en la app real**

Run: `pnpm tauri dev`
Expected: ventana "Qapture · Teams4Soft" con el estado vacío; crear "Recibos" con la plantilla Funcional la deja seleccionada. Cierra y vuelve a abrir: el proyecto persiste.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat(f1): shell de la app, selector de proyecto y nueva matriz"
```

---

### Task 13: UI — lista de casos, filtros, detalle, formulario dinámico, duplicar y eliminar

**Files:**
- Create: `src/ui/components/StatusBadge.tsx`, `src/ui/components/ConfirmDialog.tsx`, `src/ui/components/FieldControl.tsx`, `src/ui/screens/CaseListPane.tsx`, `src/ui/screens/CaseDetail.tsx`, `src/ui/screens/CaseFormDialog.tsx`
- Modify: `src/ui/screens/Workspace.tsx` (reemplazar completo)
- Test: `src/ui/screens/Cases.test.tsx`

**Interfaces:**
- Produces: `StatusBadge({ status })`, `ConfirmDialog({ title, message, confirmLabel, requireText?, onCancel, onConfirm })`, `FieldControl({ id, field, value, onChange })`, `displayValue(field, value)`, `CaseFormDialog({ caseId?, onClose })`.

- [ ] **Step 1: Escribir los tests (fallan)**

`src/ui/screens/Cases.test.tsx`:

```tsx
import { screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { renderApp, seedProject } from "../test-utils";

async function openWithProject() {
  return renderApp({ setup: async (repos) => void (await seedProject(repos)) });
}

async function createCase(user: Awaited<ReturnType<typeof renderApp>>["user"], title: string, entorno: string, extra: Record<string, string> = {}) {
  await user.click(screen.getByRole("button", { name: /Nuevo caso/ }));
  const dialog = screen.getByRole("dialog", { name: "Nuevo caso" });
  await user.type(within(dialog).getByLabelText(/Caso de prueba/), title);
  await user.type(within(dialog).getByLabelText(/^Entorno/), entorno);
  for (const [label, value] of Object.entries(extra)) await user.type(within(dialog).getByLabelText(label), value);
  await user.click(within(dialog).getByRole("button", { name: "Guardar caso" }));
}

describe("casos", () => {
  it("exige los obligatorios y crea EJ-001", async () => {
    const { user } = await openWithProject();
    await user.click(screen.getByRole("button", { name: /Nuevo caso/ }));
    const dialog = screen.getByRole("dialog", { name: "Nuevo caso" });
    await user.click(within(dialog).getByRole("button", { name: "Guardar caso" }));
    expect(within(dialog).getByText("«Entorno» es obligatorio.")).toBeInTheDocument();
    expect(within(dialog).getByText("«Caso de prueba» es obligatorio.")).toBeInTheDocument();
    await user.click(within(dialog).getByRole("button", { name: "Cancelar" }));

    await createCase(user, "Aplicación de recibos", "Máquina Virtual", { "Datos de entrada": "Recibo 1{Enter}Sucursal 2" });
    const list = screen.getByRole("list", { name: "Lista de casos" });
    expect(within(list).getByText("EJ-001")).toBeInTheDocument();
    const detail = screen.getByRole("region", { name: "Detalle del caso" });
    expect(within(detail).getByRole("heading", { name: "Aplicación de recibos" })).toBeInTheDocument();
    expect(within(detail).getByText("Recibo 1\nSucursal 2", { normalizer: (s) => s })).toBeInTheDocument();
  });

  it("duplica limpiando resultados y edita", async () => {
    const { user } = await openWithProject();
    await createCase(user, "Recibos", "QA", { "Resultado obtenido": "OK", Observaciones: "nada" });
    await user.click(screen.getByRole("button", { name: /Duplicar/ }));

    const detail = screen.getByRole("region", { name: "Detalle del caso" });
    expect(within(detail).getByText("EJ-002")).toBeInTheDocument();
    expect(within(detail).getByText("No ejecutado")).toBeInTheDocument();
    expect(within(detail).queryByText("OK")).not.toBeInTheDocument();
    expect(within(detail).getByText("QA")).toBeInTheDocument();

    await user.click(within(detail).getByRole("button", { name: /Editar/ }));
    const dialog = screen.getByRole("dialog", { name: "Editar EJ-002" });
    await user.selectOptions(within(dialog).getByLabelText("Estado"), "Fallido");
    await user.click(within(dialog).getByRole("button", { name: "Guardar caso" }));
    expect(within(screen.getByRole("region", { name: "Detalle del caso" })).getByText("Fallido")).toBeInTheDocument();
  });

  it("busca sin acentos y filtra por estado", async () => {
    const { user } = await openWithProject();
    await createCase(user, "Aplicación de recibos", "QA");
    await createCase(user, "Emisión de pólizas", "Producción");
    const list = screen.getByRole("list", { name: "Lista de casos" });

    await user.type(screen.getByRole("searchbox", { name: "Buscar casos" }), "RECÍB");
    expect(within(list).getAllByRole("listitem")).toHaveLength(1);
    expect(screen.getByTestId("case-count")).toHaveTextContent("1 / 2");

    await user.clear(screen.getByRole("searchbox", { name: "Buscar casos" }));
    await user.selectOptions(screen.getByRole("combobox", { name: "Filtrar por estado" }), "Aprobado");
    expect(within(list).getByText("Ningún caso coincide con el filtro.")).toBeInTheDocument();
  });

  it("elimina con confirmación y no reutiliza el código", async () => {
    const { user } = await openWithProject();
    await createCase(user, "Uno", "QA");
    await createCase(user, "Dos", "QA");
    await user.click(screen.getByRole("button", { name: "Eliminar caso" }));
    await user.click(within(screen.getByRole("dialog", { name: "Eliminar EJ-002" })).getByRole("button", { name: "Eliminar caso" }));
    await createCase(user, "Tres", "QA");
    expect(within(screen.getByRole("list", { name: "Lista de casos" })).getByText("EJ-003")).toBeInTheDocument();
  });
});
```

Run: `pnpm test src/ui/screens/Cases`
Expected: FAIL.

- [ ] **Step 2: Componentes**

`src/ui/components/StatusBadge.tsx`:

```tsx
import type { Status } from "../../domain/status";

const TONES: Record<Status, string> = {
  "No ejecutado": "bg-t4s-cream text-t4s-gray",
  "En ejecución": "bg-t4s-blue/10 text-t4s-blue",
  "Por documentar": "bg-amber-50 text-amber-800",
  Aprobado: "bg-t4s-lime/25 text-t4s-charcoal",
  Fallido: "bg-red-50 text-red-800",
  Bloqueado: "bg-yellow-100 text-yellow-900",
};

export function StatusBadge({ status }: { status: Status }) {
  return <span className={`inline-block rounded-full px-2 py-0.5 text-xs font-medium ${TONES[status]}`}>{status}</span>;
}
```

`src/ui/components/ConfirmDialog.tsx`:

```tsx
import { useState } from "react";
import { Dialog, DialogFooter } from "./Dialog";
import { FormField } from "./FormField";
import { btn, inputCls } from "./styles";

export function ConfirmDialog({ title, message, confirmLabel, requireText, onCancel, onConfirm }: {
  title: string;
  message: string;
  confirmLabel: string;
  requireText?: string;
  onCancel: () => void;
  onConfirm: () => void | Promise<void>;
}) {
  const [typed, setTyped] = useState("");
  const allowed = !requireText || typed.trim() === requireText;
  return (
    <Dialog title={title} onClose={onCancel}>
      <p className="text-sm">{message}</p>
      {requireText && (
        <FormField label={`Escribe «${requireText}» para confirmar`} htmlFor="confirm-text" className="mt-4">
          <input id="confirm-text" className={inputCls} value={typed} onChange={(e) => setTyped(e.target.value)} />
        </FormField>
      )}
      <DialogFooter>
        <button type="button" className={btn.secondary} onClick={onCancel}>
          Cancelar
        </button>
        <button type="button" className={btn.danger} disabled={!allowed} onClick={() => void onConfirm()}>
          {confirmLabel}
        </button>
      </DialogFooter>
    </Dialog>
  );
}
```

`src/ui/components/FieldControl.tsx`:

```tsx
import type { FieldDef } from "../../domain/fields";
import { inputCls } from "./styles";

export function FieldControl({ id, field, value, onChange }: {
  id: string;
  field: FieldDef;
  value: string;
  onChange: (value: string) => void;
}) {
  const common = { id, className: inputCls, value };
  switch (field.type) {
    case "longtext":
      return <textarea {...common} rows={4} onChange={(e) => onChange(e.target.value)} />;
    case "list":
      return (
        <select {...common} onChange={(e) => onChange(e.target.value)}>
          <option value="">—</option>
          {field.options.map((o) => (
            <option key={o} value={o}>
              {o}
            </option>
          ))}
        </select>
      );
    case "boolean":
      return (
        <select {...common} onChange={(e) => onChange(e.target.value)}>
          <option value="">—</option>
          <option value="true">Sí</option>
          <option value="false">No</option>
        </select>
      );
    case "date":
      return <input {...common} type="date" onChange={(e) => onChange(e.target.value)} />;
    case "number":
      return <input {...common} inputMode="decimal" onChange={(e) => onChange(e.target.value)} />;
    default:
      return <input {...common} type="text" onChange={(e) => onChange(e.target.value)} />;
  }
}

export function displayValue(field: FieldDef, value: string): string {
  if (!value) return "—";
  if (field.type === "boolean") return value === "true" ? "Sí" : "No";
  if (field.type === "date") {
    const [y, m, d] = value.split("-");
    return `${d}/${m}/${y}`;
  }
  return value;
}
```

- [ ] **Step 3: Pantallas de casos**

`src/ui/screens/CaseFormDialog.tsx`:

```tsx
import { useMemo, useState, type FormEvent } from "react";
import { caseToInput, emptyCaseInput, type CaseInput, type FormErrors } from "../../domain/cases";
import { activeFields } from "../../domain/fields";
import { STATUSES, type Status } from "../../domain/status";
import { Dialog, DialogFooter } from "../components/Dialog";
import { FieldControl } from "../components/FieldControl";
import { FormField } from "../components/FormField";
import { btn, inputCls } from "../components/styles";
import { useWorkspace } from "../store/context";

export function CaseFormDialog({ caseId, onClose }: { caseId?: string; onClose: () => void }) {
  const project = useWorkspace((s) => s.project);
  const cases = useWorkspace((s) => s.cases);
  const saveCase = useWorkspace((s) => s.saveCase);
  const existing = caseId ? cases.find((c) => c.id === caseId) : undefined;
  const projectFields = project?.fields ?? [];
  const fields = useMemo(() => activeFields(projectFields), [projectFields]);
  const [input, setInput] = useState<CaseInput>(() => (existing ? caseToInput(projectFields, existing) : emptyCaseInput(projectFields)));
  const [errors, setErrors] = useState<FormErrors>({});
  const [busy, setBusy] = useState(false);

  const setValue = (fieldId: string, value: string) => setInput((i) => ({ ...i, values: { ...i.values, [fieldId]: value } }));

  async function submit(e: FormEvent) {
    e.preventDefault();
    setBusy(true);
    const result = await saveCase(input, caseId);
    setBusy(false);
    if (result.ok) onClose();
    else setErrors(result.errors);
  }

  return (
    <Dialog wide title={existing ? `Editar ${existing.code}` : "Nuevo caso"} kicker="Registro de ejecución" onClose={onClose}>
      <form onSubmit={submit} noValidate className="grid grid-cols-2 gap-4">
        <p className="col-span-2 text-sm text-t4s-gray">Usa una ejecución diferente para cada entorno o repetición de la prueba.</p>
        <FormField label="Caso de prueba" required htmlFor="case-title" error={errors.title} className="col-span-2">
          <input id="case-title" className={inputCls} value={input.title} onChange={(e) => setInput((i) => ({ ...i, title: e.target.value }))} />
        </FormField>
        <FormField label="Estado" htmlFor="case-status" error={errors.status}>
          <select id="case-status" className={inputCls} value={input.status} onChange={(e) => setInput((i) => ({ ...i, status: e.target.value as Status }))}>
            {STATUSES.map((s) => (
              <option key={s}>{s}</option>
            ))}
          </select>
        </FormField>
        {fields.map((f) => (
          <FormField key={f.id} label={f.label} required={f.required} htmlFor={`f-${f.id}`} error={errors[f.id]} className={f.type === "longtext" ? "col-span-2" : ""}>
            <FieldControl id={`f-${f.id}`} field={f} value={input.values[f.id] ?? ""} onChange={(v) => setValue(f.id, v)} />
          </FormField>
        ))}
        {errors._form && (
          <p role="alert" className="col-span-2 text-sm text-red-700">
            {errors._form}
          </p>
        )}
        <DialogFooter>
          <button type="button" className={btn.secondary} onClick={onClose}>
            Cancelar
          </button>
          <button type="submit" className={btn.primary} disabled={busy}>
            Guardar caso
          </button>
        </DialogFooter>
      </form>
    </Dialog>
  );
}
```

`src/ui/screens/CaseListPane.tsx`:

```tsx
import { Plus, Search } from "lucide-react";
import { useMemo } from "react";
import { filterCases } from "../../domain/cases";
import { STATUSES, type Status } from "../../domain/status";
import { StatusBadge } from "../components/StatusBadge";
import { btn, inputCls } from "../components/styles";
import { useWorkspace } from "../store/context";

export function CaseListPane({ onNewCase }: { onNewCase: () => void }) {
  const project = useWorkspace((s) => s.project);
  const cases = useWorkspace((s) => s.cases);
  const search = useWorkspace((s) => s.search);
  const status = useWorkspace((s) => s.statusFilter);
  const selectedId = useWorkspace((s) => s.selectedCaseId);
  const setSearch = useWorkspace((s) => s.setSearch);
  const setStatusFilter = useWorkspace((s) => s.setStatusFilter);
  const selectCase = useWorkspace((s) => s.selectCase);
  const visible = useMemo(() => filterCases(cases, { search, status }), [cases, search, status]);
  const envField = project?.fields.find((f) => f.key === "entorno" && !f.archived);

  return (
    <aside className="flex w-80 shrink-0 flex-col border-r border-t4s-line bg-white">
      <div className="space-y-3 p-4">
        <div className="flex items-center justify-between">
          <h1 className="font-semibold">Casos de prueba</h1>
          <span data-testid="case-count" className="rounded-full bg-t4s-cream px-2 text-xs">
            {visible.length === cases.length ? cases.length : `${visible.length} / ${cases.length}`}
          </span>
        </div>
        <button type="button" className={`${btn.primary} w-full`} onClick={onNewCase}>
          <Plus size={16} /> Nuevo caso
        </button>
        <label className="relative block">
          <Search size={14} className="absolute left-3 top-1/2 -translate-y-1/2 text-t4s-gray" />
          <input
            type="search"
            aria-label="Buscar casos"
            placeholder="Buscar código, caso o dato…"
            className={`${inputCls} pl-8`}
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
        </label>
        <select aria-label="Filtrar por estado" className={inputCls} value={status} onChange={(e) => setStatusFilter(e.target.value as Status | "")}>
          <option value="">Todos los estados</option>
          {STATUSES.map((s) => (
            <option key={s}>{s}</option>
          ))}
        </select>
      </div>
      <ul aria-label="Lista de casos" className="flex-1 overflow-y-auto px-2 pb-4">
        {visible.map((c) => (
          <li key={c.id}>
            <button
              type="button"
              onClick={() => selectCase(c.id)}
              aria-current={c.id === selectedId}
              className={`w-full rounded-lg px-3 py-2 text-left text-sm ${c.id === selectedId ? "bg-t4s-blue/10" : "hover:bg-t4s-cream"}`}
            >
              <span className="flex items-center justify-between">
                <span className="font-mono text-xs text-t4s-blue">{c.code}</span>
                <StatusBadge status={c.status} />
              </span>
              <span className="mt-0.5 block truncate font-medium">{c.title}</span>
              {envField && c.values[envField.id] && <span className="block truncate text-xs text-t4s-gray">{c.values[envField.id]}</span>}
            </button>
          </li>
        ))}
        {visible.length === 0 && (
          <li className="px-3 py-6 text-center text-sm text-t4s-gray">
            {cases.length ? "Ningún caso coincide con el filtro." : "Aún no hay casos. Crea el primero."}
          </li>
        )}
      </ul>
    </aside>
  );
}
```

`src/ui/screens/CaseDetail.tsx`:

```tsx
import { Copy, Pencil, Trash2 } from "lucide-react";
import { useState } from "react";
import { activeFields } from "../../domain/fields";
import { ConfirmDialog } from "../components/ConfirmDialog";
import { displayValue } from "../components/FieldControl";
import { StatusBadge } from "../components/StatusBadge";
import { btn } from "../components/styles";
import { useWorkspace } from "../store/context";

export function CaseDetail({ onEdit }: { onEdit: (caseId: string) => void }) {
  const project = useWorkspace((s) => s.project);
  const cases = useWorkspace((s) => s.cases);
  const selectedId = useWorkspace((s) => s.selectedCaseId);
  const duplicateCase = useWorkspace((s) => s.duplicateCase);
  const deleteCase = useWorkspace((s) => s.deleteCase);
  const [confirming, setConfirming] = useState(false);
  const current = cases.find((c) => c.id === selectedId);

  if (!project) return null;
  if (!current) {
    return (
      <section aria-label="Detalle del caso" className="grid flex-1 place-items-center text-t4s-gray">
        Selecciona un caso para ver su detalle.
      </section>
    );
  }

  return (
    <section aria-label="Detalle del caso" className="flex-1 overflow-y-auto p-6">
      <div className="mx-auto max-w-4xl">
        <div className="flex items-start justify-between gap-4">
          <div>
            <p className="font-mono text-sm text-t4s-blue">{current.code}</p>
            <h2 className="text-2xl font-semibold">{current.title}</h2>
            <div className="mt-2">
              <StatusBadge status={current.status} />
            </div>
          </div>
          <div className="flex gap-2">
            <button type="button" className={btn.secondary} onClick={() => onEdit(current.id)}>
              <Pencil size={16} /> Editar
            </button>
            <button type="button" className={btn.secondary} onClick={() => void duplicateCase(current.id)}>
              <Copy size={16} /> Duplicar
            </button>
            <button type="button" aria-label="Eliminar caso" className={btn.ghost} onClick={() => setConfirming(true)}>
              <Trash2 size={16} />
            </button>
          </div>
        </div>
        <dl className="mt-6 grid grid-cols-2 gap-x-6 gap-y-4 rounded-xl border border-t4s-line bg-white p-5">
          {activeFields(project.fields).map((f) => (
            <div key={f.id} className={f.type === "longtext" ? "col-span-2" : ""}>
              <dt className="text-xs font-medium uppercase tracking-wide text-t4s-gray">{f.label}</dt>
              <dd className="mt-1 whitespace-pre-wrap text-sm">{displayValue(f, current.values[f.id] ?? "")}</dd>
            </div>
          ))}
        </dl>
        <div className="mt-6 rounded-xl border border-dashed border-t4s-line p-6 text-center text-sm text-t4s-gray">
          Las evidencias (capturas y anotaciones) llegan en la fase F2.
        </div>
      </div>
      {confirming && (
        <ConfirmDialog
          title={`Eliminar ${current.code}`}
          message="El caso se eliminará de forma permanente. Su código no se reutilizará."
          confirmLabel="Eliminar caso"
          onCancel={() => setConfirming(false)}
          onConfirm={async () => {
            setConfirming(false);
            await deleteCase(current.id);
          }}
        />
      )}
    </section>
  );
}
```

`src/ui/screens/Workspace.tsx` (reemplazar completo):

```tsx
import { useState } from "react";
import { CaseDetail } from "./CaseDetail";
import { CaseFormDialog } from "./CaseFormDialog";
import { CaseListPane } from "./CaseListPane";

export function Workspace() {
  const [editing, setEditing] = useState<{ caseId?: string } | null>(null);
  return (
    <main className="flex min-h-0 flex-1">
      <CaseListPane onNewCase={() => setEditing({})} />
      <CaseDetail onEdit={(caseId) => setEditing({ caseId })} />
      {editing && <CaseFormDialog caseId={editing.caseId} onClose={() => setEditing(null)} />}
    </main>
  );
}
```

- [ ] **Step 4: Ejecutar los tests**

Run: `pnpm test && pnpm typecheck`
Expected: todos PASS.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(f1): lista, detalle y formulario dinámico de casos"
```

---

### Task 14: UI — configuración del proyecto (datos, columnas y plantillas)

**Files:**
- Create: `src/ui/screens/ProjectSettingsDialog.tsx`, `src/ui/screens/ColumnEditor.tsx`, `src/ui/screens/TemplatesPanel.tsx`
- Modify: `src/ui/screens/AppShell.tsx` (reemplazar completo)
- Test: `src/ui/screens/ProjectSettings.test.tsx`

**Interfaces:**
- Consumes: `updateProjectMeta`, `saveFields`, `deleteProject`, `saveTemplateFromProject` y `deleteTemplate` del store; `fieldToInput`, `newFieldInput`, `projectToMeta`, `FIELD_TYPES` y `FIELD_TYPE_LABELS`.
- Produces: `ProjectSettingsDialog({ onClose })` con pestañas "Datos", "Columnas" y "Plantillas".

- [ ] **Step 1: Escribir los tests (fallan)**

`src/ui/screens/ProjectSettings.test.tsx`:

```tsx
import { screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { renderApp, seedProject } from "../test-utils";

async function openSettings(options: Parameters<typeof renderApp>[0] = {}) {
  const ctx = await renderApp({ setup: options.setup ?? (async (repos) => void (await seedProject(repos))) });
  await ctx.user.click(screen.getByRole("button", { name: /Configurar/ }));
  return { ...ctx, dialog: screen.getByRole("dialog", { name: "Configurar proyecto" }) };
}

describe("configuración del proyecto", () => {
  it("edita los datos del proyecto", async () => {
    const { user, dialog } = await openSettings();
    const name = within(dialog).getByLabelText(/Nombre del proyecto/);
    await user.clear(name);
    await user.type(name, "Recibos v2");
    await user.click(within(dialog).getByRole("button", { name: "Guardar datos" }));
    expect(screen.getByRole("combobox", { name: "Proyecto" })).toHaveDisplayValue("ACME · Recibos v2");
  });

  it("agrega una columna de lista y aparece en el formulario de casos", async () => {
    const { user, dialog } = await openSettings();
    await user.click(within(dialog).getByRole("tab", { name: "Columnas" }));
    await user.click(within(dialog).getByRole("button", { name: /Agregar columna/ }));
    await user.type(within(dialog).getByLabelText("Nombre de la columna 12"), "Severidad");
    await user.selectOptions(within(dialog).getByLabelText("Tipo de Severidad"), "list");
    await user.type(within(dialog).getByLabelText("Opciones de Severidad"), "Alta{Enter}Media{Enter}Baja");
    await user.click(within(dialog).getByRole("button", { name: "Guardar columnas" }));
    await user.click(within(dialog).getByRole("button", { name: "Cerrar" }));

    await user.click(screen.getByRole("button", { name: /Nuevo caso/ }));
    const form = screen.getByRole("dialog", { name: "Nuevo caso" });
    const severidad = within(form).getByLabelText("Severidad");
    expect(within(severidad).getAllByRole("option").map((o) => o.textContent)).toEqual(["—", "Alta", "Media", "Baja"]);
  });

  it("muestra errores de columnas y no guarda", async () => {
    const { user, dialog } = await openSettings();
    await user.click(within(dialog).getByRole("tab", { name: "Columnas" }));
    await user.click(within(dialog).getByRole("button", { name: /Agregar columna/ }));
    await user.type(within(dialog).getByLabelText("Nombre de la columna 12"), "entorno");
    await user.click(within(dialog).getByRole("button", { name: "Guardar columnas" }));
    expect(within(dialog).getByText("La columna «entorno» está repetida.")).toBeInTheDocument();
  });

  it("archiva una columna con datos y la restaura", async () => {
    const { user, dialog } = await openSettings({
      setup: async (repos) => {
        const p = await seedProject(repos);
        const id = (key: string) => p.fields.find((f) => f.key === key)!.id;
        await repos.cases.create(p.id, { title: "Caso", status: "Aprobado", values: { [id("entorno")]: "QA", [id("modulo")]: "Cobranzas" } });
      },
    });
    await user.click(within(dialog).getByRole("tab", { name: "Columnas" }));
    await user.click(within(dialog).getByRole("button", { name: "Quitar Módulo" }));
    await user.click(within(dialog).getByRole("button", { name: "Quitar Menú" }));
    await user.click(within(dialog).getByRole("button", { name: "Guardar columnas" }));

    const archived = within(dialog).getByRole("region", { name: "Columnas archivadas" });
    expect(within(archived).getByText("Módulo")).toBeInTheDocument();
    expect(within(archived).queryByText("Menú")).not.toBeInTheDocument();

    await user.click(within(archived).getByRole("button", { name: "Restaurar Módulo" }));
    await user.click(within(dialog).getByRole("button", { name: "Guardar columnas" }));
    await user.click(within(dialog).getByRole("button", { name: "Cerrar" }));
    expect(within(screen.getByRole("region", { name: "Detalle del caso" })).getByText("Cobranzas")).toBeInTheDocument();
  });

  it("guarda las columnas como plantilla y elimina plantillas propias", async () => {
    const { user, dialog } = await openSettings();
    await user.click(within(dialog).getByRole("tab", { name: "Plantillas" }));
    await user.type(within(dialog).getByLabelText(/Nombre de la plantilla/), "GMX Cobranzas");
    await user.click(within(dialog).getByRole("button", { name: "Guardar como plantilla" }));
    expect(within(dialog).getByText("GMX Cobranzas")).toBeInTheDocument();
    expect(within(dialog).queryByRole("button", { name: "Eliminar plantilla Funcional" })).not.toBeInTheDocument();
    await user.click(within(dialog).getByRole("button", { name: "Eliminar plantilla GMX Cobranzas" }));
    expect(within(dialog).queryByText("GMX Cobranzas")).not.toBeInTheDocument();
  });

  it("elimina el proyecto solo tras escribir su nombre", async () => {
    const { user, dialog } = await openSettings();
    await user.click(within(dialog).getByRole("button", { name: "Eliminar proyecto" }));
    const confirm = screen.getByRole("dialog", { name: "Eliminar Recibos" });
    const action = within(confirm).getByRole("button", { name: "Eliminar proyecto" });
    expect(action).toBeDisabled();
    await user.type(within(confirm).getByLabelText(/Escribe «Recibos»/), "Recibos");
    await user.click(action);
    expect(screen.getByRole("heading", { name: "Crea tu primera matriz" })).toBeInTheDocument();
  });
});
```

Run: `pnpm test src/ui/screens/ProjectSettings`
Expected: FAIL.

- [ ] **Step 2: Editor de columnas**

`src/ui/screens/ColumnEditor.tsx`:

```tsx
import { ArrowDown, ArrowUp, Plus, RotateCcw, Trash2 } from "lucide-react";
import { useEffect, useState } from "react";
import type { FormErrors } from "../../domain/cases";
import { FIELD_TYPES, FIELD_TYPE_LABELS, type FieldType } from "../../domain/fields";
import { fieldToInput, newFieldInput, type FieldInput } from "../../domain/project";
import { btn, iconButton, inputCls } from "../components/styles";
import { useWorkspace } from "../store/context";

export function ColumnEditor() {
  const fields = useWorkspace((s) => s.project?.fields);
  const saveFields = useWorkspace((s) => s.saveFields);
  const [rows, setRows] = useState<FieldInput[]>(() => (fields ?? []).map(fieldToInput));
  const [errors, setErrors] = useState<FormErrors>({});
  const [busy, setBusy] = useState(false);

  // Cuando el store recarga el proyecto tras guardar, las filas se sincronizan con las columnas
  // reales: ids nuevos, columnas archivadas por tener datos y columnas vacías eliminadas.
  useEffect(() => setRows((fields ?? []).map(fieldToInput)), [fields]);

  const active = rows.filter((r) => !r.archived);
  const archived = rows.filter((r) => r.archived);
  const update = (row: FieldInput, patch: Partial<FieldInput>) => setRows((all) => all.map((r) => (r === row ? { ...r, ...patch } : r)));
  const remove = (row: FieldInput) => setRows((all) => all.filter((r) => r !== row));
  const move = (row: FieldInput, delta: number) =>
    setRows((all) => {
      const list = all.filter((r) => !r.archived);
      const from = list.indexOf(row);
      const to = from + delta;
      if (to < 0 || to >= list.length) return all;
      [list[from], list[to]] = [list[to], list[from]];
      return [...list, ...all.filter((r) => r.archived)];
    });
  const restore = (row: FieldInput) => setRows((all) => [...all.filter((r) => !r.archived), { ...row, archived: false }, ...all.filter((r) => r.archived && r !== row)]);

  async function save() {
    setBusy(true);
    const result = await saveFields([...active, ...archived]);
    setBusy(false);
    setErrors(result.ok ? {} : result.errors);
  }

  const messages = Object.values(errors);

  return (
    <div className="space-y-4">
      {messages.length > 0 && (
        <ul role="alert" className="rounded-lg bg-red-50 p-3 text-sm text-red-800">
          {messages.map((m) => (
            <li key={m}>{m}</li>
          ))}
        </ul>
      )}
      <p className="text-sm text-t4s-gray">
        "Caso de prueba", "Estado" e "ID" son fijos. Si quitas una columna que ya tiene datos, se archiva y puedes restaurarla.
      </p>
      <ol className="space-y-2">
        {active.map((row, index) => {
          const name = row.label || `columna ${index + 1}`;
          return (
            <li key={row.id ?? `new-${index}`} className="grid grid-cols-[1fr_9rem_5rem_auto] items-start gap-2 rounded-lg border border-t4s-line p-3">
              <input aria-label={`Nombre de la columna ${index + 1}`} className={inputCls} value={row.label} onChange={(e) => update(row, { label: e.target.value })} />
              <select aria-label={`Tipo de ${name}`} className={inputCls} value={row.type} onChange={(e) => update(row, { type: e.target.value as FieldType })}>
                {FIELD_TYPES.map((t) => (
                  <option key={t} value={t}>
                    {FIELD_TYPE_LABELS[t]}
                  </option>
                ))}
              </select>
              <input
                aria-label={`Ancho de ${name}`}
                type="number"
                min={8}
                max={100}
                className={inputCls}
                value={row.width}
                onChange={(e) => update(row, { width: Number(e.target.value) })}
              />
              <div className="flex">
                <button type="button" aria-label={`Subir ${name}`} className={iconButton} disabled={index === 0} onClick={() => move(row, -1)}>
                  <ArrowUp size={16} />
                </button>
                <button type="button" aria-label={`Bajar ${name}`} className={iconButton} disabled={index === active.length - 1} onClick={() => move(row, 1)}>
                  <ArrowDown size={16} />
                </button>
                <button type="button" aria-label={`Quitar ${name}`} className={iconButton} onClick={() => remove(row)}>
                  <Trash2 size={16} />
                </button>
              </div>
              <div className="col-span-4 flex flex-wrap items-center gap-4 text-sm">
                <label className="flex items-center gap-1.5">
                  <input type="checkbox" checked={row.required} onChange={(e) => update(row, { required: e.target.checked })} /> Obligatorio
                </label>
                <label className="flex items-center gap-1.5">
                  <input type="checkbox" checked={row.clearOnDuplicate} onChange={(e) => update(row, { clearOnDuplicate: e.target.checked })} /> Limpiar al duplicar
                </label>
              </div>
              {row.type === "list" && (
                <textarea
                  aria-label={`Opciones de ${name}`}
                  placeholder="Una opción por línea"
                  rows={3}
                  className={`${inputCls} col-span-4`}
                  value={row.options.join("\n")}
                  onChange={(e) => update(row, { options: e.target.value.split("\n") })}
                />
              )}
            </li>
          );
        })}
      </ol>
      <button type="button" className={btn.secondary} onClick={() => setRows((all) => [...all.filter((r) => !r.archived), newFieldInput(), ...all.filter((r) => r.archived)])}>
        <Plus size={16} /> Agregar columna
      </button>
      {archived.length > 0 && (
        <section aria-label="Columnas archivadas" className="rounded-lg bg-t4s-cream p-3">
          <h3 className="mb-2 text-sm font-medium">Columnas archivadas</h3>
          <ul className="space-y-1">
            {archived.map((row) => (
              <li key={row.id ?? row.label} className="flex items-center justify-between text-sm">
                <span>{row.label}</span>
                <button type="button" aria-label={`Restaurar ${row.label}`} className={btn.ghost} onClick={() => restore(row)}>
                  <RotateCcw size={14} /> Restaurar
                </button>
              </li>
            ))}
          </ul>
        </section>
      )}
      <div className="flex justify-end border-t border-t4s-line pt-4">
        <button type="button" className={btn.primary} disabled={busy} onClick={() => void save()}>
          Guardar columnas
        </button>
      </div>
    </div>
  );
}
```

- [ ] **Step 3: Panel de plantillas y diálogo de configuración**

`src/ui/screens/TemplatesPanel.tsx`:

```tsx
import { Trash2 } from "lucide-react";
import { useState, type FormEvent } from "react";
import type { FormErrors } from "../../domain/cases";
import { FormField } from "../components/FormField";
import { btn, iconButton, inputCls } from "../components/styles";
import { useWorkspace } from "../store/context";

export function TemplatesPanel() {
  const templates = useWorkspace((s) => s.templates);
  const project = useWorkspace((s) => s.project);
  const saveTemplate = useWorkspace((s) => s.saveTemplateFromProject);
  const deleteTemplate = useWorkspace((s) => s.deleteTemplate);
  const [name, setName] = useState("");
  const [testType, setTestType] = useState(project?.testType ?? "");
  const [errors, setErrors] = useState<FormErrors>({});

  async function submit(e: FormEvent) {
    e.preventDefault();
    const result = await saveTemplate(name, testType);
    if (result.ok) {
      setName("");
      setErrors({});
    } else setErrors(result.errors);
  }

  return (
    <div className="space-y-6">
      <form onSubmit={submit} noValidate className="grid grid-cols-2 gap-4">
        <p className="col-span-2 text-sm text-t4s-gray">Guarda las columnas activas de este proyecto para reutilizarlas al crear nuevas matrices.</p>
        <FormField label="Nombre de la plantilla" required htmlFor="tpl-name" error={errors.name ?? errors._form}>
          <input id="tpl-name" className={inputCls} value={name} onChange={(e) => setName(e.target.value)} placeholder="Ej. GMX Cobranzas" />
        </FormField>
        <FormField label="Tipo de prueba" htmlFor="tpl-type">
          <input id="tpl-type" className={inputCls} value={testType} onChange={(e) => setTestType(e.target.value)} />
        </FormField>
        <div className="col-span-2 flex justify-end">
          <button type="submit" className={btn.primary}>
            Guardar como plantilla
          </button>
        </div>
      </form>
      <section>
        <h3 className="mb-2 text-sm font-medium">Plantillas disponibles</h3>
        <ul className="divide-y divide-t4s-line rounded-lg border border-t4s-line">
          {templates.map((t) => (
            <li key={t.id} className="flex items-center justify-between px-3 py-2 text-sm">
              <span>
                <span className="font-medium">{t.name}</span>
                <span className="ml-2 text-xs text-t4s-gray">
                  {t.fields.length} columnas{t.builtin ? " · incluida" : ""}
                </span>
              </span>
              {!t.builtin && (
                <button type="button" aria-label={`Eliminar plantilla ${t.name}`} className={iconButton} onClick={() => void deleteTemplate(t.id)}>
                  <Trash2 size={16} />
                </button>
              )}
            </li>
          ))}
        </ul>
      </section>
    </div>
  );
}
```

`src/ui/screens/ProjectSettingsDialog.tsx`:

```tsx
import { useState, type FormEvent } from "react";
import type { FormErrors } from "../../domain/cases";
import { projectToMeta, type ProjectMeta } from "../../domain/project";
import { ConfirmDialog } from "../components/ConfirmDialog";
import { Dialog } from "../components/Dialog";
import { btn } from "../components/styles";
import { useWorkspace } from "../store/context";
import { ColumnEditor } from "./ColumnEditor";
import { ProjectMetaFields } from "./ProjectMetaFields";
import { TemplatesPanel } from "./TemplatesPanel";

const TABS = ["Datos", "Columnas", "Plantillas"] as const;
type Tab = (typeof TABS)[number];

export function ProjectSettingsDialog({ onClose }: { onClose: () => void }) {
  const project = useWorkspace((s) => s.project);
  const updateMeta = useWorkspace((s) => s.updateProjectMeta);
  const deleteProject = useWorkspace((s) => s.deleteProject);
  const [tab, setTab] = useState<Tab>("Datos");
  const [meta, setMeta] = useState<ProjectMeta>(() => projectToMeta(project!));
  const [errors, setErrors] = useState<FormErrors>({});
  const [confirming, setConfirming] = useState(false);

  if (!project) return null;

  async function submit(e: FormEvent) {
    e.preventDefault();
    const result = await updateMeta(meta);
    setErrors(result.ok ? {} : result.errors);
  }

  return (
    <Dialog wide title="Configurar proyecto" kicker={project.name} onClose={onClose}>
      <div role="tablist" className="mb-4 flex gap-1 border-b border-t4s-line">
        {TABS.map((t) => (
          <button
            key={t}
            type="button"
            role="tab"
            aria-selected={tab === t}
            onClick={() => setTab(t)}
            className={`-mb-px border-b-2 px-3 py-2 text-sm ${tab === t ? "border-t4s-blue font-medium text-t4s-blue" : "border-transparent text-t4s-gray"}`}
          >
            {t}
          </button>
        ))}
      </div>
      {tab === "Datos" && (
        <form onSubmit={submit} noValidate className="grid grid-cols-2 gap-4">
          <ProjectMetaFields idPrefix="ps" meta={meta} errors={errors} onChange={setMeta} />
          {errors._form && (
            <p role="alert" className="col-span-2 text-sm text-red-700">
              {errors._form}
            </p>
          )}
          <div className="col-span-2 flex justify-between border-t border-t4s-line pt-4">
            <button type="button" className={btn.ghost} onClick={() => setConfirming(true)}>
              Eliminar proyecto
            </button>
            <button type="submit" className={btn.primary}>
              Guardar datos
            </button>
          </div>
        </form>
      )}
      {tab === "Columnas" && <ColumnEditor />}
      {tab === "Plantillas" && <TemplatesPanel />}
      {confirming && (
        <ConfirmDialog
          title={`Eliminar ${project.name}`}
          message="Se eliminarán el proyecto y todos sus casos. Esta acción no se puede deshacer."
          confirmLabel="Eliminar proyecto"
          requireText={project.name}
          onCancel={() => setConfirming(false)}
          onConfirm={async () => {
            setConfirming(false);
            onClose();
            await deleteProject();
          }}
        />
      )}
    </Dialog>
  );
}
```

`src/ui/screens/AppShell.tsx` (reemplazar completo):

```tsx
import { useEffect, useState } from "react";
import { Toasts } from "../components/Toasts";
import { useWorkspace } from "../store/context";
import { EmptyState } from "./EmptyState";
import { NewProjectDialog } from "./NewProjectDialog";
import { ProjectSettingsDialog } from "./ProjectSettingsDialog";
import { TopBar } from "./TopBar";
import { Workspace } from "./Workspace";

export function AppShell() {
  const init = useWorkspace((s) => s.init);
  const ready = useWorkspace((s) => s.ready);
  const project = useWorkspace((s) => s.project);
  const [dialog, setDialog] = useState<"new-project" | "settings" | null>(null);

  useEffect(() => {
    void init();
  }, [init]);

  if (!ready) return <div className="grid h-full place-items-center text-t4s-gray">Cargando Qapture…</div>;

  return (
    <div data-testid="app-ready" className="flex h-full flex-col">
      <TopBar onNewProject={() => setDialog("new-project")} onSettings={() => setDialog("settings")} />
      {project ? <Workspace /> : <EmptyState onCreate={() => setDialog("new-project")} />}
      {dialog === "new-project" && <NewProjectDialog onClose={() => setDialog(null)} />}
      {dialog === "settings" && project && <ProjectSettingsDialog onClose={() => setDialog(null)} />}
      <Toasts />
    </div>
  );
}
```

- [ ] **Step 4: Ejecutar los tests**

Run: `pnpm test && pnpm typecheck`
Expected: todos PASS.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(f1): configuración del proyecto con editor de columnas y plantillas"
```

---

### Task 15: Verificación integral, build local y cierre del contrato

**Files:**
- Modify: `docs/superpowers/contracts/F1-dominio-datos-crud.md` (marcar los criterios automáticos), `docs/superpowers/contracts/README.md` (estado), `README.md` (sección "Probar un build")

- [ ] **Step 1: Verificación completa**

```bash
cd src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test && cd ..
pnpm typecheck && pnpm test
git diff --exit-code src/infrastructure/tauri/__fixtures__/contracts.json
```

Expected: todo en verde; el fixture de contrato no cambia.

- [ ] **Step 2: Prueba manual en `pnpm tauri dev`**

Recorre las filas 2–19 del checklist de `docs/superpowers/contracts/F1-dominio-datos-crud.md`. Cualquier fallo se corrige con un test que lo reproduzca antes del arreglo (superpowers:systematic-debugging).

- [ ] **Step 3: Build del instalador**

Run: `pnpm tauri build`
Expected: `src-tauri/target/release/bundle/nsis/Qapture_0.1.0_x64-setup.exe` y el `.msi`. Instala el `.exe` en una cuenta limpia y verifica la fila 1 del checklist (ícono propio y estado vacío).

- [ ] **Step 4: Actualizar los documentos**

- En `F1-dominio-datos-crud.md`, marca `[x]` los cuatro criterios automáticos.
- En `contracts/README.md`, cambia F1 a `🧪 En pruebas del equipo`.
- En `README.md`, agrega debajo de "Desarrollo":

```markdown
## Probar un build

El equipo prueba cada fase con el instalador local siguiendo el checklist del contrato correspondiente en
[`docs/superpowers/contracts/`](docs/superpowers/contracts/README.md).
```

- [ ] **Step 5: Commit y entrega para revisión**

```bash
git add -A
git commit -m "docs(f1): cierre técnico de la fase y build local"
git push -u origin feat/f1-dominio-datos-crud
```

Después usa superpowers:finishing-a-development-branch para abrir el PR hacia `main`. El merge y el tag `v0.1.0` se hacen **solo** cuando el equipo firme el checklist del contrato F1.
