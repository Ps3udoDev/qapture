# ADR 0006: servidor MCP dentro de Qapture y puente local

- Fecha: 2026-10-29
- Estado: aceptado para F9a; validación manual de clientes e instalador pendiente.

## Contexto

Los agentes necesitan consultar matrices y registrar evidencias mientras el tester ve los cambios. La app posee una conexión SQLite serializada, migraciones, recuperación y barrido de imágenes huérfanas. Abrir la base desde otro proceso rompería esa coordinación.

## Decisión

**D2:** las herramientas MCP viven en el proceso Tauri y reutilizan AppState y los repositorios. Un puente `qapture-mcp` releva stdio al canal local. Solo la app escribe datos y emite `agent:data-changed`.

**D3:** named pipe en Windows y socket Unix en Linux. El nombre incluye 12 hex de SHA-256 de la ruta absoluta de datos para separar instalaciones normales y portables. No se abren puertos HTTP. ACL del usuario, secreto aleatorio en `mcp.key` y saludo versionado restringen el acceso.

**D7:** protocolo JSON-RPC escrito a mano para el subconjunto MCP requerido: initialize, ping, tools/list, tools/call, prompts/list y prompts/get. Una función procesa líneas sin E/S, facilitando pruebas. No se agrega rmcp. Se negocian las versiones indicadas en el contrato.

## Consecuencias

El puente necesita acompañar a la app en instaladores y ZIP portable. Ante reinicio reconecta y repite initialize, descarta la respuesta repetida y devuelve errores a llamadas pendientes, sin repetir escrituras. Versiones incompatibles se presentan como un servidor mínimo con un diagnóstico legible.

Los permisos están apagados por defecto y se vuelven a comprobar por llamada. Captura y exportación tienen confirmación por uso. El registro limita filas y excluye rutas, claves y contenido de imágenes. La interfaz se refresca por eventos y agrupa avisos. No se incluye borrado ni las herramientas GitHub y anotaciones de F9b.

## Validación

Pruebas unitarias de protocolo, repositorios, imágenes, políticas, reconexión y UI; canal local autenticado y construcción del puente. El checklist real con MCP Inspector, Claude Code, Codex, actualización e instalador debe quedar registrado en el contrato, sin sustituirse por pruebas unitarias. Linux requiere humo con escritorio.
