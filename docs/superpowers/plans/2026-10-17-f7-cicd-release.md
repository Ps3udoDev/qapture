# F7 — CI/CD, auto-update, release y documentación · Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Que publicar sea automático y seguro: cada PR pasa por un CI completo, un tag `vX.Y.Z` produce un release en borrador con el instalador NSIS, el MSI, el ZIP portable y la actualización firmada (`latest.json` + `.sig`), y las apps instaladas se actualizan solas. Además, documentación de usuario y de desarrollo para la `v1.0.0`.

**Architecture:**
- **Workflows:** se completan los dos que ya existen (ver *Punto de partida*). `ci.yml` suma ESLint, un build de Tauri sin instaladores, la verificación del fixture de contrato y la de versión. `release.yml` publica en borrador, firma la actualización, arma el ZIP portable y usa el CHANGELOG como notas.
- **Firma del updater:** solo se activa en el build de release, con `src-tauri/tauri.release.conf.json`, así que `pnpm tauri build` local sigue funcionando sin la clave privada.
- **Modo portable:** se decide en Rust, en un único punto (`system/portable.rs`). Si junto al exe hay un `portable.flag`, todo el directorio de datos pasa a `.\data`.
- **Updater:** vive en Rust (`system/updater.rs`) y se usa solo desde Rust, igual que los plugins de F6:
  - comandos propios y eventos `update:available` / `update:progress`;
  - revisión 15 s después de arrancar y luego cada 6 h;
  - opción «Buscar actualizaciones» en la bandeja.
- **Front:** un puerto `Updater` (spec §4.2), un slice, el diálogo «Nueva versión disponible» y la sección «Acerca de» de Ajustes.
- **Versión y CHANGELOG:** un script propio (`scripts/version.mjs`) sincroniza las cuatro fuentes de versión y genera el CHANGELOG a partir de los commits convencionales.

**Tech Stack:**
- Base de F6: Tauri 2.12, Rust 2024, React 19, TypeScript 6, Zustand 5, zod 4, Vitest 5 y Testing Library.
- Nuevo en Rust: `tauri-plugin-updater` 2.14.
- Nuevo en JS (solo dev): `eslint` 10, `@eslint/js`, `typescript-eslint` 8, `eslint-plugin-react-hooks` 7 y `globals`.
- CI: `tauri-apps/tauri-action@v0`, `dtolnay/rust-toolchain`, `swatinem/rust-cache` y `pnpm/action-setup`.

**Spec:** [`docs/superpowers/specs/2026-10-15-qapture-design.md`](../specs/2026-10-15-qapture-design.md):
- §2 (instalación sin prerrequisitos, NSIS/MSI/portable);
- §3 D6, D7, D8, D10 y D12;
- §6.8 (menú de bandeja con «Buscar actualizaciones»);
- §6.9 (Acerca de);
- §6.10 UC-23 / UC-24;
- §9 (sin red salvo el updater);
- §10 (checklist manual por release).

**Contrato:** [`docs/superpowers/contracts/F7-cicd-release.md`](../contracts/F7-cicd-release.md).

## Punto de partida (lo que ya existe)

Los dos workflows llegaron en el PR #5 (`feat/ci-cd`), adelantados a la fase:

| Archivo | Ya hace | Le falta según el contrato y el spec |
|---|---|---|
| `.github/workflows/ci.yml` | En `windows-latest`, en push y PR a `main`, con dos jobs:<br>• front: `pnpm install`, `typecheck`, `test`, `build`;<br>• Rust: `fmt --check`, `clippy -D warnings`, `cargo test`, que ya compara el fixture de contrato | **ESLint** (el proyecto no tiene linter); **`tauri build --no-bundle`**; comprobar que ningún test reescribió el fixture de contrato; comprobar que las cuatro fuentes de versión coinciden; y **protección de rama** en `main`, sin la cual un PR roto se puede mergear igual |
| `.github/workflows/release.yml` | Con un tag `v*.*.*` o a mano: tests, `tauri-action` y release **publicado** con `.exe` y `.msi` (así salió `v0.4.0`) | Release en **borrador**; firma del updater (`latest.json` + `.sig`); **ZIP portable**; notas desde el CHANGELOG; comprobar que el tag coincide con la versión. Además, `workflow_dispatch` hace checkout de `main` aunque se le pase otro tag (bug) y el MSI sale `en-US` (el spec pide `es-ES`) |

No existen: clave del updater ni secretos (`gh secret list` está vacío), modo portable, updater en la app, CHANGELOG, `docs/usuario/`, `docs/arquitectura.md`, ADRs (`docs/adr/` está vacío) ni `CONTRIBUTING.md`. El README quedó en «v0.4.0».

## Prerrequisitos manuales (los hace la persona dueña del repo, antes de la Task 3)

Esto no lo hace un agente: es una clave privada y una configuración del repo público.

1. **Generar la clave del updater** (fuera del repo):

```bash
pnpm tauri signer generate -w "$HOME/.tauri/qapture-updater.key"
```

   Pide una contraseña. Deja `qapture-updater.key`, que es privada, y `qapture-updater.key.pub`, que es pública.
   - **Guardar una copia de la clave y la contraseña en un gestor de contraseñas.** Si se pierde la clave, las apps instaladas no pueden volver a actualizarse solas: habría que reinstalar a mano.
2. **Secretos en GitHub:**

```bash
gh secret set TAURI_SIGNING_PRIVATE_KEY < "$HOME/.tauri/qapture-updater.key"
gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD
```

3. **Pasar la clave pública:** se pega el contenido de `qapture-updater.key.pub` en el chat. La Task 3 la copia en `tauri.conf.json`; es pública y se versiona.
4. **Protección de rama en `main`** (se puede hacer al cerrar la Task 1, cuando existan los checks nuevos):
   - Settings › Branches › Add rule › `main`;
   - activar «Require status checks to pass», con los checks `Frontend (Lint, Typecheck, Test & Build)`, `Backend (Rust Fmt, Clippy & Test)` y `Tauri build (sin instaladores)`;
   - activar «Require a pull request before merging».

## Global Constraints

- Rama `feat/f7-cicd-release` desde `main` actualizado (F6 ya está en `main`, PR #7). Un commit por task, terminado con una línea en blanco y `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. **Sin push, merge, tags ni releases sin pedirlo.**
- Windows; comandos en Git Bash desde `qapture/` (Rust desde `qapture/src-tauri/`).
- Textos visibles **en español**. Mensajes fijos de esta fase:
  - **Updater, diálogo:**
    - título `Nueva versión disponible`;
    - cuerpo `Qapture <nueva> está disponible (tienes <actual>).`;
    - botones `Instalar y reiniciar` y `Después`.
  - **Updater, Acerca de y bandeja:**
    - `Buscar actualizaciones`, y `Buscando actualizaciones…` mientras busca;
    - `Qapture está al día (versión <actual>).`;
    - `Descargando actualización… <n> %`;
    - en la bandeja, con una versión pendiente: `Instalar actualización <nueva>`.
  - **Updater, errores:**
    - `No se pudo buscar actualizaciones. Revisa tu conexión a internet.`;
    - `No se pudo instalar la actualización: <detalle>.`;
    - `La actualización descargada no pasó la verificación de firma y no se instaló.`;
    - `Guarda o descarta la captura abierta antes de actualizar.`
  - **Portable:**
    - en el diálogo, en vez de instalar: `Estás usando la versión portable: descarga el ZIP nuevo y reemplaza Qapture.exe. Tus datos en .\data se conservan.`, con el botón `Descargar`, que abre la página del release;
    - en Sistema: `No disponible en la versión portable.`;
    - error al arrancar: `No se puede escribir en <carpeta>. Copia Qapture a una carpeta con permisos de escritura (no en Archivos de programa).`
- **Endpoint del updater:** `https://github.com/Ps3udoDev/qapture/releases/latest/download/latest.json` (D10: si el repo pasa a la org `teams4soft`, se cambia en un release puente). `releases/latest` ignora borradores y prereleases, así que una versión llega a las apps cuando alguien **publica** el borrador.
- **Versiones solo `X.Y.Z` numéricas**, sin `-rc`, porque el MSI (WiX) no acepta prereleases. El tag es `v` + versión.
- **Fuentes de versión, siempre iguales:** `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json` y la entrada `qapture` de `src-tauri/Cargo.lock`.
- **Artefactos de un release:**
  - `Qapture_<v>_x64-setup.exe` y su `.sig`;
  - `Qapture_<v>_x64_es-ES.msi` y su `.sig`;
  - `Qapture_<v>_portable.zip`;
  - `latest.json`, que apunta al NSIS.
- **ZIP portable:**
  - lleva `Qapture.exe` (el binario `qapture.exe` renombrado), un `portable.flag` vacío y `LEEME.txt`;
  - con el flag, los datos van a `<carpeta del exe>\data`: base, `evidence\`, `backups\`, `evidence-sin-uso\`.
- **Plugins solo desde Rust** (igual que F6): ningún permiso JS de `updater`. No se agrega `tauri-plugin-process`: Rust reinicia con `AppHandle::restart` (ver *Decisiones*).
- **ACL (F3):** cada comando nuevo va en `commands/names.rs`, `generate_handler!` y `capabilities/main.json`.
- **Seguridad:**
  - la clave privada **nunca** entra al repo: `.gitignore` suma `*.key`;
  - el spec §9 se mantiene: la única red es la consulta y descarga del updater, que hace Rust, así que la CSP del WebView no cambia.
- **Dependencias nuevas, solo estas:**
  - Rust: `tauri-plugin-updater` 2.x, la última 2.x; si sube el `tauri` del lock dentro de 2.x está bien.
  - JS (devDependencies): las 5 de ESLint de *Tech Stack*, con la versión exacta que se resuelva.
- El fixture `src/infrastructure/tauri/__fixtures__/contracts.json` solo se regenera con `UPDATE_CONTRACTS=1 cargo test contract`.
- Cero warnings al cerrar cada task: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `pnpm lint` (desde la Task 1), `pnpm typecheck`, `pnpm test` y `pnpm build`.
- **Disco:** si `C:` tiene menos de 5 GB antes de un build, se borra solo `src-tauri/target/debug/incremental` (y `release/incremental`) y se anota en el reporte.

## Review Focus

1. **Portable en una carpeta sin permiso de escritura** (por ejemplo, el ZIP extraído en `C:\Program Files\Qapture`): la app no puede caer en silencio a `%APPDATA%`, porque partiría los datos en dos lugares. Se muestra el mensaje fijo y se cierra → test en Task 2 (`resolve_data_dir` devuelve el error y no cae a AppData).
2. **Instalar una actualización con una captura sin guardar en el editor** (`editorQueue` no vacía): reiniciar borraría `_staging` y se perdería la captura. Se bloquea con `Guarda o descarta la captura abierta antes de actualizar.` → test en Task 4.
3. **Sin internet, o GitHub sin `latest.json`** (por ejemplo, un repo que solo tiene borradores): la revisión automática falla en silencio, se anota en el log de consola y no aparece ningún toast cada 6 h. La revisión manual muestra el mensaje de conexión → tests en Task 3 (`check_outcome` sin red) y Task 4 (manual vs. automática).
4. **La misma versión vista dos veces** (la revisión de las 6 h encuentra otra vez la versión que se pospuso con «Después»): no vuelve a abrir el diálogo en esa sesión, pero el indicador sigue en Ajustes y en la bandeja. Una versión **más nueva** sí lo abre → test en Task 3 (`should_prompt`) y Task 4.
5. **Versiones desalineadas o tag equivocado** (`package.json` 1.0.0 y `Cargo.toml` 0.6.0, o tag `v1.0.1` sobre código 1.0.0): CI y `release.yml` fallan antes de compilar, con un mensaje que nombra el archivo distinto → test en Task 5 (`checkVersions`).

## Decisiones respecto al contrato (se anotan en el contrato en la Task 1)

- **ESLint se agrega desde cero** con reglas mínimas: `@eslint/js` recommended, `typescript-eslint` recommended, y de `react-hooks` solo `rules-of-hooks` (error) y `exhaustive-deps` (warn → error en CI). No se activan las reglas del React Compiler de `react-hooks` 7: obligarían a refactorizar código estable sin beneficio para v1.
- **`tauri build --no-bundle --debug` en CI**, no en perfil release:
  - valida `tauri.conf.json`, las capabilities, el `beforeBuildCommand` y el embebido del front en unos 5 minutos en vez de 15;
  - el perfil release (LTO) lo compila `release.yml` en cada tag.
- **Firma del updater solo en release:** `createUpdaterArtifacts` va en `src-tauri/tauri.release.conf.json`, que `release.yml` pasa con `--config`. Así `pnpm tauri build` local no exige la clave privada.
- **Sin `tauri-plugin-process`:** el contrato lo nombra para reiniciar desde JS, pero aquí todo el updater es Rust, así que alcanza con `AppHandle::restart()`. En Windows el instalador NSIS en modo `passive` cierra la app y la vuelve a abrir.
- **Script propio en vez de `release-please`:**
  - un tag creado por el bot con `GITHUB_TOKEN` no dispara `release.yml`, así que exigiría un PAT;
  - `release-please` tampoco actualiza bien `Cargo.lock` en un proyecto que no es un workspace Rust;
  - `pnpm release <X.Y.Z>` hace el bump y el CHANGELOG, el commit y el tag quedan locales, y el push lo hace una persona.
- **CHANGELOG:** las versiones 0.1.0–0.6.0 se escriben a mano, un resumen por fase sacado de los contratos. Desde la siguiente se genera de los commits `feat`/`fix` (en el repo ya son convencionales: `feat(f6): …`).
- **Portable:**
  - **WebView2:** el WebView sigue guardando su caché y su `localStorage` (preferencias de UI) en `%LOCALAPPDATA%`, porque la ventana principal se crea desde la configuración. Los datos de Qapture (base, capturas, respaldos) sí van a `.\data`.
  - **Inicio con Windows:** se desactiva en portable, porque un exe en una USB no debe quedar en el registro.
  - **Single-instance:** la versión portable y la instalada comparten identificador. Si una ya está abierta, abrir la otra muestra la que estaba abierta. Va en la FAQ.
- **Ensayo antes de `v1.0.0`:** el contrato pide probar «v1.0.0 → v1.0.1». Como el pipeline mismo es lo que se prueba, el cierre (Task 8) propone:
  1. mergear con versión `1.0.0` y crear el tag desde `main`, lo que genera un borrador;
  2. revisar los artefactos y publicar;
  3. instalar;
  4. sacar `1.0.1` con un cambio menor y ver la actualización.

  Si algo falla, se borra el borrador y el tag y se corrige.
- **E2E con WebdriverIO** (spec §10) y **`tauri-plugin-log`** (spec §8) no están en el contrato F7: quedan para después de v1.

## File Structure

```
qapture/
├─ .github/workflows/ci.yml  release.yml           (T1, T5, T6)
├─ .gitignore                                     (T1) *.key
├─ eslint.config.js  package.json  pnpm-lock.yaml  vite.config.ts   (T1, T5)
├─ CHANGELOG.md  CONTRIBUTING.md  README.md        (T5, T7, T8)
├─ scripts/version.mjs  release-lib.mjs  release-lib.test.mjs  check-version.mjs  release-notes.mjs  (T5)
├─ docs/usuario/{instalacion,uso,atajos,faq}.md    (T7)
├─ docs/arquitectura.md  docs/release-checklist.md  docs/adr/000{1..4}-*.md  (T7)
├─ src/
│  ├─ domain/update.ts  update.test.ts            (T4)
│  ├─ ports/updater.ts  repositories.ts           (T4)
│  ├─ infrastructure/tauri/updater.ts  dto.ts  repositories.ts  contract.test.ts  (T2, T4)
│  ├─ infrastructure/memory/updater.ts  settings.ts  repositories.ts              (T2, T4)
│  └─ ui/
│     ├─ store/update-slice.ts  update-slice.test.ts  workspace.ts  (T4)
│     └─ screens/UpdateDialog.tsx  AppSettingsDialog.tsx  TopBar.tsx  AppShell.tsx  Update.test.tsx  (T2, T4)
└─ src-tauri/
   ├─ Cargo.toml  Cargo.lock  tauri.conf.json  tauri.release.conf.json  (T3, T6)
   ├─ portable/LEEME.txt                          (T6)
   ├─ capabilities/main.json                      (T3)
   └─ src/
      ├─ system/portable.rs                       (T2) dónde viven los datos
      ├─ system/updater.rs                        (T3) lógica pura + comandos + revisión periódica
      ├─ system/tray.rs  runtime.rs  mod.rs       (T2, T3)
      ├─ commands/names.rs  settings.rs           (T2, T3)
      └─ dto.rs  state.rs  lib.rs                 (T2, T3)
```

Correspondencia con el contrato:

| Contrato | Tasks |
|---|---|
| F7-T1 | Task 1 (+ check de versión en Task 5) |
| F7-T2 | Prerrequisitos manuales + Task 3 (pubkey) + Task 6 (secretos en el workflow) |
| F7-T3 | Task 6 |
| F7-T4 | Task 2 (Rust) + Task 6 (ZIP) |
| F7-T5 | Tasks 3 y 4 |
| F7-T6 | Task 5 |
| F7-T7 | Task 7 |
| F7-T8 | Task 8 |

---

### Task 1: Rama, documentos, ESLint y CI completo

**Files:**
- Create: `eslint.config.js`
- Modify: `package.json` (script `lint` y devDependencies), `pnpm-lock.yaml`, `.gitignore`, `.github/workflows/ci.yml`, `docs/superpowers/contracts/README.md`, `docs/superpowers/contracts/F7-cicd-release.md`, y los archivos de `src/` que ESLint marque.
- Add: este plan.

**Interfaces:**
- Produces:
  - el script `pnpm lint` (`eslint . --max-warnings 0`);
  - en `ci.yml`, los jobs `Frontend (Lint, Typecheck, Test & Build)`, `Backend (Rust Fmt, Clippy & Test)` y `Tauri build (sin instaladores)`. La protección de rama usa esos tres nombres.

- [ ] **Step 1: Rama y documentos**

```bash
git checkout main && git pull && git checkout -b feat/f7-cicd-release
```

- En el `README.md` de contratos, la fila F7 pasa a `[plan](../plans/2026-10-17-f7-cicd-release.md) | 🟡 En curso`.
- En el contrato F7:
  - la cabecera enlaza el plan;
  - se agrega la sección «Punto de partida» (la tabla de este plan);
  - se agrega «Notas de implementación (acordadas en el plan)», con los puntos de *Decisiones*;
  - se agrega «Interfaces producidas» (comandos de la Task 3, eventos `update:available` y `update:progress`, `portable.flag`).
- `.gitignore` suma `*.key` bajo un comentario: `# Clave privada del updater: nunca al repo`.

- [ ] **Step 2: ESLint**
  - Instalar: `pnpm add -D eslint @eslint/js typescript-eslint eslint-plugin-react-hooks globals`.
  - Crear `eslint.config.js` (flat config) con lo siguiente:
    - **ignora:** `dist`, `src-tauri/target`, `node_modules`, `src/infrastructure/tauri/__fixtures__`;
    - **reglas:** `@eslint/js` recommended + `typescript-eslint` recommended + `react-hooks/rules-of-hooks: error` + `react-hooks/exhaustive-deps: error`;
    - **globals:** `browser` en `src/`, `node` en `scripts/`, `vite.config.ts` y `eslint.config.js`.
  - Ejecutar `pnpm lint` y corregir lo que marque **sin cambiar comportamiento**:
    - si un `exhaustive-deps` pide una dependencia que cambiaría el efecto, se deja un `// eslint-disable-next-line react-hooks/exhaustive-deps -- <motivo>` con el motivo real;
    - el reporte lista cada disable y cada corrección.

- [ ] **Step 3: `ci.yml`**
  - **Job front:** se renombra a `Frontend (Lint, Typecheck, Test & Build)` y suma `pnpm lint` antes de `typecheck`.
  - **Job Rust:** después de `cargo test` suma el paso «El fixture de contrato no cambió»:
    - `git diff --exit-code -- src/infrastructure/tauri/__fixtures__/contracts.json`;
    - atrapa un test que reescriba el fixture con `UPDATE_CONTRACTS` puesto por error.
  - **Job nuevo** `Tauri build (sin instaladores)`:
    - checkout, pnpm y Node 22, toolchain stable, `rust-cache` con `workspaces: src-tauri` y `shared-key: tauri-build`;
    - `pnpm install --frozen-lockfile`;
    - `pnpm tauri build --no-bundle --debug`.
  - Se mantienen los disparadores y la `concurrency` actuales.

- [ ] **Step 4: Verificación local**
  - `pnpm lint && pnpm typecheck && pnpm test && pnpm build` → PASS.
  - `pnpm tauri build --no-bundle --debug` → genera `src-tauri/target/debug/qapture.exe` sin instaladores.
  - `cd src-tauri && cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check` → PASS.
  - Validar la sintaxis de `ci.yml` con `pnpm dlx @action-validator/cli .github/workflows/ci.yml`. Si no se puede instalar, revisar a mano la indentación y que no falte ninguna clave de los jobs, y decirlo en el reporte.

- [ ] **Step 5: Commit** `ci(f7): ESLint, build de Tauri sin instaladores y fixture de contrato en CI`

---

### Task 2: Modo portable (Rust + Ajustes)

**Files:**
- Create: `src-tauri/src/system/portable.rs`
- Modify:
  - Rust: `src-tauri/src/system/mod.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/state.rs`, `src-tauri/src/dto.rs`, `src-tauri/src/commands/settings.rs`, `src-tauri/src/system/runtime.rs`;
  - front: `src/infrastructure/tauri/dto.ts`, `src/infrastructure/memory/settings.ts`, `src/ui/screens/AppSettingsDialog.tsx`, `src/ui/screens/AppSettings.test.tsx`;
  - el fixture de contrato.

**Interfaces:**
- Produces:
  - `system::portable::{PORTABLE_FLAG = "portable.flag", DataLocation { dir: PathBuf, portable: bool }, resolve_data_dir(exe_dir: &Path, app_data: &Path) -> Result<DataLocation, String>, ensure_writable(dir: &Path) -> Result<(), String>}`:
    - `resolve_data_dir` devuelve `exe_dir/data` (`portable: true`) si existe `exe_dir/portable.flag`, y si no, `app_data`;
    - `ensure_writable` crea la carpeta y escribe y borra un archivo de prueba; si falla, devuelve el mensaje fijo de portable con la ruta.
  - `AppState::is_portable() -> bool`.
  - `SettingsDto` suma `portable: bool` (`SettingsSnapshot.portable` en TS).

- [ ] **Step 1: Tests (fallan)** en `system/portable.rs`:

```rust
#[test] fn without_the_flag_data_stays_in_app_data()
// tmp/exe sin portable.flag → DataLocation { dir: app_data, portable: false }

#[test] fn with_the_flag_data_goes_next_to_the_exe()
// tmp/exe/portable.flag → DataLocation { dir: tmp/exe/data, portable: true }

#[test] fn a_read_only_folder_is_an_error_not_a_fallback()
// ensure_writable(ruta cuyo padre es un ARCHIVO, p. ej. tmp/archivo.txt/data) → Err(m)
// m == "No se puede escribir en <ruta>. Copia Qapture a una carpeta con permisos de escritura (no en Archivos de programa)."
```

Y en `commands/settings.rs`:

```rust
#[test] fn the_snapshot_says_whether_qapture_is_portable()
// AppState portable → get_settings/save → dto.portable == true; instalado → false
```

Run: `cd src-tauri && cargo test portable` → FAIL.

- [ ] **Step 2: Implementar**
  - `lib.rs` `setup`: el `data_dir` sale de `resolve_data_dir(<carpeta de current_exe>, app.path().app_data_dir()?)`, y luego `ensure_writable`.
    - Si falla, se muestra con `tauri_plugin_dialog` un `MessageDialog` bloqueante (título `Qapture`, nivel error, el mensaje) y `app.handle().exit(1)`.
    - Ningún otro código vuelve a calcular el directorio: todo usa ese `data_dir`, como hoy.
  - En portable, el asset protocol permite la carpeta de evidencias en tiempo de ejecución:
    - `app.asset_protocol_scope().allow_directory(data_dir.join("evidence"), true)`;
    - el scope de `tauri.conf.json` (`$APPDATA/evidence/**`) sigue cubriendo la versión instalada.
  - `AppState::new` recibe `portable: bool`.
  - Inicio con Windows en portable: `runtime::sync_autostart` no hace nada y `save` guarda `system.autostart = false` aunque llegue `true`.
  - Front:
    - `SettingsSnapshot.portable` en el esquema zod estricto y en el adaptador en memoria, con `portable: false` por defecto y modificable en los tests;
    - en Ajustes › Sistema, con `portable`, la casilla «Iniciar con Windows» queda deshabilitada con la nota `No disponible en la versión portable.`;
    - test en `AppSettings.test.tsx`.
  - Regenerar el fixture: `UPDATE_CONTRACTS=1 cargo test contract`.

- [ ] **Step 3: Verificación manual mínima** (la hace el implementador)
  1. `pnpm tauri build --no-bundle --debug`.
  2. Copiar `src-tauri/target/debug/qapture.exe` a una carpeta temporal fuera del repo, junto a un `portable.flag` vacío.
  3. Ejecutarlo y comprobar que se crea `.\data\qapture.db`.
  4. Cerrar con el menú de bandeja → Salir.

  Si la app instalada del usuario está abierta, single-instance la muestra y la prueba no vale: anotarlo y seguir, porque la Task 8 la repite.

- [ ] **Step 4:** todo el paquete de comandos de *Global Constraints* → PASS.
- [ ] **Step 5: Commit** `feat(f7): modo portable con datos junto al exe`

---

### Task 3: Updater en Rust (plugin, lógica, comandos, revisión periódica y bandeja)

**Files:**
- Create: `src-tauri/src/system/updater.rs`, `src-tauri/tauri.release.conf.json`
- Modify: `src-tauri/Cargo.toml`, `Cargo.lock`, `src-tauri/tauri.conf.json`, `src-tauri/src/lib.rs`, `src-tauri/src/system/{mod,tray}.rs`, `src-tauri/src/commands/names.rs`, `src-tauri/capabilities/main.json`, `src-tauri/src/dto.rs`, el fixture de contrato.

**Interfaces:**
- Consumes:
  - la clave pública de los *Prerrequisitos*;
  - `AppState::is_portable()` (Task 2);
  - `runtime::notify(app, body)` (F6).
- Produces:
  - **DTO:** `dto::UpdateInfoDto { version: String, current_version: String, notes: String, date: Option<String>, portable: bool }` (camelCase).
  - **Comandos:**
    - `check_update() -> AppResult<Option<UpdateInfoDto>>`: revisión manual; los errores de red usan el mensaje fijo;
    - `install_update() -> AppResult<()>`: descarga, emite `update:progress { downloaded: u64, total: Option<u64> }`, instala y reinicia. En portable responde `Err(validation(<mensaje portable>))`.
  - **Eventos:** `update:available` (payload `UpdateInfoDto`) y `update:progress`.
  - **Lógica pura** en `updater.rs`:
    - `should_prompt(found: &str, dismissed: Option<&str>) -> bool`: `true` si no hay versión pospuesta o si `found` es mayor por semver numérico;
    - `CheckOutcome::{UpToDate, Available(UpdateInfoDto), Failed(String)}`;
    - `fn check_outcome(result: Result<Option<UpdateInfoDto>, String>) -> CheckOutcome`;
    - `fn error_message(detail: &str) -> String`: si el detalle menciona la firma, el mensaje fijo de firma; si no, `No se pudo instalar la actualización: <detalle>.`.
  - **`AppState`:** `pending_update()` y `set_pending_update(Option<UpdateInfoDto>)`. La versión pospuesta la guarda el front (Task 4); Rust solo emite.
  - **Bandeja:** `TrayAction::CheckUpdate`, con id `check-update`.

- [ ] **Step 1: Tests (fallan)** en `system/updater.rs`:

```rust
#[test] fn a_dismissed_version_is_not_prompted_again_but_a_newer_one_is()
// should_prompt("1.0.1", None) == true; should_prompt("1.0.1", Some("1.0.1")) == false;
// should_prompt("1.0.2", Some("1.0.1")) == true; should_prompt("1.0.10", Some("1.0.9")) == true (numérico, no texto)

#[test] fn offline_is_a_failure_not_an_update()
// check_outcome(Err("error sending request")) == Failed("No se pudo buscar actualizaciones. Revisa tu conexión a internet.")
// check_outcome(Ok(None)) == UpToDate

#[test] fn a_bad_signature_has_its_own_message()
// error_message("signature verification failed") == "La actualización descargada no pasó la verificación de firma y no se instaló."
// error_message("disk full") == "No se pudo instalar la actualización: disk full."
```

En `tray.rs`: `action_for("check-update") == Some(TrayAction::CheckUpdate)`.

Run: `cd src-tauri && cargo test updater` → FAIL.

- [ ] **Step 2: Configuración**
  - `cargo add tauri-plugin-updater@2`.
  - `tauri.conf.json`, en `plugins.updater`:
    - `"pubkey": "<contenido de qapture-updater.key.pub>"`;
    - `"endpoints": ["https://github.com/Ps3udoDev/qapture/releases/latest/download/latest.json"]`;
    - `"windows": { "installMode": "passive" }`.
  - `src-tauri/tauri.release.conf.json`: `{ "bundle": { "createUpdaterArtifacts": true } }`.
  - **Sin clave pública** (no se hicieron los prerrequisitos): el implementador para con `NEEDS_CONTEXT`. Nunca inventa una clave ni genera una propia.

- [ ] **Step 3: Implementar**
  - `.plugin(tauri_plugin_updater::Builder::new().build())`.
  - **Revisión periódica:** `updater::spawn(handle)`, con el mismo patrón que `daily_backup::spawn`. Espera 15 s, revisa, y repite cada 6 h.
    - Con `Available`, guarda `pending_update`, emite `update:available` y cambia el texto del ítem de bandeja a `Instalar actualización <v>`;
    - con `Failed`, solo `eprintln!`.
    - En builds debug (`cfg!(debug_assertions)`) no arranca la revisión periódica, para no consultar GitHub en desarrollo. La manual sí funciona.
  - **Bandeja:** el ítem `Buscar actualizaciones` va antes de `Salir`. Al hacer clic:
    - con una actualización pendiente: muestra la ventana y emite `update:available` (el front abre el diálogo);
    - si no: revisa; con `Available` hace lo mismo; con `UpToDate` notifica `Qapture está al día (versión <actual>).`; con `Failed` notifica el mensaje de conexión.
  - **`install_update`:**
    - `updater().check()`, y luego `download_and_install` con un callback de progreso que emite `update:progress`;
    - al terminar, `app.restart()`;
    - los errores pasan por `error_message`.
  - Los nombres en `names.rs`, `generate_handler!` y `capabilities/main.json`.
  - `UpdateInfoDto` en `dto::sample()`, y luego `UPDATE_CONTRACTS=1 cargo test contract`.

- [ ] **Step 4:**
  - `pnpm tauri build` local **sin** `TAURI_SIGNING_PRIVATE_KEY` sigue generando los instaladores, porque la firma solo está en `tauri.release.conf.json`;
  - el paquete de comandos de *Global Constraints* → PASS.
- [ ] **Step 5: Commit** `feat(f7): updater en Rust con revisión periódica, bandeja y firma solo en release`

---

### Task 4: Updater en el front (puerto, slice, diálogo y Acerca de)

**Files:**
- Create: `src/domain/update.ts`, `src/domain/update.test.ts`, `src/ports/updater.ts`, `src/infrastructure/tauri/updater.ts`, `src/infrastructure/memory/updater.ts`, `src/ui/store/update-slice.ts`, `src/ui/store/update-slice.test.ts`, `src/ui/screens/UpdateDialog.tsx`, `src/ui/screens/Update.test.tsx`
- Modify: `src/ports/repositories.ts`, `src/infrastructure/{tauri,memory}/repositories.ts`, `src/infrastructure/tauri/dto.ts`, `src/infrastructure/tauri/contract.test.ts`, `src/ui/store/workspace.ts`, `src/ui/screens/{AppShell,TopBar,AppSettingsDialog}.tsx`

**Interfaces:**
- Consumes: los comandos y eventos de la Task 3.
- Produces:
  - **Puerto** `Updater { check(): Promise<UpdateInfo | null>; install(): Promise<void>; onAvailable(cb): Unlisten; onProgress(cb): Unlisten }`.
  - **Dominio** (`domain/update.ts`): `UpdateInfo` y los textos fijos como funciones:
    - `availableMessage(info)` (`Qapture <nueva> está disponible (tienes <actual>).`);
    - `upToDateMessage(v)`;
    - `progressMessage(downloaded, total)` (`Descargando actualización… <n> %`, que con `total` desconocido no muestra el porcentaje).
  - **Slice** (`update-slice.ts`):
    - `update: { available: UpdateInfo | null, dismissed: string | null, status: "idle" | "checking" | "downloading", progress: number | null }`;
    - `checkForUpdates(manual: boolean)`, `installUpdate()`, `dismissUpdate()`, `listenUpdates()`.

- [ ] **Step 1: Tests (fallan)**

```ts
// update-slice.test.ts
it("la revisión automática abre el diálogo una vez por versión")
// onAvailable(1.0.1) → diálogo abierto; dismissUpdate() → cerrado y dismissed "1.0.1";
// onAvailable(1.0.1) otra vez → sigue cerrado; onAvailable(1.0.2) → abierto
it("buscar a mano sin novedades avisa que está al día y sin red muestra el error")
// check → null → toast "Qapture está al día (versión 0.7.0)."; check rechaza con AppError → toast con su mensaje
it("no instala con una captura sin guardar en el editor")
// editorQueue con 1 → installUpdate() → toast "Guarda o descarta la captura abierta antes de actualizar."; updater.installs == 0
it("muestra el progreso de descarga")
// onProgress {downloaded: 50, total: 200} → progress 25 → progressMessage "Descargando actualización… 25 %"
```

```tsx
// Update.test.tsx
it("Instalar y reiniciar llama al updater; Después deja el indicador en Ajustes")
it("en la versión portable ofrece Descargar en vez de instalar")
// portable → texto portable fijo; botón "Descargar" → openLink("releases"); no hay botón "Instalar y reiniciar"
it("Acerca de busca actualizaciones y muestra el resultado")
```

Run: `pnpm vitest run src/domain/update.test.ts src/ui/store/update-slice.test.ts src/ui/screens/Update.test.tsx` → FAIL.

- [ ] **Step 2: Implementar**
  - **Adaptadores:**
    - Tauri: `invoke("check_update")` / `invoke("install_update")` y `listen("update:available" | "update:progress")`, todo validado con zod estricto;
    - memoria: `next`, `installs` y `emitAvailable(info)` / `emitProgress(p)` para los tests.
  - **`AppShell`:** llama a `listenUpdates()` igual que a `listenNewCase()` (F6) y renderiza `UpdateDialog` cuando hay `available` sin posponer.
  - **`UpdateDialog`:** usa el componente `Dialog` de siempre.
    - Muestra el título y el cuerpo fijos y las notas (`notes`) como texto plano con saltos de línea, nunca como HTML.
    - Muestra los botones fijos.
    - Mientras descarga, los botones se deshabilitan y se muestra el progreso.
  - **TopBar:** con una actualización pospuesta, el botón «Ajustes» lleva un punto indicador y `aria-label="Ajustes (actualización disponible)"`.
  - **Acerca de:** muestra el botón `Buscar actualizaciones` (y `Buscando actualizaciones…` mientras busca). Con una actualización disponible muestra la línea `Versión <v> disponible` y el botón `Instalar y reiniciar`, o `Descargar` en portable.
  - **`contract.test.ts`:** valida `UpdateInfoDto` del fixture.

- [ ] **Step 3:** el paquete de comandos de *Global Constraints* → PASS.
- [ ] **Step 4: Commit** `feat(f7): aviso de actualización, progreso y Acerca de con buscar actualizaciones`

---

### Task 5: Versión sincronizada y CHANGELOG (`pnpm release`)

**Files:**
- Create: `scripts/release-lib.mjs`, `scripts/release-lib.test.mjs`, `scripts/version.mjs`, `scripts/check-version.mjs`, `scripts/release-notes.mjs`, `CHANGELOG.md`
- Modify: `package.json` (scripts `release`, `check-version`), `vite.config.ts` (`test.include` suma `scripts/**/*.test.mjs`), `.github/workflows/ci.yml` (paso `pnpm check-version` en el job front)

**Interfaces:**
- Produces (`release-lib.mjs`, funciones puras sobre texto):
  - `readVersions(files: { pkg, cargoToml, tauriConf, cargoLock }) -> { pkg, cargoToml, tauriConf, cargoLock }`;
  - `checkVersions(versions, tag?) -> string[]`: los errores; vacío si todo coincide;
  - `bump(files, version) -> files`: cambia solo la versión de `qapture`, sin tocar el resto del formato;
  - `changelogSection(version, date, subjects: string[]) -> string`;
  - `notesFor(changelog, version) -> string`.
- Comandos:
  - `pnpm release <X.Y.Z>` (`version.mjs`):
    1. valida `X.Y.Z` y que el árbol esté limpio;
    2. hace el bump de los 4 archivos;
    3. agrega la sección al CHANGELOG con los commits desde el último tag;
    4. hace `git commit -m "chore(release): v<X.Y.Z>"` y `git tag v<X.Y.Z>`;
    5. **no hace push**: imprime los comandos para hacerlo.
  - `pnpm check-version [vX.Y.Z]`: sale con código 1 y lista los errores.
  - `node scripts/release-notes.mjs vX.Y.Z`: imprime las notas de esa versión, que usa `release.yml`.

- [ ] **Step 1: Tests (fallan)** en `scripts/release-lib.test.mjs`:

```js
it("detecta versiones desalineadas y nombra el archivo")
// pkg 1.0.0, cargoToml 0.6.0 → ["src-tauri/Cargo.toml tiene 0.6.0 y package.json 1.0.0."]
it("exige que el tag coincida")
// todo 1.0.0, tag "v1.0.1" → ["El tag v1.0.1 no coincide con la versión 1.0.0."]
it("rechaza versiones que no son X.Y.Z")
// "1.0.0-rc.1" → ["La versión debe ser X.Y.Z (el MSI no admite prereleases): 1.0.0-rc.1."]
it("el bump solo toca la versión de qapture en Cargo.lock")
// lock con [[package]] name = "qapture" version = "0.6.0" y otra dependencia en 0.6.0 → solo cambia la de qapture
it("agrupa feat y fix y omite el resto")
// ["feat(f7): updater", "fix(f7): x", "chore: y", "Merge pull request #8"] →
// "## 1.0.0 — 2026-10-20\n\n### Nuevo\n- updater\n\n### Correcciones\n- x\n"
it("extrae las notas de una versión del CHANGELOG")
```

Run: `pnpm vitest run scripts/release-lib.test.mjs` → FAIL.

- [ ] **Step 2: Implementar**
  - Los scripts de entrada (`version.mjs`, `check-version.mjs`, `release-notes.mjs`) solo leen y escriben archivos y llaman a `release-lib.mjs`.
  - El CHANGELOG arranca con `# Cambios` y, a mano, las secciones `0.6.0` … `0.1.0`: un párrafo corto por fase, tomado del objetivo de cada contrato.
- [ ] **Step 3:**
  - `pnpm check-version` → PASS con 0.6.0;
  - probar `pnpm release 9.9.9` en una copia del repo en el scratchpad, nunca en el repo real, y borrar la copia;
  - el paquete de comandos de *Global Constraints* → PASS.
- [ ] **Step 4: Commit** `build(f7): pnpm release sincroniza la versión y genera el CHANGELOG`

---

### Task 6: `release.yml` con borrador, firma del updater, MSI en español y ZIP portable

**Files:**
- Create: `src-tauri/portable/LEEME.txt`
- Modify: `.github/workflows/release.yml`, `src-tauri/tauri.conf.json` (`bundle.windows`)

**Interfaces:**
- Consumes:
  - los secretos `TAURI_SIGNING_PRIVATE_KEY` y `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` (*Prerrequisitos*);
  - `tauri.release.conf.json` (Task 3);
  - `check-version.mjs` y `release-notes.mjs` (Task 5).
- Produces: un release en borrador con los artefactos de *Global Constraints*.

- [ ] **Step 1: `tauri.conf.json`**
  - `bundle.windows.wix.language: "es-ES"`, para que salga `Qapture_<v>_x64_es-ES.msi`;
  - `bundle.windows.nsis.languages: ["Spanish"]`.
  - Comprobar con `pnpm tauri build` local que salen los dos instaladores con esos nombres.

- [ ] **Step 2: `release.yml`**
  - **Checkout:** `ref: ${{ inputs.tag_name || github.ref }}`, que corrige el bug de `workflow_dispatch`.
  - **Tag:** el paso `Resolver tag` deja `TAG` y `VERSION` (sin la `v`) en `$GITHUB_ENV`.
  - **Puertas:** `pnpm check-version $TAG`, y luego `pnpm lint`, `pnpm typecheck`, `pnpm test` y `cargo test`.
  - **Notas:** `node scripts/release-notes.mjs $TAG > release-notes.md`; se pasan a `tauri-action` como salida multilínea del paso.
  - **`tauri-action@v0`:**
    - `args: --config src-tauri/tauri.release.conf.json`;
    - `releaseDraft: true`, `prerelease: false`;
    - `includeUpdaterJson: true`, `updaterJsonPreferNsis: true`;
    - `releaseBody` con las notas, más una línea sobre SmartScreen que enlaza `docs/usuario/instalacion.md`;
    - `env`: `GITHUB_TOKEN` y los dos secretos de firma.
  - **ZIP portable** (PowerShell):
    1. crear `portable/` con `Copy-Item src-tauri/target/release/qapture.exe portable/Qapture.exe`, un `portable.flag` vacío y `src-tauri/portable/LEEME.txt`;
    2. `Compress-Archive portable/* Qapture_$VERSION_portable.zip`;
    3. `gh release upload $TAG Qapture_$VERSION_portable.zip --clobber`, con `GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}`.
  - Se mantienen `permissions: contents: write` y la concurrencia sin cancelar.

- [ ] **Step 3: `LEEME.txt`** (texto plano, CRLF, en español). Explica:
  - que es la versión portable y que los datos quedan en `.\data`;
  - que no se debe borrar `portable.flag`;
  - que para actualizar se reemplaza `Qapture.exe` por el del ZIP nuevo;
  - que no se debe extraer en «Archivos de programa»;
  - qué hacer con el aviso de SmartScreen;
  - que necesita WebView2, que ya viene en Windows 11.

- [ ] **Step 4: Verificación sin publicar**
  - validar la sintaxis del YAML, como en la Task 1;
  - `pnpm tauri build` local → los nombres de los instaladores coinciden con *Global Constraints*.
  - **No se crean tags ni se dispara el workflow.** La primera corrida real es la del cierre (Task 8), con permiso.
- [ ] **Step 5: Commit** `ci(f7): release en borrador con actualización firmada, MSI en español y ZIP portable`

---

### Task 7: Documentación de usuario y de desarrollo

**Files:**
- Create:
  - usuario: `docs/usuario/README.md`, `docs/usuario/instalacion.md`, `docs/usuario/uso.md`, `docs/usuario/atajos.md`, `docs/usuario/faq.md`;
  - desarrollo: `docs/arquitectura.md`, `docs/release-checklist.md`, `CONTRIBUTING.md`;
  - ADRs: `docs/adr/0001-react-y-rust.md` (D6), `docs/adr/0002-excel-en-rust.md` (D7), `docs/adr/0003-sqlite-y-png.md` (D8), `docs/adr/0004-tests-de-contrato.md` (D12).

Sin código, así que no hay TDD. Cada documento sale del spec, de los contratos y del código real; no se describe nada que no exista.

- [ ] **Step 1: `docs/usuario/`**, para una persona QA nueva, sin jerga de desarrollo:
  - **`instalacion.md`:**
    - qué descargar (NSIS recomendado, MSI para TI, ZIP portable);
    - SmartScreen paso a paso («Más información» → «Ejecutar de todas formas») y por qué aparece: sin firma Authenticode en v1;
    - cómo funcionan las actualizaciones automáticas;
    - cómo desinstalar y dónde quedan los datos (`%APPDATA%\com.teams4soft.qapture`).
  - **`uso.md`:** el flujo completo:
    1. crear un proyecto (o importar la Bitácora v1);
    2. registrar casos;
    3. capturar (`Ctrl+Alt+S`) o recortar con `Win+Shift+S` y pegar con `Ctrl+Alt+V`;
    4. anotar;
    5. exportar el Excel;
    6. paquetes `.qapture` y respaldos.
  - **`atajos.md`:** los 4 atajos por defecto y cómo cambiarlos. Incluye:
    - por qué `Win+Shift+S` / Impr Pant siguen siendo de Windows (la decisión de F6);
    - la nota de AltGr en teclados en español.
  - **`faq.md`:**
    - atajo ocupado por otra app;
    - la app «desaparece» al cerrar (vive en la bandeja);
    - portable e instalada a la vez (single-instance);
    - recuperar desde un respaldo;
    - cómo pasar datos a otro equipo.
- [ ] **Step 2: Desarrollo**
  - **`docs/arquitectura.md`:**
    - capas (domain / ports / infrastructure / ui y los módulos de Rust), con el diagrama mermaid del spec §4.1 actualizado;
    - el flujo de un comando: zod ↔ DTO ↔ ACL;
    - los eventos (`evidence:captured`, `shortcut:new-case`, `update:*`);
    - dónde viven los datos (instalada / portable).
  - **ADRs:** formato corto «Contexto · Decisión · Consecuencias», con la fecha original de la decisión (2026-10-15).
  - **`CONTRIBUTING.md`:**
    - requisitos y `pnpm tauri dev`;
    - ramas `feat/fN-…` y commits convencionales;
    - contratos y planes por fase;
    - comandos de verificación (los de *Global Constraints*);
    - regenerar el fixture de contrato;
    - **cómo publicar**: `pnpm release X.Y.Z` → push del commit y del tag → revisar el borrador → publicar;
    - nunca versionar datos de clientes ni claves.
  - **`docs/release-checklist.md`** (spec §10): la checklist manual por release: Excel en Excel y WPS, atajos globales, bandeja, updater, portable.
- [ ] **Step 3: Verificación**
  - Todos los enlaces relativos resuelven. Comprobarlo con un script de una línea que busque `](` y verifique que cada ruta exista, y pegar la salida en el reporte.
  - Los nombres de comandos, atajos y mensajes coinciden con el código: `grep` de cada texto citado.
- [ ] **Step 4: Commit** `docs(f7): guía de usuario, arquitectura, ADRs y CONTRIBUTING`

---

### Task 8: README, versión 1.0.0 y cierre técnico del contrato

**Files:** Modify `README.md`, `docs/superpowers/contracts/README.md`, `docs/superpowers/contracts/F7-cicd-release.md`, y los 4 archivos de versión + `CHANGELOG.md` (vía `pnpm release`).

- [ ] **Step 1: README**
  - Badges:
    - **release**: `https://img.shields.io/github/v/release/Ps3udoDev/qapture`;
    - **CI**: badge de `ci.yml`.
  - Botón o enlace «Descargar» a `https://github.com/Ps3udoDev/qapture/releases/latest`.
  - Estado `v1.0.0`.
  - «Fases implementadas»: suma F5, F6 y F7, cada una en una línea con su tag, como las existentes.
  - Enlaces a `docs/usuario/` y `CONTRIBUTING.md`.
  - Se quita la ruta fija del instalador `0.4.0`.
- [ ] **Step 2: Versión** con la herramienta nueva:
  - `pnpm release 1.0.0`;
  - **revisar** el CHANGELOG generado y editar la sección 1.0.0 si hace falta, haciendo amend sobre el commit `chore(release)`;
  - **borrar el tag local** que crea el script (`git tag -d v1.0.0`): el tag se crea después del merge, desde `main` (ver *Decisiones*).
- [ ] **Step 3: Verificación completa**
  - el paquete de comandos de *Global Constraints*, `pnpm check-version` y `pnpm tauri build` → `Qapture_1.0.0_x64-setup.exe` y `Qapture_1.0.0_x64_es-ES.msi`;
  - `pnpm tauri build --config src-tauri/tauri.release.conf.json` **sin** la clave → debe fallar pidiendo `TAURI_SIGNING_PRIVATE_KEY`; confirma que la firma solo se exige en release.
- [ ] **Step 4: Contrato**
  - columna `Estado` en la tabla de tasks (✅ por cada task hecha);
  - criterios de aceptación con el test o la corrida que los cubre;
  - fila F7 del README de contratos → `🧪 En pruebas del equipo`;
  - los OK del checklist quedan sin marcar.
- [ ] **Step 5: Commit** (amend del `chore(release): v1.0.0` con README y contrato, o un commit `docs(f7): cierre técnico del contrato F7`).
- [ ] **Step 6: Entrega al equipo** (lo hace el controlador, no el implementador). Explica la secuencia de cierre, y cada paso con efectos fuera del repo se hace solo con permiso explícito:
  1. Activar la protección de rama (*Prerrequisitos* 4) y comprobarla con un PR de prueba que rompa un test: criterio 1.
  2. Mergear el PR de F7.
  3. Crear el tag `v1.0.0` desde `main` y hacer push. Ver el borrador con los artefactos, revisarlo y publicarlo: criterio 2 y checklist 1.
  4. Instalar `1.0.0` y probar el ZIP portable: checklist 3.
  5. Sacar `1.0.1` con `pnpm release 1.0.1` sobre un cambio menor (por ejemplo, una corrección de docs). Hacer push del tag, publicar el borrador con la app abierta y ver el aviso, la instalación y el reinicio: criterio 3 y checklist 2.
  6. Una persona nueva sigue `docs/usuario`: checklist 4.
