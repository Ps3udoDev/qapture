# Checklist manual de release

Se ejecuta antes de **publicar** el borrador de cada release (spec §10, «Manual por release»), con los instaladores del borrador. Anota la versión y quién la probó.

**Versión:** ______ · **Probado por:** ______ · **Fecha:** ______

Usa datos ficticios. Si la app instalada ya está en esta máquina, cierra primero con **Salir** en la bandeja: la instancia única entrega el control a la que ya corre.

## Instalación

| # | Prueba | Esperado | OK |
|---|---|---|---|
| 1 | Descargar el `.exe` (NSIS) del borrador e instalar | Instala; el aviso de SmartScreen coincide con `docs/usuario/instalacion.md` | ☐ |
| 2 | Instalar el `.msi` | Instala sin errores | ☐ |
| 3 | La versión en **Ajustes › Acerca de** | Es la del tag | ☐ |
| 3b | Abrir el `latest.json` del borrador y revisar las claves de `platforms` | Están `windows-x86_64` y `windows-x86_64-nsis` (y `windows-x86_64-msi` si existe); `windows-x86_64` apunta al `-setup.exe` (NSIS), cada `url` es de esta versión y cada `signature` no está vacía. Sin la clave `-msi`, una instalación MSI se actualizaría con el instalador NSIS por usuario: el MSI se actualiza redesplegando el `.msi` nuevo | ☐ |

## Excel

| # | Prueba | Esperado | OK |
|---|---|---|---|
| 4 | Exportar un proyecto con varios casos y capturas; abrirlo en **Excel** | Hojas Carátula, Matriz, Evidencias y Resumen; imágenes visibles; vínculos entre hojas funcionan | ☐ |
| 5 | El mismo archivo en **WPS Office** | Se abre sin reparaciones; hojas, imágenes y vínculos correctos | ☐ |
| 6 | Exportar «Plantilla vacía» | Carátula y columnas, sin casos | ☐ |

## Atajos y bandeja

| # | Prueba | Esperado | OK |
|---|---|---|---|
| 7 | `Ctrl+Alt+Q` dos veces | Muestra y oculta la ventana | ☐ |
| 8 | `Ctrl+Alt+S` desde otro programa | Aparece el selector de región; se captura y abre el editor | ☐ |
| 9 | `Win+Shift+S` y luego `Ctrl+Alt+V` con la app oculta | Se abre el editor con la captura | ☐ |
| 10 | `Ctrl+Alt+V` sin imagen en el portapapeles | Notificación, sin ventana | ☐ |
| 11 | `Ctrl+Alt+N` | Se abre el formulario de caso | ☐ |
| 12 | Cambiar un atajo en **Ajustes › Atajos** | El nuevo funciona al instante; el anterior deja de funcionar | ☐ |
| 13 | Cerrar con la X | La app sigue en la bandeja | ☐ |
| 14 | Menú de bandeja | Abrir Qapture, Ocultar, Capturar pantalla, Pegar evidencia, Nuevo caso, Buscar actualizaciones, Salir; **Salir** termina el proceso | ☐ |
| 15 | Abrir el exe con la app ya abierta | Se enfoca la existente | ☐ |

## Updater

Requiere una versión **publicada** más nueva que la instalada: los borradores no los ve la app. Se prueba con el release anterior instalado y el nuevo ya publicado.

| # | Prueba | Esperado | OK |
|---|---|---|---|
| 16 | Con la versión anterior instalada, esperar el aviso o usar **Buscar actualizaciones** | Diálogo «Nueva versión disponible» con la versión correcta | ☐ |
| 17 | **Después** | Se cierra el aviso; hay indicador en Ajustes y «Instalar actualización <nueva>» en la bandeja | ☐ |
| 17b | Clic en «Instalar actualización <nueva>» en la bandeja | Se muestra la ventana y vuelve a abrirse el aviso | ☐ |
| 18 | **Instalar y reiniciar** | Descarga con avance, instala y reinicia en la versión nueva; los datos se conservan | ☐ |
| 19 | Con una captura sin guardar en el editor, intentar instalar | Mensaje «Guarda o descarta la captura abierta antes de actualizar.» | ☐ |
| 20 | Sin internet, **Buscar actualizaciones** | «No se pudo buscar actualizaciones. Revisa tu conexión a internet.» | ☐ |
| 21 | Con la última versión, **Buscar actualizaciones** | «Qapture está al día (versión <actual>).» | ☐ |

## Portable

| # | Prueba | Esperado | OK |
|---|---|---|---|
| 22 | Extraer el ZIP en una carpeta de usuario (o USB) y ejecutar `Qapture.exe` | Corre sin instalar; los datos quedan en `.\data` | ☐ |
| 23 | Extraerlo en `C:\Program Files\Qapture` | Mensaje «No se puede escribir en …» y la app se cierra; no se crea nada en `%APPDATA%` | ☐ |
| 24 | En **Ajustes › Sistema**, «Iniciar con Windows» | «No disponible en la versión portable.» | ☐ |
| 25 | El aviso de actualización en la portable | Texto de versión portable y botón **Descargar** que abre Releases; en la bandeja, «Descargar actualización <nueva>» | ☐ |

## Cierre

- [ ] Todo OK: publicar el borrador.
- [ ] Hallazgos: no publicar; corregir y repetir con una versión nueva.
