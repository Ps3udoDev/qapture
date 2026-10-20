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
- Cliente (Task 1): `GhClient::post_json` y `patch_json` (mismas cabeceras y errores que `get_json`); `post_blob_json` para los blobs de Git Data, con plazo propio de 120 s (`BLOB_TIMEOUT`; el resto usa los 20 s del cliente); `issues::repo_info`, `create`, `comment` y `repo_meta`.
- Subida (Task 2, módulo Rust, no comando): `evidence_upload::commit_files(client, token, repo, branch, files, message)`. La rama la pasa `report.rs` desde su única llamada a `repo_info`; los blobs se suben una vez y un 422 en la ref rehace solo tree, commit y ref.
- Comandos (Task 4): `gh_repo_meta`, `gh_project_links`, `gh_create_issue`, `gh_comment_issue`, `gh_copy_evidence` y `gh_open_issue`.
- `gh_open_issue` solo abre la `html_url` guardada si es `https://github.com/…` (`config::is_web_url`); si no, `validation`.
- Enlace de imagen: `https://github.com/<owner>/<repo>/blob/<commit_sha>/<ruta>?raw=true` (nunca `raw.githubusercontent.com`).
- `github.json`: cada vínculo exige `htmlUrl` == `https://github.com/<owner>/<repo>/issues/<issueNumber>` (dueño y repo sin distinguir mayúsculas) e `issueNumber` entre 1 e `i64::MAX`; si no, el archivo entero es inválido.

## Tasks
| ID | Task | Estado |
|---|---|---|
| F8b-T1 | Cliente con escritura y operaciones de issues | ✅ |
| F8b-T2 | Subida de evidencias en un commit | ✅ |
| F8b-T3 | Orquestación del reporte en Rust | ✅ |
| F8b-T4 | Comandos, DTO, contratos y ACL | ✅ |
| F8b-T5 | Plantilla del issue y ruteo en TS | ✅ |
| F8b-T6 | Datos de GitHub en los paquetes `.qapture` | ✅ |
| F8b-T7 | Columna «Issue» en el Excel | ✅ |
| F8b-T8 | Puerto, adaptadores y slice del reporte | ✅ |
| F8b-T9 | Diálogo «Reportar en GitHub», chips y panel asistido | ✅ |
| F8b-T10 | Documentación y prueba real | ☐ |

## Criterios de aceptación
- [x] **T1** Escritura con las mismas cabeceras y sin filtrar el token: `post_json_sends_body_and_headers`, `patch_json_maps_422_to_validation`, `write_errors_never_contain_the_token`, `blob_posts_get_their_own_longer_timeout`. Operaciones de issues: `repo_info_reads_push_permission_and_default_branch`, `create_issue_returns_number_and_url`, `create_issue_omits_empty_milestone`, `comment_returns_html_url`, `repo_meta_collects_labels_milestones_and_assignees`.
- [x] **T2** Ruta, enlace y commit único: `slug_follows_the_table`, `evidence_dir_has_the_agreed_shape`, `image_url_uses_blob_and_raw`, `commit_files_creates_one_commit_for_all_images`, `a_non_fast_forward_ref_update_is_retried` (el segundo commit cuelga del head nuevo; un blob por archivo), `gives_up_after_three_rejections`, `an_empty_evidence_repo_reports_a_clear_error`, `upload_errors_keep_their_kind`, `a_parent_commit_without_a_tree_is_an_error`.
- [x] **T3** Saneamiento y orquestación: `sanitize_body_drops_out_of_range_markers`, `sanitize_body_appends_missing_chosen_images_under_evidencias`, `sanitize_body_keeps_one_marker_per_image`, `fill_images_uses_alt_code_and_n`, `create_uploads_rendered_pngs_and_links_by_sha`, `create_without_evidence_repo_is_assisted`, `create_never_writes_outside_the_configured_evidence_repo`, `a_second_create_for_the_same_case_and_repo_conflicts`, `comment_requires_an_existing_link`.
- [x] **T4** Comandos y barrera: `open_issue_uses_the_stored_url`, `open_issue_refuses_a_stored_url_outside_github`, `copy_evidence_rejects_an_unknown_id`, `report_maps_assisted_reasons_to_fixed_texts`, `inputs_reject_unknown_fields`, `only_https_github_urls_are_web_urls`; ACL en `names.rs` y fixture de contratos.
- [x] **T5** Plantilla y ruteo (`issue-template.test.ts`, `repo-routing.test.ts`): «builds the exact body», «builds the exact comment», «trims the title to 256 characters», «routes by the mapped option, returning the linked casing».
- [x] **T6** Paquetes: `export_writes_github_json_only_when_there_is_github_data`, `round_trip_keeps_repos_routing_evidence_repo_and_links`, `links_to_unknown_case_codes_are_dropped`, `a_broken_github_json_imports_without_links`, `a_package_with_a_foreign_issue_url_imports_without_links`, `a_package_with_an_unknown_entry_still_imports`, `project_json_is_unchanged_by_github_data`.
- [x] **T7** Excel: `no_links_no_issue_column`, `one_link_is_a_hyperlink_with_repo_and_number`, `several_links_are_text_with_a_note_listing_urls`, `issue_column_goes_after_evidencias`, `a_stored_url_outside_github_is_plain_text`.
- [x] **T8** Slice (`github-slice.test.ts`): `report_success_adds_the_link_and_toasts`, `assisted_report_keeps_pending_items`, `conflict_reloads_links_and_returns_form_error`, `copy_pending_toasts_with_n`, `marking_all_done_clears_the_pending_entry`.
- [x] **T9** Diálogo (`Report.test.tsx`): «crea el issue y muestra el chip testing#1», «con vínculo existente cambia a «Comentar en Ps3udoDev/testing#1»», «modo asistido abre el panel y «Copiar» copia la captura 1», «doble clic en «Crear issue» crea un solo issue», «avisa si una captura pesa más de 10 MB», «si la descripción no se editó, cambiar las capturas la rearma sola», «si la descripción se editó, avisa que las capturas cambiaron hasta regenerarla».
- [ ] **T10** Guías revisadas y checklist de pruebas locales (abajo) completa.

## Mensajes fijos
- Validación del reporte: `Se permiten hasta 10 etiquetas.`, `Se permiten hasta 10 asignados.`, `Una de las capturas elegidas no pertenece al caso.`, `«Título» es obligatorio.`, `«Título» supera 256 caracteres.` y `La descripción supera 65 536 caracteres.`
- Repo del reporte: `El repositorio debe tener el formato dueño/repositorio.` y `«<o/r>» no está vinculado al proyecto.`
- Copiar una captura cuyo PNG no se puede decodificar: `La captura no se pudo leer.`
- Abrir un vínculo cuya URL guardada no es `https://github.com/…`: `El vínculo del issue no apunta a GitHub.`
- Diálogo, si la selección de capturas cambió y la descripción fue editada: `Cambiaste las capturas: regenera la descripción para que coincidan.` (si no fue editada, se rearma sola).
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
