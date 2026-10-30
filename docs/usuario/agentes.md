# Usar Qapture con agentes

Qapture permite conectar Claude Code, Codex, Cursor y Claude Desktop mediante MCP. El agente trabaja sobre la instancia abierta de Qapture y los cambios aparecen en la ventana. Si la app está cerrada, el puente intenta abrirla oculta.

## Activar permisos

Abre **Ajustes › Agentes**, enciende «Permitir que agentes de IA usen Qapture (MCP)» y elige los grupos. Encender el interruptor no concede permisos: todos empiezan apagados. «Activar configuración recomendada» marca únicamente Lectura y Escritura. Pulsa **Guardar** para aplicar.

| Grupo | Herramientas | Alcance |
|---|---|---|
| Lectura | list_templates, list_projects, get_project, list_cases, get_case, get_evidence_image | Proyectos, campos, casos e imágenes; pueden contener datos del cliente. |
| Escritura | create_project, create_case, update_case, add_evidence, update_evidence_description | Crear desde plantillas existentes, cambiar datos y adjuntar imágenes. Nunca borrar. |
| Captura de pantalla | capture_screen, list_windows | Capturar un monitor o una ventana visible. Confirmación por uso activada por defecto. |
| Exportar | export_excel, export_package | Crear archivos locales .xlsx y .qapture. Confirmación por uso activada por defecto. |

`qapture_status` está disponible siempre para diagnosticar permisos. Las herramientas sin permiso no se listan. Las capturas notifican al tester incluso si desactiva su confirmación. En recuperación solo se permite lectura. No hay herramientas para borrar, publicar en GitHub ni navegar páginas; el navegador lo controla el agente con sus propias herramientas.

## Conectar un cliente

En **Conectar un agente**, elige tu cliente y pulsa **Copiar**. La configuración contiene la ruta real del puente instalado o portable, sin claves.

- **Claude Code:** ejecuta el comando copiado, de la forma `claude mcp add qapture -- "<ruta de qapture-mcp>"`.
- **Codex:** pega el bloque `[mcp_servers.qapture]` copiado en tu archivo de configuración TOML y vuelve a conectar el cliente.
- **Cursor:** pega el bloque JSON `mcpServers` en su configuración MCP.
- **Claude Desktop:** pega el mismo bloque JSON en su configuración MCP y reinicia el cliente.

Mantén `qapture-mcp.exe` junto a `qapture.exe` en Windows, también en el ZIP portable. Para AppImage, Qapture mantiene una copia del puente en la carpeta de datos; copia siempre la ruta mostrada en Ajustes. El soporte MCP de Linux requiere todavía una prueba de humo con escritorio antes de considerarse validado.

## Ejemplo: documentar una migración

Activa Lectura y Escritura y pide al agente usar el prompt MCP `documentar_migracion`, con `proyecto`, `modulo`, `url_actual` y `url_nueva`. Ejemplo:

> Documenta la migración de Emisión en «GMX – Migración Emisión». Confirma el proyecto, crea un caso por pantalla, adjunta «Sistema actual» y «Sistema migrado» y marca Aprobado o Fallido. Describe las diferencias en Resultado obtenido. No exportes hasta que te lo pida.

El agente confirma campos con `get_project`. Puede adjuntar PNG, JPEG o WebP por ruta absoluta o base64, hasta 25 MB; los normaliza antes de guardarlos. Los valores se aceptan por id, key o etiqueta; las etiquetas ignoran mayúsculas y tildes. `update_case` mezcla los valores enviados con los existentes. Revisa datos personales antes de exportar. Para capturar directamente con Qapture o exportar, concede esos permisos por separado.

## Actividad y problemas frecuentes

La tabla muestra cliente, herramienta, caso y resultado. Filtra por nombre de cliente y usa **Actualizar actividad**. «Ir al caso» abre el caso relacionado. Se conservan las últimas 2 000 operaciones. Los cambios producen un aviso agrupado cada cinco segundos.

| Mensaje | Qué hacer |
|---|---|
| Los agentes están desactivados. Actívalos en Ajustes › Agentes. | Activa el interruptor y guarda. |
| El permiso «grupo» está desactivado en Ajustes › Agentes. | Concede ese grupo y guarda. |
| Qapture no pudo abrir el canal para agentes. Reinicia la app. | Cierra Qapture desde la bandeja y vuelve a abrirlo; evita dos instancias sobre la misma carpeta de datos. |
| No se encontró Qapture y no se pudo abrir. Ábrelo y vuelve a intentar. | Abre la app y comprueba que el puente está junto al ejecutable. |
| La clave del agente no es válida. Copia de nuevo la configuración desde Ajustes › Agentes. | Vuelve a conectar el cliente a la instalación correcta. |
| qapture-mcp (…) no es compatible con Qapture (…). Reinstala Qapture para actualizar los dos. | Actualiza app y puente juntos. |
| Qapture se reinició; vuelve a intentar. | Repite la llamada; el puente reconecta, pero no repite escrituras en vuelo. |
| El tester rechazó la captura. / El tester rechazó la exportación. | Respeta la decisión del tester. |
| El tester no respondió en 3 minutos; no se hizo la captura. / … la exportación. | El diálogo «Permitir / Rechazar» espera como máximo 3 minutos. Si vence, Qapture avisa con una notificación y el diálogo que quede abierto ya no tiene efecto. Pide al tester que esté atento y vuelve a intentar. |

**Regenerar clave** solicita confirmación, cambia la clave local y desconecta sesiones abiertas. Vuelve a conectar tus agentes. Las exportaciones evitan sobrescribir archivos: añaden ` (2)`, ` (3)` y así sucesivamente.
