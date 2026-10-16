<p align="center"><img src="src/assets/brand/t4s-logo.png" width="140" alt="Teams4Soft"></p>

# Qapture

**Evidencias y matrices de prueba QA, del atajo al Excel.** Parte del ecosistema Teams4Soft.

Qapture corre en segundo plano en Windows. Con un atajo capturas la pantalla, anotas la captura (flechas, números, texto, desenfoque), la asignas a un caso de prueba y exportas un Excel con carátula Teams4Soft, matriz, evidencias y resumen.

> Estado: **en construcción (v0.1)**. El diseño está en [`docs/superpowers/specs/2026-10-15-qapture-design.md`](docs/superpowers/specs/2026-10-15-qapture-design.md).

## Stack

React 19 + TypeScript + Vite · Tauri 2 (Rust) · SQLite · rust_xlsxwriter

## Desarrollo

Requisitos: Node 22+, pnpm, Rust stable (MSVC), Visual Studio Build Tools y WebView2.

```bash
pnpm install
pnpm tauri dev      # app en modo desarrollo
pnpm tauri build    # instaladores en src-tauri/target/release/bundle
```

## Probar un build

El equipo prueba cada fase con el instalador local siguiendo el checklist del contrato correspondiente en
[`docs/superpowers/contracts/`](docs/superpowers/contracts/README.md).

## Licencia

[MIT](LICENSE). El logo y la marca Teams4Soft no están cubiertos por la licencia: ver [NOTICE](NOTICE).
