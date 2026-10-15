# F0 — Scaffold, repo y spec · ✅ Cerrada (2026-10-15)

## Alcance
Base del proyecto, repositorio público y diseño aprobado. No hay funcionalidad de usuario.

## Tasks
- [x] F0-T1 Scaffold `create-tauri-app` (react-ts, pnpm), identificador `com.teams4soft.qapture`
- [x] F0-T2 Assets de marca T4S en `src/assets/brand/`
- [x] F0-T3 `.gitignore` que bloquea datos de clientes (`*.db`, `data/`, `evidence/`, `*.xlsx`)
- [x] F0-T4 Spec v1 con diagramas Mermaid, aprobado
- [x] F0-T5 Repo público `Ps3udoDev/qapture`, commit inicial y push
- [x] F0-T6 Licencia MIT + `NOTICE` de marca

## Interfaces producidas
- Estructura de carpetas por capas (`src/domain`, `application`, `ports`, `infrastructure`, `ui`).
- Toolchain verificada: Node 24, pnpm 11, Rust 1.98 MSVC, VS Build Tools, WebView2.

## Criterios de aceptación
- [x] `cargo check` y `pnpm build` sin errores.
- [x] El repo existe y `main` contiene el scaffold y el spec.
