# F2 — Evidencias y editor de anotaciones · `v0.2.0`

**Depende de:** F1 · **Plan:** se escribe al iniciar la fase.

## Objetivo
Adjuntar capturas a los casos (pegando, arrastrando o eligiendo un archivo), anotarlas (flechas, números, texto, recuadros, resaltado, desenfoque y recorte) y gestionarlas. La imagen original siempre se conserva.

## Alcance
| Entra | Fuera |
|---|---|
| UC-12 Pegar con `Ctrl+V` dentro de la app (atajo local) | Atajo global de pegado (F6) |
| UC-13 Arrastrar o elegir imagen (PNG, JPEG, WebP) | Captura de pantalla integrada (F3) |
| UC-14 Editor de anotaciones con Konva | |
| UC-15 Reordenar, reasignar a otro caso, editar descripción, re-anotar, eliminar | |
| Visor ampliado de evidencia | |
| Contador de evidencias en lista y detalle | |
| Al eliminar caso o proyecto se borran sus archivos de evidencia | |

## Tasks
| ID | Task | Estado |
|---|---|---|
| F2-T1 | Migración `0002_evidence.sql` + repositorio Rust de evidencias (transacción db + archivos, borrado en cascada de archivos) | ✅ |
| F2-T2 | Almacenamiento de archivos: `evidence/<project>/<id>.orig.png` y `.render.png`, normalización a PNG, límite de 2400 px de lado mayor en el original (como en la v1), validación de magic bytes | ✅ |
| F2-T3 | Comandos: `add_evidence`, `update_evidence` (descripción, anotaciones, render), `move_evidence`, `reorder_evidence`, `delete_evidence`, `list_evidence`. Las imágenes se sirven por el protocolo `asset:` con alcance limitado a `evidence/` | ✅ |
| F2-T4 | Modelo de anotaciones (dominio TS): tipos de forma, serialización JSON versionada, numeración automática | ✅ |
| F2-T5 | Editor Konva: herramientas, selección, deshacer/rehacer, colores T4S, export a PNG a resolución nativa | ✅ |
| F2-T6 | Desenfoque/pixelado y recorte no destructivo | ✅ |
| F2-T7 | Flujo de registro: pegar/arrastrar → editor → caso + descripción obligatoria → guardar | ✅ |
| F2-T8 | Galería en el detalle del caso: miniaturas, visor, reordenar (drag), reasignar, eliminar | ✅ |
| F2-T9 | Actualizar el contrato DTO, los tests y el checklist | ✅ |

## Interfaces producidas
- Tabla `evidence` y comandos de evidencias.
- `EvidenceStore` (puerto TS) y evento de UI `openEvidenceEditor({ caseId?, imagePath })`, que F3 y F6 reutilizan.
- `rendered_path` siempre presente: F4 lo usa para el Excel.

## Criterios de aceptación (automáticos)
- [x] Rust: guardar evidencia es atómico (si falla la db no quedan archivos huérfanos, y viceversa). Tests: `repo::evidence::tests::add_rolls_back_files_when_the_db_fails_and_can_retry`, `repo::evidence::tests::add_leaves_no_row_when_files_fail`, `repo::evidence::tests::update_replaces_the_render_and_keeps_the_original`.
- [x] Rust: eliminar caso o proyecto elimina sus archivos. Test: `repo::evidence::tests::deleting_a_case_or_project_removes_its_files`.
- [x] TS: serializar → deserializar anotaciones da el mismo resultado; la numeración se reordena al borrar un marcador. Tests: `src/domain/annotations.test.ts`.
- [x] Rechazo de imágenes inválidas o de más de 12 000 px. Tests: `media::raster::tests::rejects_non_images_and_other_formats`, `media::raster::tests::rejects_more_than_12000_px_per_side`.

## Checklist de pruebas locales
| # | Prueba | Esperado | OK |
|---|---|---|---|
| 1 | `Win+Shift+S` y luego `Ctrl+V` en Qapture | Se abre el editor con la captura y el caso activo preseleccionado | ☐ |
| 2 | Agregar flecha, ①②③, texto y recuadro; guardar sin descripción | Exige descripción | ☐ |
| 3 | Guardar con descripción | Miniatura en el detalle; contador "1" en la lista | ☐ |
| 4 | Reabrir la evidencia y mover la flecha | Se edita (no queda "quemada") | ☐ |
| 5 | Desenfocar un número de póliza | En el render no se lee | ☐ |
| 6 | Arrastrar 3 imágenes a la vez | Se registran en secuencia | ☐ |
| 7 | Reordenar y reasignar a otro caso | Se refleja en ambos casos | ☐ |
| 8 | Eliminar el caso con evidencias | Desaparecen también sus archivos en `%APPDATA%` | ☐ |
| 9 | Captura 4K | Se ajusta a 2400 px y sigue nítida al ampliar | ☐ |
