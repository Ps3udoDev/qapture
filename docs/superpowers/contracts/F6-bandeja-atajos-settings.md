# F6 — Segundo plano, bandeja, atajos globales y Settings · `v0.6.0`

**Depende de:** F2 (y F3 para el atajo de captura) · **Plan:** se escribe al iniciar la fase.

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

## Tasks
| ID | Task |
|---|---|
| F6-T1 | ✅ adelantada en PR #1 · `single-instance`: abrir el exe otra vez enfoca la ventana existente |
| F6-T2 | ✅ adelantada en PR #1 · `system/tray.rs`: ícono, tooltip, menú (Abrir Qapture, Ocultar, separador, Salir), clic izquierdo alterna mostrar/ocultar; cerrar ventana = ocultar. Pendientes en F6: Capturar, Pegar evidencia, Nuevo caso y Buscar actualizaciones |
| F6-T3 | Settings persistidos (tabla `setting` clave/valor, migración `0004`) + puerto `SettingsStore` |
| F6-T4 | `system/shortcuts.rs`: registro dinámico, detección de conflicto (conserva el anterior y reporta), re-registro al guardar |
| F6-T5 | Atajo "Pegar evidencia" global: lee la imagen del portapapeles → editor de F2, o notificación si no hay imagen |
| F6-T6 | Atajo "Nuevo caso": muestra la ventana con el formulario abierto en el proyecto activo |
| F6-T7 | Pantalla Settings con grabador de combinaciones de teclas |
| F6-T8 | Autostart y opción "iniciar oculto" |
| F6-T9 | El nombre del tester precarga "Responsable" y el diálogo de exportación (reemplaza la preferencia local de F4) |

## Criterios de aceptación
- [ ] Test de la lógica de validación y conflicto de atajos (sin registrar en el SO).
- [ ] Settings sobreviven a reinicios y migraciones.

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
