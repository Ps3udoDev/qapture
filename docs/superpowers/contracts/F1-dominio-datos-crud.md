# F1 — Dominio, SQLite y CRUD · `v0.1.0`

**Plan detallado:** [`../plans/2026-10-15-f1-dominio-datos-crud.md`](../plans/2026-10-15-f1-dominio-datos-crud.md)

## Objetivo
Que un tester cree proyectos desde plantillas, personalice columnas y registre, edite, duplique, filtre y elimine casos de prueba. Todo persistido en SQLite y con la identidad visual de Qapture / T4S.

## Alcance
| Entra | Fuera (fase) |
|---|---|
| UC-01 Crear proyecto desde plantilla | Evidencias e imágenes (F2) |
| UC-02 Editar datos del proyecto | Exportar Excel (F4) |
| UC-03 Personalizar columnas (agregar, renombrar, reordenar, tipo, obligatorio, limpiar al duplicar, archivar) | Control de versiones del documento UC-05 (F4) |
| UC-04 Guardar columnas como plantilla, eliminar plantillas propias | Importar v1 / paquetes (F5) |
| UC-06/07 Crear y editar caso con formulario dinámico | Bandeja, atajos, settings (F6) |
| UC-08 Duplicar ejecución | |
| UC-09 Buscar y filtrar por estado | |
| UC-10 Eliminar caso (con confirmación) | |
| Eliminar proyecto (con confirmación) | |
| Ícono propio de Qapture | |

## Tasks
| ID | Task | Capa |
|---|---|---|
| F1-T1 | Tooling: limpiar plantilla, Tailwind + tokens T4S, Vitest, CSP, `.gitattributes` | infra |
| F1-T2 | Ícono de Qapture (SVG) y generación de íconos de la app | marca |
| F1-T3 | SQLite: errores, apertura, migraciones `user_version`, siembra de 4 plantillas | Rust |
| F1-T4 | Validación Rust: estados, tipos de campo, textos, slugs | Rust |
| F1-T5 | Repositorios de plantillas y proyectos | Rust |
| F1-T6 | Repositorio de columnas del proyecto (reglas de archivado y cambio de tipo) | Rust |
| F1-T7 | Repositorio de casos (código EJ-###, validación, duplicar) | Rust |
| F1-T8 | DTO + test de contrato + comandos Tauri + wiring | Rust |
| F1-T9 | Dominio TS: estados, campos, validación de casos, filtros, resumen, proyecto | TS |
| F1-T10 | Puertos, adaptador Tauri (zod + contrato) y adaptador en memoria | TS |
| F1-T11 | Store de la app (Zustand) y casos de uso | TS |
| F1-T12 | UI: shell, selector de proyecto, estado vacío, nuevo proyecto | UI |
| F1-T13 | UI: lista de casos, búsqueda/filtro, detalle, formulario dinámico, duplicar/eliminar | UI |
| F1-T14 | UI: configuración del proyecto (datos + editor de columnas + guardar plantilla) | UI |
| F1-T15 | Verificación integral, build local y cierre del contrato | QA |

## Interfaces producidas (las fases siguientes dependen de esto)
- **Base de datos:** `qapture.db` en `app_data_dir`, migraciones en `src-tauri/src/db/migrations/NNNN_*.sql` con `PRAGMA user_version`. F2 agrega `0002_evidence.sql`; F4, `0003_doc_version.sql`.
- **Comandos Tauri:** `list_templates`, `save_template_from_project`, `delete_template`, `list_projects`, `get_project`, `create_project`, `update_project`, `delete_project`, `save_project_fields`, `list_cases`, `create_case`, `update_case`, `duplicate_case`, `delete_case`.
- **Error de comando:** `{ code: "validation"|"not_found"|"db"|"io"|"internal", message, detail }`.
- **Puertos TS:** `TemplateRepository`, `ProjectRepository`, `CaseRepository` (`src/ports/repositories.ts`).
- **Store:** `createWorkspaceStore(repos)` en `src/ui/store/workspace.ts`.
- **Tokens de diseño:** `--color-t4s-*` en `src/ui/theme/tokens.css`.

## Criterios de aceptación (automáticos)
- [ ] `cargo test` en verde: migraciones idempotentes, siembra, reglas de columnas, código EJ-###, duplicar, contrato.
- [ ] `pnpm test` en verde: dominio, adaptadores, store, UI.
- [ ] `pnpm typecheck` y `cargo clippy -- -D warnings` sin errores.
- [ ] `pnpm tauri build` genera el instalador NSIS.

## Checklist de pruebas locales del equipo
Instalar `Qapture_0.1.0_x64-setup.exe` en un equipo limpio.

| # | Prueba | Resultado esperado | OK |
|---|---|---|---|
| 1 | Abrir Qapture por primera vez | Ícono propio en barra de tareas; pantalla "Crea tu primera matriz" con marca T4S | ☐ |
| 2 | Crear proyecto "Recibos GMX" con plantilla **Funcional**, cliente GMX, fechas | Aparece seleccionado con 0 casos; columnas de la v1 + Observaciones | ☐ |
| 3 | Intentar fecha fin anterior a inicio | Mensaje "La fecha de fin no puede ser anterior a la de inicio." | ☐ |
| 4 | Nuevo caso sin "Entorno" | No guarda; el campo marca "«Entorno» es obligatorio." | ☐ |
| 5 | Nuevo caso completo | Código **EJ-001**, aparece en la lista y en el detalle | ☐ |
| 6 | Duplicar EJ-001 | Se crea **EJ-002**, estado "No ejecutado"; Resultado obtenido, Fecha y Observaciones vacíos; el resto se conserva | ☐ |
| 7 | Eliminar EJ-002 y crear otro caso | El nuevo es **EJ-003** (los códigos no se reutilizan) | ☐ |
| 8 | Buscar "recib" y "RECÍB" | Ambos encuentran el caso (sin distinguir mayúsculas ni acentos) | ☐ |
| 9 | Filtrar por estado "Aprobado" | Solo casos aprobados; contador actualizado | ☐ |
| 10 | Configuración → Columnas: agregar "Severidad" tipo Lista (Alta, Media, Baja) | Aparece en el formulario como lista desplegable | ☐ |
| 11 | Usar "Media" en un caso, luego intentar quitar la opción "Media" | Mensaje: la opción está en uso; no se guarda | ☐ |
| 12 | Quitar la columna "Menú" (sin datos) | Desaparece por completo | ☐ |
| 13 | Quitar la columna "Módulo" (con datos) | Queda **archivada**: no se muestra, y al restaurarla vuelven sus datos | ☐ |
| 14 | Cambiar "Fecha de ejecución" (con datos) a tipo Número | Se rechaza con mensaje claro | ☐ |
| 15 | Guardar columnas como plantilla "GMX Cobranzas" | Disponible al crear un nuevo proyecto | ☐ |
| 16 | Eliminar plantilla "Funcional" | No permitido (plantilla incluida) | ☐ |
| 17 | Cerrar la app y volver a abrir | Todo persiste; se reabre el último proyecto usado | ☐ |
| 18 | Texto largo con saltos de línea y emojis en "Datos de entrada" | Se guarda y se muestra igual | ☐ |
| 19 | Eliminar proyecto | Pide confirmación escribiendo el nombre; luego desaparece con sus casos | ☐ |

**Firmado por:** ____________  **Fecha:** ________
