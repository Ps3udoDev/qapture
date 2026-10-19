<p align="center"><img src="src/assets/brand/t4s-logo.png" width="140" alt="Teams4Soft"></p>

# Qapture

**Evidencias y matrices de prueba QA, del atajo al Excel.** Parte del ecosistema Teams4Soft.

Qapture corre en segundo plano en Windows. Con un atajo capturas la pantalla, anotas la captura (flechas, números, texto, desenfoque), la asignas a un caso de prueba y exportas un Excel con carátula Teams4Soft, matriz, evidencias y resumen.

[![Release](https://img.shields.io/github/v/release/Ps3udoDev/qapture)](https://github.com/Ps3udoDev/qapture/releases/latest)
[![CI](https://github.com/Ps3udoDev/qapture/actions/workflows/ci.yml/badge.svg)](https://github.com/Ps3udoDev/qapture/actions/workflows/ci.yml)

**[⬇ Descargar la última versión](https://github.com/Ps3udoDev/qapture/releases/latest)** (Windows: instalador `.exe`, `.msi` o ZIP portable · Linux: `.AppImage`, `.deb` o `.rpm`)

> Estado: **v1.0.1 (Fase 7 completada · En pruebas del equipo)**. El diseño está en [`docs/superpowers/specs/2026-10-15-qapture-design.md`](docs/superpowers/specs/2026-10-15-qapture-design.md). Guía de uso en [`docs/usuario/`](docs/usuario/README.md); para contribuir, ver [`CONTRIBUTING.md`](CONTRIBUTING.md).

## Demo del producto (52 s)

<p align="center">
  <a href="assets/video/qapture-promo.mp4">
    <img src="assets/video/qapture-preview.gif" width="850" alt="Vista previa animada de Qapture: captura con atajo, anotación de evidencias y exportación a Excel">
  </a>
</p>

<p align="center"><b>▶ <a href="assets/video/qapture-promo.mp4">Ver el video completo con audio</a></b></p>

> 🎬 **Lo que muestra el video (con voz en off):**
> 1. **Captura con un atajo:** `Ctrl+Alt+S` y seleccionas una región o la pantalla completa, con resolución nativa y multimonitor.
> 2. **Anota en segundos:** recuadros, flechas, pasos numerados, texto y desenfoque para ocultar datos sensibles.
> 3. **Organiza tu matriz:** cada evidencia queda en su caso de prueba, con columnas a tu medida y estados claros.
> 4. **Exporta a Excel:** carátula con indicadores, matriz, evidencias incrustadas y resumen con gráficos.
> 5. **Descarga:** instalador de Windows en [GitHub Releases](https://github.com/Ps3udoDev/qapture/releases).
>
> El video se genera con [HyperFrames](https://github.com/heygen-com/hyperframes) desde [`videos/qapture-promo/`](videos/qapture-promo/) usando capturas de la UI real.

## Fases implementadas

- **F1: Dominio, SQLite y CRUD** (`v0.1.0`) — Proyectos, columnas dinámicas, casos de prueba y plantillas.
- **F2: Evidencias y editor de anotaciones** (`v0.2.0`) — Anotador con herramientas (flechas, cajas, texto, enumeración, blur), galería de evidencias por caso y almacenamiento transaccional.
- **F3: Captura de pantalla integrada** (`v0.3.0`) — Botón «Capturar», selector de región/monitor completo con soporte multi-monitor y control de escala DPI.
- **F4: Excel con carátula Teams4Soft** (`v0.4.0`) — Exportación a `.xlsx` nativo con carátula, matriz de casos, hoja de evidencias con capturas incrustadas y vínculos bidireccionales, resumen con gráficos nativos, control de versiones del documento y plantilla vacía.
- **F5: Importación v1, paquetes y respaldos** (`v0.5.0`) — Migración de los datos de la Bitácora v1 sin pérdidas, proyectos compartibles entre compañeros mediante paquetes y respaldos automáticos que protegen los datos.
- **F6: Bandeja, atajos y ajustes** (`v0.6.0`) — Qapture vive en la bandeja del sistema y se invoca con atajos globales configurables desde Ajustes.
- **F7: CI/CD, actualizaciones y release** (`v1.0.0`) — CI en cada PR, publicación automática con cada tag (instalador NSIS, MSI y ZIP portable), actualizaciones firmadas dentro de la app, versionado con `pnpm release` y documentación de usuario y de desarrollo.

> Los tags de git publicados son `v0.4.0`, `v1.0.0` y `v1.0.1`; las demás etiquetas son la versión de cada fase. Los tags nuevos se crean desde `main` al publicar (`v1.0.0` fue el primero con el pipeline automático).

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

Los instaladores se descargan desde [Releases](https://github.com/Ps3udoDev/qapture/releases/latest). Un build local queda en `src-tauri/target/release/bundle/` (`nsis/` para el `.exe`, `msi/` para el `.msi`).

## Licencia

[MIT](LICENSE). El logo y la marca Teams4Soft no están cubiertos por la licencia: ver [NOTICE](NOTICE).
