<p align="center"><img src="src/assets/brand/t4s-logo.png" width="140" alt="Teams4Soft"></p>

# Qapture

**Evidencias y matrices de prueba QA, del atajo al Excel.** Parte del ecosistema Teams4Soft.

Qapture corre en segundo plano en Windows. Con un atajo capturas la pantalla, anotas la captura (flechas, números, texto, desenfoque), la asignas a un caso de prueba y exportas un Excel con carátula Teams4Soft, matriz, evidencias y resumen.

[![Release](https://img.shields.io/github/v/release/Ps3udoDev/qapture)](https://github.com/Ps3udoDev/qapture/releases/latest)
[![CI](https://github.com/Ps3udoDev/qapture/actions/workflows/ci.yml/badge.svg)](https://github.com/Ps3udoDev/qapture/actions/workflows/ci.yml)

**[⬇ Descargar la última versión](https://github.com/Ps3udoDev/qapture/releases/latest)** (instalador `.exe`, `.msi` o ZIP portable)

> Estado: **v1.0.0 (Fase 7 completada · En pruebas del equipo)**. El diseño está en [`docs/superpowers/specs/2026-10-15-qapture-design.md`](docs/superpowers/specs/2026-10-15-qapture-design.md). Guía de uso en [`docs/usuario/`](docs/usuario/README.md); para contribuir, ver [`CONTRIBUTING.md`](CONTRIBUTING.md).

## Demo del Producto (45s)

<p align="center">
  <video src="assets/video/qapture-demo.mp4" controls width="850">
    Tu navegador no soporta la reproducción de video HTML5. Puedes ver o descargar la demo en <a href="assets/video/qapture-demo.mp4"><code>assets/video/qapture-demo.mp4</code></a>.
  </video>
</p>

> 🎬 **Recorrido del producto mostrado en el video (subtitulado):**
> 1. **Presentación de Qapture:** Ecosistema Teams4Soft y objetivo del producto.
> 2. **Matriz de Casos:** Gestión de pruebas, columnas dinámicas y control de estados.
> 3. **Captura Integrada:** Selección de regiones o monitor completo con DPI nativo.
> 4. **Editor de Anotaciones:** Herramientas de foco (flechas, cajas, pasos numerados y blur confidencial).
> 5. **Exportación a Excel:** 4 hojas nativas (Carátula con KPIs, Matriz, Evidencias incrustadas y Resumen con gráficos).
> 6. **Descarga:** Instalador de Windows disponible en [GitHub Releases](https://github.com/Ps3udoDev/qapture/releases).

## Fases implementadas

- **F1: Dominio, SQLite y CRUD** (`v0.1.0`) — Proyectos, columnas dinámicas, casos de prueba y plantillas.
- **F2: Evidencias y editor de anotaciones** (`v0.2.0`) — Anotador con herramientas (flechas, cajas, texto, enumeración, blur), galería de evidencias por caso y almacenamiento transaccional.
- **F3: Captura de pantalla integrada** (`v0.3.0`) — Botón «Capturar», selector de región/monitor completo con soporte multi-monitor y control de escala DPI.
- **F4: Excel con carátula Teams4Soft** (`v0.4.0`) — Exportación a `.xlsx` nativo con carátula, matriz de casos, hoja de evidencias con capturas incrustadas y vínculos bidireccionales, resumen con gráficos nativos, control de versiones del documento y plantilla vacía.
- **F5: Importación v1, paquetes y respaldos** (`v0.5.0`) — Migración de los datos de la Bitácora v1 sin pérdidas, proyectos compartibles entre compañeros mediante paquetes y respaldos automáticos que protegen los datos.
- **F6: Bandeja, atajos y ajustes** (`v0.6.0`) — Qapture vive en la bandeja del sistema y se invoca con atajos globales configurables desde Ajustes.
- **F7: CI/CD, actualizaciones y release** (`v1.0.0`) — CI en cada PR, publicación automática con cada tag (instalador NSIS, MSI y ZIP portable), actualizaciones firmadas dentro de la app, versionado con `pnpm release` y documentación de usuario y de desarrollo.

> Solo `v0.4.0` existe hoy como tag de git; las demás etiquetas son la versión de cada fase y los tags nuevos se crean desde `main` al publicar (`v1.0.0` es el primero con el pipeline automático).

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
