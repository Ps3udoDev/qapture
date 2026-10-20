# F8b — Reportar casos como issues de GitHub · Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Que desde cualquier caso el tester cree un issue en el repo que corresponda, o comente en el issue ya vinculado. El issue sale con los datos del caso y las capturas **renderizadas** embebidas, subidas en un solo commit al repo de evidencias del proyecto. Si no se puede subir, entra el modo asistido (copiar y Ctrl+V). Además, el vínculo se ve en el caso, en el Excel y viaja en los paquetes `.qapture`.

**Architecture:**
- **TypeScript:** arma el título y el cuerpo del issue (plantilla pura en `domain/issue-template.ts`) y el tester lo edita en el diálogo.
- **Rust:** recibe el cuerpo, lo sanea (marcadores de imagen y comentario oculto), decide el modo de subida y crea el issue. El repo de evidencias sale **solo** de la base. El commit de las capturas usa la API de Git de bajo nivel (blobs → tree → commit → ref) y las enlaza por SHA con `blob/<sha>/<ruta>?raw=true`. Después guarda `case_issue_link`.
- **Modo asistido:** deja `> 📎 Evidencia n: pendiente de pegar` en el issue, y un comando copia el PNG renderizado al portapapeles desde Rust.
- **Excel:** suma una columna «Issue» solo si hay vínculos.
- **Paquetes:** llevan un `github.json` opcional; la v1.0 ignora entradas desconocidas del ZIP.

**Tech Stack:** igual que F8a. Rust: `reqwest` 0.13 (rustls + ring), `rusqlite`, `image`, `base64`, `rust_xlsxwriter`, `tauri-plugin-clipboard-manager` (ya presente), `mockito` (dev). Front: React 19, TS, Zustand 5, zod 4, Vitest. **Sin dependencias nuevas.**

**Spec:** [`docs/superpowers/specs/2026-10-19-integracion-github-design.md`](../specs/2026-10-19-integracion-github-design.md): §3 (D6–D10), §5.2 (comandos `b`), §6 (paquetes y duplicados), §7 completo, §11 y §12. Contexto: el plan [F8a](2026-10-19-f8a-github-conexion.md) y su contrato.

**Contrato:** `docs/superpowers/contracts/F8b-reportar-issues.md` (se crea en la Task 1).

**Base:** se trabaja en la rama `feat/f8b-reportar-issues`, creada desde `feat/f8a-github-conexion` (PR #13 todavía sin merge). Cuando el #13 entre a `main`, se hace rebase sobre `main`.

## Global Constraints

- **Rama y commits:**
  - Un commit por task, terminado con una línea en blanco y `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
  - **Sin push ni merge sin pedirlo.**
  - Comandos en Git Bash desde `qapture/` (Rust desde `qapture/src-tauri/`).
- **Rigen todas las Global Constraints de F8a:**
  - el token nunca sale de Rust;
  - errores `AppError::GitHub` con sus mensajes fijos;
  - ACL en tres lugares;
  - el fixture de contratos solo se regenera con `UPDATE_CONTRACTS=1 cargo test contract`;
  - CSP sin cambios;
  - cero warnings;
  - vitest con `--maxWorkers=4`, porque con el paralelismo por defecto ya había timeouts de 5 s antes de esta rama.
- **Enlace de imagen** (validado en el spike de F8a, fila 9 del contrato):

  `https://github.com/<owner>/<repo>/blob/<commit_sha>/<ruta>?raw=true`

  Se arma con `WEB_BASE`. **Nunca** `raw.githubusercontent.com`, porque con un repo privado da 404.
- **Ruta en el repo de evidencias:** `<slug-proyecto>/<code>/<AAAA-MM-DD_HHmmss>_<slug-tester>/<n>.png`.
  - La hora es la local del equipo.
  - `n` es la posición 1..N **dentro de las capturas elegidas**, en el orden del caso.
  - `slug`: minúsculas; las tildes y la ñ se pasan a su letra base con la tabla `áàäâã→a, éèëê→e, íìïî→i, óòöôõ→o, úùüû→u, ñ→n, ç→c`; todo lo demás que no sea `[a-z0-9]` se convierte en `-`; los guiones se colapsan y se recortan; máximo 60 caracteres; si queda vacío, `sin-nombre`.
  - Tester: `settings.profile.tester` o, si está vacío, el login de GitHub.
  - El código del caso va tal cual.
  - **La ruta se genera solo en Rust.**
- **Commit de evidencias:**
  - mensaje `qapture: <code> <título recortado a 50 caracteres> (<tester>)`;
  - va a la rama predeterminada del repo de evidencias;
  - si el `PATCH` de la ref responde 422, se reintenta desde la lectura de la ref, hasta 3 veces.
- **Plantilla del issue** (`domain/issue-template.ts`, copiar tal cual):
  - **Título:** `[<code>] <title>`, recortado a 256 caracteres.
  - **Cuerpo**, en este orden y separado por líneas en blanco:
    1. `**Proyecto:** <nombre>` + ` · **Cliente:** <cliente>` (si hay) + ` · **Estado:** <estado>`
    2. `**Reportado por:** <tester>` + ` (<cargo>)` (si hay) + ` vía Qapture <versión> · <AAAA-MM-DD>`
    3. Una línea `**<label>:** <valor>` por cada campo activo con valor que no sea `longtext`, en orden de columnas. Los booleanos se muestran como `Sí`/`No`.
    4. Por cada campo `longtext` con valor: `### <label>` + salto de línea + valor.
    5. Si hay capturas elegidas: `### Evidencias`; por cada una, `**<n>.** <descripción>` (sin descripción: `**<n>.**`), luego un salto de línea y `{{img:<n>}}`.
    6. `<!-- qapture:case=<code>;v=1 -->`.
- **Comentario de re-test** (`buildRetestComment`): `Re-test el <AAAA-MM-DD> por <tester>: estado <estado>.` + línea en blanco + `<!-- notas -->`. Le siguen la sección `### Evidencias` (igual que arriba) y el mismo comentario oculto.
- **Saneamiento en Rust** (`report::sanitize_body`):
  - (a) se quita todo `{{img:n}}` cuyo `n` no esté entre 1 y la cantidad elegida;
  - (b) cada `n` elegido que no aparezca en el texto se agrega al final, bajo `### Evidencias` (que se crea si no existe);
  - (c) si falta `<!-- qapture:case=<code>;v=1 -->`, se agrega al final.

  Al final: `{{img:n}}` → `![<code> · <n>](<url>)` si se subió; si es modo asistido, `> 📎 Evidencia <n>: pendiente de pegar`.
- **Límites:** cuerpo hasta 65 536 caracteres (`La descripción supera 65 536 caracteres.`); hasta 10 etiquetas y 10 asignados. Una captura de más de 10 MB muestra el aviso `La captura <n> pesa más de 10 MB; GitHub podría rechazarla.`, pero no bloquea.
- **Modo de subida:**
  - `repo`: hay repo de evidencias configurado y `GET /repos/{o}/{r}` dice `permissions.push = true`.
  - `assisted`: no hay repo de evidencias, no hay `push`, o la subida responde 403/404.
  - Un 401 aborta con `gh_auth`.
  - Cualquier otro error (red, 5xx, rate limit) **aborta sin crear el issue**.
  - Si el POST del issue falla después de subir las capturas, quedan PNG huérfanos en el repo de evidencias; se acepta.
- **Barrera:** `evidence_upload` solo escribe en el `evidence_owner/evidence_repo` guardado del proyecto del caso. Ningún comando recibe un repo de evidencias desde el WebView.
- **Duplicados:** como máximo un vínculo por (caso, repo), con la UNIQUE de 0005. Si ya existe, `gh_create_issue` responde `gh_conflict` con `El caso ya tiene un issue en <owner/repo> (#<n>).`
- **Repo vacío:** si leer la ref de la rama predeterminada responde 409 o 404, el error es `gh_validation` con `El repo de evidencias <owner/repo> está vacío: crea un primer archivo (por ejemplo, un README) y vuelve a intentar.`
- **Textos de UI (fijos):**
  - Botón del caso: `Reportar en GitHub`.
  - Título del diálogo: `Reportar en GitHub` (nuevo) / `Comentar en <owner/repo>#<n>` (existente).
  - Etiquetas del formulario: `Repositorio`, `Título`, `Descripción` (pestañas `Editar` / `Vista previa`), `Etiquetas`, `Asignados`, `Milestone` (opción `Sin milestone`), `Capturas`.
  - Línea de destino: `Se creará en <owner/repo> como @<login>.` / `Se comentará en <owner/repo>#<n> como @<login>.`
  - Botones: `Crear issue` / `Comentar` / `Cancelar`.
  - Progreso: `Subiendo capturas…` y luego `Creando issue…` / `Comentando…`.
  - Toasts: `Issue #<n> creado en <owner/repo>.` / `Comentario agregado en <owner/repo>#<n>.`
  - Sin repos: `Este proyecto no tiene repositorios vinculados.` + botón `Vincular repositorio`.
  - Sin cuenta: `Conecta tu cuenta de GitHub en Ajustes › GitHub.` + botón `Abrir Ajustes`.
  - Panel asistido: título `Termina de adjuntar las capturas`; motivo (uno de estos):
    - `No hay repo de evidencias configurado.`
    - `Tu cuenta no puede escribir en <owner/repo>.`
    - `Qapture no está instalada en <owner/repo>.`

    Instrucción: `Copia cada captura y pégala con Ctrl+V en un comentario del issue (o editando la descripción).` Por fila: `Copiar` y la casilla `Pegada`. Botones `Abrir issue` y `Cerrar`.
  - Toast al copiar: `Captura <n> copiada. Pégala con Ctrl+V en GitHub.`
  - Chip del caso: `<repo>#<n>` (solo el nombre del repo, sin dueño), con el ícono de abierto o cerrado; si hay pendientes: `<k> capturas sin pegar`.
- **Excel:** columna `Issue` al final, después de `Evidencias`, solo si algún caso del proyecto tiene vínculos.
  - Con un vínculo, la celda es un hipervínculo con texto `<repo>#<n>`.
  - Con varios, el texto es `<repo>#<n>, <repo>#<m>` y lleva una nota de celda con una URL por línea.
- **Paquetes:** entrada opcional `github.json`; `project.json` y `FORMAT_VERSION` (1) no cambian. Formato:
  ```json
  { "version": 1,
    "repos": [{ "fullName": "o/r", "isDefault": true, "defaultLabels": ["bug"] }],
    "routingFieldKey": "componente" | null,
    "routingMap": { "Front": "o/r" },
    "evidenceRepo": "o/r" | null,
    "links": [{ "caseCode": "EJ-014", "owner": "o", "repo": "r", "issueNumber": 45,
                "nodeId": "…", "htmlUrl": "https://github.com/o/r/issues/45",
                "direction": "exported", "issueState": "open" }] }
  ```
  Al importar:
  - un `github.json` inválido se ignora entero y suma un aviso a la importación: `No se pudieron leer los datos de GitHub del paquete; el proyecto se importó sin ellos.`;
  - los vínculos cuyo `caseCode` no exista se descartan;
  - los vínculos se reasignan por código a los IDs nuevos;
  - la columna de ruteo se busca por `key`.

## Review Focus

1. **El tester borra o reordena los `{{img:n}}` en el editor, o escribe `{{img:9}}` con 3 capturas:** el issue sale igual, con cada captura elegida una sola vez y sin marcadores colgando → test en Task 3 (`sanitize_body_*`).
2. **Doble clic en «Crear issue», o dos ventanas reportando el mismo caso al mismo repo:** sale un único issue. El segundo intento ve el vínculo (`gh_conflict`) y no crea otro → test en Task 4 (`a_second_create_for_the_same_case_and_repo_conflicts`) y en Task 8 (el botón se deshabilita mientras está ocupado).
3. **Otra persona sube evidencias al mismo repo justo a la vez (422 en el PATCH de la ref):** se reintenta y las dos subidas quedan en la historia, sin perder commits → test en Task 2 (`a_non_fast_forward_ref_update_is_retried`).
4. **Repo de evidencias vacío (recién creado, sin commits):** mensaje claro y nada creado a medias → test en Task 2 (`an_empty_evidence_repo_reports_a_clear_error`).
5. **Un paquete exportado por la v1.1 abierto en la v1.0, o un `github.json` dañado:** la v1.0 abre el proyecto sin vínculos; la v1.1 importa el proyecto y muestra el aviso → tests en Task 6 (`a_package_with_an_unknown_entry_still_imports`, `a_broken_github_json_imports_without_links`).

## Decisiones respecto al spec (se anotan en el contrato en la Task 1)

- **La ruta de evidencias solo existe en Rust.** `domain/evidence-path.ts` (spec §5.3) no se crea: el WebView nunca propone rutas, y así la barrera queda en un solo lugar.
- **`gh_report_preview` no se crea.** El diálogo usa `gh_repo_meta` (etiquetas, milestones y asignados) y `gh_project_links` (vínculos existentes). La plantilla vive solo en TS (spec §7.1).
- **Vínculos del proyecto en un solo comando:** `gh_project_links({ projectId })` alimenta los chips del caso y el ícono de la lista, sin cambiar `CaseDto`.
- **Pendientes del modo asistido:** viven en memoria del slice, con clave `caseId|owner/repo`. Se pierden al cerrar la app, que es lo que permite el spec §7.4.
- **El aviso de más de 10 MB** se calcula en TS con el tamaño del PNG renderizado (`readImage`), antes de enviar.
- **Pendientes heredados de F8a que se resuelven aquí:** quitar el `allow(dead_code)` de `GhErrorKind::Conflict` (Task 4); el texto de la guía del admin pasa a tiempo presente real («rechaza», ya que ahora sí hay escritura) y se revisa (Task 10).

## File Structure

```
src-tauri/src/
  integrations/github/client.rs      (+ post_json, patch_json)
  integrations/github/issues.rs      NUEVO: crear, comentar, metadatos del repo, permisos
  integrations/github/evidence_upload.rs NUEVO: slug, ruta, commit con la API de Git, URL
  integrations/github/report.rs      NUEVO: sanitize_body, modo de subida, orquestación
  repo/integrations.rs               (+ vínculos de casos: insert, list_for_project, get)
  commands/github.rs, commands/names.rs, lib.rs, capabilities/main.json (+ 6 comandos)
  dto.rs                             (+ DTO y contrato)
  transfer/package.rs, transfer/github_json.rs NUEVO
  excel/matrix.rs, excel/data.rs     (+ columna Issue)
src/
  domain/issue-template.ts (+ test), domain/repo-routing.ts (+ test)
  ports/issue-tracker.ts, infrastructure/{tauri,memory}/github.ts, infrastructure/tauri/dto.ts
  ui/store/github-slice.ts (+ reporte y pendientes)
  ui/screens/ReportIssueDialog.tsx, ui/screens/AssistedPastePanel.tsx, ui/screens/IssueChips.tsx
  ui/screens/CaseDetail.tsx, ui/screens/CaseListPane.tsx, ui/screens/Github.test.tsx / Report.test.tsx
docs/ contrato F8b, usuario/github.md, usuario/github-admin.md, arquitectura.md, CHANGELOG.md
```

---

### Task 1: Cliente con escritura y operaciones de issues

**Files:**
- Create: `src-tauri/src/integrations/github/issues.rs`, `docs/superpowers/contracts/F8b-reportar-issues.md`
- Modify: `integrations/github/client.rs`, `integrations/github/mod.rs`, `docs/superpowers/contracts/README.md` (fila F8b «🚧 En desarrollo», tag `v1.1.0`)
- Commit también: este plan

**Interfaces:**
- Produces:
  - `GhClient::post_json<B: Serialize, T: DeserializeOwned>(&self, token, path, body: &B, repo: Option<&RepoRef>) -> AppResult<T>` y `patch_json` con la misma firma. Mismo manejo de errores y cabeceras que `get_json`.
  - `issues::RepoInfo { default_branch: String, can_push: bool }` y `issues::repo_info(client, token, &RepoRef) -> AppResult<RepoInfo>` (`GET /repos/{o}/{r}`).
  - `issues::NewIssue { title, body, labels: Vec<String>, assignees: Vec<String>, milestone: Option<u64> }`
  - `issues::IssueRef { number: u64, node_id: String, html_url: String, state: String }`
  - `issues::create(client, token, &RepoRef, &NewIssue) -> AppResult<IssueRef>`
  - `issues::comment(client, token, &RepoRef, number, body: &str) -> AppResult<String>` (devuelve la `html_url` del comentario)
  - `issues::RepoMeta { labels: Vec<String>, milestones: Vec<(u64, String)>, assignees: Vec<String> }` y `issues::repo_meta(client, token, &RepoRef) -> AppResult<RepoMeta>`, que lee paginado `/labels`, `/milestones?state=open` y `/assignees`.

- [ ] **Step 1: Crear la rama y el contrato.** `git switch -c feat/f8b-reportar-issues feat/f8a-github-conexion`. Escribir `F8b-reportar-issues.md` con el formato de F8a: Alcance (Entra: spec §7 y paquetes; Fuera: importar y sincronizar), las «Decisiones respecto al spec» de este plan, las interfaces (comandos de la Task 4) y las Tasks T1–T10. Checklist local:
  1. Crear un issue con 2 capturas en `Ps3udoDev/testing`, con evidencias en `Ps3udoDev/qa-evidencias-prueba`: las imágenes se ven.
  2. Comentar un re-test con 1 captura.
  3. Proyecto sin repo de evidencias → modo asistido, copiar y pegar.
  4. Mismo caso hacia un segundo repo → dos chips.
  5. Excel con la columna Issue.
  6. Exportar el paquete y abrirlo en otra instalación → vínculos conservados.
  7. Cuando el owner apruebe: `TEAMS4SOFT/GMX-frontend` con `TEAMS4SOFT/qa-evidencias`, y un miembro sin acceso a `qa-evidencias` no ve las imágenes.
- [ ] **Step 2: Tests que fallan** (mockito):
  - `post_json_sends_body_and_headers`
  - `patch_json_maps_422_to_validation`
  - `repo_info_reads_push_permission_and_default_branch`
  - `create_issue_returns_number_and_url` (el mock verifica el JSON enviado: title, body, labels, assignees, milestone)
  - `create_issue_omits_empty_milestone` (sin milestone, la clave no viaja)
  - `comment_returns_html_url`
  - `repo_meta_collects_labels_milestones_and_assignees` (2 páginas de labels)
  - `write_errors_never_contain_the_token`
- [ ] **Step 3: Ejecutar** `cargo test integrations::github::issues integrations::github::client` → FAIL.
- [ ] **Step 4: Implementar.**
- [ ] **Step 5: Ejecutar** `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test` → PASS.
- [ ] **Step 6: Commit** `feat(f8b): escritura en la API de GitHub y operaciones de issues`.

### Task 2: Subida de evidencias en un commit

**Files:**
- Create: `src-tauri/src/integrations/github/evidence_upload.rs`

**Interfaces:**
- Consumes: `GhClient::{get_json, post_json, patch_json}`, `issues::repo_info` (Task 1) y `config::WEB_BASE`.
- Produces:
  - `pub fn slug(text: &str) -> String`
  - `pub fn evidence_dir(project_name: &str, code: &str, at: chrono::NaiveDateTime, tester: &str) -> String`, que devuelve `<slug>/<code>/<AAAA-MM-DD_HHmmss>_<slug>`.
  - `pub struct Upload { pub path: String, pub png: Vec<u8> }`
  - `pub async fn commit_files(client, token, repo: &RepoRef, files: &[Upload], message: &str) -> AppResult<String>`, que devuelve el SHA del commit. Pasos:
    1. `repo_info` (rama predeterminada);
    2. `GET git/ref/heads/<rama>`;
    3. `GET git/commits/<sha>` (árbol base);
    4. un `POST git/blobs` por archivo (`encoding: base64`);
    5. `POST git/trees` con `base_tree` (modo `100644`, tipo `blob`);
    6. `POST git/commits` con un padre;
    7. `PATCH git/refs/heads/<rama>` con `force: false`.

    Un 422 en el paso 7 vuelve al paso 2, hasta 3 intentos. Un 409 o 404 en el paso 2 da el error de repo vacío.
  - `pub fn image_url(web_base: &str, repo: &RepoRef, sha: &str, path: &str) -> String`, que devuelve `<web>/<o>/<r>/blob/<sha>/<path>?raw=true`. Cada segmento de la ruta va codificado como URL; el código del caso puede llevar caracteres seguros.

- [ ] **Step 1: Tests que fallan:**
  - `slug_follows_the_table`: `Ana Pérez` → `ana-perez`; `GMX — Portal Clientes` → `gmx-portal-clientes`; `Ñandú_Ç` → `nandu-c`; `***` → `sin-nombre`; 80 caracteres se recortan a 60, sin guion final.
  - `evidence_dir_has_the_agreed_shape`: `gmx-portal-clientes/EJ-014/2026-11-02_153012_ana-perez`.
  - `image_url_uses_blob_and_raw`: empieza con `https://github.com/` y termina en `?raw=true`; no contiene `raw.githubusercontent.com`.
  - `commit_files_creates_one_commit_for_all_images`: mockito, 2 archivos. Se verifican 2 blobs, un tree con las 2 rutas y `base_tree`, un commit con un padre, y que el PATCH lleve el SHA nuevo.
  - `a_non_fast_forward_ref_update_is_retried`: el PATCH responde 422 una vez y después 200 → `Ok`, con la ref leída 2 veces.
  - `gives_up_after_three_rejections`
  - `an_empty_evidence_repo_reports_a_clear_error`: la ref responde 409 → `gh_validation` con el mensaje fijo.
  - `upload_errors_keep_their_kind`: el blob responde 403 → `GhErrorKind::Forbidden` (la Task 3 decide el modo asistido con esto).
- [ ] **Step 2: Ejecutar** `cargo test evidence_upload` → FAIL.
- [ ] **Step 3: Implementar.** Los blobs se envían en serie, para respetar los límites secundarios de GitHub.
- [ ] **Step 4: Ejecutar** `cargo test` → PASS. Ejecutar también fmt y clippy.
- [ ] **Step 5: Commit** `feat(f8b): subida de capturas al repo de evidencias en un solo commit`.

### Task 3: Orquestación del reporte en Rust

**Files:**
- Create: `src-tauri/src/integrations/github/report.rs`
- Modify: `src-tauri/src/repo/integrations.rs` (vínculos de casos)

**Interfaces:**
- Consumes: Tasks 1–2, `repo::integrations::get_project_github`, `repo::evidence::list`, `EvidenceFiles::absolute` (PNG **renderizado**: `rendered_path`) y los ajustes (`settings.profile.tester`).
- Produces:
  - En `repo::integrations`:
    - `pub struct CaseIssueLink { case_id, owner, repo, issue_number: u64, node_id, html_url, direction: String, issue_state: String }`
    - `insert_link(conn, &CaseIssueLink) -> AppResult<()>`: si la UNIQUE salta, devuelve `gh(Conflict, "El caso ya tiene un issue en <o/r> (#<n>).")`, con el número del vínculo existente.
    - `find_link(conn, case_id, &RepoRef) -> AppResult<Option<CaseIssueLink>>`
    - `links_for_project(conn, project_id) -> AppResult<Vec<CaseIssueLink>>`
  - `pub fn sanitize_body(body: &str, code: &str, chosen: usize) -> String`: hace (a), (b) y (c) de las Global Constraints y deja los `{{img:n}}` normalizados, un solo marcador por `n`.
  - `pub fn fill_images(body: &str, code: &str, urls: Option<&[String]>) -> String`: con `Some`, los convierte en `![<code> · n](url)`; con `None`, en el texto pendiente.
  - `pub enum UploadMode { Repo, Assisted(AssistedReason) }` y `pub enum AssistedReason { NoEvidenceRepo, CannotWrite(RepoRef), NotInstalled(RepoRef) }`.
  - `pub struct ReportInput { case_id, repo: RepoRef, title, body, labels, assignees, milestone: Option<u64>, evidence_ids: Vec<String> }`
  - `pub struct ReportOutcome { link: CaseIssueLink, mode: UploadMode, pending: Vec<String> /* evidence ids */ }`
  - `pub async fn create_issue(state: &AppState, input: ReportInput, now: NaiveDateTime) -> AppResult<ReportOutcome>`:
    1. revalidar (título 1–256, cuerpo ≤ 65 536, etiquetas y asignados ≤ 10, evidencias que pertenezcan al caso);
    2. verificar el token;
    3. `find_link` → si existe, `Conflict`;
    4. modo: sin evidencias elegidas → `Repo` sin subir nada;
    5. subir: `Forbidden` / `NotFound` → asistido; cualquier otro error se propaga;
    6. `sanitize_body` + `fill_images`;
    7. `issues::create`;
    8. `insert_link` con `direction = exported`.

    **Nunca mantiene el guard de la base durante un `.await`.**
  - `pub async fn comment_issue(state, input: ReportInput, now) -> AppResult<(String /* comment url */, UploadMode, Vec<String>)>`: igual, pero exige un vínculo existente para (caso, repo) (si no hay, `gh(NotFound, …)`) y comenta en `issue_number`.

- [ ] **Step 1: Tests que fallan:**
  - `sanitize_body_drops_out_of_range_markers`
  - `sanitize_body_appends_missing_chosen_images_under_evidencias`
  - `sanitize_body_creates_the_evidencias_heading_when_missing`
  - `sanitize_body_restores_the_hidden_marker`
  - `sanitize_body_keeps_one_marker_per_image`
  - `fill_images_uses_alt_code_and_n`
  - `fill_images_assisted_leaves_the_pending_text`
  - `links_conflict_on_the_same_case_and_repo`
  - Con `AppState` de pruebas (`GithubState::for_tests` apuntando a mockito):
    - `create_uploads_rendered_pngs_and_links_by_sha`: el blob subido coincide con los bytes del PNG renderizado, no con el original.
    - `create_without_evidence_repo_is_assisted`
    - `create_with_403_on_upload_is_assisted_and_still_creates_the_issue`
    - `create_with_a_5xx_on_upload_creates_nothing`: no hay POST de issue ni vínculo.
    - `create_never_writes_outside_the_configured_evidence_repo`: el mock solo acepta rutas `/repos/<evidencias>/git/*`; cualquier otra petición de escritura falla el test.
    - `a_second_create_for_the_same_case_and_repo_conflicts`
    - `comment_requires_an_existing_link`
- [ ] **Step 2: Ejecutar** → FAIL.
- [ ] **Step 3: Implementar.**
- [ ] **Step 4: Ejecutar** `cargo test` → PASS (más fmt y clippy).
- [ ] **Step 5: Commit** `feat(f8b): crear y comentar issues con capturas o en modo asistido`.

### Task 4: Comandos, DTO, contratos y ACL

**Files:**
- Modify: `commands/github.rs`, `commands/names.rs`, `lib.rs`, `capabilities/main.json`, `dto.rs`, `error.rs` (quitar el `allow(dead_code)` de `Conflict`), `src/infrastructure/tauri/dto.ts`, `src/infrastructure/tauri/contract.test.ts`, el fixture `contracts.json` (regenerado)

**Interfaces:**
- Produces (DTO `camelCase`, inputs con `deny_unknown_fields`):
  - `CaseIssueLinkDto { caseId, owner, repo, fullName, issueNumber, htmlUrl, direction, issueState }`
  - `RepoMetaDto { labels: Vec<String>, milestones: Vec<{ number, title }>, assignees: Vec<String> }`
  - `ReportInputDto { caseId, repo: String /* o/r */, title, body, labels, assignees, milestone: Option<u64>, evidenceIds }`
  - `ReportResultDto { link: CaseIssueLinkDto, uploadMode: "repo" | "assisted", assistedReason: Option<String> /* mensaje fijo de la UI */, pendingEvidence: Vec<String>, commentUrl: Option<String> }`
  - Comandos:
    - `gh_repo_meta({ repo }) -> RepoMetaDto`
    - `gh_project_links({ projectId }) -> Vec<CaseIssueLinkDto>`
    - `gh_create_issue({ input }) -> ReportResultDto`
    - `gh_comment_issue({ input }) -> ReportResultDto`
    - `gh_copy_evidence({ evidenceId }) -> ()`: decodifica el PNG renderizado con `image` y lo pone en el portapapeles (`ClipboardExt::write_image`).
    - `gh_open_issue({ caseId, repo }) -> ()`: abre la `html_url` **guardada** de ese vínculo con el opener; el WebView no manda la URL.

    Los errores `gh_auth` de los comandos de red llaman a `mark_auth`, igual que `gh_list_repos`.
  - `assistedReason` lleva los textos fijos de la UI (Global Constraints).

- [ ] **Step 1: Tests que fallan:**
  - Contrato: muestras de los DTO nuevos y `inputs_reject_unknown_fields` para `ReportInputDto`.
  - Los tests de ACL de `names.rs` fallan hasta registrar los 6 comandos.
  - `commands::github::tests`:
    - `open_issue_uses_the_stored_url`
    - `copy_evidence_rejects_an_unknown_id`
    - `report_maps_assisted_reasons_to_fixed_texts`
    - `a_401_while_reporting_marks_reconnect`
  - TS: los esquemas zod nuevos en `contract.test.ts`.
- [ ] **Step 2: Ejecutar** → FAIL.
- [ ] **Step 3: Implementar** y regenerar el fixture.
- [ ] **Step 4: Ejecutar** la verificación completa de Rust y `pnpm typecheck && pnpm vitest run --maxWorkers=4` → PASS.
- [ ] **Step 5: Commit** `feat(f8b): comandos para reportar, comentar y copiar capturas`.

### Task 5: Plantilla del issue y ruteo en TS

**Files:**
- Create: `src/domain/issue-template.ts`, `src/domain/issue-template.test.ts`, `src/domain/repo-routing.ts`, `src/domain/repo-routing.test.ts`

**Interfaces:**
- Produces:
  - `buildIssue(input: { caseRecord: CaseRecord; fields: FieldDef[]; project: { name: string; client: string }; evidences: { description: string }[]; tester: string; role: string; appVersion: string; today: string }): { title: string; body: string }`
  - `buildRetestComment(input: { caseRecord; evidences; tester; today }): string`
  - `suggestRepo(config: ProjectGithub, fields: FieldDef[], caseRecord: CaseRecord): string | null`. Busca el valor del caso en la columna de ruteo (activa y de tipo lista) y lo traduce con `routingMap`; si no hay coincidencia, devuelve el predeterminado; sin repos, `null`.
  - `shortRepo(fullName: string): string`, que es la parte después de `/`.

- [ ] **Step 1: Tests que fallan:**
  - `buildIssue`:
    - un caso con `text`, `list`, `boolean`, `longtext` y 2 capturas → cuerpo exacto, línea por línea, según las Global Constraints;
    - sin cliente ni cargo, sin capturas (no hay sección Evidencias);
    - título recortado a 256;
    - campos archivados o vacíos omitidos;
    - el comentario oculto al final.
  - `buildRetestComment`: texto exacto.
  - `suggestRepo`:
    - con ruteo → repo mapeado;
    - opción sin mapeo → predeterminado;
    - columna archivada → predeterminado;
    - sin repos → `null`.
- [ ] **Step 2: Ejecutar** → FAIL.
- [ ] **Step 3: Implementar.**
- [ ] **Step 4: Ejecutar** `pnpm typecheck && pnpm vitest run --maxWorkers=4` → PASS.
- [ ] **Step 5: Commit** `feat(f8b): plantilla del issue y repo sugerido por caso`.

### Task 6: Datos de GitHub en los paquetes `.qapture`

**Files:**
- Create: `src-tauri/src/transfer/github_json.rs`
- Modify: `src-tauri/src/transfer/package.rs` (escribir y leer la entrada y avisar), `src-tauri/src/transfer/mod.rs`

**Interfaces:**
- Consumes: `repo::integrations::{get_project_github, save_project_github, links_for_project, insert_link}`.
- Produces:
  - `github_json::collect(conn, project_id) -> AppResult<Option<GithubJson>>`: devuelve `None` si el proyecto no tiene repos, ajustes ni vínculos.
  - `github_json::apply(conn, project_id, &GithubJson, code_to_case: &HashMap<String, String>, key_to_field: &HashMap<String, String>) -> AppResult<()>`
  - La entrada `GITHUB_JSON = "github.json"` se escribe en el ZIP solo si `collect` devuelve `Some`.

- [ ] **Step 1: Tests que fallan:**
  - `export_writes_github_json_only_when_there_is_github_data`
  - `round_trip_keeps_repos_routing_evidence_repo_and_links`: los vínculos se reasignan a los IDs nuevos por código; la columna de ruteo se resuelve por `key`.
  - `links_to_unknown_case_codes_are_dropped`
  - `a_broken_github_json_imports_without_links`: el proyecto se importa y el resultado de la importación incluye el aviso fijo.
  - `a_package_with_an_unknown_entry_still_imports`: un ZIP con `otro.json` se importa sin error. Este test fija que el lector, igual al de la v1.0, ignora entradas desconocidas.
  - `project_json_is_unchanged_by_github_data`: `project.json` es idéntico, byte a byte, con y sin datos de GitHub.
- [ ] **Step 2: Ejecutar** → FAIL.
- [ ] **Step 3: Implementar.** Para el aviso, usar el mecanismo de avisos o resumen que ya devuelve la importación de paquetes. Si no existe, agregar un `warnings: Vec<String>` al resultado y actualizar el DTO, el contrato y el toast del front.
- [ ] **Step 4: Ejecutar** la verificación completa (Rust + pnpm) → PASS.
- [ ] **Step 5: Commit** `feat(f8b): datos de GitHub en los paquetes .qapture`.

### Task 7: Columna «Issue» en el Excel

**Files:**
- Modify: `src-tauri/src/excel/data.rs` (cargar los vínculos por caso), `src-tauri/src/excel/matrix.rs` (variante `Column::Issue`)

**Interfaces:**
- Consumes: `repo::integrations::links_for_project`.
- Produces: `ExportCase.issues: Vec<(String /* texto repo#n */, String /* url */)>`. `columns()` agrega `Column::Issue` al final solo si algún caso tiene vínculos (ancho 22).

- [ ] **Step 1: Tests que fallan** (con `calamine`, como los tests actuales de `matrix`):
  - `no_links_no_issue_column`
  - `one_link_is_a_hyperlink_with_repo_and_number`
  - `several_links_are_text_with_a_note_listing_urls`
  - `issue_column_goes_after_evidencias`
- [ ] **Step 2: Ejecutar** → FAIL.
- [ ] **Step 3: Implementar** (`Note::new` de `rust_xlsxwriter` para la nota).
- [ ] **Step 4: Ejecutar** `cargo test` → PASS (más fmt y clippy).
- [ ] **Step 5: Commit** `feat(f8b): columna Issue en la matriz de Excel`.

### Task 8: Puerto, adaptadores y slice del reporte

**Files:**
- Modify: `src/domain/github.ts` (tipos `CaseIssueLink`, `RepoMeta`, `ReportInput`, `ReportResult`), `src/ports/issue-tracker.ts`, `src/infrastructure/tauri/github.ts`, `src/infrastructure/memory/github.ts`, `src/ui/store/github-slice.ts`, `src/ui/store/github-slice.test.ts`

**Interfaces:**
- Produces:
  - `IssueTracker` suma: `repoMeta(repo)`, `projectLinks(projectId)`, `createIssue(input)`, `commentIssue(input)`, `copyEvidence(evidenceId)` y `openIssue(caseId, repo)`.
  - Memory: guarda los vínculos, simula `gh_conflict`, permite forzar `uploadMode`, y cuenta las copias.
  - Slice:
    - estado `github.links: Record<string /* projectId */, CaseIssueLink[]>` y `github.pending: Record<string /* caseId|repo */, { issueUrl; items: { evidenceId; n; done: boolean }[]; reason }>`;
    - acciones:
      - `loadProjectLinks(projectId)`
      - `reportIssue(input): Promise<FormResult & { result?: ReportResult }>`: al terminar recarga los vínculos, guarda los pendientes si fue asistido y muestra el toast fijo;
      - `commentIssue(input)`
      - `copyPendingEvidence(key, evidenceId)`: muestra el toast fijo de copia;
      - `markPendingDone(key, evidenceId, done)`
      - `dismissPending(key)`
      - `openIssue(caseId, repo)`

    Un `gh_conflict` vuelve como error del formulario (`_form`) y recarga los vínculos.

- [ ] **Step 1: Tests que fallan:**
  - `report_success_adds_the_link_and_toasts`
  - `assisted_report_keeps_pending_items`
  - `conflict_reloads_links_and_returns_form_error`
  - `copy_pending_toasts_with_n`
  - `marking_all_done_clears_the_pending_entry`
- [ ] **Step 2: Ejecutar** → FAIL.
- [ ] **Step 3: Implementar.**
- [ ] **Step 4: Ejecutar** `pnpm typecheck && pnpm vitest run --maxWorkers=4` → PASS.
- [ ] **Step 5: Commit** `feat(f8b): puerto y estado para reportar issues`.

### Task 9: Diálogo «Reportar en GitHub», chips y panel asistido

**Files:**
- Create: `src/ui/screens/ReportIssueDialog.tsx`, `src/ui/screens/AssistedPastePanel.tsx`, `src/ui/screens/IssueChips.tsx`, `src/ui/screens/Report.test.tsx`
- Modify: `src/ui/screens/CaseDetail.tsx` (botón `Reportar en GitHub` junto a `Duplicar`, chips debajo del estado y panel), `src/ui/screens/CaseListPane.tsx` (ícono `Bug` de lucide si el caso tiene vínculos, con `aria-label="Tiene issues en GitHub"`)

**Comportamiento:**
- **Al abrir el diálogo:**
  - si no hay cuenta, se muestra el mensaje fijo y `Abrir Ajustes`;
  - si no hay repos vinculados, el mensaje fijo y `Vincular repositorio` (abre Configurar proyecto en la pestaña GitHub);
  - si no, se carga la configuración del proyecto y `Repositorio` se precarga con `suggestRepo`.
- **Al cambiar de repo:**
  - se recargan las etiquetas, milestones y asignados (`repoMeta`);
  - si existe un vínculo (caso, repo), el diálogo pasa a modo comentario: título fijo, cuerpo `buildRetestComment` y botón `Comentar`;
  - si no, el cuerpo es `buildIssue` y las etiquetas parten de las `defaultLabels` del repo.
- **Capturas:**
  - todas marcadas por defecto, con miniatura (`evidence.imageUrl`);
  - si alguna pasa de 10 MB, se muestra el aviso fijo;
  - si el tester cambia la selección, el cuerpo **no** se regenera solo; un botón `Regenerar descripción` lo vuelve a armar.
- **Vista previa:** Markdown como texto preformateado. No hay librería nueva, así que no se renderiza Markdown.
- **Envío:**
  - los botones se deshabilitan mientras está ocupado (Review Focus 2);
  - se muestran los mensajes de progreso fijos;
  - si sale bien, se cierra y queda el toast;
  - si fue asistido, se abre `AssistedPastePanel` y `openIssue`.
- **Chips:** muestran `IssueChips` con `shortRepo#n`; al hacer clic se llama a `openIssue`. Si hay pendientes, el chip muestra `<k> capturas sin pegar` y abre el panel.

- [ ] **Step 1: Tests que fallan** (adaptador en memoria):
  - `sin cuenta muestra «Conecta tu cuenta…»`
  - `sin repos muestra «Este proyecto no tiene repositorios vinculados.»`
  - `precarga el repo sugerido por la columna de ruteo`
  - `crea el issue y muestra el chip testing#1`
  - `con vínculo existente cambia a «Comentar en Ps3udoDev/testing#1»`
  - `modo asistido abre el panel y «Copiar» copia la captura 1`
  - `doble clic en «Crear issue» crea un solo issue`
  - `la lista marca los casos con issues`
- [ ] **Step 2: Ejecutar** → FAIL.
- [ ] **Step 3: Implementar**, con el estilo visual de los diálogos actuales (`Dialog`, `btn`, tokens t4s).
- [ ] **Step 4: Ejecutar** `pnpm typecheck && pnpm build && pnpm vitest run --maxWorkers=4` → PASS.
- [ ] **Step 5: Commit** `feat(f8b): reportar un caso en GitHub desde la app`.

### Task 10: Documentación y prueba real

**Files:**
- Modify: `docs/usuario/github.md` (sección «Reportar un caso», modo asistido, re-test y Excel), `docs/usuario/github-admin.md` (escritura real en el repo de evidencias), `docs/usuario/faq.md` («Las imágenes salen rotas» → acceso de lectura al repo de evidencias; «Repo de evidencias vacío»), `docs/arquitectura.md` (comandos y `report.rs`), `CHANGELOG.md` («Sin publicar»), `docs/superpowers/contracts/F8b-reportar-issues.md`, `docs/superpowers/contracts/README.md`

- [ ] **Step 1: Escribir la documentación.**
- [ ] **Step 2: Verificación completa** (Rust + pnpm) → todo en verde.
- [ ] **Step 3: Instalador para el dueño del producto:** `pnpm tauri build`. Anotar en el reporte la ruta del `setup.exe`. La prueba con GitHub real (checklist del contrato) la hace la persona.
- [ ] **Step 4: Commit** `docs(f8b): reportar casos en GitHub, modo asistido y Excel`.
