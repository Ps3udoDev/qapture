# Handoff — Ícono de Qapture en la bandeja del sistema (iconos ocultos)

> Fecha: 2026-10-15 · Rama: `feat/f1-dominio-datos-crud` (PR #1 abierto, sin merge) · Último commit: `34565ea`
> Ejecutor previsto: **Codex** (vía plugin de Codex en Claude Code o `codex` CLI directo). Claude Code orquesta y revisa.

## 1. Contexto (para quien retome sin historial)

- **Proyecto:** Qapture (`C:\Users\DELL\Desktop\code\Team4soft\GMX\evidencias\qapture`), app de escritorio Tauri 2 + React 19 + TypeScript + Rust (SQLite) para matrices de prueba y evidencias QA de Teams4Soft. Repo público: https://github.com/Ps3udoDev/qapture
- **Documentos fuente de verdad:**
  - Spec aprobado: `docs/superpowers/specs/2026-10-15-qapture-design.md` (ver §6.8 ciclo de vida de la ventana y §6.9 settings)
  - Contratos por fase: `docs/superpowers/contracts/` (F0–F7). F1 está "🧪 En pruebas del equipo".
  - Plan F1 ejecutado: `docs/superpowers/plans/2026-10-15-f1-dominio-datos-crud.md`
- **Estado de F1:** las 15 tasks están implementadas y revisadas, junto con la revisión final y sus correcciones. Rust: 39 tests; Vitest: 60 tests; clippy, fmt y typecheck limpios. PR #1: https://github.com/Ps3udoDev/qapture/pull/1
- **Hallazgo de QA del equipo:** *"todo bien, salvo que la app no aparece en la barra de tareas en iconos ocultos (donde están Epic, Bluetooth, etc.)"*.
- **Diagnóstico:** no es una regresión. La bandeja del sistema **no está implementada**, porque pertenece a la fase **F6** (`docs/superpowers/contracts/F6-bandeja-atajos-settings.md`, tasks F6-T1 y F6-T2). Hoy `src-tauri/src/lib.rs` no crea ningún `TrayIcon`, el crate `tauri` está sin la feature `tray-icon` y cerrar la ventana termina el proceso.

## 2. Objetivo de este handoff (alcance acotado)

Adelantar **solo** la parte de F6 que resuelve el hallazgo. Esto equivale a F6-T1 + F6-T2 sin atajos ni settings:

1. **Ícono en la bandeja** (área de notificación de Windows; por defecto aparece en "iconos ocultos" ^), con el ícono de la app y el tooltip `Qapture · Teams4Soft`.
2. **Menú contextual** (clic derecho), en español:
   - `Abrir Qapture`: muestra, desminimiza y enfoca la ventana `main`.
   - `Ocultar`: oculta la ventana.
   - separador
   - `Salir`: termina el proceso (`app.exit(0)`).
3. **Clic izquierdo** sobre el ícono: alterna entre mostrar+enfocar y ocultar.
4. **Cerrar la ventana con la X la oculta** y la app sigue en la bandeja (spec §6.8). Solo "Salir" del menú termina el proceso.
5. **Instancia única** (`tauri-plugin-single-instance`): ejecutar el `.exe` otra vez enfoca la ventana existente en lugar de abrir otra. Sin esto, cerrar con la X y volver a abrir el acceso directo crearía un segundo proceso que compite por la base de datos.

**Fuera de alcance** (sigue en F6): atajos globales, pantalla de Settings, autostart con Windows, notificaciones, "iniciar oculto". No crear la tabla `setting` ni migraciones.

## 3. Guía técnica

- `src-tauri/Cargo.toml`: `tauri = { version = "2", features = ["tray-icon"] }` y `cargo add tauri-plugin-single-instance` (versión 2.x compatible).
- Nuevo módulo `src-tauri/src/system/tray.rs` (y `system/mod.rs`), siguiendo la estructura del spec §4.3:
  - `pub fn setup_tray(app: &tauri::App) -> tauri::Result<()>` con `tauri::tray::TrayIconBuilder::with_id("main-tray")`, `.icon(app.default_window_icon().unwrap().clone())`, `.tooltip("Qapture · Teams4Soft")`, `.menu(&menu)`, `.show_menu_on_left_click(false)`, `.on_menu_event(...)` y `.on_tray_icon_event(...)`. El clic izquierdo se detecta con `TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. }`.
  - El menú se arma con `tauri::menu::{Menu, MenuItem, PredefinedMenuItem}` e ids `open`, `hide` y `quit`.
  - Helpers `show_main(app)` (`show`, `unminimize`, `set_focus`) y `toggle_main(app)` (según `is_visible`).
  - **Mantener una referencia viva al TrayIcon** (por ejemplo con `app.manage(tray)`, o confiando en el registro por id del builder). Si el handle se suelta, en Windows el ícono puede desaparecer.
- En `src-tauri/src/lib.rs`:
  - Registrar **primero** `tauri_plugin_single_instance::init(|app, _args, _cwd| show_main(app))`. Debe ser el primer plugin.
  - Llamar a `system::tray::setup_tray(app)?` dentro de `.setup`.
  - Agregar `.on_window_event(|window, event| if let WindowEvent::CloseRequested { api, .. } = event { api.prevent_close(); let _ = window.hide(); })` solo para la ventana `main`.
- Capabilities: el tray y la ventana se manejan desde Rust, así que **no hace falta abrir permisos al webview**. Mantener `src-tauri/capabilities/default.json` en `["core:default"]` (spec §9). Si el plugin single-instance exige un permiso, agregar solo ese y justificarlo.
- Lógica testeable: separar en una función pura la decisión de qué hacer con cada id de menú (por ejemplo `enum TrayAction { Show, Hide, Quit }` y `fn action_for(id: &str) -> Option<TrayAction>`), con tests en `cargo test`. El resto solo se puede verificar con la prueba manual.

## 4. Restricciones (del plan global)

- Textos visibles en español. Paleta y nombre: `Qapture · Teams4Soft`.
- Cero warnings: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `pnpm typecheck`, `pnpm test`, `pnpm build` deben quedar en verde.
- No tocar el fixture `src/infrastructure/tauri/__fixtures__/contracts.json` (no hay cambios de DTO).
- Commits en la rama `feat/f1-dominio-datos-crud`, con mensaje `feat(f1): ícono en bandeja, cerrar a bandeja e instancia única`, terminado en una línea en blanco y `Co-Authored-By: …` según la herramienta que haga el commit. **No hacer push ni merge sin pedirlo.**
- Actualizar el contrato F6 (`docs/superpowers/contracts/F6-bandeja-atajos-settings.md`): marcar F6-T1 y F6-T2 como adelantadas a F1 (`✅ adelantada en PR #1`).

## 5. Verificación manual (la hace la persona usuaria)

```bash
pnpm tauri build   # instalador: src-tauri/target/release/bundle/nsis/Qapture_0.1.0_x64-setup.exe
```

| # | Prueba | Esperado |
|---|---|---|
| 1 | Abrir Qapture | El ícono aparece en la bandeja (flecha ^ de iconos ocultos) con el tooltip "Qapture · Teams4Soft" |
| 2 | Cerrar con la X | La ventana desaparece y el ícono sigue en la bandeja; el proceso sigue vivo |
| 3 | Clic izquierdo en el ícono | La ventana vuelve al frente; otro clic la oculta |
| 4 | Clic derecho → Abrir / Ocultar | Funcionan |
| 5 | Doble clic en el acceso directo con la app ya abierta | Se enfoca la existente; no aparece una segunda instancia en el Administrador de tareas |
| 6 | Clic derecho → Salir | El ícono desaparece y el proceso termina |
| 7 | Arrastrar el ícono desde "ocultos" a la barra | Queda fijo y visible (comportamiento de Windows) |
| 8 | Los datos persisten tras Salir y volver a abrir | Sí |

> Nota: Windows coloca todo ícono nuevo de bandeja dentro de "iconos ocultos". Que aparezca ahí es lo correcto. Fijarlo a la vista lo decide cada persona desde Configuración → Personalización → Barra de tareas.

## 6. Cómo retomar en una sesión nueva de Claude Code

1. Abrir Claude Code en `C:\Users\DELL\Desktop\code\Team4soft\GMX\evidencias\qapture`.
2. Decir: *"Lee `docs/superpowers/handoffs/2026-10-15-bandeja-sistema.md` y delega la implementación a Codex; tú revisas el diff contra la sección 2–4."*
3. Si el plugin de Codex está instalado, usar sus comandos de delegación (por ejemplo `/codex:rescue <instrucción>`; revisa `/help` para ver los nombres exactos). Si no lo está, usar el CLI: `codex exec --full-auto "Implementa docs/superpowers/handoffs/2026-10-15-bandeja-sistema.md secciones 2-4"` desde la raíz del repo.
4. Al terminar, Claude revisa el diff, corre la verificación de la sección 4 y pide a la persona usuaria la prueba manual de la sección 5.
