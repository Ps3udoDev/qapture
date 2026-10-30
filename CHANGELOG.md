# Cambios

## 1.2.0 — 2026-10-30

### Nuevo
- Agentes de IA en Qapture (MCP): Claude Code, Codex, Claude Desktop o Cursor pueden documentar pruebas en la app. Crean proyectos desde tus plantillas, crean y actualizan casos, adjuntan evidencias, capturan la pantalla y exportan, y la ventana se actualiza en vivo
- Ajustes › Agentes: interruptor general y permisos por grupo (Lectura, Escritura, Captura de pantalla, Exportar), todos apagados por defecto; configuración lista para copiar en cada cliente, registro de actividad y cambio de clave
- Capturar la pantalla y exportar piden «Permitir / Rechazar» cada vez (se puede desactivar). La pregunta vence a los 3 minutos y la captura se toma después de cerrarla
- Los agentes nunca borran nada y solo se conectan desde tu equipo, con una clave local; las exportaciones no sobrescriben archivos
- Prompt «documentar_migracion» y guía para conectar agentes
- El puente qapture-mcp se incluye en el instalador, el MSI y el ZIP portable

### Nota al actualizar
- Si tienes un agente conectado, el instalador cierra su conexión con Qapture: al terminar, reconecta el servidor MCP en el agente (en Claude Code, /mcp)

## 1.1.0 — 2026-10-20

### Nuevo
- Conectar tu cuenta de GitHub desde Ajustes › GitHub, sin contraseñas ni tokens manuales: código en pantalla, autorización en el navegador y sesión guardada en el llavero del sistema
- Vincular repos de GitHub a cada proyecto en Configurar proyecto › GitHub: repo predeterminado, etiquetas, ruteo por columna y repo de evidencias
- Guías para conectar con GitHub y para que el administrador de una organización instale la App
- Reportar un caso como issue de GitHub desde su detalle, con título, descripción editable, etiquetas, asignados, milestone y capturas ya renderizadas; o comentar un re-test si el caso ya tiene un issue en ese repo
- Las capturas se suben en un solo commit al repo de evidencias del proyecto y se enlazan en el issue (las ve quien tenga lectura de ese repo)
- Modo asistido cuando no se pueden subir: copias cada captura y la pegas con Ctrl+V en GitHub, con aviso de las pendientes mientras la app siga abierta
- Chips `repo#n` en el caso para abrir sus issues, ícono en la lista de casos y columna «Issue» en el Excel
- Los paquetes `.qapture` pueden llevar los datos de GitHub (repos, ruteo y vínculos) en un `github.json` opcional; la versión 1.0 los abre sin esos datos

## 1.0.2 — 2026-10-19

### Nuevo
- Qapture para Linux: instaladores `.AppImage` (se actualiza solo), `.deb` y `.rpm` en cada release

## 1.0.1 — 2026-10-18

### Nuevo
- Video de presentación de Qapture y vista previa en el README

## 1.0.0 — 2026-10-17

### Nuevo
- Actualizaciones dentro de la app: revisa al iniciar y cada 6 horas, avisa, muestra el progreso, instala y reinicia; también desde Acerca de y la bandeja
- Versión portable en ZIP: los datos quedan junto al exe
- Publicación automática: cada tag genera el instalador, el MSI, el ZIP portable y la actualización firmada
- Documentación de usuario y de desarrollo

## 0.6.0

Qapture vive en la bandeja del sistema y se invoca con atajos globales configurables desde Ajustes.

## 0.5.0

Migración de los datos de la Bitácora v1 sin pérdidas, proyectos compartibles entre compañeros y respaldos que protegen los datos.

## 0.4.0 — 2026-10-16

Exportación del proyecto a un `.xlsx` con las hojas Carátula, Matriz (columnas dinámicas), Evidencias y Resumen, con branding Teams4Soft.

## 0.3.0

Captura de pantalla o de una región desde Qapture, sin herramientas externas, directo al editor de anotaciones.

## 0.2.0

Evidencias adjuntas a los casos (pegando, arrastrando o eligiendo un archivo) y anotaciones con flechas, números, texto, recuadros, resaltado, desenfoque y recorte. La imagen original siempre se conserva.

## 0.1.0

Proyectos desde plantillas, columnas personalizables y registro, edición, duplicado, filtro y eliminación de casos de prueba, todo en SQLite y con la identidad visual de Qapture / T4S.
