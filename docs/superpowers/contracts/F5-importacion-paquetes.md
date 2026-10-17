# F5 — Importación v1, paquetes `.qapture` y respaldos · `v0.5.0`

**Depende de:** F2 (evidencias) · **Plan:** [2026-10-16-f5-importacion-paquetes.md](../plans/2026-10-16-f5-importacion-paquetes.md)

## Objetivo
Migrar los datos de la Bitácora v1 sin pérdidas, compartir proyectos entre compañeros y proteger los datos con respaldos.

## Alcance
| Entra | Fuera |
|---|---|
| UC-19 Importar `matrices.json` o respaldo JSON v1 (schema 1) | Sincronización automática entre equipos |
| UC-18 Exportar/importar paquete `.qapture` (ZIP: `project.json` + `evidence/`) | Fusionar cambios de dos personas sobre el mismo proyecto |
| UC-20 Respaldo automático diario de la db (se conservan 7), respaldo manual y restauración | |
| Modo recuperación si la db no abre | |

## Tasks
| ID | Task |
|---|---|
| F5-T1 | `legacy.rs`: parser y validación del schema v1 (mismas reglas que `store.mjs`), mapeo a la plantilla Funcional, decodificación de base64 a PNG |
| F5-T2 | Fixture v1 **anonimizado** (sin datos de GMX) para tests |
| F5-T3 | Formato `.qapture` versionado (`formatVersion: 1`), exportar e importar como copia ("(importado)" si el nombre existe) |
| F5-T4 | Respaldos: copia diaria con la API de backup de SQLite, rotación de 7, respaldo manual y restaurar con confirmación |
| F5-T5 | Modo recuperación al arrancar con db corrupta |
| F5-T6 | UI: menú Datos (importar v1, importar/exportar paquete, respaldos) con resumen del resultado |

## Notas de implementación (acordadas en el plan)
- Las capturas PNG de la v1 se guardan byte a byte (original = render, sin reducir). Las JPEG se convierten a PNG. «Mismo hash» se verifica comparando bytes.
- La validación v1 usa las reglas y mensajes de `store.mjs`, pero junta hasta 10 problemas con su ubicación (proyecto, caso, captura).
- Si un valor v1 supera 500 caracteres en una columna de texto corto, la columna se crea como «Texto largo». Las descripciones de hasta 4000 caracteres se importan completas.
- `project.json` no lleva ids ni la plantilla de origen. Las imágenes van como `evidence/<código>-<n>.orig.png` y `.render.png`. Exportar → importar → exportar da el mismo `project.json` byte a byte.
- Al exportar un paquete, las capturas sin imagen se omiten y se avisa cuántas fueron.
- Respaldos: `backups/qapture-diario-AAAAMMDD.db` (rotan 7), `qapture-manual-*` y `qapture-antes-de-restaurar-*` (no rotan). Restaurar deja antes una copia del estado actual y mueve a `evidence-sin-uso/<fecha>/` las capturas que el respaldo no conoce.
- Modo recuperación: base que no abre o que no pasa `PRAGMA quick_check`. La base dañada se conserva como `qapture-danada-<fecha>.db`. Mientras dura, no se limpian evidencias ni se hacen respaldos.
- Las acciones viven en el menú **Datos** de la barra superior hasta que F6 las lleve a Settings › Datos.

## Interfaces producidas
- Comandos `import_legacy`, `import_package`, `export_package(projectId)`, `list_backups`, `create_backup`, `restore_backup(fileName)`, `get_startup_status`, `recover_database(fileName | null)`. F6 los reutiliza en Settings › Datos.
- Formato `.qapture` `formatVersion: 1` (ver plan, Task 4). Un cambio incompatible sube `formatVersion`; las versiones viejas de la app rechazan los paquetes nuevos con un mensaje claro.

## Criterios de aceptación
- [ ] Importar el fixture v1 conserva los códigos EJ-###, `next_sequence`, los textos con saltos de línea y las imágenes (mismo hash).
- [ ] La importación es todo o nada; un error deja la db intacta.
- [ ] Exportar paquete → importar → exportar da el mismo `project.json` (salvo ids).

## Checklist de pruebas locales
| # | Prueba | Esperado | OK |
|---|---|---|---|
| 1 | Importar el `matrices.json` real de la v1 | "Aplicación de recibos" con EJ-001 y sus 2 capturas | ☐ |
| 2 | Importar un respaldo v1 dañado | Error claro y nada importado | ☐ |
| 3 | Exportar un `.qapture` y que un compañero lo importe | Ve el mismo proyecto con evidencias | ☐ |
| 4 | Importar dos veces el mismo paquete | Crea copia "(importado)", sin sobrescribir | ☐ |
| 5 | Restaurar un respaldo | Vuelve al estado del respaldo tras confirmar | ☐ |
