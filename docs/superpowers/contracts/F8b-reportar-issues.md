# F8b — Reportar casos como issues · `v1.1.0`

**Depende de:** F8a (conexión con GitHub) · **Plan:** [2026-10-20-f8b-reportar-issues](../plans/2026-10-20-f8b-reportar-issues.md) · **Spec:** [2026-10-19-integracion-github-design](../specs/2026-10-19-integracion-github-design.md).

## Objetivo
El tester reporta un caso de prueba como issue de GitHub (o comenta un re-test en uno existente) desde Qapture, con las capturas elegidas subidas a un repo de evidencias y enlazadas en el cuerpo. Si no puede subirlas, usa el modo asistido (copiar y pegar).

## Alcance
| Entra | Fuera |
|---|---|
| Crear un issue desde un caso, con plantilla, etiquetas, asignados y milestone (spec §7) | Importar issues (F8c) |
| Comentar un re-test en un issue existente | Sincronizar estados (F8d) |
| Subir capturas al repo de evidencias (commit a la rama predeterminada) | |
| Modo asistido cuando no se pueden subir | |
| Vínculos caso ↔ issue (chips en el caso, columna `Issue` en el Excel) | |
| `github.json` opcional en los paquetes `.qapture` | |

## Decisiones respecto al spec
- **La ruta de evidencias solo existe en Rust.** `domain/evidence-path.ts` (spec §5.3) no se crea: el WebView nunca propone rutas, y así la barrera queda en un solo lugar.
- **`gh_report_preview` no se crea.** El diálogo usa `gh_repo_meta` (etiquetas, milestones y asignados) y `gh_project_links` (vínculos existentes). La plantilla vive solo en TS (spec §7.1).
- **Vínculos del proyecto en un solo comando:** `gh_project_links({ projectId })` alimenta los chips del caso y el ícono de la lista, sin cambiar `CaseDto`.
- **Pendientes del modo asistido:** viven en memoria del slice, con clave `caseId|owner/repo`. Se pierden al cerrar la app, que es lo que permite el spec §7.4.
- **El aviso de más de 10 MB** se calcula en TS con el tamaño del PNG renderizado (`readImage`), antes de enviar.
- **Pendientes heredados de F8a que se resuelven aquí:** quitar el `allow(dead_code)` de `GhErrorKind::Conflict` (Task 4); el texto de la guía del admin pasa a tiempo presente real («rechaza», ya que ahora sí hay escritura) y se revisa (Task 10).

## Interfaces producidas
- Cliente (Task 1): `GhClient::post_json` y `patch_json` (mismas cabeceras y errores que `get_json`); `issues::repo_info`, `create`, `comment` y `repo_meta`.
- Comandos (Task 4): `gh_repo_meta`, `gh_project_links`, `gh_create_issue`, `gh_comment_issue` y `evidence_upload`.
- Enlace de imagen: `https://github.com/<owner>/<repo>/blob/<commit_sha>/<ruta>?raw=true` (nunca `raw.githubusercontent.com`).

## Tasks
| ID | Task | Estado |
|---|---|---|
| F8b-T1 | Cliente con escritura y operaciones de issues | ✅ |
| F8b-T2 | Subida de evidencias al repo (árbol y commit) | ✅ |
| F8b-T3 | Saneamiento del cuerpo y plantilla | ✅ |
| F8b-T4 | Comandos de reporte y vínculos | ✅ |
| F8b-T5 | Dominio, puerto y adaptadores TS | ✅ |
| F8b-T6 | `github.json` en los paquetes | ✅ |
| F8b-T7 | Columna `Issue` en el Excel | ✅ |
| F8b-T8 | Diálogo «Reportar en GitHub» | ✅ |
| F8b-T9 | Chips y modo asistido | ✅ |
| F8b-T10 | Documentación y prueba real | ☐ |

## Criterios de aceptación
- [x] Escritura con las mismas cabeceras y sin filtrar el token: `post_json_sends_body_and_headers`, `patch_json_maps_422_to_validation`, `write_errors_never_contain_the_token`.
- [x] Operaciones de issues: `repo_info_reads_push_permission_and_default_branch`, `create_issue_returns_number_and_url`, `create_issue_omits_empty_milestone`, `comment_returns_html_url`, `repo_meta_collects_labels_milestones_and_assignees`.

## Mensajes fijos
- Validación del reporte: `Se permiten hasta 10 etiquetas.`, `Se permiten hasta 10 asignados.`, `Una de las capturas elegidas no pertenece al caso.`, `«Título» es obligatorio.` y `«Título» supera 256 caracteres.`
- Comentar sin vínculo: `El caso no tiene un issue en <o/r>.` (`gh_comment_issue`, lo que ve el usuario). Abrir un issue sin vínculo (`gh_open_issue`) responde `El caso no tiene un issue en ese repo.`; es un camino que la UI no alcanza porque solo muestra chips de vínculos existentes.
- `Ya se está creando un issue de este caso en <o/r>.` y `Ya se está comentando este caso en <o/r>.` (envío duplicado en curso).
- `El caso ya tiene un issue en <o/r> (#<n>).` (`gh_conflict`).
- `El repo de evidencias <o/r> está vacío: crea un primer archivo (por ejemplo, un README) y vuelve a intentar.`
- Al importar un paquete con repo de evidencias (no se importa): `El paquete traía un repo de evidencias; confírmalo en Configurar proyecto › GitHub.`
- `No se pudieron leer los datos de GitHub del paquete; el proyecto se importó sin ellos.`

## Checklist de pruebas locales
| # | Prueba | Esperado | OK |
|---|---|---|---|
| 1 | Crear un issue con 2 capturas en `Ps3udoDev/testing`, con evidencias en `Ps3udoDev/qa-evidencias-prueba` | Las imágenes se ven | ☐ |
| 2 | Comentar un re-test con 1 captura | El comentario aparece con la imagen | ☐ |
| 3 | Proyecto sin repo de evidencias | Modo asistido, copiar y pegar | ☐ |
| 4 | Mismo caso hacia un segundo repo | Dos chips | ☐ |
| 5 | Exportar a Excel | Columna `Issue` | ☐ |
| 6 | Exportar el paquete y abrirlo en otra instalación | Vínculos conservados | ☐ |
| 7 | Cuando el owner apruebe: `TEAMS4SOFT/GMX-frontend` con `TEAMS4SOFT/qa-evidencias` | Un miembro sin acceso a `qa-evidencias` no ve las imágenes | ☐ |
