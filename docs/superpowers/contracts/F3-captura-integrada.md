# F3 — Captura de pantalla integrada · `v0.3.0`

**Depende de:** F2 (editor y flujo de registro) · **Plan:** [2026-10-16-f3-captura-integrada.md](../plans/2026-10-16-f3-captura-integrada.md)

## Objetivo
Capturar la pantalla o una región desde Qapture, sin herramientas externas, y llevarla directo al editor de anotaciones.

## Alcance
| Entra | Fuera |
|---|---|
| UC-11 Captura multi-monitor con `xcap` | Grabación de video / GIF |
| Overlay de selección: arrastrar región, `Enter` = monitor completo, `Esc` = cancelar | Captura de ventana específica por título (posible v1.1) |
| Botón "Capturar" en la app (oculta Qapture, captura y vuelve) | Atajo global (lo registra F6, que reutiliza este comando) |
| Soporte de escalado DPI (125 %, 150 %) y monitores con distinta escala | |

## Tasks
| ID | Task | Estado |
|---|---|---|
| F3-T1 | `capture/screen.rs`: captura de todos los monitores a PNG temporal + metadatos de posición y escala | ✅ |
| F3-T2 | Ventana overlay (label `overlay`) sin bordes, siempre encima, una por monitor; capability mínima propia | ✅ |
| F3-T3 | UI del overlay: imagen congelada, oscurecido, rectángulo de selección con medidas, lupa opcional | ✅ |
| F3-T4 | `capture/crop.rs`: recorte en píxeles físicos según DPI; pruebas con escalas mixtas | ✅ |
| F3-T5 | Comando `start_capture` → evento `evidence:captured` → editor de F2 | ✅ |
| F3-T6 | Limpieza de temporales (al guardar, al cancelar y al iniciar la app) | ✅ |

## Notas de implementación (acordadas en el plan)
- Las imágenes de los monitores se guardan **en memoria** mientras el overlay está abierto, no como PNG temporales. El único temporal es el recorte, en `evidence/_staging/`, con la limpieza de F2 (al guardar, al cancelar y al iniciar la app).
- La lupa queda fuera de esta fase. Junto a la selección se muestra su medida en píxeles reales.
- `Enter` captura el monitor completo que está bajo el puntero. Una selección vive dentro de un solo monitor.
- Si Qapture estaba oculto (por ejemplo, con el atajo de F6), al cancelar sigue oculto y al capturar se muestra con el editor.

## Interfaces producidas
- Comando `start_capture()` y evento `evidence:captured { tmpPath }`, que el atajo global de F6 invoca.

## Criterios de aceptación
- [x] Prueba Rust del recorte con escalas 1.0 / 1.25 / 1.5 y monitores con offset negativo. Cubierta por:
  - `capture::crop::tests::scales_css_pixels_to_physical_pixels_at_100_125_and_150_percent`
  - `capture::session::tests::finish_crops_the_selected_monitor_even_with_a_negative_offset`
  - `capture::screen::tests::finds_the_monitor_under_a_point_with_negative_offsets`
- [x] No quedan temporales después de cancelar. Cubierta por:
  - `capture::session::tests::cancel_leaves_no_temporary_files_and_restores_visibility`
  - `src/ui/store/evidence-slice.test.ts` («cancelar el editor de una captura no deja temporales»)

## Checklist de pruebas locales
| # | Prueba | Esperado | OK |
|---|---|---|---|
| 1 | Botón Capturar con un monitor | Qapture se oculta, aparece el overlay; al seleccionar se abre el editor | ☐ |
| 2 | Dos monitores con distinta escala | La región capturada coincide exactamente con lo seleccionado | ☐ |
| 3 | `Esc` en el overlay | Vuelve a Qapture sin crear nada | ☐ |
| 4 | `Enter` en el overlay | Captura el monitor completo | ☐ |
| 5 | Captura sobre un video o una app en pantalla completa | Funciona | ☐ |
