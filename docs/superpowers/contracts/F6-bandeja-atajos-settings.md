# F6 — Segundo plano, bandeja, atajos globales y Settings · `v0.6.0`

**Depende de:** F2 (y F3 para el atajo de captura) · **Plan:** [2026-10-17-f6-bandeja-atajos-settings](../plans/2026-10-17-f6-bandeja-atajos-settings.md).

## Objetivo
Qapture vive en la bandeja del sistema y se invoca con atajos globales configurables desde Settings.

## Alcance
| Entra | Fuera |
|---|---|
| UC-21 Bandeja con menú; cerrar = ocultar; instancia única | Atajos por proyecto |
| UC-22 Settings: perfil (tester), atajos, sistema, exportación, datos, acerca de | Sincronizar settings entre equipos |
| Atajos por defecto: Mostrar/ocultar `Ctrl+Alt+Q` · Capturar `Ctrl+Alt+S` · Pegar evidencia `Ctrl+Alt+V` · Nuevo caso `Ctrl+Alt+N` | |
| Iniciar con Windows (autostart), opción minimizado | |
| Notificaciones nativas para eventos en segundo plano | |

## Notas de implementación (acordadas en el plan)
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

## Interfaces producidas
- Comandos (Task 4): `get_settings() -> SettingsDto`, `save_settings({ settings }) -> SettingsDto`, `pick_export_folder() -> string | null`, `open_data_folder() -> null`, `hide_main_window() -> null`, `open_link({ target: "releases" | "docs" }) -> null` (F8a añadió `githubDevice | githubInstall | githubRevoke | githubAdminGuide`; ver [F8a](F8a-github-conexion.md)).
- `SettingsDto { settings, shortcutFailures: ShortcutFailureDto[], appVersion, dataDir }`; `ShortcutFailureDto { action: "toggle"|"capture"|"paste"|"newCase", accelerator }`.
- Evento `shortcut:new-case`: sin payload, hacia la ventana `main`; lo emite el atajo «Nuevo caso».

## Tasks
| ID | Task | Estado |
|---|---|---|
| F6-T1 | ✅ adelantada en PR #1 · `single-instance`: abrir el exe otra vez enfoca la ventana existente | ✅ |
| F6-T2 | ✅ adelantada en PR #1 · `system/tray.rs`: ícono, tooltip, menú (Abrir Qapture, Ocultar, separador, Salir), clic izquierdo alterna mostrar/ocultar; cerrar ventana = ocultar. Pendientes en F6: Capturar, Pegar evidencia, Nuevo caso y Buscar actualizaciones | ✅ |
| F6-T3 | Settings persistidos (tabla `setting` clave/valor, migración `0004`) + puerto `SettingsStore` | ✅ |
| F6-T4 | `system/shortcuts.rs`: registro dinámico, detección de conflicto (conserva el anterior y reporta), re-registro al guardar | ✅ |
| F6-T5 | Atajo "Pegar evidencia" global: lee la imagen del portapapeles → editor de F2, o notificación si no hay imagen | ✅ |
| F6-T6 | Atajo "Nuevo caso": muestra la ventana con el formulario abierto en el proyecto activo | ✅ |
| F6-T7 | Pantalla Settings con grabador de combinaciones de teclas | ✅ |
| F6-T8 | Autostart y opción "iniciar oculto" | ✅ |
| F6-T9 | El nombre del tester precarga "Responsable" y el diálogo de exportación (reemplaza la preferencia local de F4) | ✅ |

## Criterios de aceptación
- [x] Test de la lógica de validación y conflicto de atajos (sin registrar en el SO): `system::shortcuts::tests::*` y `commands::settings::tests::a_refused_shortcut_saves_nothing`.
- [x] Settings sobreviven a reinicios y migraciones: `repo::settings::tests::saved_settings_survive_reopening_the_file_db` y `repo::settings::tests::settings_survive_the_0004_migration`.

## Checklist de pruebas locales
| # | Prueba | Esperado | OK |
|---|---|---|---|
| 1 | Cerrar con la X | La app sigue en la bandeja | ☐ |
| 2 | `Ctrl+Alt+Q` dos veces | Muestra y oculta | ☐ |
| 3 | `Win+Shift+S` y luego `Ctrl+Alt+V` con la app oculta | Aparece el editor con la captura | ☐ |
| 4 | `Ctrl+Alt+V` sin imagen en el portapapeles | Notificación, sin ventana | ☐ |
| 5 | Cambiar Mostrar/ocultar a `Ctrl+Shift+F12` | Funciona de inmediato; el anterior deja de funcionar | ☐ |
| 6 | Asignar un atajo usado por otra app | Mensaje de conflicto; se conserva el anterior | ☐ |
| 7 | Activar "Iniciar con Windows" y reiniciar | Qapture en la bandeja tras iniciar sesión | ☐ |
| 8 | Doble clic en el exe con la app ya abierta | Se enfoca la existente, no se abre otra | ☐ |
| 9 | Menú bandeja → Salir | El proceso termina | ☐ |
