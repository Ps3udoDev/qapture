# F8a — Conexión con GitHub · `v1.1.0`

**Depende de:** F6 (Ajustes) y F7 (release) · **Plan:** [2026-10-19-f8a-github-conexion](../plans/2026-10-19-f8a-github-conexion.md) · **Spec:** [2026-10-19-integracion-github-design](../specs/2026-10-19-integracion-github-design.md).

## Objetivo
El tester conecta su cuenta de GitHub desde Ajustes con Device Flow (sin backend), Qapture lista los repos donde está instalada la App `qapture-qa`, y cada proyecto se vincula a varios repos (uno predeterminado, con ruteo opcional por columna) y a un repo de evidencias.

## Alcance
| Entra | Fuera |
|---|---|
| Conectar, reconectar y desconectar la cuenta (spec §8) con Device Flow | Reportar casos como issues (F8b) |
| Token en el llavero del sistema (Windows Credential Manager / Secret Service), sin respaldo en archivo | Importar issues (F8c) |
| Migración `0005` con todas las tablas de F8a–F8d | Sincronizar estados (F8d) |
| Listar los repos accesibles de la App | Paquetes `.qapture` con datos de GitHub |
| Pestaña «GitHub» en Ajustes y en Configurar proyecto | |
| Repos vinculados por proyecto, predeterminado, etiquetas, ruteo por columna y repo de evidencias | |

## Decisiones respecto al spec
- **Paquetes `.qapture` con `github.json`** (spec §6) pasan a **F8b**, donde aparecen los vínculos de casos. Así el formato del paquete se toca una sola vez. En F8a los paquetes no llevan datos de GitHub.
- **Ruteo sugerido** (`domain/repo-routing.ts`) y `evidence-path` también pasan a F8b, que es quien los usa.
- **El avatar** se descarga una vez al conectar (`<avatar_url>&s=64`) y se guarda como `data:` URL en `gh_account.avatar_data`. Por eso la migración agrega esa columna al modelo del spec.
- **Estado «Reconectar»:** basta con un indicador en memoria (`GithubState.needs_reconnect`) que se activa con cualquier 401 y se apaga al conectar. Al arrancar, también se considera «Reconectar» si hay fila en `gh_account` pero no hay token en el llavero.
- **Permisos del repo** (de `permissions` en la respuesta de instalación): `canWriteIssues = has_issues && pull`, porque quien puede leer un repo puede abrir issues si están habilitados; `canWriteContents = push`.
- **Pestaña en Ajustes:** se llama **«GitHub»** (el spec decía «Cuenta GitHub»), para mantener nombres de una palabra como el resto de las pestañas.
- **Comandos de configuración del proyecto:** el spec §5.2 los llama `gh_project_repos_get/set`; aquí son `gh_project_github_get/set` porque guardan más que repos.

## Notas de implementación
- **Datos en SQLite (`0005_integrations.sql`):** `gh_account` (fila única `id = 1`), `project_repo_link` (PK `(project_id, owner, repo)`, orden por `position`), `project_gh_settings` y `case_issue_link` (UNIQUE `(case_id, owner, repo)`; sin uso hasta F8b). Todo con `ON DELETE CASCADE` desde proyecto o caso.
- **Guardar la config de un proyecto** reemplaza los repos en una transacción y **conserva** `field_mapping` e `import_filter` de los repos que siguen vinculados.
- **El token nunca sale de Rust:** no va en ningún DTO, evento, `AppError.detail`, log, tabla, respaldo ni paquete.
- **Llavero:** servicio `com.teams4soft.qapture`, cuenta `github`.
- **CSP sin cambios:** el avatar llega como `data:` URL.
- **Revisión final (fixes):**
  - El cliente HTTP nunca entra en pánico al construirse: si falla, cada solicitud devuelve un error.
  - El header `Authorization` va marcado como sensible.
  - El sondeo reintenta tras un fallo pasajero (sin red o 5xx) hasta que vence el código.
  - Un `error` desconocido del Device Flow se muestra como «GitHub rechazó la autorización (código: <error>).».
  - Cancelar mientras se guarda un **reconectar** restaura la cuenta y el token anteriores.
  - `gh_disconnect` borra la cuenta aunque el llavero falle y después devuelve ese error.
- **Front:**
  - Al llegar la cuenta regresiva a 0, el front cancela y muestra el aviso «El código venció. Vuelve a pulsar «Conectar con GitHub».» (Rust ya no emite nada porque el flujo se canceló).
  - La pestaña del proyecto no deja editar ni guardar hasta cargar la config guardada; si no carga, lo avisa.
  - «¿No ves tu repo?» copia el enlace de instalación y abre la guía para el admin (`open_link` con `githubAdminGuide`).

## Interfaces producidas
- Repositorio Rust `repo::integrations` (Task 1): `GhAccountRow`, `get_account`/`save_account`/`delete_account`, `RepoRef` (`full_name`, `parse`), `LinkedRepo`, `ProjectGithub`, `get_project_github`/`save_project_github`.
- `open_link({ target })` acepta además `githubDevice | githubInstall | githubRevoke | githubAdminGuide`.
- Comandos (Task 5): `gh_connect_start() -> DeviceCodeDto`, `gh_connect_cancel()`, `gh_account() -> GhAccountDto | null`, `gh_disconnect()`, `gh_list_repos() -> GhRepoDto[]`, `gh_project_github_get({ projectId }) -> ProjectGithubDto`, `gh_project_github_set({ projectId, config }) -> ProjectGithubDto`.
- Eventos hacia `main`: `gh:connected` (con `GhAccountDto`) y `gh:connect-failed` (con `{ message }`).
- Errores nuevos `AppError::GitHub { kind, message }` con códigos `gh_auth`, `gh_forbidden`, `gh_not_found`, `gh_rate_limited`, `gh_offline`, `gh_keyring`, `gh_validation` y `gh_conflict`.

## Tasks
| ID | Task | Estado |
|---|---|---|
| F8a-T0 | Spike manual de imágenes entre repos privados (no bloquea el resto) | ☐ |
| F8a-T1 | Migración `0005` y repositorio de integraciones | ✅ |
| F8a-T2 | Errores de GitHub y cliente HTTP | ✅ |
| F8a-T3 | Almacén del token (llavero) | ✅ |
| F8a-T4 | Device Flow y cuenta | ✅ |
| F8a-T5 | Repos, configuración del proyecto y comandos | ✅ |
| F8a-T6 | Dominio, puerto y adaptadores TS | ✅ |
| F8a-T7 | Slice y pestaña «GitHub» de Ajustes | ✅ |
| F8a-T8 | Pestaña «GitHub» de Configurar proyecto | ✅ |
| F8a-T9 | Documentación y prueba real | 🚧 |

## Criterios de aceptación
- [x] La migración crea las cuatro tablas y deja `user_version = 5`: `repo::integrations::tests::migration_0005_creates_the_github_tables`.
- [x] La config del proyecto se guarda y se lee igual, y conserva los ajustes de importación: `project_github_round_trips` y `saving_keeps_import_settings_of_repos_that_stay_linked`.
- [x] Borrar un proyecto borra su config de GitHub: `deleting_a_project_removes_its_github_config`.
- [x] Solo hay una cuenta; el formato de repo se valida: `account_is_a_single_row` y `repo_ref_parse`.
- [x] Device Flow, llavero, «Reconectar» y ruteo obsoleto: tests de las Tasks 2–5 (`a_second_start_cancels_the_first_poll`, `keyring_failure_leaves_no_account_row`, `a_401_marks_the_account_for_reconnect`, `routing_on_an_archived_field_is_reported_stale`, `html_body_maps_to_a_friendly_error`, `connection_refused_maps_to_offline`).

## Checklist de pruebas locales
| # | Prueba | Esperado | OK |
|---|---|---|---|
| 1 | Conectar con `Ps3udoDev` | Aparece «Conectado como @Ps3udoDev» con su avatar | ✅ 2026-10-20 (instalador local) |
| 2 | Ver los repos personales | Aparece `Ps3udoDev/booknow-mcp-service` | ✅ 2026-10-20 |
| 3 | Vincular `booknow-mcp-service` como predeterminado | Se guarda y sigue ahí al reiniciar la app | ✅ 2026-10-20 |
| 4 | Configurar un repo de evidencias | Se guarda en la pestaña GitHub del proyecto | ☐ |
| 5 | Desconectar y reconectar | La cuenta se borra; los vínculos del proyecto se conservan | ☐ |
| 6 | Revocar en `https://github.com/settings/apps/authorizations` | Al listar repos aparece «Reconectar» | ✅ 2026-10-20 (y reconexión OK) |
| 7 | Linux AppImage: conectar | Conecta, o muestra el mensaje del llavero antes de abrir el navegador | ☐ (llavero real de Linux pendiente; el de Windows pasó en el test ignorado de la Task 3) |
| 8 | Cuando el owner apruebe la instalación: ver los repos de TEAMS4SOFT | Aparecen en la lista | ☐ |
| 9 | Spike de la Task 0 (imágenes entre repos privados) | Resultado anotado: `blob?raw` / `raw` / ❌ | ☐ |

**Spike de imágenes:** pendiente.
