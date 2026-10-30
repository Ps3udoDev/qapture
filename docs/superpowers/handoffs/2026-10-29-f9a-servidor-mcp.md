# Handoff — F9a: servidor MCP para agentes

> Fecha: 2026-10-29 · Rama: `feat/f9a-servidor-mcp` · Base: `main` (`bd63691`). Sin push, merge ni tag.
> Ejecución: Codex con tres subagentes GPT-6.1 Sol para persistencia/revisión, herramientas y transporte. Revisión final de producto pendiente.

## 1. Contexto para retomar

Qapture es una aplicación Tauri 2 + React 19 + TypeScript + Rust/SQLite para matrices y evidencias QA. El objetivo F9a es conectar agentes MCP locales con permisos explícitos del tester y refrescar la UI en vivo. Los documentos fuente son:

- [Plan de 12 tareas](../plans/2026-10-29-f9a-servidor-mcp.md).
- [Spec aprobado](../specs/2026-10-29-mcp-design.md), excepto §5.2 (F9b).
- [Contrato F9a y checklist manual](../contracts/F9a-servidor-mcp.md).
- [Guía de usuario](../../usuario/agentes.md) y [ADR 0006](../../adr/0006-servidor-mcp.md).

Se creó la rama desde main y se preservaron los documentos editados/sin seguimiento en el commit inicial `f428a4b`: cierre F8b, contratos F8a/F8b/README, spec GitHub, spec MCP, revisión Manus y plan. No se descartó trabajo previo.

## 2. Implementación

- T1: `Settings.agents` en Rust/TS con defaults compatibles; persistencia explícita de la sección; migración 0006; actividad con filtro, orden estable y poda transaccional a 2 000 filas; DTO y fixture regenerado con el comando autorizado.
- T2: protocolo JSON-RPC por líneas, negociación de versiones, catálogo condicionado por permisos, prompt `documentar_migracion`, errores y actividad sin argumentos privados.
- T3: lectura de plantillas/proyectos/casos/imágenes, páginas con cursor y filtros, resolución de campos por id/key/etiqueta sin tildes/mayúsculas y miniaturas PNG de hasta 1568 px.
- T4: creación de proyectos desde plantillas existentes, creación/mezcla de casos, evidencia por path/base64 y descripciones. Se reutilizan validaciones y staging de los repositorios; la conexión serializada evita códigos duplicados entre sesiones.
- T5: captura de monitor o ventana con confirmación y notificación; exportación Excel/paquete con nombres sin colisión y confirmación. La captura real selecciona únicamente el monitor solicitado.
- T6: pipe Windows con ACL/denegación remota o socket Unix con permisos de usuario; secreto local, saludo y versiones, límite de línea, timeout y desconexión. El listener arranca después de cargar ajustes.
- T7: binario `qapture-mcp`, arranque oculto, fallback legible, reconexión y replay interno de initialize, errores para llamadas en vuelo y stdout exclusivamente MCP.
- T8: configuración/actividad/rotación de clave por comandos Tauri, registrados en los tres puntos de ACL.
- T9: puerto/adaptadores real y memoria, slice, recarga selectiva por eventos y avisos agrupados en ventanas de cinco segundos, con cancelación de suscripciones.
- T10: Ajustes › Agentes, permisos sin concesión implícita, configuración recomendada solo Lectura/Escritura, configuraciones copiables, actividad/filtrado/navegación y confirmación para rotar clave.
- T11: script de sidecar, configuración de bundler y CI, ZIP portable con puente y extracción persistente para AppImage.
- T12: guía, ADR, arquitectura, README/índice y changelog 1.2.0. La prueba real con clientes e instalación sigue pendiente; no se declara cerrada F9a.

## 3. Decisiones y diferencias del plan

1. **MCP compatible:** `structuredContent` solo admite objetos en la versión 2025-06-18. Se conservan los objetos y se envuelven las listas como `{ "items": [...] }`; el texto conserva el JSON original. [Schema oficial consultado](https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/schema/2025-06-18/schema.json).
2. **Fechas:** actividad usa `db::now()` en UTC RFC3339, igual que los repositorios existentes. La frase del plan «local, como now()» no coincidía con el helper real.
3. **Captura selectiva:** se extendió `AgentScreen` con `monitor_count` y `capture_monitor`; listar monitores devuelve imágenes según la interfaz original y habría capturado monitores ajenos o un destino inválido. El host Xcap consulta cantidad sin capturar y captura solo tras permiso.
4. **Excel sin versiones:** MCP admite exportar antes de la primera revisión del documento. Construye opciones explícitas con `version: None`; no cambia la validación de la exportación interactiva. Si hay versiones, usa la última disponible.
5. **Tokio:** `macros` también en producción por `tokio::select!`. Las features dev hacían pasar tests, pero el primer release detectó la ausencia.
6. **Bootstrap del sidecar:** `build.rs` permite compilar/tests en checkout limpio sin sidecar previo. El bundler sigue exigiendo el binario real, generado por `beforeBuildCommand`; no se comitean ejecutables de relleno.
7. **Arranque del puente y lectura cancelable:** la clave se lee después de intentar abrir la app para permitir el primer arranque que la genera. Los fragmentos de líneas pertenecen a la conexión, evitando pérdida de JSON cuando `select!` elige otra rama. El puente conserva bytes recibidos junto al saludo.
8. **Seguridad y actividad:** metadatos de evidencias sin rutas internas; errores de archivos genéricos; la actividad verifica ids contra la base y guarda solo nombre de herramienta en detail. No se registran contenido, paths, base64 o clave.
9. **Interfaz:** cargas de actividad tienen secuencia para que una respuesta lenta no reemplace el filtro nuevo. Rotar clave conserva el filtro. «Ir al caso» valida que siga existiendo, navega y cierra Ajustes.
10. **Herramientas de trabajo:** las skills superpowers citadas por el plan no estaban disponibles; se siguieron los pasos explícitos y se delegó en tres subagentes menores. Se leyó `postman:api-engineer`; no se creó workspace HTTP/Postman porque este cambio es un protocolo local sin dependencia nueva de servicios remotos.
11. **Verificación:** se separaron pruebas focalizadas y una verificación integrada final. T1/T2 tienen evidencia rojo/verde; no se afirma haber ejecutado todos los pasos rojos ni los seis gates tras cada tarea en aislamiento. Los commits se dividieron preparando el index sin descartar el worktree compartido.
12. **Autoría:** los commits de implementación usan `Co-Authored-By: OpenAI Codex` para reflejar la herramienta ejecutora; el commit inicial de documentos conserva el trailer literal previsto en el plan.
13. **Actualización portable:** se corrigieron LEEME, la guía de instalación y el aviso de actualización Rust/TS para reemplazar app y puente juntos, evitando versiones mezcladas.
14. **Versión:** los manifiestos conservan 1.1.0; el plan solo pide changelog 1.2.0 sin fecha. El release 1.2.0 y su actualización coordinada de versiones no se publicaron.

## 4. Evidencia de validación

Gates integrados correctos. Empaquetado release pendiente de finalizar; no se declara validado el instalador todavía.

- Frontend: `pnpm lint` y `pnpm typecheck`, correctos.
- `pnpm vitest run --maxWorkers=4`: **381 pruebas, 53 archivos**, correctos.
- `pnpm build`: build de producción correcto, también después del ajuste del aviso portable. `pnpm check-version`: versión 1.1.0 coherente en los cuatro archivos.
- Después del último ajuste de texto portable: `pnpm vitest run --maxWorkers=4 Update update`, 23 aprobadas; `cargo test system::updater --lib`, 4 aprobadas; fmt, lint y typecheck correctos.
- Rust: `cargo fmt --check` y `cargo clippy --all-targets -- -D warnings`, correctos. `cargo test`: **473 unitarias aprobadas, 3 ignoradas de escritorio y 2 pruebas de integración del puente aprobadas**.
- Protocolo focalizado: **16/16**.
- Canal: **7/7**, incluyendo pipe real Windows/ACL, cliente autenticado initialize/list, falta de saludo, list_changed, desconexión, límite y lectura parcial cancelada.
- Binario del puente: **2/2** pruebas de integración, stdout JSON y reconexión sobre pipes reales con replay oculto y error a la llamada en vuelo.
- Scripts: `node --test scripts/sidecar.test.mjs`, **2/2**. El comando literal del plan `node --test scripts/` falla resolviendo el directorio en Node 24/Windows; los otros scripts usan Vitest.
- Hubo un fallo inicial del test de migración que fijaba 5; se corrigió usando `db::latest_version()` para admitir 0006.
- Hubo un fallo inicial de metadatos de actividad; el focalizado posterior confirma ids de proyecto/caso y ausencia de argumentos privados.

## 5. Bloqueos y validación pendiente

No se marcaron pruebas manuales como ejecutadas. La app instalada del usuario estaba abierta (`AppData/Local/Qapture/qapture.exe`); no se cerró ni se instaló sobre sus datos. Las pruebas del puente usan carpetas portables temporales y pipes independientes.

Faltan las filas 1–8 del checklist contractual con el instalador local: MCP Inspector, Claude Code y Codex, confirmaciones/notificaciones nativas, exportación, actualización con sesión viva, portable y migración completa de 10+ casos/20+ capturas. Esto es la puerta de salida obligatoria para cerrar F9a. También falta el humo Linux con distribución/versión/escritorio; no se anuncia ese soporte MCP como validado.

La compilación de paquetes y las pruebas automáticas no prueban que NSIS instale el puente en la ruta final ni sustituyen la interacción con clientes reales. Tampoco se verificó una actualización publicada, pues no se hizo push, tag ni release.

## 6. Revisión sugerida y cómo continuar

1. Revisar diff contra main y los commits por tarea; comprobar las decisiones de §3.
2. Repetir los gates desde la raíz salvo Rust desde `src-tauri/`:

```powershell
pnpm lint
pnpm typecheck
pnpm vitest run --maxWorkers=4
node --test scripts/sidecar.test.mjs
pnpm build
Set-Location src-tauri
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --bin qapture-mcp
```

3. Para empaquetar desde raíz: `pnpm tauri build`. Revisar sidecar en `src-tauri/binaries/`, bundles y ZIP portable. El script admite `TAURI_ENV_TARGET_TRIPLE`; respetar la salida con subdirectorio target cuando se pasa `--target`.
4. Ejecutar el checklist real del contrato con proyectos ficticios, fechar cada fila y recoger evidencia. No usar datos reales para capturas ni exportaciones de prueba.
5. Revisar especialmente concurrencia de códigos, archivos falsos `.png`, etiquetas ambiguas, no sobrescritura, revocación de permisos, clave nueva y reinicio con llamada en vuelo.
6. Mantener F9a en validación hasta terminar la puerta manual. F9b (GitHub y anotaciones MCP), HTTP y borrado quedan fuera.
7. No hacer push, merge, tag o release sin una instrucción posterior del usuario.

## 7. Commits de referencia

- Documentos iniciales: `f428a4b`.
- T1: `19a40a8`; T2: `303c3d6`; T3: `dce8cc8`; T4: `ed78ef6`; T5: `17c6d61`.
- T6: `5effc9d`; T7: `4395c47`; T8: `8100355`.
- T9: `c27bdce`; T10: `bbb6f1e`.
- T11: `fdd9177`. Cierre documental: el commit que incorpora este handoff; consultar `git log -1`.
