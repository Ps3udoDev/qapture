# F6 — Segundo plano, bandeja, atajos globales y Settings · Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Que Qapture viva en la bandeja y se invoque con atajos globales configurables (`Ctrl+Alt+Q` mostrar/ocultar, `Ctrl+Alt+S` capturar, `Ctrl+Alt+V` pegar evidencia, `Ctrl+Alt+N` nuevo caso), con una pantalla **Ajustes** persistida en la base (perfil, atajos, sistema, exportación, datos, acerca de), inicio con Windows y notificaciones nativas para lo que pasa en segundo plano.

**Architecture:** Los ajustes viven en la tabla `setting` (migración `0004`), una fila JSON por sección. Rust los lee al arrancar y los guarda en `AppState`, porque los atajos, el cierre de la ventana y el inicio oculto los necesitan sin pasar por el front. La lógica de atajos es pura y testeable: normalizar combinaciones, validarlas y aplicarlas con un `ShortcutRegistrar` que en producción envuelve `tauri-plugin-global-shortcut`. Si una combinación nueva falla, se revierten todas y se conservan las anteriores. Cada atajo despacha una acción:
- **Mostrar/ocultar:** usa la bandeja de F1.
- **Capturar:** usa `CaptureController` de F3.
- **Pegar evidencia:** lee el portapapeles, deja la imagen en `_staging` y emite el `evidence:captured` de F3; si no hay imagen, muestra una notificación nativa.
- **Nuevo caso:** muestra la ventana y emite `shortcut:new-case`.

En React, un diálogo **Ajustes** con pestañas y un grabador de combinaciones reemplaza a la preferencia local del tester de F4.

**Tech Stack:** Tauri 2.12, Rust 2024, rusqlite 0.40. Plugins nuevos: `tauri-plugin-global-shortcut` 2.4.1, `tauri-plugin-clipboard-manager` 2.5.0, `tauri-plugin-notification` 2.7.0 y `tauri-plugin-autostart` 2.7.0. Front: React 19, TypeScript 6, Zustand 5, zod 4, lucide-react, Vitest 5 y Testing Library.

**Spec:** [`docs/superpowers/specs/2026-10-15-qapture-design.md`](../specs/2026-10-15-qapture-design.md):
- §6.2 UC-11 (captura con atajo);
- §6.3 UC-12 (pegar evidencia);
- §6.8 UC-21 (ciclo de vida de la ventana);
- §6.9 UC-22 (Configuración);
- §8 (atajo ocupado);
- §9 (capabilities mínimas).

**Contrato:** [`docs/superpowers/contracts/F6-bandeja-atajos-settings.md`](../contracts/F6-bandeja-atajos-settings.md).

## Global Constraints

- Rama `feat/f6-bandeja-atajos-settings` desde `main` actualizado (F5 ya está en `main`, PR #6). Un commit por task, terminado con una línea en blanco y `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. **Sin push ni merge sin pedirlo.**
- Windows; comandos en Git Bash desde `qapture/` (Rust desde `qapture/src-tauri/`).
- Textos visibles **en español**. Nombres de los atajos: `Mostrar/ocultar` · `Capturar pantalla` · `Pegar evidencia` · `Nuevo caso`.
- Combinaciones por defecto: `Ctrl+Alt+Q` · `Ctrl+Alt+S` · `Ctrl+Alt+V` · `Ctrl+Alt+N`. Formato canónico de almacenamiento y de pantalla: modificadores en el orden `Ctrl`, `Alt`, `Shift`, `Win`, y luego la tecla (`A`–`Z`, `0`–`9`, `F1`–`F24`), unidos con `+`.
- Mensajes fijos de esta fase:
  - **Combinaciones inválidas:**
    - `«<atajo>»: combinación no válida.`
    - `«<atajo>»: usa al menos Ctrl, Alt o Win.`
    - `«<atajo>»: Windows reserva <combinación>.`
    - `«<a>» y «<b>» usan la misma combinación (<combinación>).`
  - **Rechazo del SO:** `No se pudo registrar <combinación> para «<atajo>»: otra aplicación ya la usa. Se conservan los atajos anteriores.` El toast al arrancar, si un atajo no se pudo registrar, dice: `No se pudo registrar <combinación> («<atajo>»): otra aplicación ya la usa. Cámbialo en Ajustes › Atajos.`
  - **Portapapeles:** notificación con título `Qapture` y cuerpo `El portapapeles no contiene una imagen.`
  - **Nuevo caso:** `Crea o abre una matriz para registrar casos.` (toast, si se usa sin proyecto abierto).
  - **Ajustes:**
    - `Ajustes guardados.`
    - `«Nombre del tester» supera 100 caracteres.`
    - `«Cargo» supera 100 caracteres.`
    - `«Correo» no es válido.`
    - `La carpeta de exportación no existe.`
    - `El ancho máximo de imagen debe estar entre 400 y 1600 px.` (el de F4)
- Combinaciones reservadas por Windows, siempre rechazadas: `Alt+F4`, `Ctrl+Alt+Delete`, `Win+L`, `Alt+Tab`, `Ctrl+Esc`, `Win+D`, `Win+E`, `Win+R`, `Win+Tab`.
- Inicio con Windows: el plugin arranca la app con el argumento `--hidden`. Solo en ese caso, y si «Iniciar oculto» está activo, la ventana no se muestra al arrancar.
- **ACL (F3):** cada comando nuevo va en `commands/names.rs`, `generate_handler!` y `capabilities/main.json`. Los plugins nuevos se usan **solo desde Rust**: ningún permiso JS de `global-shortcut`, `clipboard-manager`, `notification` ni `autostart` (spec §9).
- Dependencias nuevas, solo estas: los 4 plugins de *Tech Stack*. JS: ninguna.
- El fixture `src/infrastructure/tauri/__fixtures__/contracts.json` cambia y solo se regenera con `UPDATE_CONTRACTS=1 cargo test contract`.
- Cero warnings: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `pnpm typecheck`, `pnpm test` y `pnpm build` en verde al cerrar cada task.

## Review Focus

1. **Atajo ya usado por otra app al arrancar:** la app arranca igual, los demás atajos funcionan, aparece el toast y Ajustes › Atajos marca cuál falló → test en Task 2 (`apply` con un registrador que rechaza uno) y Task 6 (toast).
2. **Guardar atajos y que el SO rechace uno:** no cambia **ninguno**, los anteriores siguen funcionando y no se guarda nada → test en Task 2 (rollback completo) y Task 4 (`save_settings` no persiste).
3. **Ajustes guardados por una versión vieja o dañados** (JSON inválido, campos faltantes o desconocidos): la app arranca con el valor por defecto en cada campo que falte o sea inválido, sin perder el resto → test en Task 1.
4. **`Ctrl+Alt+V` con una imagen enorme o rara en el portapapeles** (más de 12000 px, sin canal alfa): si se puede normalizar, abre el editor; si no, muestra una notificación con el motivo, sin abrir una ventana vacía → test en Task 3.
5. **Un atajo mientras el editor ya tiene una evidencia sin guardar:** la nueva captura entra en la cola de F2 (`editorQueue`) y no reemplaza la abierta; «Nuevo caso» espera a que se cierre el editor → test en Task 7 («Nuevo caso con una evidencia en el editor no la descarta»). Para capturas y pegados ya existen los tests de `editorQueue` de F2/F3.

## Decisiones respecto al contrato (se anotan en el contrato en la Task 1)

- **Una fila JSON por sección** (`profile`, `shortcuts`, `system`, `export`) en `setting(key, value, updated_at)`. Cada campo tiene `#[serde(default)]`, así que agregar ajustes nuevos no requiere migraciones.
- **Tema claro/oscuro** (spec §6.9) queda fuera: no hay tema oscuro diseñado. Se anota para después de v1.
- **«Buscar actualizaciones»** (bandeja y Acerca de) llega con el updater en F7. En F6, Acerca de muestra la versión y los enlaces a releases y documentación.
- **El tester de Ajustes reemplaza la preferencia local de F4.** La primera vez que se cargan los ajustes, si el perfil está vacío y existe `localStorage['qapture.tester']`, se copia al perfil una sola vez. Un tester distinto escrito en el diálogo de exportación vale solo para esa exportación y no modifica Ajustes.
- **Cerrar con la X:** por defecto oculta en la bandeja. Con «Cerrar oculta en la bandeja» desactivado, cierra la app.
- **Esc oculta:** desactivado por defecto. Solo actúa si no hay un diálogo ni el editor abiertos.
- **Notificaciones nativas:** solo para eventos con la ventana oculta: pegar sin imagen, o una imagen del portapapeles que no se pudo usar. Con la ventana visible se usan los toasts de siempre.
- **Pendientes heredados de F5** (anotados en el PR #6) se resuelven en la Task 9:
  - la base bloqueada durante importaciones grandes;
  - el cambio de «Texto largo» a «Texto» con valores de más de 500 caracteres;
  - el teclado en el menú Datos.

## File Structure

```
qapture/
├─ package.json                                   (T10 versión)
├─ src/
│  ├─ domain/settings.ts  settings.test.ts        (T6) tipos, defaults, validación, formatAccelerator
│  ├─ ports/settings.ts  repositories.ts          (T6) puerto SettingsStore
│  ├─ infrastructure/tauri/settings.ts  dto.ts  repositories.ts  contract.test.ts   (T6)
│  ├─ infrastructure/memory/settings.ts  repositories.ts                            (T6)
│  └─ ui/
│     ├─ store/settings-slice.ts  settings-slice.test.ts  workspace.ts  export-slice.ts  preferences.ts  (T6)
│     ├─ screens/AppSettingsDialog.tsx  ShortcutRecorder.tsx  AppSettings.test.tsx  (T7)
│     ├─ screens/TopBar.tsx  AppShell.tsx  Workspace.tsx  CaseFormDialog.tsx  ExportDialog.tsx  (T7)
│     ├─ screens/DataMenu.tsx  Data.test.tsx      (T9)
│     └─ test-utils.tsx                           (T6) opción `tester` siembra los ajustes en memoria
└─ src-tauri/
   ├─ Cargo.toml  Cargo.lock  tauri.conf.json      (T3, T5, T10)
   ├─ capabilities/main.json                       (T4)
   └─ src/
      ├─ db/migrations/0004_setting.sql  db/mod.rs (T1)
      ├─ settings.rs                               (T1) modelo + defaults + validación
      ├─ repo/settings.rs  repo/mod.rs             (T1)
      ├─ system/shortcuts.rs                       (T2) lógica pura + ShortcutRegistrar
      ├─ system/clipboard_paste.rs                 (T3) portapapeles → _staging
      ├─ system/tray.rs                            (T3) menú: Capturar, Pegar evidencia, Nuevo caso
      ├─ system/runtime.rs                         (T5) plugins, despacho de atajos, inicio oculto, cierre
      ├─ commands/settings.rs  names.rs  mod.rs    (T4)
      ├─ dto.rs  state.rs  lib.rs                  (T1, T4, T5)
      ├─ repo/fields.rs                            (T9) no pasar a Texto con valores largos
      └─ commands/data.rs  transfer/legacy.rs  transfer/package.rs   (T9) base bloqueada solo al insertar
```

Correspondencia con el contrato:

| Contrato | Tasks |
|---|---|
| F6-T2 (resto del menú) | Tasks 3 y 5 |
| F6-T3 | Tasks 1, 4 y 6 |
| F6-T4 | Tasks 2 y 5 |
| F6-T5 | Tasks 3 y 5 |
| F6-T6 | Tasks 5 y 7 |
| F6-T7 | Task 7 |
| F6-T8 | Task 5 |
| F6-T9 | Tasks 6 y 7 |

---

### Task 1: Rama, documentos, migración `0004` y ajustes persistidos (Rust)

**Files:**
- Create: `src-tauri/src/db/migrations/0004_setting.sql`, `src-tauri/src/settings.rs`, `src-tauri/src/repo/settings.rs`
- Modify: `src-tauri/src/db/mod.rs` (`MIGRATIONS`), `src-tauri/src/repo/mod.rs`, `src-tauri/src/lib.rs` (`mod settings;`), `docs/superpowers/contracts/README.md`, `docs/superpowers/contracts/F6-bandeja-atajos-settings.md`; add este plan.

**Interfaces:**
- Produces:
  - Tabla `setting (key TEXT PRIMARY KEY, value TEXT NOT NULL, updated_at TEXT NOT NULL)`.
  - `settings::Settings { profile: Profile, shortcuts: Shortcuts, system: SystemPrefs, export: ExportPrefs }`. Todo con `Serialize + Deserialize`, `rename_all = "camelCase"`, `#[serde(default)]` en cada struct y `Default` con los valores del contrato:
    - `Profile { tester: String, role: String, email: String }`, todo `""`.
    - `Shortcuts { toggle, capture, paste, new_case: String }`, con los 4 por defecto.
    - `SystemPrefs { autostart: bool = false, start_hidden: bool = true, close_to_tray: bool = true, esc_hides: bool = false }`.
    - `ExportPrefs { default_folder: String = "", image_max_width: u32 = 940, include_summary: bool = true }`.
  - `settings::validate(&Settings) -> AppResult<Settings>`: recorta textos y devuelve el primer error con los mensajes del contrato. No valida atajos (Task 2) ni la existencia de la carpeta (la revisa la Task 4).
  - `repo::settings::{load(conn) -> AppResult<Settings>, save(conn, &Settings) -> AppResult<()>}`. `load` lee cada sección por separado: una fila ausente, un JSON inválido o un campo de tipo erróneo dejan esa sección (o ese campo) con su valor por defecto, sin fallar.

- [ ] **Step 1: Rama y documentos**

```bash
git checkout main && git pull && git checkout -b feat/f6-bandeja-atajos-settings
```

- En `README.md` de contratos, la fila F6 pasa a `[plan](../plans/2026-10-17-f6-bandeja-atajos-settings.md) | 🟡 En curso`.
- En el contrato F6, la cabecera enlaza el plan, y se agregan las secciones «Notas de implementación (acordadas en el plan)» (los puntos de *Decisiones*) e «Interfaces producidas» (comandos de la Task 4 y el evento `shortcut:new-case`).

- [ ] **Step 2: Tests (fallan)** en `repo/settings.rs`:

```rust
#[test] fn a_new_db_returns_the_defaults()
// load(&conn()) == Settings::default(); default().shortcuts.toggle == "Ctrl+Alt+Q", .new_case == "Ctrl+Alt+N",
// default().export.image_max_width == 940, default().system.close_to_tray == true

#[test] fn saved_settings_survive_reopening_the_file_db()
// db::open(tmp/qapture.db) → save(tester "Ana Pérez", capture "Ctrl+Shift+F12", autostart true) → drop
// → db::open again → load == lo guardado

#[test] fn damaged_or_partial_rows_fall_back_per_field()
// INSERT setting('profile', '{"tester":"Ana"}'), ('system', 'no es json'), ('shortcuts', '{"toggle":7,"capture":"Ctrl+Alt+X"}')
// → load: profile.tester == "Ana", profile.role == "", system == SystemPrefs::default(),
//   shortcuts.capture == "Ctrl+Alt+X", shortcuts.toggle == "Ctrl+Alt+Q" (tipo erróneo → defecto)

#[test] fn settings_survive_the_0004_migration()
// db v3 (MIGRATIONS[..3], user_version 3, un proyecto) → db::open migra a 4 → el proyecto sigue y load == default
```

En `settings.rs`:

```rust
#[test] fn validate_trims_and_rejects_with_fixed_messages()
// tester "  Ana  " → "Ana"; tester de 101 chars → "«Nombre del tester» supera 100 caracteres."
// email "ana@" → "«Correo» no es válido."; email "" ok; email "ana@acme.com" ok
// image_max_width 399 / 1601 → "El ancho máximo de imagen debe estar entre 400 y 1600 px."
```

Run: `cd src-tauri && cargo test settings` → FAIL (no existen).

- [ ] **Step 3: Implementar**
  - La migración crea la tabla.
  - `load` deserializa cada fila a `serde_json::Value` y luego a la sección. Para que un campo de tipo erróneo no tumbe la sección, combina campo a campo sobre `Default` (por ejemplo, serializa el default a `Value`, superpone las claves cuyo tipo coincide y deserializa).
  - El correo es válido si tiene exactamente una `@` con texto antes y un punto después.

- [ ] **Step 4:** `cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check` → PASS. El test de F1 `migrations_are_idempotent` sigue en verde con 4 migraciones.

- [ ] **Step 5: Commit** `feat(f6): ajustes persistidos por sección con valores por defecto tolerantes`

---

### Task 2: Lógica de atajos: normalizar, validar y aplicar con rollback (`system/shortcuts.rs`)

**Files:** Create `src-tauri/src/system/shortcuts.rs`; Modify `src-tauri/src/system/mod.rs`.

**Interfaces:**
- Consumes: `settings::Shortcuts` (Task 1).
- Produces:
  - `ShortcutAction::{Toggle, Capture, Paste, NewCase}` con `label() -> &'static str` (los 4 nombres del contrato) y `ALL: [ShortcutAction; 4]`.
  - `normalize(raw: &str) -> Option<String>`: acepta mayúsculas o minúsculas, espacios, `Control`/`Ctrl`, `Super`/`Win`/`Meta` y modificadores en cualquier orden, y devuelve la forma canónica, o `None` si no hay tecla válida o se repite un modificador.
  - `validate(&Shortcuts) -> AppResult<Shortcuts>`: canónicas, con los mensajes del contrato y en este orden: no válida, falta Ctrl/Alt/Win, reservada, duplicada.
  - `trait ShortcutRegistrar { fn register(&mut self, accel: &str) -> Result<(), String>; fn unregister(&mut self, accel: &str); }`.
  - `apply(reg: &mut impl ShortcutRegistrar, current: &Shortcuts, wanted: &Shortcuts) -> AppResult<()>`: desregistra solo las combinaciones que cambian y registra las nuevas. Si una falla, desregistra las nuevas que ya entraron, vuelve a registrar las anteriores y devuelve el mensaje de rechazo del SO.
  - `register_all(reg, &Shortcuts) -> Vec<(ShortcutAction, String)>`: al arrancar registra las que puede y devuelve las que fallaron. No revierte nada.
  - `action_for(accel: &str, current: &Shortcuts) -> Option<ShortcutAction>`.

- [ ] **Step 1: Tests (fallan)**, con un `FakeRegistrar { registered: BTreeSet<String>, refuse: BTreeSet<String>, log: Vec<String> }`:

```rust
#[test] fn normalize_gives_the_canonical_form()
// "alt+ctrl+q" → "Ctrl+Alt+Q"; " Control + Shift + f12 " → "Ctrl+Shift+F12"; "Super+Alt+1" → "Alt+Win+1"
// "Ctrl+Alt" → None; "Ctrl+Ctrl+Q" → None; "Ctrl+Alt+Ñ" → None; "Ctrl+Alt+F25" → None

#[test] fn validate_reports_the_first_problem_with_fixed_messages()
// capture "Shift+S" → "«Capturar pantalla»: usa al menos Ctrl, Alt o Win."
// toggle "Alt+F4" → "«Mostrar/ocultar»: Windows reserva Alt+F4."
// paste == capture "Ctrl+Alt+S" → "«Capturar pantalla» y «Pegar evidencia» usan la misma combinación (Ctrl+Alt+S)."
// new_case "Ctrl+" → "«Nuevo caso»: combinación no válida."
// defaults → Ok(defaults); "alt+ctrl+q" se devuelve canónico

#[test] fn apply_only_touches_changed_combinations()
// current = defaults (registrados); wanted.toggle = "Ctrl+Shift+F12"
// → log == ["unregister Ctrl+Alt+Q", "register Ctrl+Shift+F12"]; registered contiene F12 y no Ctrl+Alt+Q

#[test] fn a_refused_combination_rolls_everything_back()
// wanted cambia toggle a "Ctrl+Shift+F12" y capture a "Ctrl+Alt+P"; refuse = {"Ctrl+Alt+P"}
// → Err("No se pudo registrar Ctrl+Alt+P para «Capturar pantalla»: otra aplicación ya la usa. Se conservan los atajos anteriores.")
// → registered == los 4 defaults exactos (F12 se quitó, Ctrl+Alt+Q volvió)

#[test] fn register_all_keeps_going_and_reports_failures()
// refuse = {"Ctrl+Alt+V"} → devuelve [(Paste, "Ctrl+Alt+V")]; los otros 3 quedan registrados

#[test] fn action_for_maps_the_pressed_combination()
// action_for("Ctrl+Alt+S", &defaults) == Some(Capture); "Ctrl+Alt+Z" → None
```

Run: `cargo test system::shortcuts` → FAIL.

- [ ] **Step 2: Implementar.**
  - `normalize`: separa por `+`, recorta, clasifica cada parte (modificador o tecla) sin distinguir mayúsculas y reconstruye en el orden canónico.
  - Teclas válidas: una letra ASCII, un dígito o `F1`–`F24`.
  - El rollback de `apply` deja el estado del registrador idéntico al de antes. Los tests 3 y 4 lo prueban.

- [ ] **Step 3:** `cargo test system::shortcuts` → PASS. Gates de Rust en verde.

- [ ] **Step 4: Commit** `feat(f6): validación y aplicación de atajos globales con rollback ante conflictos`

---

### Task 3: Pegar desde el portapapeles y menú de bandeja ampliado (lógica pura)

**Files:** Create `src-tauri/src/system/clipboard_paste.rs`; Modify `src-tauri/src/system/tray.rs`, `src-tauri/src/system/mod.rs`, `src-tauri/Cargo.toml` (agrega `tauri-plugin-clipboard-manager = "2.5.0"`, `tauri-plugin-notification = "2.7.0"`).

**Interfaces:**
- Consumes: `media::raster::normalize`, `repo::evidence::stage(files, &[u8]) -> AppResult<StagedImageDto>` (F2).
- Produces:
  - `clipboard_paste::rgba_to_png(width: u32, height: u32, rgba: &[u8]) -> AppResult<Vec<u8>>`. Error de validación si `rgba.len() != width*height*4`.
  - `clipboard_paste::PasteOutcome::{Staged(StagedImageDto), NoImage, Unusable(String)}`.
  - `clipboard_paste::stage_from_clipboard(files, image: Option<(u32, u32, Vec<u8>)>) -> PasteOutcome`. `None` → `NoImage`. Una imagen que `raster::normalize` rechaza → `Unusable(<mensaje de normalize>)`, por ejemplo una de más de 12000 px.
  - `clipboard_paste::NO_IMAGE = "El portapapeles no contiene una imagen."`.
  - `tray::TrayAction` suma `Capture`, `Paste` y `NewCase` (ids de menú `capture`, `paste`, `new-case`). El menú queda así: `Abrir Qapture`, `Ocultar`, separador, `Capturar pantalla`, `Pegar evidencia`, `Nuevo caso`, separador, `Salir`.

- [ ] **Step 1: Tests (fallan)**

```rust
// clipboard_paste.rs
#[test] fn rgba_from_the_clipboard_becomes_a_staged_png()
// stage_from_clipboard(files, Some((80, 60, vec![255; 80*60*4]))) → Staged(s); s.width == 80; el archivo en s.path es PNG
#[test] fn an_empty_clipboard_is_reported_not_staged()
// None → NoImage; staging vacío
#[test] fn an_oversized_or_malformed_image_is_unusable()
// (12_001, 1, vec![0; 12_001*4]) → Unusable(m) con m == "La imagen mide 12001×1 px; el máximo es 12000 px por lado."
// (10, 10, vec![0; 7]) → Unusable(_); staging vacío en ambos casos

// tray.rs (amplía el test existente)
#[test] fn menu_ids_map_to_the_expected_actions()
// "capture" → Capture, "paste" → Paste, "new-case" → NewCase, más los de F1
```

Run: `cargo test clipboard_paste tray` → FAIL.

- [ ] **Step 2: Implementar.** `rgba_to_png` arma un `image::RgbaImage` y lo codifica como PNG. `stage_from_clipboard` pasa el PNG por `evidence::stage`, que ya normaliza a máximo 2400 px. En el `.menu` de la bandeja, los tres ids nuevos llaman a una función `on_action(app, TrayAction)` que la Task 5 conecta. Por ahora, `Capture`/`Paste`/`NewCase` solo muestran la ventana.

- [ ] **Step 3:** tests en verde; gates de Rust.

- [ ] **Step 4: Commit** `feat(f6): imagen del portapapeles a _staging y menú de bandeja con captura, pegar y nuevo caso`

---

### Task 4: Comandos de Ajustes, ACL y contrato

**Files:** Create `src-tauri/src/commands/settings.rs`; Modify `commands/mod.rs`, `commands/names.rs`, `lib.rs` (`generate_handler!`), `capabilities/main.json`, `state.rs`, `dto.rs` (contract samples), `src/infrastructure/tauri/__fixtures__/contracts.json` (regenerado).

**Interfaces:**
- Consumes: `settings::{Settings, validate}`, `repo::settings`, `system::shortcuts::{validate, apply, ShortcutAction}`.
- Produces:
  - `AppState`:
    - `settings() -> Settings` (clon desde un `RwLock`) y `set_settings(Settings)`.
    - `shortcut_failures() -> Vec<ShortcutFailureDto>` y `set_shortcut_failures(..)`.
    - Un registrador `Mutex<Box<dyn ShortcutRegistrar + Send>>` que la Task 5 instala con `install_registrar(..)`. Hasta entonces se usa uno nulo que acepta todo.
  - DTOs:
    - `SettingsDto { settings: Settings, shortcutFailures: ShortcutFailureDto[], appVersion: string, dataDir: string }`;
    - `ShortcutFailureDto { action: "toggle"|"capture"|"paste"|"newCase", accelerator: string }`.
  - Comandos:
    - `get_settings() -> SettingsDto`;
    - `save_settings({ settings }) -> SettingsDto`;
    - `pick_export_folder() -> string | null` («Seleccionar carpeta» nativo desde Rust);
    - `open_data_folder() -> null`;
    - `hide_main_window() -> null`;
    - `open_link({ target: "releases" | "docs" }) -> null`, con URLs fijas: `https://github.com/Ps3udoDev/qapture/releases` y `https://github.com/Ps3udoDev/qapture#readme`.
  - `save_settings` funciona así:
    1. `ensure_ready`.
    2. `settings::validate`.
    3. Comprueba que `default_folder` vacía o existente sea una carpeta; si no, `La carpeta de exportación no existe.`
    4. `shortcuts::validate`.
    5. `shortcuts::apply` contra los actuales; si falla, devuelve el error **sin guardar nada**.
    6. Guarda y llama a `set_settings`.
    7. Aplica el autostart (lo conecta la Task 5 mediante un hook `on_saved` en `AppState`; hasta entonces no hace nada).

- [ ] **Step 1: Tests (fallan)**
  - En `commands/settings.rs`, la lógica de guardado se extrae a `pub(crate) fn save(state: &AppState, input: Settings) -> AppResult<SettingsDto>` para probarla sin Tauri. Los tests usan `AppState::new(db::open(tmp)…)` con un `FakeRegistrar` instalado:

```rust
#[test] fn save_persists_validated_settings_and_applies_shortcuts()
// tester "  Ana " , toggle "ctrl+shift+f12" → dto.settings.profile.tester == "Ana", .shortcuts.toggle == "Ctrl+Shift+F12";
// repo::settings::load == dto.settings; registrador contiene "Ctrl+Shift+F12"
#[test] fn a_refused_shortcut_saves_nothing()
// refuse {"Ctrl+Shift+F12"} → Err(mensaje de rechazo); load == defaults; state.settings() == defaults
#[test] fn a_missing_export_folder_is_rejected()
// default_folder "C:\\no\\existe" → Err("La carpeta de exportación no existe.")
#[test] fn saving_is_blocked_in_recovery_mode()
// AppState con recovery Some → Err(mensaje de ensure_ready de F5)
```

  - Más los tests de ACL de `names.rs` y del contrato: claves nuevas `settings` (un `SettingsDto` de ejemplo con un fallo de `capture`) y `inputs.saveSettings`.

- [ ] **Step 2: Implementar** los comandos, los permisos `allow-*` de los 6 comandos y la regeneración del fixture (`UPDATE_CONTRACTS=1 cargo test contract`).

- [ ] **Step 3:** gates de Rust en verde.

- [ ] **Step 4: Commit** `feat(f6): comandos de ajustes con validación, rollback de atajos y ACL`

---

### Task 5: Arranque y segundo plano: plugins, despacho de atajos, inicio oculto y cierre

**Files:** Create `src-tauri/src/system/runtime.rs`; Modify `src-tauri/src/lib.rs`, `src-tauri/Cargo.toml` (agrega `tauri-plugin-global-shortcut = "2.4.1"`, `tauri-plugin-autostart = "2.7.0"`), `src-tauri/tauri.conf.json` (`"visible": false` en la ventana `main`), `system/tray.rs` (conecta `on_action`).

**Interfaces:**
- Consumes: Tasks 1–4; `CaptureController::start` y `capture::windows::{TauriHost, CAPTURED_EVENT}` (F3); `tray::{show_main, toggle_main}`.
- Produces:
  - `runtime::NEW_CASE_EVENT = "shortcut:new-case"`, evento sin payload hacia la ventana `main`.
  - `runtime::should_start_hidden(args: &[String], prefs: &SystemPrefs) -> bool`: `true` solo con `--hidden` en los argumentos y `start_hidden` activo.
  - `runtime::close_action(prefs: &SystemPrefs) -> CloseAction::{Hide, Exit}`.
  - `runtime::dispatch(app, ShortcutAction)`:
    - `Toggle` → `toggle_main`.
    - `Capture` → el mismo camino que `start_capture` (F3).
    - `Paste` → lee la imagen con `clipboard().read_image()` y llama a `stage_from_clipboard`. Con `Staged`, `show_main` y emite `CAPTURED_EVENT` con la ruta. Con `NoImage` o `Unusable(m)`, muestra una notificación nativa (título `Qapture`, cuerpo `NO_IMAGE` o `m`) y **no muestra la ventana**.
    - `NewCase` → `show_main` y emite `NEW_CASE_EVENT`.
  - El menú de bandeja usa el mismo `dispatch`.
  - `TauriRegistrar` implementa `ShortcutRegistrar` sobre `app.global_shortcut()`. Convierte la forma canónica al formato del plugin (`Win` → `Super`) y lo cubre un test de conversión.

- [ ] **Step 1: Tests (fallan)** en `runtime.rs`, solo de lógica pura:

```rust
#[test] fn starts_hidden_only_when_launched_by_autostart_with_the_option_on()
// (["qapture.exe","--hidden"], start_hidden true) → true; sin "--hidden" → false; con "--hidden" y start_hidden false → false
#[test] fn closing_hides_or_exits_according_to_the_setting()
// close_to_tray true → Hide; false → Exit
#[test] fn canonical_accelerators_convert_to_the_plugin_format()
// "Ctrl+Alt+Q" y "Alt+Win+1" y "Ctrl+Shift+F12" → parsean con tauri_plugin_global_shortcut::Shortcut::from_str sin error
```

- [ ] **Step 2: Implementar en `lib.rs`**
  - Registra los plugins: `global_shortcut` con un `with_handler` que, ante `ShortcutState::Pressed`, busca `action_for` sobre `state.settings().shortcuts` y llama a `dispatch`; además `clipboard_manager`, `notification` y `autostart` con args `["--hidden"]`.
  - En `setup`, después de crear el estado:
    1. `repo::settings::load` → `set_settings`;
    2. instala `TauriRegistrar`;
    3. `register_all` → `set_shortcut_failures`;
    4. sincroniza el autostart con `system.autostart` (`enable`/`disable`; un error solo se registra en el log);
    5. muestra la ventana salvo que `should_start_hidden`.
  - `on_window_event` de `main` usa `close_action(&state.settings().system)`: `Exit` → `app.exit(0)`.
  - El hook `on_saved` de la Task 4 sincroniza el autostart al guardar.

- [ ] **Step 3:** gates de Rust en verde. Smoke con `pnpm tauri dev`:
  - `Ctrl+Alt+Q` alterna la ventana.
  - `Win+Shift+S` → `Ctrl+Alt+V` abre el editor con la captura.
  - `Ctrl+Alt+V` con texto en el portapapeles muestra la notificación sin abrir la ventana.
  - `Ctrl+Alt+S` abre el overlay de F3.

- [ ] **Step 4: Commit** `feat(f6): atajos globales, pegar desde el portapapeles, inicio oculto y cierre configurables`

---

### Task 6: Ajustes en el front: dominio, puerto, adaptadores y store

**Files:**
- Create: `src/domain/settings.ts` (+ test), `src/ports/settings.ts`, `src/infrastructure/tauri/settings.ts`, `src/infrastructure/memory/settings.ts`, `src/ui/store/settings-slice.ts` (+ test)
- Modify: `ports/repositories.ts`, `infrastructure/tauri/{dto,repositories,contract.test}.ts`, `infrastructure/memory/repositories.ts`, `ui/store/{workspace,export-slice,preferences}.ts`, `ui/test-utils.tsx`

**Interfaces:**
- Consumes: los comandos y el fixture de la Task 4.
- Produces:
  - `domain/settings.ts`:
    - Tipos espejo de Rust: `Settings`, `ShortcutAction` (`"toggle" | "capture" | "paste" | "newCase"`), `ShortcutFailure`, `SettingsSnapshot { settings, shortcutFailures, appVersion, dataDir }`.
    - `DEFAULT_SETTINGS`.
    - `SHORTCUT_LABEL: Record<ShortcutAction, string>`.
    - `validateSettings(s) -> FormErrors`, con los mismos mensajes, más los de duplicado y «usa al menos Ctrl, Alt o Win».
    - `formatAccelerator(e: { ctrlKey, altKey, shiftKey, metaKey, code }) -> string | null`: forma canónica desde un `KeyboardEvent`, `null` si solo hay modificadores o la tecla no es válida (`code` `KeyQ` → `Q`, `Digit1` → `1`, `F12` → `F12`).
    - `failureMessage(f) -> string`: el toast de arranque del contrato.
  - `ports/settings.ts` `SettingsStore`:
    - `load(): Promise<SettingsSnapshot>`, `save(s): Promise<SettingsSnapshot>`;
    - `pickExportFolder(): Promise<string | null>`, `openDataFolder()`, `hideWindow()`, `openLink(target)`;
    - `onNewCase(handler): Promise<() => void>`.
    - `Repositories.settings: SettingsStore`.
  - Memoria `MemorySettings`, con controles para los tests:
    - `refuse: Set<string>` simula el rechazo del SO con el mismo mensaje;
    - `failuresAtStart`;
    - `hidden: number`;
    - `emitNewCase()`.
  - Store `SettingsSlice`:
    - `settings: SettingsSnapshot | null`;
    - `loadSettings()` (se llama en `init`, también en modo recuperación);
    - `saveSettings(s): Promise<FormResult>`, con toast `Ajustes guardados.`;
    - `testerName(): string`, que reemplaza a `rememberedTester` de F4;
    - `caseFormRequest: { caseId: string | null } | null` con `openCaseForm(caseId?)` y `closeCaseForm()`;
    - `listenNewCase()`: con proyecto abierto, `openCaseForm(null)`; sin proyecto, el toast del contrato.
  - `init` también:
    - muestra un toast de error por cada fallo de atajo al arrancar;
    - migra el tester local una sola vez: si el perfil está vacío y `prefs.getTester()` no, guarda los ajustes con ese tester. `Preferences` conserva `getTester` solo para esto y pierde `setTester`.
  - Exportación: `ExportDialog` toma `imageMaxWidth`/`includeSummary` por defecto de los ajustes, y `exportExcel` ya **no** escribe el tester en ninguna preferencia.
  - `renderApp({ tester })` siembra `profile.tester` en los ajustes en memoria.

- [ ] **Step 1: Tests (fallan)**

```ts
// domain/settings.test.ts
it("formatAccelerator da la forma canónica")
// {ctrlKey,altKey,code:"KeyQ"} → "Ctrl+Alt+Q"; {ctrlKey,shiftKey,code:"F12"} → "Ctrl+Shift+F12";
// {altKey,metaKey,code:"Digit1"} → "Alt+Win+1"; {ctrlKey,code:"ControlLeft"} → null; {ctrlKey,code:"BracketLeft"} → null
it("validateSettings replica las reglas de Rust")
// capture "Shift+S" → errors["shortcuts.capture"] == "«Capturar pantalla»: usa al menos Ctrl, Alt o Win."
// paste == capture → errors["shortcuts.paste"] == "«Capturar pantalla» y «Pegar evidencia» usan la misma combinación (Ctrl+Alt+S)."
// imageMaxWidth 300 → "El ancho máximo de imagen debe estar entre 400 y 1600 px."

// ui/store/settings-slice.test.ts
it("init carga los ajustes y avisa los atajos que no se registraron")
// failuresAtStart [{action:"paste", accelerator:"Ctrl+Alt+V"}] → toast "No se pudo registrar Ctrl+Alt+V («Pegar evidencia»): otra aplicación ya la usa. Cámbialo en Ajustes › Atajos."
it("migra una sola vez el tester guardado en el equipo")
// prefs tester "Luis", ajustes vacíos → settings.profile.tester == "Luis"; segundo init no vuelve a guardar
it("un atajo rechazado deja el error en el formulario y no cambia los ajustes")
// refuse {"Ctrl+Alt+P"}; saveSettings(capture "Ctrl+Alt+P") → { ok:false, errors:{ _form: <mensaje de rechazo> } }; settings sin cambio
it("Nuevo caso desde el atajo abre el formulario o avisa sin proyecto")
// sin proyecto: emitNewCase → toast "Crea o abre una matriz para registrar casos."; con proyecto: caseFormRequest == { caseId: null }
```

  - Además se ajustan los tests de F4 que usan `rememberedTester` o `prefs.setTester`, y se agregan al contrato las claves `settings` e `inputs.saveSettings`.

- [ ] **Step 2: Implementar.** Gates de front (`pnpm typecheck && pnpm test && pnpm build`) en verde.

- [ ] **Step 3: Commit** `feat(f6): ajustes en el front con migración del tester y aviso de atajos ocupados`

---

### Task 7: Pantalla Ajustes, grabador de combinaciones y comportamientos de la ventana

**Files:**
- Create: `src/ui/screens/AppSettingsDialog.tsx`, `src/ui/screens/ShortcutRecorder.tsx`, `src/ui/screens/AppSettings.test.tsx`
- Modify: `TopBar.tsx` (botón con ícono `SlidersHorizontal`, `aria-label="Ajustes"`), `AppShell.tsx`, `Workspace.tsx` (el formulario de caso lo abre `caseFormRequest`), `CaseFormDialog.tsx`, `ExportDialog.tsx`

**Interfaces:**
- Consumes: `SettingsSlice` (Task 6), `BackupsDialog` y las acciones de Datos (F5).
- Produces:
  - `<AppSettingsDialog onClose />` con pestañas (`role="tab"`) `Perfil`, `Atajos`, `Sistema`, `Exportación`, `Datos` y `Acerca de`, y un botón `Guardar` que guarda todo junto:
    - **Perfil:** `Nombre del tester`, `Cargo`, `Correo`.
    - **Atajos:** una fila por acción con `<ShortcutRecorder>`, el error de conflicto de esa fila y una marca `No registrado` para los que fallaron al arrancar.
    - **Sistema:** casillas `Iniciar con Windows`, `Iniciar oculto`, `Cerrar oculta en la bandeja` y `Esc oculta la ventana`.
    - **Exportación:** `Carpeta por defecto` (texto de solo lectura con los botones `Elegir…` y `Quitar`), `Ancho máximo de imagen (px)` e `Incluir hoja Resumen`.
    - **Datos:** `Abrir carpeta de datos` (con la ruta visible), `Respaldos…` (abre el `BackupsDialog` de F5) e `Importar Bitácora v1…`.
    - **Acerca de:** `Qapture <versión>` y los enlaces `Releases` y `Documentación`.
  - `<ShortcutRecorder value onChange label />`: un botón con `aria-label="Grabar «<atajo>»"` que, mientras graba, muestra `Presiona la combinación…`, captura `keydown` con `formatAccelerator`, `Esc` cancela y `Retroceso` restaura el valor por defecto.
  - `CaseFormDialog` nuevo precarga la columna con clave `responsable` con `testerName()` si la plantilla la tiene.
  - Escape global: si `system.escHides` está activo y no hay ningún `[role=dialog]` en el documento ni el editor está abierto, llama a `hideWindow()`.
  - `AppShell` monta `listenNewCase` igual que `listenCaptures`.

- [ ] **Step 1: Tests (fallan)** en `AppSettings.test.tsx`:

```tsx
it("graba una combinación y la guarda")
// abrir Ajustes › Atajos; clic en "Grabar «Mostrar/ocultar»"; keyDown {ctrlKey, shiftKey, code:"F12"} → muestra "Ctrl+Shift+F12";
// Guardar → toast "Ajustes guardados."; repos.settings guardó toggle "Ctrl+Shift+F12"
it("muestra el conflicto en su fila y no cierra")
// grabar Capturar = Ctrl+Alt+V → texto "«Capturar pantalla» y «Pegar evidencia» usan la misma combinación (Ctrl+Alt+V)." en la fila; sin llamada a save
it("marca los atajos que no se registraron al arrancar")
// failuresAtStart paste → en la fila de Pegar evidencia aparece "No registrado"
it("elige la carpeta de exportación y la usa el diálogo de exportar")
// Elegir… (memoria devuelve "C:\\Exports") → se ve la ruta; ancho 1200; Guardar; abrir Exportar → el export usa imageMaxWidth 1200
it("el nuevo caso precarga el Responsable con el tester del perfil")
// renderApp({ tester: "Ana Pérez" }) + proyecto; "Nuevo caso" → campo Responsable con "Ana Pérez"
it("Esc oculta la ventana solo si la opción está activa y no hay diálogos")
// escHides false → Escape no llama hideWindow; true → la llama una vez; con un diálogo abierto → no
it("Nuevo caso con una evidencia en el editor no la descarta")
// editor abierto con una imagen en staging → emitNewCase → el editor sigue abierto y repos.evidence no recibió discardStaged;
// al cancelar/guardar el editor aparece el formulario "Nuevo caso"
```

  - `Workspace` muestra el formulario de `caseFormRequest` solo cuando no hay editor abierto. Las capturas y pegados que llegan con el editor abierto ya los encola `editorQueue` (tests de F2/F3).

- [ ] **Step 2: Implementar.** Gates de front en verde.

- [ ] **Step 3: Smoke** con `pnpm tauri dev`: las pruebas 1–6 del checklist del contrato.

- [ ] **Step 4: Commit** `feat(f6): pantalla Ajustes con grabador de atajos, Responsable precargado y Esc para ocultar`

---

### Task 8: Notificación de atajos fallidos con la ventana oculta y verificación del inicio con Windows

**Files:** Modify `src-tauri/src/system/runtime.rs`, `src-tauri/src/lib.rs`.

**Interfaces:**
- Consumes: `register_all` (Task 2) y `should_start_hidden` (Task 5).
- Produces: si la app arranca oculta (autostart) y algún atajo falló, una notificación nativa con el mismo texto que el toast. Con la ventana visible, solo el toast de la Task 6.

- [ ] **Step 1: Test (falla)** en `runtime.rs`:

```rust
#[test] fn failures_are_notified_only_when_starting_hidden()
// startup_notifications(&[(Paste,"Ctrl+Alt+V")], hidden=true) == ["No se pudo registrar Ctrl+Alt+V («Pegar evidencia»): otra aplicación ya la usa. Cámbialo en Ajustes › Atajos."]
// hidden=false → []
```

- [ ] **Step 2: Implementar** `startup_notifications(failures, hidden) -> Vec<String>` y enviarlas con el plugin de notificaciones en `setup`.

- [ ] **Step 3:** gates en verde. Prueba 7 del checklist con el build de release: activar «Iniciar con Windows», reiniciar y comprobar que Qapture queda en la bandeja sin ventana.

- [ ] **Step 4: Commit** `feat(f6): notificar atajos no registrados al iniciar con Windows`

---

### Task 9: Pendientes heredados de F5

**Files:** Modify `src-tauri/src/repo/fields.rs`, `src-tauri/src/transfer/legacy.rs`, `src-tauri/src/transfer/package.rs`, `src-tauri/src/commands/data.rs`, `src/ui/screens/DataMenu.tsx`, `src/ui/screens/Data.test.tsx`.

**Interfaces:**
- Produces:
  - **Columnas:** `repo::fields::save` rechaza cambiar una columna `longtext` con datos a `text` si algún valor supera 500 caracteres, con el mensaje `No se puede cambiar «<columna>» a Texto: tiene valores de más de 500 caracteres.`
  - **v1:** `legacy::prepare(template: &[BundleField], files: &EvidenceFiles, bytes: &[u8]) -> AppResult<(Vec<ProjectBundle>, ImportFiles)>` hace parse, decodificación y escritura de imágenes **sin** conexión. `legacy::import` queda como `prepare` + `bundle::insert` para los tests existentes.
  - **Paquete:** `package::prepare(files, path) -> AppResult<(ProjectBundle, ImportFiles)>`, mismo patrón; `package::import` sigue igual hacia afuera.
  - **Comandos:** `import_legacy` e `import_package` bloquean la base solo para `template_fields` y para `bundle::insert`.
  - **Menú Datos:**
    - al abrirse enfoca el primer ítem habilitado;
    - `ArrowDown`/`ArrowUp` recorren los ítems habilitados de forma circular;
    - `Home`/`End` van al primero y al último;
    - `Escape` cierra y devuelve el foco al botón «Datos».

- [ ] **Step 1: Tests (fallan)**

```rust
// repo/fields.rs
#[test] fn longtext_with_long_values_cannot_become_text()
// columna datos_entrada (longtext) con un valor de 600 chars → save con type text → Err("No se puede cambiar «Datos de entrada» a Texto: tiene valores de más de 500 caracteres.")
// con valores ≤ 500 → Ok
```

```tsx
// Data.test.tsx
it("el menú Datos se maneja con el teclado")
// abrir → foco en "Importar Bitácora v1…"; ArrowDown → "Importar paquete .qapture…"; ArrowUp ×2 → "Respaldos…" (circular, salta deshabilitados)
// Escape → menú cerrado y foco en el botón "Datos"
```

  - Los tests existentes de `transfer::legacy` y `transfer::package` siguen pasando sin cambios. Son la red de seguridad del refactor de `prepare`.

- [ ] **Step 2: Implementar.** Gates de Rust y de front en verde.

- [ ] **Step 3: Commit** `fix(f6): pendientes de F5 — columnas largas, base libre durante importaciones y teclado en el menú Datos`

---

### Task 10: Verificación integral, versión `0.6.0` y cierre técnico del contrato

**Files:** Modify `package.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `src-tauri/tauri.conf.json`, `docs/superpowers/contracts/F6-bandeja-atajos-settings.md`, `docs/superpowers/contracts/README.md`.

- [ ] **Step 1:** versión `0.5.0` → `0.6.0` en los tres manifiestos; `cargo check` para actualizar el lock.

- [ ] **Step 2:** Verificación completa:

```bash
cd src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test && cd ..
pnpm typecheck && pnpm test && pnpm build
pnpm tauri build   # borra instaladores viejos (F5) y deja Qapture_0.6.0_x64-setup.exe
```

- [ ] **Step 3:** en el contrato:
  - Marca los criterios de aceptación con sus tests:
    - validación y conflicto → `system::shortcuts::tests::*` y `commands::settings::tests::a_refused_shortcut_saves_nothing`;
    - sobrevivir reinicios y migraciones → `repo::settings::tests::saved_settings_survive_reopening_the_file_db` y `settings_survive_the_0004_migration`.
  - Agrega la columna `Estado` con ✅ en F6-T3…T9.
  - README: F6 pasa a `🧪 En pruebas del equipo`.

- [ ] **Step 4: Commit** `chore(f6): versión 0.6.0 y cierre técnico del contrato F6`

- [ ] **Step 5:** pedir al equipo el checklist F6 (pruebas 1–9) con `Qapture_0.6.0_x64-setup.exe`. Push y PR solo cuando lo pida la persona usuaria.

---

## Ejecución

Recomendado: **subagent-driven**, como en F1–F5:
- **Tasks 1–4:** son Rust con lógica pura y tests sin Tauri (atajos, portapapeles, ajustes), cada una con su revisor.
- **Task 5:** es la única que integra plugins del SO y conviene revisarla con el modelo más capaz.
- **Tasks 6–7:** dependen de los nombres exactos del contrato (`shortcutFailures`, `newCase`, `shortcut:new-case`).
- **Task 9:** es independiente y se puede ejecutar en cualquier momento después de la 4.
