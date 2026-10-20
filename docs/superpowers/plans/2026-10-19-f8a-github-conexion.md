# F8a — Conexión con GitHub · Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Que el tester conecte su cuenta de GitHub desde Ajustes con Device Flow, sin backend; que Qapture liste los repos donde está instalada la App `qapture-qa`; y que cada proyecto se vincule a varios repos (uno predeterminado, con ruteo opcional por columna) y a un repo de evidencias.

**Architecture:** Todo el I/O de GitHub vive en Rust, en `src-tauri/src/integrations/github/`. Ahí se encuentran:
- un cliente `reqwest` con URL base inyectable (para probarlo con `mockito`);
- un `TokenStore` detrás de un trait (llavero del sistema en producción, memoria en tests);
- el Device Flow modelado como una máquina de estados pura más un bucle de sondeo cancelable que emite `gh:connected` / `gh:connect-failed`.

Los datos (cuenta, repos vinculados y ajustes de GitHub del proyecto) van en SQLite (migración `0005`). Como pide el spec, esa migración ya crea también las tablas y columnas de F8b–F8d. En TS sigue el patrón del resto: puerto `IssueTracker` → adaptador Tauri con `zod` + adaptador en memoria, un slice de Zustand y dos pantallas: la pestaña **GitHub** de Ajustes y la pestaña **GitHub** de Configurar proyecto.

**Tech Stack:** Tauri 2, Rust 2024, rusqlite 0.40. Dependencias nuevas de Rust: `reqwest` 0.13 (ya está en el lock por el updater), `keyring` 3, `tokio` 1 (`time`) y `mockito` 1 (solo dev). Front: React 19, TypeScript, Zustand 5, zod 4, Vitest y Testing Library.

**Spec:** [`docs/superpowers/specs/2026-10-19-integracion-github-design.md`](../specs/2026-10-19-integracion-github-design.md): §3 (D1–D5, D11–D13), §4, §5.1–5.3 (solo lo de F8a), §6, §8 y §11. Como contexto, la [propuesta](../../propuestas/2026-10-19-integracion-github.md) §3.2 (secuencia del Device Flow) y §5.

**Contrato:** `docs/superpowers/contracts/F8a-github-conexion.md` (se crea en la Task 1).

## Global Constraints

- **Rama y commits:** rama `feat/f8a-github-conexion` desde `main` actualizado. Un commit por task, terminado con una línea en blanco y `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. **Sin push ni merge sin pedirlo.**
- **Comandos:** en Git Bash desde `qapture/` (Rust desde `qapture/src-tauri/`). Windows y Linux.
- **Constantes** (`integrations/github/config.rs`, copiadas tal cual):
  - `API_BASE = "https://api.github.com"`
  - `WEB_BASE = "https://github.com"`
  - `CLIENT_ID = "Iv23liai2kBF7qDefVd8"`
  - `APP_SLUG = "qapture-qa"`
  - `API_VERSION = "2022-11-28"` (header `X-GitHub-Api-Version`)
  - `User-Agent: Qapture/<CARGO_PKG_VERSION>`
  - `Accept: application/vnd.github+json` (API) o `application/json` (endpoints de login)
- **Enlaces fijos:**
  - instalación: `https://github.com/apps/qapture-qa/installations/new`
  - revocar: `https://github.com/settings/apps/authorizations`
- **Llavero:** servicio `com.teams4soft.qapture`, cuenta `github`. **Nunca** hay respaldo en archivo.
- **El token nunca sale de Rust:** no va en ningún DTO, evento, `AppError.detail`, log, tabla, respaldo ni paquete.
- **Errores:** nueva variante `AppError::GitHub { kind: GhErrorKind, message: String }`. `code()` devuelve el código del `kind` y `user_message()` devuelve `message`. `Display` es `github: {message}`.

  | `kind` | `code` | Mensaje fijo |
  |---|---|---|
  | `Auth` | `gh_auth` | `Tu cuenta de GitHub no está conectada o la sesión venció. Conéctala en Ajustes › GitHub.` |
  | `Forbidden` | `gh_forbidden` | `Tu cuenta de GitHub no tiene permiso para esta acción en <owner/repo>.` (sin repo: `…para esta acción.`) |
  | `NotFound` | `gh_not_found` | `No se encontró <owner/repo> o Qapture no está instalada en él.` |
  | `RateLimited` | `gh_rate_limited` | `GitHub limitó las solicitudes. Intenta de nuevo a las <HH:mm>.` (hora local del `x-ratelimit-reset` o de `Retry-After`) |
  | `Offline` | `gh_offline` | `No hay conexión con GitHub. Revisa tu red e inténtalo de nuevo.` |
  | `Keyring` | `gh_keyring` | Windows: `No se pudo usar el Administrador de credenciales de Windows.` · Linux: `No se encontró un llavero del sistema (Secret Service). Instala y desbloquea gnome-keyring o KWallet y vuelve a intentar.` |
  | `Validation` | `gh_validation` | `GitHub rechazó la solicitud: <message de GitHub>.` |
  | `Conflict` | `gh_conflict` | (se define en F8b; la variante existe desde ya) |
- **Mensajes del Device Flow** (evento `gh:connect-failed`):
  - `El código venció. Vuelve a pulsar «Conectar con GitHub».`
  - `Se canceló la autorización en GitHub.`
  - `La App de GitHub no tiene habilitado el Device Flow.`
  - Cualquier otro error usa el mensaje del `AppError`.
- **Mensajes de validación de la configuración del proyecto** (Rust y espejo en TS):
  - `Marca un repositorio como predeterminado.`
  - `«<valor>» no es un repositorio válido (usa dueño/repositorio).`
  - `El repositorio «<owner/repo>» está repetido.`
  - `La columna de ruteo no existe en este proyecto.`
  - `La columna de ruteo debe ser de tipo Lista.`
  - `«<opción>» no es una opción de «<columna>».`
  - `«<owner/repo>» no está vinculado al proyecto.`
  - `Las etiquetas deben tener entre 1 y 50 caracteres, sin comas (máximo 10).`
- **Formato de repo:** dueño `^[A-Za-z0-9](?:[A-Za-z0-9-]{0,38})$` y nombre `^[A-Za-z0-9._-]{1,100}$` (distinto de `.` y `..`). Se compara sin distinguir mayúsculas y se guarda como lo devuelve GitHub.
- **ACL:** cada comando nuevo va en `commands/names.rs`, `generate_handler!` y `capabilities/main.json`. Ningún permiso JS de plugins nuevos. El CSP **no cambia**: el avatar llega como `data:` URL.
- **Contratos:** `src/infrastructure/tauri/__fixtures__/contracts.json` se regenera **solo** con `UPDATE_CONTRACTS=1 cargo test contract`.
- **Dependencias nuevas, solo estas:**
  - Rust: `reqwest = { version = "0.13", default-features = false, features = ["json", "rustls"] }`, `keyring = { version = "3", features = ["windows-native", "sync-secret-service", "crypto-rust"] }`, `tokio = { version = "1", features = ["time"] }`.
  - Dev: `mockito = "1"`.
  - JS: ninguna.
  - Si el nombre del feature de rustls de `reqwest` 0.13 no es `rustls`, usar el equivalente documentado, sin `native-tls`.
- **Cero warnings:** `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `pnpm typecheck`, `pnpm test` y `pnpm build` en verde al cerrar cada task.
- **Textos visibles en español.**

## Review Focus

1. **El tester cierra Ajustes o la app a mitad del Device Flow, o pulsa «Conectar» dos veces:** no quedan dos sondeos vivos; el segundo `gh_connect_start` cancela el primero, y al cancelar no llega un `gh:connected` tardío → test en Task 4 (`a_second_start_cancels_the_first_poll`).
2. **El token guardado ya no sirve** (revocado en GitHub o borrado del llavero a mano): `gh_list_repos` responde `gh_auth`, la cuenta pasa a «Reconectar» y la app no se rompe → tests en Task 5 (`a_401_marks_the_account_for_reconnect`) y Task 7 (UI «Reconectar»).
3. **Linux sin Secret Service, o llavero bloqueado:** «Conectar» falla **antes** de abrir el navegador, con el mensaje `gh_keyring`, y no deja una cuenta a medias en la base → test en Task 3 (el store falla en `set`) y Task 4 (`keyring_failure_leaves_no_account_row`).
4. **Una columna de ruteo que después se archiva o cambia de tipo, o una opción que se renombra:** la configuración guardada no rompe la carga del proyecto. `gh_project_repos_get` devuelve la config y marca `routingStale: true`, y la pestaña lo avisa → test en Task 5 (`routing_on_an_archived_field_is_reported_stale`).
5. **Sin red, o GitHub responde 5xx o HTML en lugar de JSON:** `gh_offline` o un error genérico con mensaje en español, nunca un pánico ni un `contract` en el front → test en Task 2 (`html_body_maps_to_a_friendly_error`, `connection_refused_maps_to_offline`).

## Decisiones respecto al spec (se anotan en el contrato en la Task 1)

- **Paquetes `.qapture` con `github.json`** (spec §6) pasan a **F8b**, donde aparecen los vínculos de casos. Así el formato del paquete se toca una sola vez. En F8a los paquetes no llevan datos de GitHub.
- **Ruteo sugerido** (`domain/repo-routing.ts`) y `evidence-path` también pasan a F8b, que es quien los usa.
- **El avatar** se descarga una vez al conectar (`<avatar_url>&s=64`) y se guarda como `data:` URL en `gh_account.avatar_data`. Por eso la migración agrega esa columna al modelo del spec.
- **Estado «Reconectar»:** basta con un indicador en memoria (`GithubState.needs_reconnect`) que se activa con cualquier 401 y se apaga al conectar. Al arrancar, también se considera «Reconectar» si hay fila en `gh_account` pero no hay token en el llavero.
- **Permisos del repo** (de `permissions` en la respuesta de instalación): `canWriteIssues = has_issues && pull`, porque quien puede leer un repo puede abrir issues si están habilitados; `canWriteContents = push`.
- **Pestaña en Ajustes:** se llama **«GitHub»** (el spec decía «Cuenta GitHub»), para mantener nombres de una palabra como el resto de las pestañas.

## File Structure

```
src-tauri/
  Cargo.toml                                  (+ reqwest, keyring, tokio; dev: mockito)
  capabilities/main.json                      (+ 7 permisos)
  src/
    error.rs                                  (+ AppError::GitHub, GhErrorKind)
    dto.rs                                    (+ DTO de GitHub y muestras de contrato)
    state.rs                                  (+ GithubState en AppState)
    lib.rs                                    (+ handlers, TokenStore real)
    db/mod.rs                                 (+ 0005)
    db/migrations/0005_integrations.sql       NUEVO
    repo/mod.rs, repo/integrations.rs         NUEVO: cuenta, repos del proyecto, ajustes GH
    integrations/mod.rs                       NUEVO
    integrations/github/mod.rs                NUEVO
    integrations/github/config.rs             NUEVO
    integrations/github/client.rs             NUEVO: GhClient, mapeo de errores, paginación
    integrations/github/token_store.rs        NUEVO: TokenStore, KeyringStore, MemoryTokenStore
    integrations/github/auth.rs               NUEVO: Device Flow (pasos puros + bucle) y /user
    integrations/github/repos.rs              NUEVO: instalaciones y repos
    integrations/github/project_config.rs     NUEVO: validación de la config del proyecto
    commands/mod.rs, commands/names.rs        (+ github)
    commands/github.rs                        NUEVO
src/
  domain/github.ts (+ .test.ts)               NUEVO: tipos, parseo/validación de repos y config
  ports/issue-tracker.ts                      NUEVO
  ports/repositories.ts                       (+ github: IssueTracker)
  infrastructure/tauri/dto.ts                 (+ esquemas zod)
  infrastructure/tauri/github.ts              NUEVO
  infrastructure/tauri/repositories.ts        (+ github)
  infrastructure/memory/github.ts             NUEVO
  infrastructure/memory/repositories.ts       (+ github)
  ui/store/github-slice.ts (+ .test.ts)       NUEVO
  ui/store/workspace.ts                       (+ slice)
  ui/screens/GithubAccountPanel.tsx           NUEVO (pestaña de Ajustes)
  ui/screens/ProjectGithubPanel.tsx           NUEVO (pestaña de Configurar proyecto)
  ui/screens/AppSettingsDialog.tsx            (+ pestaña)
  ui/screens/ProjectSettingsDialog.tsx        (+ pestaña)
  ui/screens/Github.test.tsx                  NUEVO
docs/
  superpowers/contracts/F8a-github-conexion.md, contracts/README.md
  usuario/github.md, usuario/github-admin.md, usuario/faq.md
  adr/0005-integracion-github.md, arquitectura.md
CHANGELOG.md
```

---

### Task 0 (manual, no bloquea el resto): spike de imágenes entre repos privados

No lleva código. Lo hace la persona, porque requiere el navegador con su sesión iniciada. Valida §7.3 del spec antes de F8b.

- [ ] **Step 1: Variante personal (se puede hacer hoy).** Crear el repo **privado** `Ps3udoDev/qa-evidencias-prueba` y subir `1.png` desde la web (Add file → Upload). Copiar el SHA del commit. En un issue de **otro** repo privado de `Ps3udoDev`, escribir en la vista previa:
  ```markdown
  ![sha](https://github.com/Ps3udoDev/qa-evidencias-prueba/blob/<sha>/1.png?raw=true)
  ![raw](https://raw.githubusercontent.com/Ps3udoDev/qa-evidencias-prueba/<sha>/1.png)
  ```
  Esperado: la variante `blob/…?raw=true` se ve embebida. Anotar cuál de las dos se ve.
- [ ] **Step 2: Variante org.** Cuando el owner apruebe la instalación, repetir con `TEAMS4SOFT/qa-evidencias` → issue en `TEAMS4SOFT/GMX-frontend`.
- [ ] **Step 3:** Anotar el resultado en el contrato F8a («Spike de imágenes: ✅ blob?raw / ✅ raw / ❌»). Si ninguna se ve, avisar antes de planificar F8b (spec §14: la v1.1 sale en modo asistido).

---

### Task 1: Migración 0005 y repositorio de integraciones

**Files:**
- Create: `src-tauri/src/db/migrations/0005_integrations.sql`, `src-tauri/src/repo/integrations.rs`, `docs/superpowers/contracts/F8a-github-conexion.md`
- Modify: `src-tauri/src/db/mod.rs` (`MIGRATIONS`), `src-tauri/src/repo/mod.rs`, `docs/superpowers/contracts/README.md` (fila F8a «🚧 En desarrollo»)
- Commit también: el spec y la propuesta (`docs/superpowers/specs/2026-10-19-integracion-github-design.md`, `docs/propuestas/2026-10-19-integracion-github.md`) y este plan

**Interfaces:**
- Produces (Rust, `repo::integrations`):
  - `pub struct GhAccountRow { pub login: String, pub name: Option<String>, pub avatar_data: Option<String>, pub connected_at: String }`
  - `pub fn get_account(conn) -> AppResult<Option<GhAccountRow>>` · `pub fn save_account(conn, &GhAccountRow) -> AppResult<()>` (reemplaza la fila 1) · `pub fn delete_account(conn) -> AppResult<()>`
  - `pub struct RepoRef { pub owner: String, pub repo: String }` con `full_name() -> String` y `parse(&str) -> Option<RepoRef>` (aplica el formato de las Global Constraints)
  - `pub struct LinkedRepo { pub repo: RepoRef, pub is_default: bool, pub default_labels: Vec<String> }`
  - `pub struct ProjectGithub { pub repos: Vec<LinkedRepo>, pub routing_field_id: Option<String>, pub routing_map: BTreeMap<String, String>, pub evidence_repo: Option<RepoRef> }`
  - `pub fn get_project_github(conn, project_id) -> AppResult<ProjectGithub>` (vacío si no hay filas) · `pub fn save_project_github(conn, project_id, &ProjectGithub) -> AppResult<()>` (una transacción: reemplaza los repos y hace upsert de los ajustes, **conservando** `field_mapping`/`import_filter` de los repos que siguen vinculados)

- [ ] **Step 1: Crear la rama y el contrato.** `git switch -c feat/f8a-github-conexion`. Escribir `F8a-github-conexion.md` con el formato de `F6-…md`:
  - Objetivo y Alcance (Entra: §8 del spec, migración, llavero; Fuera: reportar, importar, sincronizar y paquetes).
  - Las «Decisiones respecto al spec» de este plan.
  - Interfaces producidas: los comandos de la Task 5 y los eventos.
  - Tasks T1–T9.
  - Checklist de pruebas locales:
    1. Conectar con `Ps3udoDev`.
    2. Ver los repos personales.
    3. Vincular `booknow-mcp-service` como predeterminado.
    4. Configurar un repo de evidencias.
    5. Desconectar y reconectar.
    6. Revocar en GitHub → «Reconectar».
    7. Linux AppImage: conectar.
    8. Cuando el owner apruebe: ver los repos de TEAMS4SOFT.
    9. Spike de la Task 0.
- [ ] **Step 2: Tests que fallan** en `repo::integrations::tests`:
  - `migration_0005_creates_the_github_tables`: tras `db::open_in_memory()`, existen `gh_account`, `project_repo_link`, `project_gh_settings` y `case_issue_link`, y `PRAGMA user_version` es 5.
  - `project_github_round_trips`: guardar 2 repos (uno predeterminado, labels `["bug","qapture"]`), ruteo `{"Front":"TEAMS4SOFT/GMX-frontend"}` y evidencias `TEAMS4SOFT/qa-evidencias`; leer devuelve lo mismo, con los repos en orden de inserción.
  - `saving_keeps_import_settings_of_repos_that_stay_linked`: un `UPDATE` manual pone `field_mapping='{"a":1}'` en un repo; al guardar de nuevo con ese repo más otro, el mapeo sigue ahí.
  - `deleting_a_project_removes_its_github_config` (CASCADE).
  - `account_is_a_single_row`: dos `save_account` dejan una fila con los datos del segundo; `delete_account` la quita.
  - `repo_ref_parse` acepta `TEAMS4SOFT/GMX-frontend` y `Ps3udoDev/booknow-mcp-service`, y rechaza `a/b/c`, `/x`, `x/`, `-a/b`, `a/..` y un dueño de 40 caracteres.
- [ ] **Step 3: Ejecutar** `cargo test repo::integrations` → FAIL (no compila).
- [ ] **Step 4: Escribir `0005_integrations.sql`** con el modelo del spec §6 más `gh_account.avatar_data`:
  - `gh_account(id INTEGER PRIMARY KEY CHECK (id = 1), login, name, avatar_url, avatar_data, connected_at)`.
  - `project_repo_link(project_id REFERENCES project(id) ON DELETE CASCADE, owner, repo, is_default INTEGER NOT NULL DEFAULT 0, position INTEGER NOT NULL, default_labels TEXT NOT NULL DEFAULT '', field_mapping TEXT, import_filter TEXT, PRIMARY KEY (project_id, owner, repo))`.
  - `project_gh_settings(project_id PRIMARY KEY REFERENCES project(id) ON DELETE CASCADE, routing_field_id, routing_map TEXT NOT NULL DEFAULT '{}', evidence_owner, evidence_repo, section_map TEXT)`.
  - `case_issue_link(case_id REFERENCES test_case(id) ON DELETE CASCADE, owner, repo, issue_number INTEGER NOT NULL, node_id, html_url NOT NULL, direction NOT NULL CHECK (direction IN ('exported','imported')), issue_state NOT NULL DEFAULT 'open', etag, remote_updated_at, synced_at, UNIQUE (case_id, owner, repo))` + `CREATE INDEX case_issue_link_issue ON case_issue_link(owner, repo, issue_number)`.
  - Columnas de texto `TEXT`, con `NOT NULL` donde el modelo lo pide.

  Agregar el archivo a `MIGRATIONS` e implementar `repo/integrations.rs`.
- [ ] **Step 5: Ejecutar** `cargo test` → PASS (incluidos los tests de migraciones y respaldos que ya existían).
- [ ] **Step 6: Commit** `feat(f8a): migración 0005 y repositorio de integraciones GitHub`.

### Task 2: Errores de GitHub y cliente HTTP

**Files:**
- Create: `src-tauri/src/integrations/mod.rs`, `integrations/github/{mod.rs, config.rs, client.rs}`
- Modify: `src-tauri/Cargo.toml` (reqwest, tokio y mockito; keyring va en la Task 3), `src-tauri/src/lib.rs` (`mod integrations;`), `src-tauri/src/error.rs`

**Interfaces:**
- Produces:
  - `pub enum GhErrorKind { Auth, Forbidden, NotFound, RateLimited, Offline, Keyring, Validation, Conflict }` con `code(&self) -> &'static str`.
  - `AppError::GitHub { kind, message }` y el helper `pub fn gh(kind: GhErrorKind, message: impl Into<String>) -> AppError`.
  - `pub struct GhClient { api: String, web: String, http: reqwest::Client }`:
    - `GhClient::new(api_base: &str, web_base: &str) -> Self` · `GhClient::production() -> Self`
    - `async fn get_json<T: DeserializeOwned>(&self, token: &str, path: &str, repo: Option<&RepoRef>) -> AppResult<T>`
    - `async fn get_paginated<T: DeserializeOwned>(&self, token: &str, path: &str, items_key: Option<&str>) -> AppResult<Vec<T>>`. Sigue `Link: rel="next"` con `per_page=100`. `items_key` sirve para respuestas envueltas como `{ repositories: [...] }`. Tope de 50 páginas.
    - `async fn post_form_web<T: DeserializeOwned>(&self, path: &str, form: &[(&str, &str)]) -> AppResult<T>`, para los endpoints de login en `WEB_BASE` con `Accept: application/json`.
    - `async fn get_bytes(&self, url: &str) -> AppResult<Vec<u8>>`, para el avatar (sin token).
  - `pub(crate) fn map_status(status: u16, headers: &HeaderMap, body: &str, repo: Option<&RepoRef>) -> AppError`. Es pura y se prueba aparte.

- [ ] **Step 1: Tests que fallan.**
  - En `error.rs`: `github_errors_use_their_code_and_message` (`gh(GhErrorKind::Offline, "…")` serializa `code = "gh_offline"`, `message` = el texto y `detail = "github: …"`).
  - En `client.rs` (`mockito::Server::new_async()`):
    - `status_401_maps_to_auth`
    - `status_403_with_rate_limit_remaining_0_maps_to_rate_limited` (con `x-ratelimit-reset`; el mensaje contiene la hora `HH:mm`)
    - `status_403_otherwise_maps_to_forbidden_with_repo` (mensaje con `TEAMS4SOFT/GMX-frontend`)
    - `status_404_maps_to_not_found`
    - `status_422_maps_to_validation_with_github_message` (body `{"message":"Validation Failed"}` → `GitHub rechazó la solicitud: Validation Failed.`)
    - `html_body_maps_to_a_friendly_error` (200 con HTML → error, sin pánico)
    - `connection_refused_maps_to_offline` (URL `http://127.0.0.1:9` → `gh_offline`)
    - `pagination_follows_link_next` (2 páginas → 150 elementos, y `items_key = Some("repositories")` desenvuelve)
    - `requests_carry_the_required_headers` (el mock exige `authorization: Bearer t0k`, `x-github-api-version: 2022-11-28`, `user-agent` que empiece con `Qapture/`)
    - `errors_never_contain_the_token`: en cada caso de error anterior, `serde_json::to_string(&err)` no contiene `t0k`.
- [ ] **Step 2: Ejecutar** `cargo test integrations::github::client error::` → FAIL.
- [ ] **Step 3: Implementar.** `config.rs` lleva las constantes de las Global Constraints. Un error de conexión, timeout o DNS de `reqwest` → `Offline`. Un 5xx o un JSON inválido → `AppError::Io(...)` con el mensaje genérico existente. Timeout del cliente: 20 s.
- [ ] **Step 4: Ejecutar** `cargo test` y `cargo clippy --all-targets -- -D warnings` → PASS.
- [ ] **Step 5: Commit** `feat(f8a): cliente HTTP de GitHub con errores tipados`.

### Task 3: Almacén del token (llavero)

**Files:**
- Create: `src-tauri/src/integrations/github/token_store.rs`
- Modify: `src-tauri/Cargo.toml` (keyring)

**Interfaces:**
- Produces:
  - `pub trait TokenStore: Send + Sync { fn get(&self) -> AppResult<Option<String>>; fn set(&self, token: &str) -> AppResult<()>; fn delete(&self) -> AppResult<()>; }`
  - `pub struct KeyringStore` (servicio `com.teams4soft.qapture`, cuenta `github`). `keyring::Error::NoEntry` → `Ok(None)` en `get` y `Ok(())` en `delete`. Cualquier otro error → `gh(Keyring, <mensaje según cfg!(windows)>)`.
  - `pub struct MemoryTokenStore { token: Mutex<Option<String>>, fail: AtomicBool }` con `MemoryTokenStore::failing()` (toda operación devuelve `Keyring`). Es `pub` para usarlo en los tests de otros módulos.

- [ ] **Step 1: Tests que fallan:**
  - `memory_store_round_trip`: get `None` → set → get `Some` → delete → `None`.
  - `failing_store_reports_keyring_error`: `code = gh_keyring` y el mensaje coincide con la plataforma.
  - `keyring_store_round_trip`: `#[ignore]`, usa el llavero real. Se ejecuta a mano con `cargo test keyring_store -- --ignored` en Windows y en Linux, y se anota en el contrato.
- [ ] **Step 2: Ejecutar** → FAIL.
- [ ] **Step 3: Implementar.**
- [ ] **Step 4: Ejecutar** `cargo test token_store` → PASS. `cargo test keyring_store -- --ignored` en Windows → PASS.
- [ ] **Step 5: Commit** `feat(f8a): token de GitHub en el llavero del sistema`.

### Task 4: Device Flow y cuenta

**Files:**
- Create: `src-tauri/src/integrations/github/auth.rs`
- Modify: `src-tauri/src/state.rs` (agrega `GithubState`)

**Interfaces:**
- Consumes: `GhClient` (Task 2), `TokenStore` (Task 3), `repo::integrations::{save_account, GhAccountRow}` (Task 1).
- Produces:
  - `pub struct DeviceCode { pub device_code: String, pub user_code: String, pub verification_uri: String, pub expires_in: u64, pub interval: u64 }`
  - `pub async fn start(client: &GhClient) -> AppResult<DeviceCode>`: `POST {web}/login/device/code` con `client_id`.
  - `pub enum PollStep { Wait(Duration), Token(String), Fail(String) }`
  - `pub fn next_step(body: &serde_json::Value, interval: Duration) -> PollStep`: pura.
    - `authorization_pending` → `Wait(interval)`
    - `slow_down` → `Wait(interval + 5s)` (o el `interval` que traiga la respuesta)
    - `access_token` → `Token`
    - `expired_token` / `access_denied` / `device_flow_disabled` → `Fail(<mensaje fijo>)`
    - otro `error` → `Fail(<error_description>)`
  - `pub async fn poll(client, code: &DeviceCode, cancel: &AtomicBool) -> AppResult<Option<String>>`: `POST {web}/login/oauth/access_token` con `client_id`, `device_code` y `grant_type=urn:ietf:params:oauth:grant-type:device_code`. Duerme con `tokio::time::sleep`. Devuelve `None` si se canceló y falla con `Fail` al pasar `expires_in`.
  - `pub async fn fetch_account(client, token) -> AppResult<GhAccountRow>`: `GET /user` y luego el avatar `&s=64` como `data:image/png;base64,…`. Si el avatar falla, `avatar_data = None` sin error.
  - `pub async fn complete(client, tokens: &dyn TokenStore, conn: &Mutex<Connection>, token: String) -> AppResult<GhAccountRow>`: primero `tokens.set` (si falla, **no** se escribe la base), después `fetch_account` y `save_account`.
  - `pub struct GithubState { pub client: GhClient, pub tokens: Box<dyn TokenStore>, pub needs_reconnect: AtomicBool, pending: Mutex<Option<Arc<AtomicBool>>> }`:
    - `begin_flow(&self) -> Arc<AtomicBool>`: cancela el anterior y devuelve la bandera nueva.
    - `cancel_flow(&self)`
    - `token(&self) -> AppResult<String>`: falta el token → `gh(Auth, …)`.
  - `AppState::github(&self) -> &GithubState`. `AppState::new` recibe `GithubState`. Los tests usan `GithubState::for_tests(api, web)` con `MemoryTokenStore`.

- [ ] **Step 1: Tests que fallan:**
  - `next_step_handles_every_device_flow_answer`, uno por cada rama, con los mensajes fijos de las Global Constraints.
  - `poll_returns_the_token_after_pending` (mockito: responde `authorization_pending` 1 vez y luego `access_token`; `interval` 0).
  - `poll_stops_when_cancelled` (la bandera se activa antes del segundo intento → `Ok(None)`).
  - `a_second_start_cancels_the_first_poll` (`begin_flow` dos veces → la primera bandera queda en `true`).
  - `complete_saves_token_then_account` (mock `/user` → token en el store y fila con `login = "Ps3udoDev"`).
  - `keyring_failure_leaves_no_account_row` (`MemoryTokenStore::failing()` → error `gh_keyring` y `get_account` = `None`).
  - `avatar_failure_does_not_block_connecting` (avatar 500 → cuenta guardada con `avatar_data = None`).
- [ ] **Step 2: Ejecutar** `cargo test integrations::github::auth` → FAIL.
- [ ] **Step 3: Implementar.**
- [ ] **Step 4: Ejecutar** `cargo test` → PASS.
- [ ] **Step 5: Commit** `feat(f8a): Device Flow de GitHub y cuenta conectada`.

### Task 5: Repos, configuración del proyecto y comandos

**Files:**
- Create: `src-tauri/src/integrations/github/repos.rs`, `integrations/github/project_config.rs`, `src-tauri/src/commands/github.rs`
- Modify: `commands/mod.rs`, `commands/names.rs`, `lib.rs` (`generate_handler!` y `GithubState` con `KeyringStore` + `GhClient::production()`), `capabilities/main.json`, `dto.rs` (DTO y muestras de contrato), `src/infrastructure/tauri/__fixtures__/contracts.json` (regenerado)

**Interfaces:**
- Consumes: Tasks 1–4.
- Produces (DTO `camelCase`, inputs con `deny_unknown_fields`):
  - `GhAccountDto { login, name: Option<String>, avatarDataUrl: Option<String>, connectedAt, needsReconnect: bool }`
  - `DeviceCodeDto { userCode, verificationUri, expiresIn, interval }`. **Sin** `device_code`.
  - `GhRepoDto { owner, name, fullName, private: bool, canWriteIssues: bool, canWriteContents: bool }`
  - `LinkedRepoDto { fullName, isDefault, defaultLabels: Vec<String> }`
  - `ProjectGithubDto { repos: Vec<LinkedRepoDto>, routingFieldId: Option<String>, routingMap: BTreeMap<String,String>, evidenceRepo: Option<String>, routingStale: bool }`
  - `ProjectGithubInput`: igual que el DTO, pero sin `routingStale`.
  - `GhConnectFailedDto { message }`
  - `repos::list_accessible(client, token) -> AppResult<Vec<GhRepoDto>>`: `GET /user/installations` y, por cada una, `GET /user/installations/{id}/repositories` paginado. Ordena por `fullName` sin distinguir mayúsculas y quita duplicados.
  - `project_config::validate(input, fields: &[FieldDto]) -> AppResult<ProjectGithub>`: aplica los mensajes de validación de las Global Constraints en este orden: repos válidos → no repetidos → exactamente un predeterminado si hay repos → labels → columna de ruteo (existe, no archivada, tipo `list`) → claves del mapa en `options` → valores del mapa entre los repos vinculados → evidencias en formato válido.
  - Comandos:
    - `gh_connect_start() -> DeviceCodeDto`: hace `begin_flow` y lanza el sondeo con `tauri::async_runtime::spawn`. Al terminar: `complete` → emite `gh:connected` (con `GhAccountDto`), o emite `gh:connect-failed` (con `GhConnectFailedDto`). Si se canceló, no emite nada.
    - `gh_connect_cancel() -> ()`
    - `gh_account() -> Option<GhAccountDto>`
    - `gh_disconnect() -> ()`: borra el token y la cuenta, apaga `needs_reconnect` y conserva los vínculos de los proyectos.
    - `gh_list_repos() -> Vec<GhRepoDto>`
    - `gh_project_github_get({ projectId }) -> ProjectGithubDto`
    - `gh_project_github_set({ projectId, config: ProjectGithubInput }) -> ProjectGithubDto`

    Cualquier `gh_auth` que devuelva un comando enciende `needs_reconnect`.

    > El spec §5.2 los llama `gh_project_repos_get/set`. Aquí se llaman `gh_project_github_*` porque guardan más que repos. Anotarlo en el contrato.
  - Eventos: `GH_CONNECTED_EVENT = "gh:connected"`, `GH_CONNECT_FAILED_EVENT = "gh:connect-failed"`, ambos hacia `main`.

- [ ] **Step 1: Tests que fallan:**
  - `repos::tests::lists_repos_of_every_installation` (mockito: 2 instalaciones, la segunda con 2 páginas; permisos `pull`/`push` → flags según las «Decisiones»).
  - `project_config::tests`: un test por mensaje de validación, más `valid_config_passes`.
  - `commands::github::tests` (funciones sin Tauri, como `commands::settings::save`):
    - `a_401_marks_the_account_for_reconnect`
    - `account_without_token_needs_reconnect`
    - `disconnect_keeps_project_links`
    - `routing_on_an_archived_field_is_reported_stale` (y lo mismo con una opción que ya no existe)
    - `set_rejects_an_unknown_project` (`not_found`)
  - `dto::contract`: muestras de todos los DTO nuevos y de `projectGithubInput` en `inputs_json`, más `inputs_reject_unknown_fields` para `ProjectGithubInput`.
  - Los tests de ACL de `names.rs` fallan hasta registrar los 7 comandos en los tres lugares.
- [ ] **Step 2: Ejecutar** `cargo test` → FAIL.
- [ ] **Step 3: Implementar** y regenerar el fixture con `UPDATE_CONTRACTS=1 cargo test contract`.
- [ ] **Step 4: Ejecutar** `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test` → PASS. `pnpm test`: el contrato TS falla hasta la Task 6 (es lo esperado; anotarlo en el commit).
- [ ] **Step 5: Commit** `feat(f8a): comandos de cuenta, repos y configuración GitHub del proyecto`.

### Task 6: Dominio, puerto y adaptadores TS

**Files:**
- Create: `src/domain/github.ts`, `src/domain/github.test.ts`, `src/ports/issue-tracker.ts`, `src/infrastructure/tauri/github.ts`, `src/infrastructure/memory/github.ts`
- Modify: `src/ports/repositories.ts` (`github: IssueTracker`), `src/infrastructure/tauri/dto.ts`, `src/infrastructure/tauri/repositories.ts`, `src/infrastructure/memory/repositories.ts`, `src/infrastructure/tauri/contract.test.ts`

**Interfaces:**
- Produces:
  - `domain/github.ts`:
    - tipos `GhAccount`, `DeviceCode`, `GhRepo`, `LinkedRepo`, `ProjectGithub`, `ProjectGithubInput` (mismos campos que los DTO);
    - `parseRepo(value: string): { owner: string; repo: string } | null`;
    - `validateProjectGithub(input: ProjectGithubInput, fields: FieldDef[]): FormErrors`, espejo de Rust con los mismos mensajes y claves `repos`, `labels`, `routing` y `evidenceRepo`;
    - `INSTALL_URL`, `REVOKE_URL`, `DEFAULT_LABELS = ["bug", "qapture"]`.
  - `ports/issue-tracker.ts`:
    ```ts
    export interface IssueTracker {
      account(): Promise<GhAccount | null>;
      connectStart(): Promise<DeviceCode>;
      connectCancel(): Promise<void>;
      disconnect(): Promise<void>;
      listRepos(): Promise<GhRepo[]>;
      getProjectGithub(projectId: string): Promise<ProjectGithub>;
      setProjectGithub(projectId: string, config: ProjectGithubInput): Promise<ProjectGithub>;
      onConnected(handler: (account: GhAccount) => void): Promise<Unlisten>;
      onConnectFailed(handler: (message: string) => void): Promise<Unlisten>;
    }
    ```
  - `createMemoryGithub(): MemoryGithub` con `repos`, `accountValue`, `nextError` y `emitConnected` / `emitConnectFailed`, siguiendo el estilo de `createMemoryUpdater`.

- [ ] **Step 1: Tests que fallan:** `github.test.ts` con los mismos casos de validación que `project_config::tests` (mismos textos) y `parseRepo`; `contract.test.ts` con los esquemas nuevos contra `contracts.json`.
- [ ] **Step 2: Ejecutar** `pnpm test` → FAIL.
- [ ] **Step 3: Implementar.** Los esquemas `zod` son estrictos, como los demás de `dto.ts`.
- [ ] **Step 4: Ejecutar** `pnpm typecheck && pnpm test` → PASS.
- [ ] **Step 5: Commit** `feat(f8a): dominio, puerto y adaptadores de GitHub en TS`.

### Task 7: Slice y pestaña «GitHub» de Ajustes

**Files:**
- Create: `src/ui/store/github-slice.ts`, `src/ui/store/github-slice.test.ts`, `src/ui/screens/GithubAccountPanel.tsx`, `src/ui/screens/Github.test.tsx`
- Modify: `src/ui/store/workspace.ts`, `src/ui/screens/AppSettingsDialog.tsx` (`TABS` agrega `"GitHub"` antes de `"Acerca de"`), el lugar donde hoy se llama a `listenUpdates` (para suscribir `listenGithub`), `src-tauri/src/commands/settings.rs` y `src/domain/settings.ts` (`LinkTarget`)

**Enlaces:** los tres enlaces de GitHub pasan por el `open_link` que ya existe. Así ninguna URL viaja desde el WebView. `LinkTarget` suma:
- `GithubDevice` → `https://github.com/login/device`
- `GithubInstall` → `INSTALL_URL`
- `GithubRevoke` → `REVOKE_URL`

En TS: `"githubDevice" | "githubInstall" | "githubRevoke"`, con serde `camelCase` para esas variantes. Hace falta el test `commands::settings::tests::github_links_open_fixed_urls` y actualizar el contrato si `LinkTarget` aparece en él.

**Interfaces:**
- Produces (`GithubSlice`):
  - `github: { account: GhAccount | null; flow: { code: DeviceCode; startedAt: number } | null; repos: GhRepo[] | null; status: "idle" | "loading" }`
  - `loadGithubAccount()`, `connectGithub()`, `cancelGithubConnect()`, `disconnectGithub()`, `loadGithubRepos()`, `listenGithub(): Promise<() => void>`
  - `loadProjectGithub(projectId): Promise<ProjectGithub | null>`, `saveProjectGithub(projectId, input): Promise<FormResult>`

  Los errores pasan por `h.report`. Un error `gh_auth` además vuelve a cargar la cuenta, para mostrar «Reconectar».

**Pantalla** (textos fijos):
- **Sin cuenta:** el párrafo `Conecta tu cuenta para reportar casos como issues e importar issues como casos.` y el botón `Conectar con GitHub`.
- **Durante el flujo:**
  - `Ingresa este código en GitHub:`
  - el código en grande (monoespaciado) y los botones `Copiar código` y `Abrir GitHub` (`openLink("githubDevice")`);
  - `El código vence en m:ss.` (cuenta regresiva) y el botón `Cancelar`.
- **Conectado:**
  - avatar, `@login` y nombre;
  - los enlaces `Instalar o configurar Qapture en una cuenta u organización` (`INSTALL_URL`) y `Revocar acceso en GitHub` (`REVOKE_URL`);
  - el botón `Desconectar`, con confirmación `Se borrará el acceso guardado en este equipo. Tus proyectos conservan los repos vinculados.`
- **Reconectar:** el aviso `La sesión de GitHub venció o fue revocada.` y el botón `Reconectar`.
- **Toasts:** `Conectado como @<login>.` (en `gh:connected`) y el mensaje de `gh:connect-failed` como error.

- [ ] **Step 1: Tests que fallan** (memory adapter):
  - `github-slice.test.ts`:
    - `connect_shows_the_code_until_connected_event`
    - `connect_failed_clears_the_flow_and_toasts`
    - `gh_auth_error_reloads_account`
    - `disconnect_clears_account_and_repos`
  - `Github.test.tsx`:
    - `«Conectar con GitHub» muestra el código y lo copia`
    - `al llegar gh:connected muestra @login`
    - `cuenta con needsReconnect muestra «Reconectar»`
    - `«Desconectar» pide confirmación`
- [ ] **Step 2: Ejecutar** `pnpm test` → FAIL.
- [ ] **Step 3: Implementar.**
- [ ] **Step 4: Ejecutar** `pnpm typecheck && pnpm test && pnpm build` (y `cargo test` si se tocó Rust por `openLink`) → PASS.
- [ ] **Step 5: Commit** `feat(f8a): conectar la cuenta de GitHub desde Ajustes`.

### Task 8: Pestaña «GitHub» de Configurar proyecto

**Files:**
- Create: `src/ui/screens/ProjectGithubPanel.tsx`
- Modify: `src/ui/screens/ProjectSettingsDialog.tsx` (`TABS` agrega `"GitHub"` después de `"Columnas"`), `src/ui/screens/Github.test.tsx`

**Pantalla** (textos fijos):
- **Sin cuenta:** `Conecta tu cuenta de GitHub en Ajustes › GitHub para vincular repositorios.` y un botón que abre Ajustes en esa pestaña.
- **Sección «Repositorios vinculados»:**
  - una lista con radio `Predeterminado`, etiquetas editables (por defecto `bug, qapture`) y el botón `Quitar`;
  - un selector `Agregar repositorio` con búsqueda, agrupado por dueño y con un candado en los privados;
  - el aviso `Tu cuenta no puede crear issues en este repositorio.` cuando `!canWriteIssues`;
  - un enlace `¿No ves tu repo?` que copia `INSTALL_URL` y muestra `Pide a un owner de la organización que instale Qapture en ese repositorio. Enlace copiado.`
- **Sección «Ruteo por columna (opcional)»:** un selector entre las columnas de tipo Lista (o `Sin ruteo`) y, para cada opción, un selector de repo vinculado o `Predeterminado`. Si `routingStale`, muestra `La columna o alguna opción del ruteo cambió; revísalo.`
- **Sección «Repo de evidencias (opcional)»:** un selector entre todos los repos accesibles (o `Ninguno: pegar capturas a mano`). Si `!canWriteContents`, muestra `Tu cuenta no puede subir archivos a este repositorio; se usará el modo asistido.`
- **Botón `Guardar`:** valida con `validateProjectGithub` (errores en línea) y luego llama a `saveProjectGithub`. Toast `Configuración de GitHub guardada.`

- [ ] **Step 1: Tests que fallan:**
  - `vincular dos repos y marcar el predeterminado guarda la config`
  - `sin predeterminado muestra «Marca un repositorio como predeterminado.» y no guarda`
  - `el ruteo solo ofrece columnas de tipo Lista`
  - `routingStale muestra el aviso`
  - `sin cuenta muestra el enlace a Ajustes`
- [ ] **Step 2: Ejecutar** `pnpm test` → FAIL.
- [ ] **Step 3: Implementar.**
- [ ] **Step 4: Ejecutar** `pnpm typecheck && pnpm test && pnpm build` → PASS.
- [ ] **Step 5: Commit** `feat(f8a): vincular repos de GitHub a un proyecto`.

### Task 9: Documentación y prueba real

**Files:**
- Create: `docs/usuario/github.md`, `docs/usuario/github-admin.md`, `docs/adr/0005-integracion-github.md`
- Modify: `docs/usuario/README.md` (índice), `docs/usuario/faq.md`, `docs/arquitectura.md` (módulo `integrations/`, eventos `gh:*` y comandos), `CHANGELOG.md` (sección «Sin publicar»), `docs/superpowers/contracts/F8a-github-conexion.md`, `docs/superpowers/contracts/README.md`

- [ ] **Step 1: Escribir la documentación.**
  - `github.md`: conectar, reconectar, desconectar y vincular repos (lo de F8a; F8b completa reportar).
  - `github-admin.md`: el texto de spec §4.2 con `qapture-qa`, incluido el caso *member* → **Request** → aprobación del owner, y la lectura de `qa-evidencias` para el equipo.
  - FAQ: `No veo mi repo`, `Error de llavero en Linux`, `Cómo revoco el acceso`.
  - ADR 0005: GitHub App + Device Flow sin backend, token en el llavero, repo de evidencias dedicado + modo asistido, y por qué no se imita el Ctrl+V web (spec D1–D13).
- [ ] **Step 2: Prueba real con `pnpm tauri dev`**:
  1. Conectar con `Ps3udoDev`.
  2. Ver `Ps3udoDev/booknow-mcp-service` en la lista.
  3. Vincularlo como predeterminado.
  4. Reiniciar la app → la cuenta y el vínculo siguen ahí.
  5. Revocar en `https://github.com/settings/apps/authorizations` → al listar repos aparece «Reconectar».
  6. Reconectar.

  Marcar en el checklist del contrato lo que se probó y dejar sin marcar lo que depende del owner o de Linux.
- [ ] **Step 3: Ejecutar** la verificación completa: `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test` (en `src-tauri/`) y `pnpm typecheck && pnpm test && pnpm build` → todo en verde.
- [ ] **Step 4: Commit** `docs(f8a): guías de conexión con GitHub, guía para el admin y ADR 0005`.
