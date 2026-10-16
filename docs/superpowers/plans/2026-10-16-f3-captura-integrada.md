# F3 — Captura de pantalla integrada · Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Que el tester capture la pantalla completa o una región desde Qapture con el botón «Capturar» (sin herramientas externas), en uno o varios monitores con distinta escala, y que la captura llegue directo al editor de anotaciones de F2.

**Architecture:** `start_capture` oculta la ventana principal, captura todos los monitores con `xcap` y guarda las imágenes congeladas **en memoria** dentro de un `CaptureController` (máquina de estados `Idle → Starting → Selecting`). Abre una ventana overlay sin bordes por monitor (`overlay-<n>`), ubicada en píxeles físicos. Cada overlay pide su imagen (`capture_frame`), la muestra oscurecida y devuelve la selección en píxeles CSS junto con el tamaño de su ventana. Rust convierte eso a píxeles físicos con el cociente imagen/ventana, que absorbe cualquier escala DPI. Después recorta, normaliza el recorte con el `stage` de F2 (lado mayor ≤ 2400 px, en `evidence/_staging/`), cierra los overlays, vuelve a mostrar Qapture y emite `evidence:captured { tmpPath }`. El front abre el editor de F2 con `openEvidenceEditor`. La orquestación depende de dos traits (`Screen`, `CaptureHost`), así que toda la lógica se prueba sin Tauri ni pantalla real. Con un *app manifest* de Tauri, cada ventana solo puede invocar los comandos de su capability: el overlay únicamente sus 4 comandos (spec §9).

**Tech Stack:** Tauri 2.12 (app manifest ACL, multi-ventana), Rust 2024, xcap 0.9.8, image 0.25 · React 19, TypeScript 6, Vite 8 (multipágina), Zustand 5, zod 4, lucide-react · Vitest 5, Testing Library, jsdom.

**Spec:** [`docs/superpowers/specs/2026-10-15-qapture-design.md`](../specs/2026-10-15-qapture-design.md) (§4.3 `capture/`, §6.2 UC-11, §9 capabilities) · **Contrato:** [`docs/superpowers/contracts/F3-captura-integrada.md`](../contracts/F3-captura-integrada.md)

## Global Constraints

- Rama `feat/f3-captura-integrada` creada desde `main` actualizado. Un commit por task, y todos los mensajes terminan con una línea en blanco y `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>` (o la línea de la herramienta que haga el commit). **No hacer push ni merge sin pedirlo.**
- Plataforma Windows; comandos en Git Bash desde la raíz `qapture/` (los de Rust, desde `qapture/src-tauri/`).
- Textos visibles **en español**. Mensajes fijos de esta fase (se usan tal cual en código y tests):
  - `Ya hay una captura en curso.` · `La captura ya terminó.` · `La selección es demasiado pequeña.` · `La selección no es válida.`
  - `No se pudo capturar la pantalla. Inténtalo nuevamente.` (mensaje del nuevo `AppError::Capture`, código `capture`)
  - `Solo el overlay de captura puede usar este comando.` · `Crea un caso antes de capturar evidencias.`
  - Pista del overlay: `Arrastra para seleccionar · Enter: pantalla completa · Esc: cancelar`
- **Las imágenes de los monitores no se escriben en disco:** viven en memoria mientras el overlay está abierto. El único temporal es el recorte, en `evidence/_staging/`, que ya tiene el ciclo de vida de F2: se mueve al guardar, se descarta al cancelar el editor y se vacía al iniciar la app.
- El recorte pasa por `repo::evidence::stage` (F2): PNG con **lado mayor ≤ 2400 px**, sin agrandar los pequeños.
- Selección mínima en el overlay: **4 px CSS** por lado (si es menor, se toma como un clic). Rust rechaza recortes de menos de 1 px físico.
- Ventanas overlay: label `overlay-<n>` (n = índice del monitor según `xcap::Monitor::all()`), sin bordes, siempre encima, fuera de la barra de tareas y ocultas hasta que su imagen se pinta. Una selección vive dentro de un solo monitor.
- `Enter` captura el **monitor completo bajo el puntero** (si no se puede saber, el del overlay que recibió la tecla). `Esc` cancela en todos los monitores.
- Pausa tras ocultar Qapture: **200 ms** (`HIDE_DELAY`), solo si la ventana estaba visible.
- Evento `evidence:captured` con payload `{ "tmpPath": "<ruta absoluta>" }`, solo hacia la ventana `main`.
- **ACL:** `src-tauri/src/commands/names.rs` es la única lista de comandos propios. `build.rs` la usa para generar los permisos `allow-*`. `capabilities/main.json` = `core:default` + todos los comandos salvo los del overlay; `capabilities/overlay.json` = solo `allow-capture-frame`, `allow-overlay-ready`, `allow-finish-capture` y `allow-cancel-capture`. Un comando nuevo se agrega en `names.rs`, en `lib.rs` y en su capability; un test lo verifica.
- Dependencias nuevas: solo `xcap = "0.9.8"` (Rust). Ninguna nueva en JS.
- Fixtures y tests con datos ficticios ("ACME", "Recibos", "Pólizas"). Nunca capturas reales.
- El fixture `src/infrastructure/tauri/__fixtures__/contracts.json` **cambia en esta fase**, y solo se regenera con `UPDATE_CONTRACTS=1 cargo test contract`.
- Cero warnings: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `pnpm typecheck`, `pnpm test` y `pnpm build` en verde al cerrar cada task.

## Review Focus

1. **Monitores con distinta escala o con offset negativo** (un monitor a la izquierda en `x = -1920` al 100 % y el principal al 150 %): la región guardada coincide píxel a píxel con lo que se ve en el overlay, sin desplazarse ni perder bordes → tests en Task 1 (`monitor_at`), Task 2 (escalas 1.0 / 1.25 / 1.5 y bordes fraccionarios) y Task 3 (recorte del monitor con offset negativo).
2. **La captura falla** (monitor desconectado, escritorio seguro de UAC, error de WebView al abrir el overlay): Qapture vuelve a aparecer con `No se pudo capturar la pantalla. Inténtalo nuevamente.`, nunca queda oculto, y el siguiente intento funciona → tests en Task 3 y Task 8.
3. **Un overlay se cierra por fuera** (Alt+F4, o el WebView se cae): se cancela toda la sesión, se cierran los demás overlays, Qapture vuelve si estaba visible y se puede capturar de nuevo → test en Task 3 (más el cableado de `WindowEvent::Destroyed` en Task 6).
4. **Llega una captura cuando no corresponde:** con el editor abierto, queda en la cola del editor (no se pierde); sin proyecto o sin casos, se descarta el temporal y se avisa → tests en Task 8.
5. **Selección degenerada o manipulada:** un clic sin arrastre no captura; un rectángulo fuera de la pantalla, con `NaN` o con viewport 0 se rechaza sin cerrar el overlay (el usuario puede reintentar); la ventana principal no puede llamar a los comandos del overlay, ni el overlay a `start_capture` ni a los de datos → tests en Task 2, Task 3, Task 5 y Task 6.

## Decisiones respecto al contrato (se anotan en el contrato en la Task 1)

- F3-T1 pedía "PNG temporal" por monitor; se guardan **en memoria**. Así no hay archivos de monitor que limpiar y el overlay recibe la imagen por IPC binario. El único temporal es el recorte en `_staging/`.
- La **lupa** (opcional en F3-T3) queda fuera de esta fase. La medida en píxeles físicos junto a la selección cubre la precisión.
- `start_capture()` ya queda listo para que F6 lo invoque desde el atajo global: si Qapture estaba oculto, al cancelar sigue oculto y al capturar se muestra con el editor.

## File Structure

```
qapture/
├─ overlay.html                                        (T5) entrada de la ventana overlay
├─ vite.config.ts                                      (T5) build multipágina
├─ package.json                                        (T9 versión)
├─ src/
│  ├─ overlay-main.tsx                                 (T5) monta OverlayApp
│  ├─ domain/capture.ts  capture.test.ts               (T5) geometría pura de la selección
│  ├─ ports/
│  │  ├─ capture.ts                                    (T5 OverlayApi, T7 ScreenCapture)
│  │  ├─ errors.ts                                     (T6) código "capture"
│  │  └─ repositories.ts                               (T7) Repositories.capture
│  ├─ infrastructure/
│  │  ├─ tauri/capture.ts  capture.test.ts             (T5 overlay, T7 start/onCaptured)
│  │  ├─ tauri/dto.ts  contract.test.ts                (T6)
│  │  ├─ tauri/repositories.ts                         (T7)
│  │  └─ memory/capture.ts  capture.test.ts  evidence.ts  repositories.ts   (T7)
│  └─ ui/
│     ├─ overlay/OverlayApp.tsx  OverlayApp.test.tsx   (T5)
│     ├─ store/evidence-slice.ts  evidence-slice.test.ts (T8)
│     ├─ gallery/EvidenceGallery.tsx                   (T8) botón «Capturar»
│     └─ screens/AppShell.tsx  Evidence.test.tsx       (T8) suscripción al evento
└─ src-tauri/
   ├─ Cargo.toml  tauri.conf.json  build.rs  .gitignore (T1, T4, T9)
   ├─ capabilities/main.json (antes default.json)  overlay.json   (T4, T6)
   └─ src/
      ├─ error.rs                                      (T1) AppError::Capture
      ├─ capture/mod.rs                                (T1…T6)
      ├─ capture/screen.rs                             (T1) xcap, MonitorBounds, monitor_at, preview_png
      ├─ capture/crop.rs                               (T2) Selection → PixelRect, crop_png
      ├─ capture/session.rs                            (T3) CaptureController + CaptureHost
      ├─ capture/windows.rs                            (T6) TauriHost, labels overlay-<n>
      ├─ commands/names.rs                             (T4, T6) lista única de comandos + test ACL
      ├─ commands/capture.rs  commands/mod.rs          (T6)
      ├─ dto.rs                                        (T6) CapturedEventDto + contrato
      ├─ state.rs  lib.rs                              (T6)
```

Correspondencia con el contrato: F3-T1 → Task 1 · F3-T2 → Tasks 4 y 6 · F3-T3 → Task 5 · F3-T4 → Task 2 · F3-T5 → Tasks 3, 6, 7 y 8 · F3-T6 → Task 3 (más la limpieza de `_staging` de F2, que ya existe).

---

### Task 1: Rama, dependencia `xcap` y captura de monitores (`capture/screen.rs`)

**Files:**
- Create: `src-tauri/src/capture/mod.rs`, `src-tauri/src/capture/screen.rs`
- Modify: `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `src-tauri/src/error.rs`, `src-tauri/src/lib.rs`
- Modify: `docs/superpowers/contracts/README.md`, `docs/superpowers/contracts/F3-captura-integrada.md`; Add: `docs/superpowers/plans/2026-10-16-f3-captura-integrada.md` (este plan)

**Interfaces:**
- Produces:
  - `AppError::Capture(String)`: código `"capture"`, mensaje `No se pudo capturar la pantalla. Inténtalo nuevamente.`, detalle `captura: <causa>`.
  - `capture::screen::MonitorBounds { x: i32, y: i32, width: u32, height: u32 }` (píxeles físicos del escritorio virtual) con `contains(x: f64, y: f64) -> bool`.
  - `capture::screen::CapturedMonitor { bounds: MonitorBounds, image: image::RgbaImage }`.
  - `trait capture::screen::Screen { fn capture_all(&self) -> AppResult<Vec<CapturedMonitor>>; }` e implementación `XcapScreen`.
  - `capture::screen::monitor_at(monitors: &[MonitorBounds], x: f64, y: f64) -> Option<usize>`.
  - `capture::screen::preview_png(image: &RgbaImage) -> AppResult<Vec<u8>>` (PNG con compresión rápida).

- [ ] **Step 1: Rama y documentos**

```bash
git checkout main && git pull
git checkout -b feat/f3-captura-integrada
```

El plan ya está en `docs/superpowers/plans/2026-10-16-f3-captura-integrada.md` (se escribió en `main` sin commitear, así que pasa a la rama).

En `docs/superpowers/contracts/README.md`, tabla **Estado**: cambia la fila F2 a `✅ Cerrada` y la fila F3 a:

```markdown
| F3 | [Captura integrada](F3-captura-integrada.md) | [plan](../plans/2026-10-16-f3-captura-integrada.md) | 🟡 En curso | `v0.3.0` |
```

En `docs/superpowers/contracts/F3-captura-integrada.md`, cambia la línea de cabecera por:

```markdown
**Depende de:** F2 (editor y flujo de registro) · **Plan:** [2026-10-16-f3-captura-integrada.md](../plans/2026-10-16-f3-captura-integrada.md)
```

y agrega, antes de `## Interfaces producidas`:

```markdown
## Notas de implementación (acordadas en el plan)
- Las imágenes de los monitores se guardan **en memoria** mientras el overlay está abierto, no como PNG temporales. El único temporal es el recorte, en `evidence/_staging/`, con la limpieza de F2 (al guardar, al cancelar y al iniciar la app).
- La lupa queda fuera de esta fase. Junto a la selección se muestra su medida en píxeles reales.
- `Enter` captura el monitor completo que está bajo el puntero. Una selección vive dentro de un solo monitor.
- Si Qapture estaba oculto (por ejemplo, con el atajo de F6), al cancelar sigue oculto y al capturar se muestra con el editor.
```

- [ ] **Step 2: Dependencia**

En `src-tauri/Cargo.toml`, en `[dependencies]`, después de la línea de `image`:

```toml
xcap = "0.9.8"
```

`xcap 0.9.8` no tiene features por defecto y usa `image 0.25` y `windows 0.62`, las mismas versiones que ya usa el proyecto. Run: `cd src-tauri && cargo check`. Expected: compila y `Cargo.lock` incluye `xcap`.

- [ ] **Step 3: Test que falla — error de captura**

Al final de `src-tauri/src/error.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_errors_have_their_own_code_and_a_friendly_message() {
        let error = AppError::Capture("xcap: monitor desconectado".into());
        let json = serde_json::to_value(&error).unwrap();
        assert_eq!(json["code"], "capture");
        assert_eq!(
            json["message"],
            "No se pudo capturar la pantalla. Inténtalo nuevamente."
        );
        assert_eq!(json["detail"], "captura: xcap: monitor desconectado");
    }
}
```

Run: `cargo test error::tests`. Expected: FAIL (no compila: `no variant named Capture`).

- [ ] **Step 4: Implementar `AppError::Capture`**

En `src-tauri/src/error.rs`, agrega la variante después de `Json`:

```rust
    #[error("captura: {0}")]
    Capture(String),
```

En `code()`, agrega `Self::Capture(_) => "capture",`. En `user_message()`, deja así el `match`:

```rust
        match self {
            Self::Validation(m) | Self::NotFound(m) => m.clone(),
            Self::Capture(_) => "No se pudo capturar la pantalla. Inténtalo nuevamente.".into(),
            _ => "No se pudo completar la operación. Revisa el espacio disponible e inténtalo nuevamente.".into(),
        }
```

Run: `cargo test error::tests`. Expected: PASS.

- [ ] **Step 5: Tests que fallan — `screen.rs`**

Crea `src-tauri/src/capture/mod.rs`:

```rust
// Hasta la Task 6 ningún comando usa este módulo: solo lo ejercitan los tests.
#![allow(dead_code)]

pub mod screen;
```

En `src-tauri/src/lib.rs`, agrega `mod capture;` como primera línea de la lista de módulos (antes de `mod commands;`).

Crea `src-tauri/src/capture/screen.rs` solo con los tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn bounds(x: i32, y: i32, width: u32, height: u32) -> MonitorBounds {
        MonitorBounds {
            x,
            y,
            width,
            height,
        }
    }

    #[test]
    fn finds_the_monitor_under_a_point_with_negative_offsets() {
        // Izquierda 1920×1080 en x negativo; principal 2880×1620 (150 %); arriba 2560×1440 en y negativo.
        let monitors = [
            bounds(-1920, 0, 1920, 1080),
            bounds(0, 0, 2880, 1620),
            bounds(0, -1440, 2560, 1440),
        ];
        assert_eq!(monitor_at(&monitors, -1.0, 500.0), Some(0));
        assert_eq!(monitor_at(&monitors, -1920.0, 0.0), Some(0));
        assert_eq!(monitor_at(&monitors, 0.0, 0.0), Some(1));
        assert_eq!(monitor_at(&monitors, 2879.5, 1619.5), Some(1));
        assert_eq!(monitor_at(&monitors, 100.0, -1.0), Some(2));
        assert_eq!(monitor_at(&monitors, 2880.0, 0.0), None);
        assert_eq!(monitor_at(&monitors, -1921.0, 0.0), None);
        assert_eq!(monitor_at(&[], 0.0, 0.0), None);
    }

    #[test]
    fn preview_is_a_png_with_the_monitor_size() {
        let image = RgbaImage::from_pixel(64, 36, image::Rgba([43, 98, 158, 255]));
        let png = preview_png(&image).unwrap();
        assert_eq!(
            image::guess_format(&png).unwrap(),
            image::ImageFormat::Png
        );
        assert_eq!(crate::media::raster::dimensions(&png).unwrap(), (64, 36));
    }

    /// Manual, con un escritorio real: `cargo test real_screen -- --ignored --nocapture`.
    #[test]
    #[ignore = "necesita un escritorio real"]
    fn captures_the_real_screen() {
        let monitors = XcapScreen.capture_all().unwrap();
        assert!(!monitors.is_empty());
        for m in &monitors {
            println!("{:?} → imagen {}×{}", m.bounds, m.image.width(), m.image.height());
            assert!(m.image.width() > 0 && m.image.height() > 0);
        }
    }
}
```

Run: `cargo test capture::screen`. Expected: FAIL (no compila: faltan `MonitorBounds`, `monitor_at`, `preview_png`, `XcapScreen`).

- [ ] **Step 6: Implementar `screen.rs`**

Agrega al inicio de `src-tauri/src/capture/screen.rs`, antes de los tests:

```rust
use image::{
    ExtendedColorType, ImageEncoder, RgbaImage,
    codecs::png::{CompressionType, FilterType, PngEncoder},
};

use crate::error::{AppError, AppResult};

/// Posición y tamaño de un monitor en píxeles físicos del escritorio virtual: el principal
/// empieza en 0,0 y los que están a la izquierda o arriba tienen offset negativo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MonitorBounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl MonitorBounds {
    pub fn contains(&self, x: f64, y: f64) -> bool {
        let (left, top) = (f64::from(self.x), f64::from(self.y));
        x >= left && y >= top && x < left + f64::from(self.width) && y < top + f64::from(self.height)
    }
}

pub struct CapturedMonitor {
    pub bounds: MonitorBounds,
    pub image: RgbaImage,
}

/// La pantalla real (`XcapScreen`) o un doble en los tests.
pub trait Screen {
    fn capture_all(&self) -> AppResult<Vec<CapturedMonitor>>;
}

fn capture_error(error: impl std::fmt::Display) -> AppError {
    AppError::Capture(error.to_string())
}

pub struct XcapScreen;

impl Screen for XcapScreen {
    fn capture_all(&self) -> AppResult<Vec<CapturedMonitor>> {
        let monitors = xcap::Monitor::all().map_err(capture_error)?;
        let mut captured = Vec::with_capacity(monitors.len());
        for monitor in monitors {
            // En Windows, xcap lee posición y tamaño de DEVMODE: siempre en píxeles físicos.
            let bounds = MonitorBounds {
                x: monitor.x().map_err(capture_error)?,
                y: monitor.y().map_err(capture_error)?,
                width: monitor.width().map_err(capture_error)?,
                height: monitor.height().map_err(capture_error)?,
            };
            let image = monitor.capture_image().map_err(capture_error)?;
            captured.push(CapturedMonitor { bounds, image });
        }
        if captured.is_empty() {
            return Err(capture_error("no se encontró ningún monitor"));
        }
        Ok(captured)
    }
}

/// Índice del monitor que contiene el punto (coordenadas físicas del escritorio virtual).
pub fn monitor_at(monitors: &[MonitorBounds], x: f64, y: f64) -> Option<usize> {
    monitors.iter().position(|m| m.contains(x, y))
}

/// PNG con compresión rápida para mostrar la imagen congelada en el overlay; el recorte
/// final se hace sobre la imagen en memoria, no sobre este PNG.
pub fn preview_png(image: &RgbaImage) -> AppResult<Vec<u8>> {
    let mut png = Vec::new();
    PngEncoder::new_with_quality(&mut png, CompressionType::Fast, FilterType::Sub)
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            ExtendedColorType::Rgba8,
        )
        .map_err(capture_error)?;
    Ok(png)
}
```

Run: `cargo test capture::screen`. Expected: PASS (2 passed, 1 ignored). Luego `cargo test real_screen -- --ignored --nocapture` en tu equipo: imprime cada monitor con su imagen (solo informativo).

- [ ] **Step 7: Verificar y commit**

```bash
cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test && cd ..
git add docs src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/error.rs src-tauri/src/lib.rs src-tauri/src/capture
git commit -m "feat(f3): captura de monitores con xcap y error de captura propio

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: Recorte en píxeles físicos según DPI (`capture/crop.rs`)

**Files:**
- Create: `src-tauri/src/capture/crop.rs`
- Modify: `src-tauri/src/capture/mod.rs`

**Interfaces:**
- Consumes: `AppError::Capture` (Task 1).
- Produces:
  - `capture::crop::Selection { x, y, width, height, viewport_width, viewport_height }` (todos `f64`, píxeles CSS de la ventana overlay). `Deserialize` en camelCase (`viewportWidth`, `viewportHeight`) con `deny_unknown_fields`: es el argumento `selection` de `finish_capture`.
  - `capture::crop::PixelRect { x: u32, y: u32, width: u32, height: u32 }`.
  - `capture::crop::to_pixels(selection: &Selection, image_width: u32, image_height: u32) -> AppResult<PixelRect>`.
  - `capture::crop::full(width: u32, height: u32) -> PixelRect`.
  - `capture::crop::crop_png(image: &RgbaImage, rect: PixelRect) -> AppResult<Vec<u8>>` (PNG RGB).

- [ ] **Step 1: Tests que fallan**

En `src-tauri/src/capture/mod.rs`, agrega `pub mod crop;` antes de `pub mod screen;`.

Crea `src-tauri/src/capture/crop.rs` solo con los tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn sel(x: f64, y: f64, width: f64, height: f64, vw: f64, vh: f64) -> Selection {
        Selection {
            x,
            y,
            width,
            height,
            viewport_width: vw,
            viewport_height: vh,
        }
    }

    #[test]
    fn scales_css_pixels_to_physical_pixels_at_100_125_and_150_percent() {
        // La ventana overlay mide (imagen / escala) px CSS: así lo reporta WebView2 en cada monitor.
        for (scale, (iw, ih)) in [(1.0, (1920, 1080)), (1.25, (1920, 1080)), (1.5, (2880, 1620))] {
            let (vw, vh) = (f64::from(iw) / scale, f64::from(ih) / scale);
            let rect = to_pixels(&sel(100.0, 40.0, 200.0, 120.0, vw, vh), iw, ih).unwrap();
            let px = |v: f64| (v * scale).round() as u32;
            assert_eq!(
                rect,
                PixelRect { x: px(100.0), y: px(40.0), width: px(200.0), height: px(120.0) },
                "escala {scale}"
            );
        }
    }

    #[test]
    fn partially_covered_pixels_are_included() {
        // 125 %: 10.4 → 13.0, 10.6 → 13.25 (piso 13); 30.6 → 38.25 (techo 39); 15.6 → 19.5 (techo 20).
        let rect = to_pixels(&sel(10.4, 10.6, 20.2, 5.0, 1536.0, 864.0), 1920, 1080).unwrap();
        assert_eq!(rect, PixelRect { x: 13, y: 13, width: 26, height: 7 });
    }

    #[test]
    fn float_noise_does_not_add_an_extra_pixel() {
        // 0.1 + 0.2 = 0.30000000000000004: sin ajuste, el techo daría un píxel de más.
        let rect = to_pixels(&sel(0.1, 0.0, 0.2, 10.0, 100.0, 100.0), 1000, 1000).unwrap();
        assert_eq!(rect, PixelRect { x: 1, y: 0, width: 2, height: 100 });
    }

    #[test]
    fn selections_beyond_the_monitor_are_clamped() {
        let rect = to_pixels(&sel(-50.0, -50.0, 5000.0, 5000.0, 1536.0, 864.0), 1920, 1080).unwrap();
        assert_eq!(rect, full(1920, 1080));
    }

    #[test]
    fn rejects_empty_offscreen_or_malformed_selections() {
        let too_small = "La selección es demasiado pequeña.";
        let invalid = "La selección no es válida.";
        for (s, message) in [
            (sel(10.0, 10.0, 0.0, 50.0, 100.0, 100.0), too_small),
            (sel(2000.0, 10.0, 50.0, 50.0, 100.0, 100.0), too_small),
            (sel(f64::NAN, 10.0, 50.0, 50.0, 100.0, 100.0), invalid),
            (sel(10.0, 10.0, f64::INFINITY, 50.0, 100.0, 100.0), invalid),
            (sel(10.0, 10.0, 50.0, 50.0, 0.0, 100.0), invalid),
            (sel(10.0, 10.0, -5.0, 50.0, 100.0, 100.0), invalid),
        ] {
            assert_eq!(to_pixels(&s, 1920, 1080).unwrap_err().to_string(), message, "{s:?}");
        }
    }

    #[test]
    fn selection_deserializes_from_camel_case_and_rejects_unknown_fields() {
        let ok = r#"{"x":1,"y":2,"width":3,"height":4,"viewportWidth":5,"viewportHeight":6}"#;
        assert_eq!(serde_json::from_str::<Selection>(ok).unwrap(), sel(1.0, 2.0, 3.0, 4.0, 5.0, 6.0));
        let extra = r#"{"x":1,"y":2,"width":3,"height":4,"viewportWidth":5,"viewportHeight":6,"monitor":0}"#;
        assert!(serde_json::from_str::<Selection>(extra).is_err());
    }

    #[test]
    fn crop_png_keeps_only_the_selected_pixels_without_alpha() {
        let mut image = RgbaImage::from_pixel(4, 2, image::Rgba([200, 30, 30, 255]));
        for y in 0..2 {
            for x in 2..4 {
                image.put_pixel(x, y, image::Rgba([43, 98, 158, 255]));
            }
        }
        let png = crop_png(&image, PixelRect { x: 2, y: 0, width: 2, height: 2 }).unwrap();
        let decoded = image::load_from_memory(&png).unwrap();
        assert_eq!(decoded.color(), image::ColorType::Rgb8);
        assert_eq!((decoded.width(), decoded.height()), (2, 2));
        assert!(decoded.to_rgb8().pixels().all(|p| p.0 == [43, 98, 158]));
    }
}
```

Run: `cd src-tauri && cargo test capture::crop`. Expected: FAIL (no compila).

- [ ] **Step 2: Implementar `crop.rs`**

Agrega al inicio de `src-tauri/src/capture/crop.rs`:

```rust
use std::io::Cursor;

use image::{DynamicImage, ImageFormat, RgbaImage, imageops};
use serde::Deserialize;

use crate::error::{AppError, AppResult, validation};

const TOO_SMALL: &str = "La selección es demasiado pequeña.";
const INVALID: &str = "La selección no es válida.";

/// Rectángulo que envía el overlay en píxeles CSS de su ventana, junto con el tamaño de esa
/// ventana. El cociente imagen/ventana absorbe el escalado DPI del monitor (100 %, 125 %, 150 %…),
/// sin depender del factor de escala que reporte el sistema.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Selection {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub viewport_width: f64,
    pub viewport_height: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

pub fn full(width: u32, height: u32) -> PixelRect {
    PixelRect {
        x: 0,
        y: 0,
        width,
        height,
    }
}

/// Evita que un 375.00000000001 se convierta en 376 al redondear hacia arriba.
fn snap(v: f64) -> f64 {
    (v * 1e6).round() / 1e6
}

/// Convierte la selección a píxeles de la imagen del monitor. Los bordes izquierdo y superior
/// se redondean hacia abajo y los bordes derecho e inferior hacia arriba, para no perder la
/// fila o columna que el usuario ve parcialmente incluida. Luego se ajusta al monitor.
pub fn to_pixels(selection: &Selection, image_width: u32, image_height: u32) -> AppResult<PixelRect> {
    let s = selection;
    let values = [s.x, s.y, s.width, s.height, s.viewport_width, s.viewport_height];
    if values.iter().any(|v| !v.is_finite())
        || s.viewport_width <= 0.0
        || s.viewport_height <= 0.0
        || s.width < 0.0
        || s.height < 0.0
    {
        return Err(validation(INVALID));
    }
    let sx = f64::from(image_width) / s.viewport_width;
    let sy = f64::from(image_height) / s.viewport_height;
    let clamp = |v: f64, max: u32| v.clamp(0.0, f64::from(max));
    let left = clamp(snap(s.x * sx).floor(), image_width);
    let top = clamp(snap(s.y * sy).floor(), image_height);
    let right = clamp(snap((s.x + s.width) * sx).ceil(), image_width);
    let bottom = clamp(snap((s.y + s.height) * sy).ceil(), image_height);
    if right - left < 1.0 || bottom - top < 1.0 {
        return Err(validation(TOO_SMALL));
    }
    Ok(PixelRect {
        x: left as u32,
        y: top as u32,
        width: (right - left) as u32,
        height: (bottom - top) as u32,
    })
}

/// Recorta y codifica en PNG RGB: las capturas de pantalla no tienen transparencia.
pub fn crop_png(image: &RgbaImage, rect: PixelRect) -> AppResult<Vec<u8>> {
    let region = imageops::crop_imm(image, rect.x, rect.y, rect.width, rect.height).to_image();
    let rgb = DynamicImage::ImageRgb8(DynamicImage::ImageRgba8(region).to_rgb8());
    let mut png = Vec::new();
    rgb.write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
        .map_err(|e| AppError::Capture(e.to_string()))?;
    Ok(png)
}
```

Run: `cargo test capture::crop`. Expected: PASS (7 passed).

- [ ] **Step 3: Verificar y commit**

```bash
cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test && cd ..
git add src-tauri/src/capture
git commit -m "feat(f3): recorte en píxeles físicos según la escala de cada monitor

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: Sesión de captura — orquestación, cancelación y limpieza (`capture/session.rs`)

**Files:**
- Create: `src-tauri/src/capture/session.rs`
- Modify: `src-tauri/src/capture/mod.rs`

**Interfaces:**
- Consumes: `Screen`, `CapturedMonitor`, `MonitorBounds`, `monitor_at`, `preview_png` (Task 1); `Selection`, `to_pixels`, `full`, `crop_png` (Task 2); `repo::evidence::stage(files: &EvidenceFiles, bytes: &[u8]) -> AppResult<StagedImageDto>` y `EvidenceFiles::{staging_dir, discard_staged}` (F2).
- Produces:
  - `trait CaptureHost` con `hide_main(&self) -> bool` (devuelve si estaba visible), `show_main(&self)`, `open_overlays(&self, monitors: &[MonitorBounds]) -> AppResult<()>`, `close_overlays(&self)`, `show_overlay(&self, index: usize, focus: bool)`, `emit_captured(&self, tmp_path: &str)` y `cursor(&self) -> Option<(f64, f64)>`.
  - `pub const HIDE_DELAY: Duration` (200 ms).
  - `CaptureController::new(hide_delay: Duration)` con:
    - `start(&self, host: &impl CaptureHost, screen: &impl Screen) -> AppResult<()>`
    - `frame(&self, index: usize) -> AppResult<Vec<u8>>`
    - `overlay_ready(&self, host: &impl CaptureHost, index: usize) -> AppResult<()>`
    - `finish(&self, host: &impl CaptureHost, files: &EvidenceFiles, index: usize, selection: Option<Selection>) -> AppResult<StagedImageDto>`
    - `cancel(&self, host: &impl CaptureHost)` y `overlay_closed(&self, host: &impl CaptureHost)`

- [ ] **Step 1: Tests que fallan**

En `src-tauri/src/capture/mod.rs`, agrega `pub mod session;` después de `pub mod screen;`.

Crea `src-tauri/src/capture/session.rs` solo con los tests:

```rust
#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use image::{Rgba, RgbaImage};

    use super::*;
    use crate::{error::AppError, media::storage::TempDir};

    const RED: [u8; 3] = [200, 30, 30];
    const BLUE: [u8; 3] = [43, 98, 158];

    struct FakeHost {
        visible: Cell<bool>,
        cursor: Cell<Option<(f64, f64)>>,
        fail_overlays: bool,
        log: RefCell<Vec<String>>,
    }

    impl FakeHost {
        fn new() -> Self {
            Self { visible: Cell::new(true), cursor: Cell::new(None), fail_overlays: false, log: RefCell::new(vec![]) }
        }
        fn push(&self, entry: impl Into<String>) {
            self.log.borrow_mut().push(entry.into());
        }
        fn log(&self) -> Vec<String> {
            self.log.borrow().clone()
        }
        fn clear(&self) {
            self.log.borrow_mut().clear();
        }
    }

    impl CaptureHost for FakeHost {
        fn hide_main(&self) -> bool {
            self.push("hide_main");
            self.visible.replace(false)
        }
        fn show_main(&self) {
            self.push("show_main");
            self.visible.set(true);
        }
        fn open_overlays(&self, monitors: &[MonitorBounds]) -> AppResult<()> {
            self.push(format!("open_overlays:{}", monitors.len()));
            if self.fail_overlays { Err(AppError::Capture("webview".into())) } else { Ok(()) }
        }
        fn close_overlays(&self) {
            self.push("close_overlays");
        }
        fn show_overlay(&self, index: usize, focus: bool) {
            self.push(format!("show_overlay:{index}:{focus}"));
        }
        fn emit_captured(&self, tmp_path: &str) {
            self.push(format!("emit:{tmp_path}"));
        }
        fn cursor(&self) -> Option<(f64, f64)> {
            self.cursor.get()
        }
    }

    /// `None` simula un fallo de xcap.
    struct FakeScreen(Option<Vec<(MonitorBounds, [u8; 3])>>);

    impl Screen for FakeScreen {
        fn capture_all(&self) -> AppResult<Vec<CapturedMonitor>> {
            let list = self.0.as_ref().ok_or_else(|| AppError::Capture("xcap".into()))?;
            Ok(list
                .iter()
                .map(|(b, [r, g, bl])| CapturedMonitor {
                    bounds: *b,
                    image: RgbaImage::from_pixel(b.width, b.height, Rgba([*r, *g, *bl, 255])),
                })
                .collect())
        }
    }

    fn bounds(x: i32, y: i32, width: u32, height: u32) -> MonitorBounds {
        MonitorBounds { x, y, width, height }
    }

    /// Izquierda (offset negativo) al 100 % y principal al 150 %, a escala reducida para que el test sea rápido.
    fn two_monitors() -> FakeScreen {
        FakeScreen(Some(vec![(bounds(-192, 0, 192, 108), RED), (bounds(0, 0, 288, 162), BLUE)]))
    }

    fn sel(x: f64, y: f64, width: f64, height: f64, vw: f64, vh: f64) -> Selection {
        Selection { x, y, width, height, viewport_width: vw, viewport_height: vh }
    }

    fn controller() -> CaptureController {
        CaptureController::new(Duration::ZERO)
    }

    fn staged_files(files: &EvidenceFiles) -> usize {
        std::fs::read_dir(files.staging_dir()).map(|d| d.count()).unwrap_or(0)
    }

    fn pixels(path: &str) -> image::RgbImage {
        image::open(path).unwrap().to_rgb8()
    }

    #[test]
    fn start_hides_qapture_and_opens_one_overlay_per_monitor() {
        let (host, capture) = (FakeHost::new(), controller());
        capture.start(&host, &two_monitors()).unwrap();
        assert_eq!(host.log(), ["hide_main", "open_overlays:2"]);
        assert_eq!(crate::media::raster::dimensions(&capture.frame(0).unwrap()).unwrap(), (192, 108));
        assert_eq!(crate::media::raster::dimensions(&capture.frame(1).unwrap()).unwrap(), (288, 162));
        assert_eq!(capture.frame(2).unwrap_err().to_string(), "La captura ya terminó.");
    }

    #[test]
    fn a_second_start_is_rejected_until_the_session_ends() {
        let (host, capture) = (FakeHost::new(), controller());
        capture.start(&host, &two_monitors()).unwrap();
        assert_eq!(capture.start(&host, &two_monitors()).unwrap_err().to_string(), "Ya hay una captura en curso.");
        capture.cancel(&host);
        capture.start(&host, &two_monitors()).unwrap();
    }

    #[test]
    fn a_failed_capture_shows_qapture_again_and_frees_the_session() {
        let (host, capture) = (FakeHost::new(), controller());
        let error = capture.start(&host, &FakeScreen(None)).unwrap_err();
        assert_eq!(error.code(), "capture");
        assert_eq!(host.log(), ["hide_main", "show_main"]);
        capture.start(&host, &two_monitors()).unwrap();
    }

    #[test]
    fn failed_overlays_close_everything_and_show_qapture() {
        let host = FakeHost { fail_overlays: true, ..FakeHost::new() };
        let capture = controller();
        assert!(capture.start(&host, &two_monitors()).is_err());
        assert_eq!(host.log(), ["hide_main", "open_overlays:2", "close_overlays", "show_main"]);
        assert!(capture.frame(0).is_err());
    }

    #[test]
    fn overlay_ready_focuses_only_the_monitor_under_the_cursor() {
        let (host, capture) = (FakeHost::new(), controller());
        capture.start(&host, &two_monitors()).unwrap();
        host.clear();
        host.cursor.set(Some((-10.0, 50.0)));
        capture.overlay_ready(&host, 0).unwrap();
        capture.overlay_ready(&host, 1).unwrap();
        assert_eq!(host.log(), ["show_overlay:0:true", "show_overlay:1:false"]);
        assert!(capture.overlay_ready(&host, 5).is_err());
    }

    #[test]
    fn finish_crops_the_selected_monitor_even_with_a_negative_offset() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let (host, capture) = (FakeHost::new(), controller());
        capture.start(&host, &two_monitors()).unwrap();
        host.clear();
        let staged = capture.finish(&host, &files, 0, Some(sel(10.0, 10.0, 50.0, 20.0, 192.0, 108.0))).unwrap();
        let img = pixels(&staged.path);
        assert_eq!((img.width(), img.height()), (50, 20));
        assert!(img.pixels().all(|p| p.0 == RED));
        assert_eq!((staged.width, staged.height), (50, 20));
        assert_eq!(host.log(), ["close_overlays".to_string(), "show_main".into(), format!("emit:{}", staged.path)]);
        assert!(capture.frame(0).is_err(), "la sesión terminó");

        // Monitor al 150 %: su ventana mide 192×108 px CSS para una imagen de 288×162.
        capture.start(&host, &two_monitors()).unwrap();
        let staged = capture.finish(&host, &files, 1, Some(sel(10.0, 10.0, 20.0, 20.0, 192.0, 108.0))).unwrap();
        let img = pixels(&staged.path);
        assert_eq!((img.width(), img.height()), (30, 30));
        assert!(img.pixels().all(|p| p.0 == BLUE));
    }

    #[test]
    fn enter_captures_the_whole_monitor_under_the_cursor() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let (host, capture) = (FakeHost::new(), controller());
        capture.start(&host, &two_monitors()).unwrap();
        host.cursor.set(Some((-100.0, 50.0)));
        let staged = capture.finish(&host, &files, 1, None).unwrap();
        assert_eq!((staged.width, staged.height), (192, 108), "el del puntero, no el que recibió la tecla");

        capture.start(&host, &two_monitors()).unwrap();
        host.cursor.set(None);
        let staged = capture.finish(&host, &files, 1, None).unwrap();
        assert_eq!((staged.width, staged.height), (288, 162), "sin puntero: el que recibió la tecla");
    }

    #[test]
    fn a_large_capture_is_staged_with_its_longest_side_at_2400_px() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let (host, capture) = (FakeHost::new(), controller());
        capture.start(&host, &FakeScreen(Some(vec![(bounds(0, 0, 4800, 100), BLUE)]))).unwrap();
        let staged = capture.finish(&host, &files, 0, None).unwrap();
        assert_eq!((staged.width, staged.height), (2400, 50));
    }

    #[test]
    fn a_bad_selection_keeps_the_overlay_open_for_another_try() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let (host, capture) = (FakeHost::new(), controller());
        capture.start(&host, &two_monitors()).unwrap();
        host.clear();
        let error = capture.finish(&host, &files, 0, Some(sel(10.0, 10.0, 0.0, 5.0, 192.0, 108.0))).unwrap_err();
        assert_eq!(error.to_string(), "La selección es demasiado pequeña.");
        assert!(host.log().is_empty(), "no cierra nada");
        assert_eq!(staged_files(&files), 0);
        assert!(capture.finish(&host, &files, 0, Some(sel(0.0, 0.0, 10.0, 10.0, 192.0, 108.0))).is_ok());
    }

    #[test]
    fn cancel_leaves_no_temporary_files_and_restores_visibility() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let (host, capture) = (FakeHost::new(), controller());
        capture.start(&host, &two_monitors()).unwrap();
        capture.cancel(&host);
        assert_eq!(host.log(), ["hide_main", "open_overlays:2", "close_overlays", "show_main"]);
        assert_eq!(staged_files(&files), 0);
        assert!(capture.frame(0).is_err());

        // Qapture oculto antes de capturar (atajo global de F6): al cancelar sigue oculto.
        host.visible.set(false);
        host.clear();
        capture.start(&host, &two_monitors()).unwrap();
        capture.cancel(&host);
        assert_eq!(host.log(), ["hide_main", "open_overlays:2", "close_overlays"]);
        capture.cancel(&host);
        assert_eq!(host.log().len(), 3, "cancelar sin sesión no hace nada");
    }

    #[test]
    fn closing_an_overlay_cancels_and_closing_after_finish_is_a_no_op() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let (host, capture) = (FakeHost::new(), controller());
        capture.start(&host, &two_monitors()).unwrap();
        capture.overlay_closed(&host);
        assert!(capture.frame(0).is_err());
        assert_eq!(host.log().last().unwrap(), "show_main");

        capture.start(&host, &two_monitors()).unwrap();
        capture.finish(&host, &files, 0, None).unwrap();
        host.clear();
        capture.overlay_closed(&host);
        assert!(host.log().is_empty());
    }
}
```

Run: `cd src-tauri && cargo test capture::session`. Expected: FAIL (no compila).

- [ ] **Step 2: Implementar `session.rs`**

Agrega al inicio de `src-tauri/src/capture/session.rs`:

```rust
use std::{
    sync::{Mutex, MutexGuard},
    thread,
    time::Duration,
};

use super::{
    crop::{self, Selection},
    screen::{CapturedMonitor, MonitorBounds, Screen, monitor_at, preview_png},
};
use crate::{
    dto::StagedImageDto,
    error::{AppResult, validation},
    media::storage::EvidenceFiles,
    repo::evidence,
};

/// Tiempo para que Windows termine de ocultar Qapture antes de capturar.
pub const HIDE_DELAY: Duration = Duration::from_millis(200);

const BUSY: &str = "Ya hay una captura en curso.";
const NO_SESSION: &str = "La captura ya terminó.";

/// Lo que la sesión necesita de Tauri (`windows::TauriHost`); los tests usan un doble.
pub trait CaptureHost {
    /// Oculta la ventana principal y devuelve si estaba visible.
    fn hide_main(&self) -> bool;
    fn show_main(&self);
    fn open_overlays(&self, monitors: &[MonitorBounds]) -> AppResult<()>;
    fn close_overlays(&self);
    fn show_overlay(&self, index: usize, focus: bool);
    fn emit_captured(&self, tmp_path: &str);
    /// Posición del puntero en píxeles físicos del escritorio virtual.
    fn cursor(&self) -> Option<(f64, f64)>;
}

struct Session {
    monitors: Vec<CapturedMonitor>,
    main_was_visible: bool,
}

enum Phase {
    Idle,
    Starting,
    Selecting(Session),
}

/// Ciclo de una captura. Las imágenes congeladas viven solo en memoria mientras los overlays
/// están abiertos. El candado nunca se mantiene mientras se llama al host, porque cerrar un
/// overlay dispara `overlay_closed` desde el hilo de eventos.
pub struct CaptureController {
    phase: Mutex<Phase>,
    hide_delay: Duration,
}

impl CaptureController {
    pub fn new(hide_delay: Duration) -> Self {
        Self {
            phase: Mutex::new(Phase::Idle),
            hide_delay,
        }
    }

    fn phase(&self) -> MutexGuard<'_, Phase> {
        self.phase.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn take_session(&self) -> Option<Session> {
        let mut phase = self.phase();
        match std::mem::replace(&mut *phase, Phase::Idle) {
            Phase::Selecting(session) => Some(session),
            other => {
                *phase = other;
                None
            }
        }
    }

    fn bounds(&self) -> AppResult<Vec<MonitorBounds>> {
        match &*self.phase() {
            Phase::Selecting(session) => Ok(session.monitors.iter().map(|m| m.bounds).collect()),
            _ => Err(validation(NO_SESSION)),
        }
    }

    pub fn start(&self, host: &impl CaptureHost, screen: &impl Screen) -> AppResult<()> {
        {
            let mut phase = self.phase();
            if !matches!(*phase, Phase::Idle) {
                return Err(validation(BUSY));
            }
            *phase = Phase::Starting;
        }
        let main_was_visible = host.hide_main();
        if main_was_visible {
            thread::sleep(self.hide_delay);
        }
        let monitors = match screen.capture_all() {
            Ok(monitors) => monitors,
            Err(error) => {
                *self.phase() = Phase::Idle;
                // Siempre se muestra: si no, el mensaje de error no tendría dónde verse.
                host.show_main();
                return Err(error);
            }
        };
        let bounds: Vec<MonitorBounds> = monitors.iter().map(|m| m.bounds).collect();
        *self.phase() = Phase::Selecting(Session {
            monitors,
            main_was_visible,
        });
        if let Err(error) = host.open_overlays(&bounds) {
            self.take_session();
            host.close_overlays();
            host.show_main();
            return Err(error);
        }
        Ok(())
    }

    /// Imagen congelada del monitor `index`, para el overlay que lo cubre.
    pub fn frame(&self, index: usize) -> AppResult<Vec<u8>> {
        let phase = self.phase();
        let Phase::Selecting(session) = &*phase else {
            return Err(validation(NO_SESSION));
        };
        let monitor = session.monitors.get(index).ok_or_else(|| validation(NO_SESSION))?;
        preview_png(&monitor.image)
    }

    /// El overlay ya pintó su imagen: se muestra, y toma el foco si el puntero está en su monitor.
    pub fn overlay_ready(&self, host: &impl CaptureHost, index: usize) -> AppResult<()> {
        let bounds = self.bounds()?;
        if index >= bounds.len() {
            return Err(validation(NO_SESSION));
        }
        let under = host.cursor().and_then(|(x, y)| monitor_at(&bounds, x, y)).unwrap_or(0);
        host.show_overlay(index, under == index);
        Ok(())
    }

    /// `selection: None` (tecla Enter) captura el monitor completo bajo el puntero, o el del
    /// overlay que llamó. Recorta y deja la imagen en `_staging` **antes** de cerrar los
    /// overlays: si algo falla, el overlay sigue abierto y muestra el error.
    pub fn finish(
        &self,
        host: &impl CaptureHost,
        files: &EvidenceFiles,
        index: usize,
        selection: Option<Selection>,
    ) -> AppResult<StagedImageDto> {
        let bounds = self.bounds()?;
        let index = match selection {
            Some(_) => index,
            None => host.cursor().and_then(|(x, y)| monitor_at(&bounds, x, y)).unwrap_or(index),
        };
        let png = {
            let phase = self.phase();
            let Phase::Selecting(session) = &*phase else {
                return Err(validation(NO_SESSION));
            };
            let monitor = session.monitors.get(index).ok_or_else(|| validation(NO_SESSION))?;
            let (width, height) = monitor.image.dimensions();
            let rect = match &selection {
                Some(s) => crop::to_pixels(s, width, height)?,
                None => crop::full(width, height),
            };
            crop::crop_png(&monitor.image, rect)?
        };
        let staged = evidence::stage(files, &png)?;
        if self.take_session().is_none() {
            // Se canceló mientras se recortaba: no debe quedar el temporal.
            let _ = files.discard_staged(&staged.path);
            return Err(validation(NO_SESSION));
        }
        host.close_overlays();
        host.show_main();
        host.emit_captured(&staged.path);
        Ok(staged)
    }

    pub fn cancel(&self, host: &impl CaptureHost) {
        if let Some(session) = self.take_session() {
            host.close_overlays();
            if session.main_was_visible {
                host.show_main();
            }
        }
    }

    /// Un overlay se cerró sin pasar por `finish` o `cancel` (Alt+F4, WebView caído).
    /// Si la sesión ya terminó (los overlays se cierran al terminar), no hace nada.
    pub fn overlay_closed(&self, host: &impl CaptureHost) {
        self.cancel(host);
    }
}
```

Run: `cargo test capture::session`. Expected: PASS (11 passed).

- [ ] **Step 3: Verificar y commit**

```bash
cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test && cd ..
git add src-tauri/src/capture
git commit -m "feat(f3): sesión de captura con cancelación, reintento y sin temporales

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: ACL por ventana — lista única de comandos y capability de la principal

**Files:**
- Create: `src-tauri/src/commands/names.rs`
- Rename: `src-tauri/capabilities/default.json` → `src-tauri/capabilities/main.json`
- Modify: `src-tauri/build.rs`, `src-tauri/src/commands/mod.rs`, `src-tauri/.gitignore`

**Interfaces:**
- Produces: `commands::names::APP_COMMANDS: &[&str]` (todos los comandos propios) y `commands::names::OVERLAY_COMMANDS: &[&str]` (vacía en esta task; se llena en la Task 6). Permisos generados `allow-<comando-con-guiones>`.

> Por qué: Tauri 2 permite cualquier comando propio desde cualquier ventana mientras no exista un *app manifest*. En cuanto existe uno, **cada** comando necesita su `allow-*` en alguna capability (verificado en `tauri-2.12.4/src/webview/mod.rs`, `has_app_acl_manifest`). Por eso esta task lista los 24 comandos actuales antes de agregar el overlay.

- [ ] **Step 1: Tests que fallan**

Crea `src-tauri/src/commands/names.rs`:

```rust
// Lista única de comandos propios. build.rs la incluye (`include!`) para generar los permisos
// `allow-*`. Al agregar un comando: regístralo aquí, en `generate_handler!` (lib.rs) y en la
// capability de su ventana (capabilities/*.json). Los tests de abajo verifican las tres cosas.

pub const APP_COMMANDS: &[&str] = &[
    "list_templates",
    "save_template_from_project",
    "delete_template",
    "list_projects",
    "get_project",
    "create_project",
    "update_project",
    "delete_project",
    "save_project_fields",
    "list_cases",
    "create_case",
    "update_case",
    "duplicate_case",
    "delete_case",
    "stage_evidence_image",
    "discard_staged_image",
    "read_evidence_image",
    "list_evidence",
    "add_evidence",
    "update_evidence",
    "update_evidence_description",
    "move_evidence",
    "reorder_evidence",
    "delete_evidence",
];

/// Lo único que puede invocar una ventana overlay (spec §9). El resto es de la principal.
pub const OVERLAY_COMMANDS: &[&str] = &[];

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    fn capability(file: &str) -> (Vec<String>, BTreeSet<String>) {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("capabilities").join(file);
        let json: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let strings = |key: &str| -> Vec<String> {
            json[key].as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect()
        };
        (strings("windows"), strings("permissions").into_iter().collect())
    }

    fn allow<'a>(names: impl IntoIterator<Item = &'a &'a str>) -> BTreeSet<String> {
        names.into_iter().map(|n| format!("allow-{}", n.replace('_', "-"))).collect()
    }

    #[test]
    fn command_names_are_unique_and_overlay_commands_are_app_commands() {
        let unique: BTreeSet<_> = APP_COMMANDS.iter().collect();
        assert_eq!(unique.len(), APP_COMMANDS.len());
        assert!(OVERLAY_COMMANDS.iter().all(|c| APP_COMMANDS.contains(c)));
    }

    #[test]
    fn main_window_gets_core_and_every_non_overlay_command() {
        let (windows, permissions) = capability("main.json");
        assert_eq!(windows, ["main"]);
        let mut expected = allow(APP_COMMANDS.iter().filter(|c| !OVERLAY_COMMANDS.contains(c)));
        expected.insert("core:default".into());
        assert_eq!(permissions, expected);
    }

    #[test]
    fn every_registered_handler_is_listed_and_vice_versa() {
        let lib = include_str!("../lib.rs");
        let registered: BTreeSet<&str> = lib
            .lines()
            .map(str::trim)
            .filter(|l| l.starts_with("commands::") && l.ends_with(','))
            .map(|l| l.trim_end_matches(',').rsplit("::").next().unwrap())
            .collect();
        assert_eq!(registered, APP_COMMANDS.iter().copied().collect::<BTreeSet<_>>());
    }
}
```

En `src-tauri/src/commands/mod.rs`, agrega al final:

```rust
// Solo lo leen build.rs (vía include!) y los tests de ACL.
#[cfg(test)]
mod names;
```

Renombra la capability:

```bash
git mv src-tauri/capabilities/default.json src-tauri/capabilities/main.json
```

Run: `cd src-tauri && cargo test commands::names`. Expected: FAIL en `main_window_gets_core_and_every_non_overlay_command` (main.json solo tiene `core:default`).

- [ ] **Step 2: Capability de la principal y app manifest**

Reemplaza el contenido de `src-tauri/capabilities/main.json`:

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "main",
  "description": "Ventana principal: núcleo de Tauri y los comandos propios de Qapture (el I/O pasa por comandos con rutas validadas).",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "allow-list-templates",
    "allow-save-template-from-project",
    "allow-delete-template",
    "allow-list-projects",
    "allow-get-project",
    "allow-create-project",
    "allow-update-project",
    "allow-delete-project",
    "allow-save-project-fields",
    "allow-list-cases",
    "allow-create-case",
    "allow-update-case",
    "allow-duplicate-case",
    "allow-delete-case",
    "allow-stage-evidence-image",
    "allow-discard-staged-image",
    "allow-read-evidence-image",
    "allow-list-evidence",
    "allow-add-evidence",
    "allow-update-evidence",
    "allow-update-evidence-description",
    "allow-move-evidence",
    "allow-reorder-evidence",
    "allow-delete-evidence"
  ]
}
```

Reemplaza `src-tauri/build.rs`:

```rust
// Con un app manifest, cada comando propio necesita su permiso `allow-*` en alguna
// capability (capabilities/*.json): así el overlay de captura no puede llamar a los de datos.
#[allow(dead_code)]
mod names {
    include!("src/commands/names.rs");
}

fn main() {
    println!("cargo:rerun-if-changed=src/commands/names.rs");
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(names::APP_COMMANDS)),
    )
    .expect("no se pudo preparar el build de Tauri");
}
```

Agrega al final de `src-tauri/.gitignore`:

```
# Permisos allow-*/deny-* que genera build.rs a partir de src/commands/names.rs
/permissions/autogenerated
```

Run: `cargo test commands::names`. Expected: PASS (3 passed). Si `cargo build` falla con "Permission allow-… not found", hay un nombre mal escrito en `main.json`; el mensaje sugiere el correcto.

- [ ] **Step 3: Smoke manual (la ACL no rompe la app)**

Run: `cd .. && pnpm tauri dev`. Comprueba que se listan los proyectos, que se abre un caso y su galería, y que un `Ctrl+V` con una imagen abre el editor. En la consola de DevTools no debe aparecer `not allowed` ni `Command … not allowed by ACL`. Cierra la app.

- [ ] **Step 4: Verificar y commit**

```bash
cd src-tauri && cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test && cd ..
pnpm typecheck && pnpm test
git add src-tauri/build.rs src-tauri/.gitignore src-tauri/capabilities src-tauri/src/commands
git commit -m "feat(f3): ACL por ventana con app manifest y lista única de comandos

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: UI del overlay — imagen congelada, oscurecido y selección con medidas

**Files:**
- Create: `overlay.html`, `src/overlay-main.tsx`
- Create: `src/domain/capture.ts`, `src/domain/capture.test.ts`
- Create: `src/ports/capture.ts`
- Create: `src/infrastructure/tauri/capture.ts`, `src/infrastructure/tauri/capture.test.ts`
- Create: `src/ui/overlay/OverlayApp.tsx`, `src/ui/overlay/OverlayApp.test.tsx`
- Modify: `vite.config.ts`

**Interfaces:**
- Consumes: `call`, `callBytes` (`infrastructure/tauri/client.ts`), `AppError` (`ports/errors.ts`).
- Produces:
  - `domain/capture.ts`: tipos `Point`, `Rect`, `Viewport` y `CaptureSelection` (`Rect` + `viewportWidth`, `viewportHeight`); `MIN_SELECTION = 4`; `rectFrom(start, current, viewport): Rect`; `isUsable(rect): boolean`; `sizeLabel(rect, pixelRatio): string`; `toSelection(rect, viewport): CaptureSelection`.
  - `ports/capture.ts`: `interface OverlayApi { loadFrame(): Promise<Uint8Array>; ready(): Promise<void>; finish(selection: CaptureSelection | null): Promise<void>; cancel(): Promise<void> }`.
  - `infrastructure/tauri/capture.ts`: `tauriOverlayApi(): OverlayApi`, que usa `capture_frame {}`, `overlay_ready {}`, `finish_capture { selection }` y `cancel_capture {}`.
  - `ui/overlay/OverlayApp({ api }: { api: OverlayApi })`.

- [ ] **Step 1: Tests que fallan — dominio**

Crea `src/domain/capture.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { isUsable, MIN_SELECTION, rectFrom, sizeLabel, toSelection } from "./capture";

const viewport = { width: 1536, height: 864 };

describe("selección del overlay", () => {
  it("normaliza un arrastre en cualquier dirección", () => {
    expect(rectFrom({ x: 300, y: 200 }, { x: 100, y: 50 }, viewport)).toEqual({ x: 100, y: 50, width: 200, height: 150 });
  });

  it("limita la selección a la ventana", () => {
    expect(rectFrom({ x: -20, y: 800 }, { x: 2000, y: 900 }, viewport)).toEqual({ x: 0, y: 800, width: 1536, height: 64 });
  });

  it("un clic sin arrastre no es una selección", () => {
    expect(isUsable({ x: 10, y: 10, width: MIN_SELECTION - 1, height: 50 })).toBe(false);
    expect(isUsable({ x: 10, y: 10, width: MIN_SELECTION, height: MIN_SELECTION })).toBe(true);
  });

  it("muestra la medida en píxeles reales del monitor", () => {
    expect(sizeLabel({ x: 0, y: 0, width: 200, height: 100 }, 1.25)).toBe("250 × 125");
    expect(sizeLabel({ x: 0, y: 0, width: 33.3, height: 10 }, 1.5)).toBe("50 × 15");
  });

  it("agrega el tamaño de la ventana para que Rust calcule la escala", () => {
    expect(toSelection({ x: 1, y: 2, width: 3, height: 4 }, viewport)).toEqual({
      x: 1,
      y: 2,
      width: 3,
      height: 4,
      viewportWidth: 1536,
      viewportHeight: 864,
    });
  });
});
```

Run: `pnpm vitest run src/domain/capture.test.ts`. Expected: FAIL (`Failed to resolve import "./capture"`).

- [ ] **Step 2: Implementar el dominio**

Crea `src/domain/capture.ts`:

```ts
export interface Point {
  x: number;
  y: number;
}

export interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface Viewport {
  width: number;
  height: number;
}

/** Lo que recibe `finish_capture` (`capture::crop::Selection` en Rust), en píxeles CSS del overlay. */
export interface CaptureSelection extends Rect {
  viewportWidth: number;
  viewportHeight: number;
}

/** Por debajo de esto (px CSS por lado) se toma como un clic sin arrastre. */
export const MIN_SELECTION = 4;

const clamp = (v: number, max: number) => Math.min(Math.max(v, 0), max);

/** Rectángulo entre el punto inicial y el actual, en cualquier dirección y limitado a la ventana. */
export function rectFrom(start: Point, current: Point, viewport: Viewport): Rect {
  const x1 = clamp(start.x, viewport.width);
  const x2 = clamp(current.x, viewport.width);
  const y1 = clamp(start.y, viewport.height);
  const y2 = clamp(current.y, viewport.height);
  return { x: Math.min(x1, x2), y: Math.min(y1, y2), width: Math.abs(x2 - x1), height: Math.abs(y2 - y1) };
}

export const isUsable = (rect: Rect) => rect.width >= MIN_SELECTION && rect.height >= MIN_SELECTION;

/** Medida junto a la selección: el overlay cubre el monitor, así que px CSS × devicePixelRatio = px de la captura. */
export function sizeLabel(rect: Rect, pixelRatio: number): string {
  return `${Math.round(rect.width * pixelRatio)} × ${Math.round(rect.height * pixelRatio)}`;
}

export const toSelection = (rect: Rect, viewport: Viewport): CaptureSelection => ({
  ...rect,
  viewportWidth: viewport.width,
  viewportHeight: viewport.height,
});
```

Run: `pnpm vitest run src/domain/capture.test.ts`. Expected: PASS (5 passed).

- [ ] **Step 3: Puerto y adaptador Tauri del overlay (test primero)**

Crea `src/ports/capture.ts`:

```ts
import type { CaptureSelection } from "../domain/capture";

/** Lo único que puede hacer una ventana overlay (capability `overlay`, spec §9). */
export interface OverlayApi {
  /** PNG del monitor que cubre esta ventana (Rust lo elige por el label `overlay-<n>`). */
  loadFrame(): Promise<Uint8Array>;
  /** La imagen ya se pintó: Rust muestra la ventana y le da el foco si el puntero está en su monitor. */
  ready(): Promise<void>;
  /** `null` = monitor completo (Enter). */
  finish(selection: CaptureSelection | null): Promise<void>;
  cancel(): Promise<void>;
}
```

Crea `src/infrastructure/tauri/capture.test.ts`:

```ts
import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => invoke(...args) }));

const { tauriOverlayApi } = await import("./capture");

describe("adaptador Tauri del overlay", () => {
  beforeEach(() => invoke.mockReset());

  it("pide su imagen congelada como bytes", async () => {
    invoke.mockResolvedValue(new Uint8Array([1, 2, 3]).buffer);
    await expect(tauriOverlayApi().loadFrame()).resolves.toEqual(new Uint8Array([1, 2, 3]));
    expect(invoke).toHaveBeenCalledWith("capture_frame", {});
  });

  it("envía la selección o null para el monitor completo", async () => {
    invoke.mockResolvedValue(null);
    const api = tauriOverlayApi();
    const selection = { x: 1, y: 2, width: 3, height: 4, viewportWidth: 5, viewportHeight: 6 };
    await api.finish(selection);
    await api.finish(null);
    await api.ready();
    await api.cancel();
    expect(invoke.mock.calls).toEqual([
      ["finish_capture", { selection }],
      ["finish_capture", { selection: null }],
      ["overlay_ready", {}],
      ["cancel_capture", {}],
    ]);
  });

  it("traduce el error de Rust para mostrarlo en el overlay", async () => {
    invoke.mockRejectedValue({ code: "validation", message: "La selección es demasiado pequeña.", detail: "x" });
    await expect(tauriOverlayApi().finish(null)).rejects.toMatchObject({ message: "La selección es demasiado pequeña." });
  });
});
```

Run: `pnpm vitest run src/infrastructure/tauri/capture.test.ts`. Expected: FAIL (no existe `./capture`).

Crea `src/infrastructure/tauri/capture.ts`:

```ts
import { z } from "zod";
import type { OverlayApi } from "../../ports/capture";
import { call, callBytes } from "./client";

const nothing = z.null();

export function tauriOverlayApi(): OverlayApi {
  return {
    loadFrame: () => callBytes("capture_frame", {}),
    ready: async () => {
      await call("overlay_ready", {}, nothing);
    },
    finish: async (selection) => {
      await call("finish_capture", { selection }, nothing);
    },
    cancel: async () => {
      await call("cancel_capture", {}, nothing);
    },
  };
}
```

Run: `pnpm vitest run src/infrastructure/tauri/capture.test.ts`. Expected: PASS (3 passed).

- [ ] **Step 4: Tests que fallan — `OverlayApp`**

Crea `src/ui/overlay/OverlayApp.test.tsx`:

```tsx
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { AppError } from "../../ports/errors";
import type { OverlayApi } from "../../ports/capture";
import { OverlayApp } from "./OverlayApp";

const HINT = "Arrastra para seleccionar · Enter: pantalla completa · Esc: cancelar";

function fakeApi(overrides: Partial<OverlayApi> = {}) {
  return {
    loadFrame: vi.fn(async () => new Uint8Array([0x89, 0x50, 0x4e, 0x47])),
    ready: vi.fn(async () => {}),
    finish: vi.fn(async () => {}),
    cancel: vi.fn(async () => {}),
    ...overrides,
  } satisfies OverlayApi;
}

async function renderOverlay(api = fakeApi()) {
  render(<OverlayApp api={api} />);
  const frame = await screen.findByTestId("frame");
  return { api, frame, surface: screen.getByTestId("overlay-surface") };
}

const drag = (surface: HTMLElement, from: [number, number], to: [number, number]) => {
  fireEvent.pointerDown(surface, { button: 0, clientX: from[0], clientY: from[1], pointerId: 1 });
  fireEvent.pointerMove(surface, { clientX: to[0], clientY: to[1], pointerId: 1 });
  fireEvent.pointerUp(surface, { clientX: to[0], clientY: to[1], pointerId: 1 });
};

describe("overlay de captura", () => {
  beforeEach(() => {
    Object.assign(URL, { createObjectURL: vi.fn(() => "blob:frame"), revokeObjectURL: vi.fn() });
  });

  it("muestra la imagen congelada con la pista y avisa cuando se pintó", async () => {
    const { api, frame } = await renderOverlay();
    expect(frame).toHaveAttribute("src", "blob:frame");
    expect(screen.getByRole("status")).toHaveTextContent(HINT);
    fireEvent.load(frame);
    expect(api.ready).toHaveBeenCalledTimes(1);
  });

  it("arrastrar muestra la medida y envía la selección con el tamaño de la ventana", async () => {
    const { api, surface } = await renderOverlay();
    fireEvent.pointerDown(surface, { button: 0, clientX: 100, clientY: 50, pointerId: 1 });
    fireEvent.pointerMove(surface, { clientX: 300, clientY: 150, pointerId: 1 });
    expect(screen.getByTestId("selection")).toHaveTextContent("200 × 100");
    fireEvent.pointerUp(surface, { clientX: 300, clientY: 150, pointerId: 1 });
    expect(api.finish).toHaveBeenCalledWith({
      x: 100,
      y: 50,
      width: 200,
      height: 100,
      viewportWidth: window.innerWidth,
      viewportHeight: window.innerHeight,
    });
  });

  it("un clic sin arrastre no captura", async () => {
    const { api, surface } = await renderOverlay();
    drag(surface, [100, 100], [102, 101]);
    expect(api.finish).not.toHaveBeenCalled();
    expect(screen.queryByTestId("selection")).not.toBeInTheDocument();
  });

  it("Enter captura el monitor completo y Esc cancela, una sola vez", async () => {
    const { api } = await renderOverlay();
    fireEvent.keyDown(window, { key: "Enter" });
    fireEvent.keyDown(window, { key: "Enter" });
    expect(api.finish).toHaveBeenCalledTimes(1);
    expect(api.finish).toHaveBeenCalledWith(null);

    const other = fakeApi();
    render(<OverlayApp api={other} />);
    fireEvent.keyDown(window, { key: "Escape" });
    expect(other.cancel).toHaveBeenCalledTimes(1);
  });

  it("si Rust rechaza la selección muestra el motivo y permite reintentar", async () => {
    const finish = vi
      .fn<OverlayApi["finish"]>()
      .mockRejectedValueOnce(new AppError("validation", "La selección es demasiado pequeña."))
      .mockResolvedValue(undefined);
    const { surface } = await renderOverlay(fakeApi({ finish }));
    drag(surface, [10, 10], [20, 20]);
    expect(await screen.findByRole("status")).toHaveTextContent("La selección es demasiado pequeña.");
    expect(screen.queryByTestId("selection")).not.toBeInTheDocument();
    await act(async () => {
      fireEvent.keyDown(window, { key: "Enter" });
    });
    expect(finish).toHaveBeenCalledTimes(2);
  });

  it("si no puede leer su imagen cancela la captura", async () => {
    const api = fakeApi({ loadFrame: vi.fn(async () => Promise.reject(new Error("sin sesión"))) });
    render(<OverlayApp api={api} />);
    await waitFor(() => expect(api.cancel).toHaveBeenCalledTimes(1));
  });
});
```

Run: `pnpm vitest run src/ui/overlay`. Expected: FAIL (no existe `./OverlayApp`).

- [ ] **Step 5: Implementar `OverlayApp`**

Crea `src/ui/overlay/OverlayApp.tsx`:

```tsx
import { useEffect, useRef, useState, type PointerEvent } from "react";
import { isUsable, rectFrom, sizeLabel, toSelection, type Point, type Rect } from "../../domain/capture";
import type { OverlayApi } from "../../ports/capture";
import { AppError } from "../../ports/errors";

const HINT = "Arrastra para seleccionar · Enter: pantalla completa · Esc: cancelar";
const DIM = "rgba(0, 0, 0, 0.45)";

const viewport = () => ({ width: window.innerWidth, height: window.innerHeight });

/** Una ventana por monitor: imagen congelada a pantalla completa, oscurecida, con la selección en claro. */
export function OverlayApp({ api }: { api: OverlayApi }) {
  const [url, setUrl] = useState<string | null>(null);
  const [start, setStart] = useState<Point | null>(null);
  const [rect, setRect] = useState<Rect | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const busy = useRef(false);

  useEffect(() => {
    let alive = true;
    let created: string | null = null;
    api
      .loadFrame()
      .then((bytes) => {
        if (!alive) return;
        created = URL.createObjectURL(new Blob([bytes as BlobPart], { type: "image/png" }));
        setUrl(created);
      })
      .catch(() => void api.cancel());
    return () => {
      alive = false;
      if (created) URL.revokeObjectURL(created);
    };
  }, [api]);

  const send = useRef(async (run: () => Promise<void>) => {
    if (busy.current) return;
    busy.current = true;
    try {
      await run();
    } catch (error) {
      // La sesión sigue abierta en Rust: se puede volver a seleccionar.
      busy.current = false;
      setRect(null);
      setMessage(error instanceof AppError ? error.message : "No se pudo completar la captura.");
    }
  }).current;

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        void send(() => api.cancel());
      } else if (e.key === "Enter") {
        e.preventDefault();
        void send(() => api.finish(null));
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [api, send]);

  const point = (e: PointerEvent): Point => ({ x: e.clientX, y: e.clientY });

  return (
    <div
      data-testid="overlay-surface"
      className="fixed inset-0 cursor-crosshair select-none overflow-hidden bg-black"
      onContextMenu={(e) => e.preventDefault()}
      onPointerDown={(e) => {
        if (e.button !== 0 || busy.current) return;
        try {
          // Sigue recibiendo el arrastre aunque el puntero salga de la ventana.
          e.currentTarget.setPointerCapture(e.pointerId);
        } catch {
          // jsdom no implementa la captura de puntero.
        }
        const p = point(e);
        setStart(p);
        setRect(rectFrom(p, p, viewport()));
        setMessage(null);
      }}
      onPointerMove={(e) => {
        if (start) setRect(rectFrom(start, point(e), viewport()));
      }}
      onPointerUp={(e) => {
        if (!start) return;
        const final = rectFrom(start, point(e), viewport());
        setStart(null);
        if (!isUsable(final)) {
          setRect(null);
          return;
        }
        setRect(final);
        void send(() => api.finish(toSelection(final, viewport())));
      }}
    >
      {url && (
        <img
          data-testid="frame"
          src={url}
          alt=""
          draggable={false}
          className="pointer-events-none absolute inset-0 h-full w-full"
          onLoad={() => void api.ready().catch(() => {})}
        />
      )}
      {rect ? (
        <div
          data-testid="selection"
          className="pointer-events-none absolute border-2 border-t4s-lime"
          style={{ left: rect.x, top: rect.y, width: rect.width, height: rect.height, boxShadow: `0 0 0 100000px ${DIM}` }}
        >
          <span
            className="absolute left-0 whitespace-nowrap rounded bg-t4s-charcoal px-2 py-0.5 text-xs text-white"
            style={{ top: rect.y >= 28 ? -28 : 4 }}
          >
            {sizeLabel(rect, window.devicePixelRatio || 1)}
          </span>
        </div>
      ) : (
        <div className="pointer-events-none absolute inset-0" style={{ background: DIM }} />
      )}
      <p
        role="status"
        className="pointer-events-none absolute left-1/2 top-4 -translate-x-1/2 rounded-full bg-t4s-charcoal/90 px-4 py-2 text-sm text-white shadow-lg"
      >
        {message ?? HINT}
      </p>
    </div>
  );
}
```

Run: `pnpm vitest run src/ui/overlay src/domain/capture.test.ts`. Expected: PASS (6 + 5 passed).

- [ ] **Step 6: Entrada `overlay.html` y build multipágina**

Crea `overlay.html` en la raíz (junto a `index.html`):

```html
<!doctype html>
<html lang="es">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Qapture · Captura</title>
  </head>

  <body>
    <div id="root"></div>
    <script type="module" src="/src/overlay-main.tsx"></script>
  </body>
</html>
```

Crea `src/overlay-main.tsx`:

```tsx
import "./ui/theme/tokens.css";
import ReactDOM from "react-dom/client";
import { tauriOverlayApi } from "./infrastructure/tauri/capture";
import { OverlayApp } from "./ui/overlay/OverlayApp";

// Sin StrictMode: el doble montaje pediría dos veces la imagen del monitor.
ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(<OverlayApp api={tauriOverlayApi()} />);
```

En `vite.config.ts`, agrega `import { resolve } from "node:path";` después de `import process from "node:process";`, y dentro del objeto que devuelve `defineConfig`, después de `clearScreen: false,`:

```ts
  build: {
    // Dos páginas: la app (index.html) y la ventana overlay de captura (overlay.html).
    rolldownOptions: {
      input: {
        main: resolve(import.meta.dirname, "index.html"),
        overlay: resolve(import.meta.dirname, "overlay.html"),
      },
    },
  },
```

Run: `pnpm build`. Expected: OK, y `dist/` contiene `index.html` y `overlay.html`.

- [ ] **Step 7: Verificar y commit**

```bash
pnpm typecheck && pnpm test && pnpm build
git add overlay.html vite.config.ts src/overlay-main.tsx src/domain/capture.ts src/domain/capture.test.ts src/ports/capture.ts src/infrastructure/tauri/capture.ts src/infrastructure/tauri/capture.test.ts src/ui/overlay
git commit -m "feat(f3): overlay de selección con imagen congelada, medidas y atajos

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6: Ventanas overlay, comandos de captura, evento y contrato

**Files:**
- Create: `src-tauri/src/capture/windows.rs`, `src-tauri/src/commands/capture.rs`, `src-tauri/capabilities/overlay.json`
- Modify: `src-tauri/src/capture/mod.rs`, `src-tauri/src/commands/mod.rs`, `src-tauri/src/commands/names.rs`, `src-tauri/capabilities/main.json`
- Modify: `src-tauri/src/state.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/dto.rs` (DTO + contrato)
- Modify: `src/infrastructure/tauri/__fixtures__/contracts.json` (regenerado), `src/infrastructure/tauri/dto.ts`, `src/infrastructure/tauri/contract.test.ts`, `src/ports/errors.ts`

**Interfaces:**
- Consumes: `CaptureController`, `CaptureHost`, `HIDE_DELAY` (Task 3); `XcapScreen`, `MonitorBounds` (Task 1); `Selection` (Task 2); `system::tray::show_main(app: &AppHandle)` (F1).
- Produces:
  - `capture::windows::{OVERLAY_PREFIX, CAPTURED_EVENT, overlay_label(index) -> String, overlay_index(label) -> Option<usize>, TauriHost<'a> { app: &'a AppHandle }}`.
  - Comandos: `start_capture {}` → `null` (solo `main`); `capture_frame {}` → bytes (`ArrayBuffer`), `overlay_ready {}` → `null`, `finish_capture { selection: Selection | null }` → `null`, `cancel_capture {}` → `null` (solo `overlay-*`).
  - Evento `evidence:captured` con `CapturedEventDto { tmp_path }` (JSON `{ "tmpPath" }`) hacia `main`.
  - `AppState::capture() -> &CaptureController`.
  - TS: `capturedEventSchema` (`z.strictObject({ tmpPath: z.string() })`); `appErrorSchema.code` y `AppErrorCode` aceptan `"capture"`.

- [ ] **Step 1: Tests que fallan — labels, ACL del overlay y contrato**

Crea `src-tauri/src/capture/windows.rs` solo con los tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlay_labels_round_trip_and_reject_other_windows() {
        assert_eq!(overlay_label(0), "overlay-0");
        assert_eq!(overlay_index("overlay-0"), Some(0));
        assert_eq!(overlay_index(&overlay_label(12)), Some(12));
        for label in ["main", "overlay-", "overlay-x", "overlay-01", "overlay--1", "xoverlay-1", "overlay-1 "] {
            assert_eq!(overlay_index(label), None, "{label}");
        }
    }
}
```

En `src-tauri/src/capture/mod.rs`, agrega `pub mod windows;` al final de la lista de módulos.

En `src-tauri/src/commands/names.rs`, agrega al final de `APP_COMMANDS` (antes del `];`):

```rust
    "start_capture",
    "capture_frame",
    "overlay_ready",
    "finish_capture",
    "cancel_capture",
```

y reemplaza `OVERLAY_COMMANDS` por:

```rust
pub const OVERLAY_COMMANDS: &[&str] = &[
    "capture_frame",
    "overlay_ready",
    "finish_capture",
    "cancel_capture",
];
```

Dentro de `mod tests` de `names.rs`, agrega:

```rust
    #[test]
    fn overlay_can_only_call_the_capture_commands() {
        let (windows, permissions) = capability("overlay.json");
        assert_eq!(windows, ["overlay-*"]);
        assert_eq!(permissions, allow(OVERLAY_COMMANDS));
    }
```

En `src-tauri/src/dto.rs`, dentro de `mod contract`: en `inputs_json()`, agrega después de la entrada `"evidenceUpdateMeta"` (pon una coma al final de esa línea):

```rust
            "selection": { "x": 10.5, "y": 20.0, "width": 300.0, "height": 200.0,
                           "viewportWidth": 1536.0, "viewportHeight": 864.0 }
```

En `sample()`, agrega después de la entrada `"stagedImage"`:

```rust
            "capturedEvent": CapturedEventDto { tmp_path: "C:\\datos\\evidence\\_staging\\stg-2.png".into() },
            "captureError": AppError::Capture("xcap: monitor desconectado".into()),
```

y en `inputs_deserialize_from_ts_shape`, agrega al final:

```rust
        serde_json::from_value::<crate::capture::crop::Selection>(i["selection"].clone()).unwrap();
```

Run: `cd src-tauri && cargo test`. Expected: FAIL (no compila: faltan `overlay_label`, `overlay_index` y `CapturedEventDto`).

- [ ] **Step 2: `CapturedEventDto` y `TauriHost`**

En `src-tauri/src/dto.rs`, agrega después de `StagedImageDto`:

```rust
/// Payload del evento `evidence:captured` (Rust → ventana principal).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapturedEventDto {
    pub tmp_path: String,
}
```

Agrega al inicio de `src-tauri/src/capture/windows.rs`, antes de los tests:

```rust
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder,
};

use super::{screen::MonitorBounds, session::CaptureHost};
use crate::{
    dto::CapturedEventDto,
    error::{AppError, AppResult},
    system::tray,
};

pub const OVERLAY_PREFIX: &str = "overlay-";
pub const CAPTURED_EVENT: &str = "evidence:captured";

pub fn overlay_label(index: usize) -> String {
    format!("{OVERLAY_PREFIX}{index}")
}

/// Índice del monitor de una ventana overlay; `None` para cualquier otra ventana.
pub fn overlay_index(label: &str) -> Option<usize> {
    let digits = label.strip_prefix(OVERLAY_PREFIX)?;
    let index: usize = digits.parse().ok()?;
    (overlay_label(index) == label).then_some(index)
}

fn window_error(error: tauri::Error) -> AppError {
    AppError::Capture(error.to_string())
}

pub struct TauriHost<'a> {
    pub app: &'a AppHandle,
}

impl CaptureHost for TauriHost<'_> {
    fn hide_main(&self) -> bool {
        let Some(main) = self.app.get_webview_window("main") else {
            return false;
        };
        let visible = main.is_visible().unwrap_or(false) && !main.is_minimized().unwrap_or(false);
        let _ = main.hide();
        visible
    }

    fn show_main(&self) {
        tray::show_main(self.app);
    }

    fn open_overlays(&self, monitors: &[MonitorBounds]) -> AppResult<()> {
        for (index, m) in monitors.iter().enumerate() {
            let window = WebviewWindowBuilder::new(
                self.app,
                overlay_label(index),
                WebviewUrl::App("overlay.html".into()),
            )
            .title("Qapture · Captura")
            .decorations(false)
            .resizable(false)
            .shadow(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .focused(false)
            // Oculta hasta que pinta la imagen congelada (`overlay_ready`): sin parpadeo blanco.
            .visible(false)
            .build()
            .map_err(window_error)?;
            // Primero se mueve al monitor (la ventana adopta su DPI) y después se fija el tamaño físico.
            window
                .set_position(PhysicalPosition::new(m.x, m.y))
                .map_err(window_error)?;
            window
                .set_size(PhysicalSize::new(m.width, m.height))
                .map_err(window_error)?;
        }
        Ok(())
    }

    fn close_overlays(&self) {
        for (label, window) in self.app.webview_windows() {
            if overlay_index(&label).is_some() {
                let _ = window.destroy();
            }
        }
    }

    fn show_overlay(&self, index: usize, focus: bool) {
        if let Some(window) = self.app.get_webview_window(&overlay_label(index)) {
            let _ = window.show();
            if focus {
                let _ = window.set_focus();
            }
        }
    }

    fn emit_captured(&self, tmp_path: &str) {
        let payload = CapturedEventDto {
            tmp_path: tmp_path.into(),
        };
        let _ = self.app.emit_to("main", CAPTURED_EVENT, payload);
    }

    fn cursor(&self) -> Option<(f64, f64)> {
        self.app.cursor_position().ok().map(|p| (p.x, p.y))
    }
}
```

Quita de `src-tauri/src/capture/mod.rs` el comentario y la línea `#![allow(dead_code)]`; desde esta task todo se usa.

- [ ] **Step 3: Comandos, estado y cableado**

Crea `src-tauri/src/commands/capture.rs`:

```rust
use tauri::{AppHandle, State, Window, ipc::Response};

use crate::{
    capture::{
        crop::Selection,
        screen::XcapScreen,
        windows::{TauriHost, overlay_index},
    },
    error::{AppResult, validation},
    state::AppState,
};

/// El monitor sale del label de la ventana que llama, nunca de un argumento: un overlay solo
/// puede leer y recortar su propio monitor.
fn caller(window: &Window) -> AppResult<usize> {
    overlay_index(window.label())
        .ok_or_else(|| validation("Solo el overlay de captura puede usar este comando."))
}

/// UC-11. F6 lo invoca también desde el atajo global.
#[tauri::command(async)]
pub fn start_capture(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    state.capture().start(&TauriHost { app: &app }, &XcapScreen)
}

#[tauri::command(async)]
pub fn capture_frame(window: Window, state: State<'_, AppState>) -> AppResult<Response> {
    Ok(Response::new(state.capture().frame(caller(&window)?)?))
}

#[tauri::command(async)]
pub fn overlay_ready(app: AppHandle, window: Window, state: State<'_, AppState>) -> AppResult<()> {
    state
        .capture()
        .overlay_ready(&TauriHost { app: &app }, caller(&window)?)
}

#[tauri::command(async)]
pub fn finish_capture(
    app: AppHandle,
    window: Window,
    state: State<'_, AppState>,
    selection: Option<Selection>,
) -> AppResult<()> {
    let index = caller(&window)?;
    state
        .capture()
        .finish(&TauriHost { app: &app }, state.files(), index, selection)
        .map(|_| ())
}

#[tauri::command(async)]
pub fn cancel_capture(app: AppHandle, window: Window, state: State<'_, AppState>) -> AppResult<()> {
    caller(&window)?;
    state.capture().cancel(&TauriHost { app: &app });
    Ok(())
}
```

En `src-tauri/src/commands/mod.rs`, agrega `pub mod capture;` como primera línea.

En `src-tauri/src/state.rs`: agrega `use crate::capture::session::{CaptureController, HIDE_DELAY};`, el campo `capture: CaptureController,` en `AppState`, `capture: CaptureController::new(HIDE_DELAY),` en `new()` y el método:

```rust
    pub fn capture(&self) -> &CaptureController {
        &self.capture
    }
```

En `src-tauri/src/lib.rs`, reemplaza el bloque `.on_window_event(...)` completo por:

```rust
        .on_window_event(|window, event| {
            if capture::windows::overlay_index(window.label()).is_some() {
                // Un overlay cerrado por fuera (Alt+F4, WebView caído) cancela toda la captura.
                if let tauri::WindowEvent::Destroyed = event {
                    let app = window.app_handle();
                    app.state::<state::AppState>()
                        .capture()
                        .overlay_closed(&capture::windows::TauriHost { app });
                }
                return;
            }
            if window.label() != "main" {
                return;
            }
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
```

y agrega al final de `generate_handler![…]`, después de `commands::evidence::delete_evidence,`:

```rust
            commands::capture::start_capture,
            commands::capture::capture_frame,
            commands::capture::overlay_ready,
            commands::capture::finish_capture,
            commands::capture::cancel_capture,
```

En `src-tauri/capabilities/main.json`, agrega `"allow-start-capture"` al final de `permissions` (con la coma en la línea anterior).

Crea `src-tauri/capabilities/overlay.json`:

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "overlay",
  "description": "Overlay de captura: solo lee la imagen congelada de su monitor y devuelve la selección (spec §9).",
  "windows": ["overlay-*"],
  "permissions": ["allow-capture-frame", "allow-overlay-ready", "allow-finish-capture", "allow-cancel-capture"]
}
```

Regenera el contrato y corre todo:

```bash
UPDATE_CONTRACTS=1 cargo test contract
cargo test
```

Expected: PASS. `git diff ../src/infrastructure/tauri/__fixtures__/contracts.json` muestra solo `capturedEvent`, `captureError` e `inputs.selection`.

- [ ] **Step 4: Contrato del lado TS**

En `src/ports/errors.ts`:

```ts
export type AppErrorCode = "validation" | "not_found" | "db" | "io" | "capture" | "internal" | "contract";
```

En `src/infrastructure/tauri/dto.ts`, cambia el enum de `appErrorSchema` a `z.enum(["validation", "not_found", "db", "io", "capture", "internal"])` y agrega después de `stagedImageSchema`:

```ts
export const capturedEventSchema = z.strictObject({
  tmpPath: z.string(),
});
```

En `src/infrastructure/tauri/contract.test.ts`: importa `capturedEventSchema` junto a los demás esquemas y `toSelection` desde `"../../domain/capture"`. Agrega a la tabla del `it.each`:

```ts
    ["capturedEvent", capturedEventSchema],
    ["captureError", appErrorSchema],
```

y al final del test "los inputs de TS tienen exactamente las claves que Rust acepta":

```ts
    const selection = toSelection({ x: 0, y: 0, width: 1, height: 1 }, { width: 1, height: 1 });
    expect(sortedKeys(selection)).toEqual(sortedKeys(fixture.inputs.selection));
```

Run: `cd .. && pnpm vitest run src/infrastructure/tauri`. Expected: PASS.

- [ ] **Step 5: Smoke manual del flujo nativo**

Run: `pnpm tauri dev`. En la ventana principal abre DevTools (`Ctrl+Shift+I`) y ejecuta:

```js
window.__TAURI_INTERNALS__.invoke("start_capture")
```

Comprueba:
1. Qapture se oculta y aparece el overlay oscurecido con la pista en cada monitor.
2. `Esc` cierra los overlays y vuelve Qapture.
3. Repite y arrastra una región: los overlays se cierran, vuelve Qapture y aparece en `%APPDATA%\com.teams4soft.qapture\evidence\_staging\` un PNG nuevo del tamaño de la región. El editor todavía no se abre: eso llega en la Task 8.
4. Repite y pulsa `Alt+F4` sobre el overlay: se cierran todos y vuelve Qapture.
5. Desde la principal, `window.__TAURI_INTERNALS__.invoke("capture_frame")` falla con un error de ACL (`not allowed`).

- [ ] **Step 6: Verificar y commit**

```bash
cd src-tauri && cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test && cd ..
pnpm typecheck && pnpm test && pnpm build
git add src-tauri src/infrastructure/tauri src/ports/errors.ts
git commit -m "feat(f3): ventanas overlay por monitor, comandos de captura y evento evidence:captured

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 7: Puerto `ScreenCapture` con adaptadores Tauri y en memoria

**Files:**
- Modify: `src/ports/capture.ts`, `src/ports/repositories.ts`
- Modify: `src/infrastructure/tauri/capture.ts`, `src/infrastructure/tauri/capture.test.ts`, `src/infrastructure/tauri/repositories.ts`
- Create: `src/infrastructure/memory/capture.ts`, `src/infrastructure/memory/capture.test.ts`
- Modify: `src/infrastructure/memory/evidence.ts`, `src/infrastructure/memory/repositories.ts`

**Interfaces:**
- Consumes: `capturedEventSchema` (Task 6), `EvidenceStore.stage` (F2).
- Produces:
  - `ports/capture.ts`: `interface ScreenCapture { start(): Promise<void>; onCaptured(handler: (stagedPath: string) => void): Promise<() => void> }`.
  - `Repositories.capture: ScreenCapture`.
  - `infrastructure/tauri/capture.ts`: `CAPTURED_EVENT = "evidence:captured"` y `tauriScreenCapture(): ScreenCapture`.
  - `infrastructure/memory/capture.ts`: `createMemoryCapture(evidence: EvidenceStore): MemoryCapture`, donde `MemoryCapture` extiende `ScreenCapture` con `mode: "image" | "cancel" | "error"`, `starts: number`, `emit(stagedPath: string): void` y `listeners(): number`.
  - `MemoryEvidence.stagedCount(): number`. `createMemoryRepositories(): MemoryRepositories`, donde `MemoryRepositories` extiende `Repositories` con `evidence: MemoryEvidence` y `capture: MemoryCapture`.

- [ ] **Step 1: Tests que fallan**

Agrega a `src/infrastructure/tauri/capture.test.ts`, después de la línea `vi.mock("@tauri-apps/api/core", …)`:

```ts
const listen = vi.fn();
vi.mock("@tauri-apps/api/event", () => ({ listen: (...args: unknown[]) => listen(...args) }));
```

cambia la importación a `const { CAPTURED_EVENT, tauriOverlayApi, tauriScreenCapture } = await import("./capture");` y agrega al final:

```ts
describe("adaptador Tauri de la captura", () => {
  beforeEach(() => {
    invoke.mockReset();
    listen.mockReset();
  });

  it("start invoca start_capture", async () => {
    invoke.mockResolvedValue(null);
    await tauriScreenCapture().start();
    expect(invoke).toHaveBeenCalledWith("start_capture", {});
  });

  it("onCaptured entrega la ruta e ignora payloads que no cumplen el contrato", async () => {
    const off = vi.fn();
    listen.mockImplementation(async (_event: string, callback: (e: { payload: unknown }) => void) => {
      callback({ payload: { tmpPath: "C:\\datos\\evidence\\_staging\\stg-2.png" } });
      callback({ payload: { path: "x" } });
      return off;
    });
    const handler = vi.fn();
    const unlisten = await tauriScreenCapture().onCaptured(handler);
    expect(listen).toHaveBeenCalledWith(CAPTURED_EVENT, expect.any(Function));
    expect(handler.mock.calls).toEqual([["C:\\datos\\evidence\\_staging\\stg-2.png"]]);
    unlisten();
    expect(off).toHaveBeenCalled();
  });
});
```

Crea `src/infrastructure/memory/capture.test.ts`:

```ts
import { describe, expect, it, vi } from "vitest";
import { createMemoryRepositories } from "./repositories";

describe("captura en memoria", () => {
  it("en modo image deja la captura en staging y emite el evento", async () => {
    const repos = createMemoryRepositories();
    const handler = vi.fn();
    const off = await repos.capture.onCaptured(handler);
    await repos.capture.start();
    expect(repos.capture.starts).toBe(1);
    expect(repos.evidence.stagedCount()).toBe(1);
    const [path] = handler.mock.calls[0];
    await expect(repos.evidence.readImage(path)).resolves.toBeInstanceOf(Uint8Array);
    off();
    expect(repos.capture.listeners()).toBe(0);
  });

  it("cancel no crea nada y error falla con el código capture", async () => {
    const repos = createMemoryRepositories();
    const handler = vi.fn();
    await repos.capture.onCaptured(handler);
    repos.capture.mode = "cancel";
    await repos.capture.start();
    repos.capture.mode = "error";
    await expect(repos.capture.start()).rejects.toMatchObject({
      code: "capture",
      message: "No se pudo capturar la pantalla. Inténtalo nuevamente.",
    });
    expect(handler).not.toHaveBeenCalled();
    expect(repos.evidence.stagedCount()).toBe(0);
  });
});
```

Run: `pnpm vitest run src/infrastructure`. Expected: FAIL (`tauriScreenCapture` no existe; `repos.capture` es undefined).

- [ ] **Step 2: Puerto**

Agrega a `src/ports/capture.ts`:

```ts
/** Captura integrada (UC-11). F6 la dispara también desde el atajo global. */
export interface ScreenCapture {
  /** Oculta Qapture y abre el overlay; termina cuando el overlay está abierto, no cuando el usuario elige. */
  start(): Promise<void>;
  /** Suscribe al evento `evidence:captured`; devuelve la función para desuscribirse. */
  onCaptured(handler: (stagedPath: string) => void): Promise<() => void>;
}
```

En `src/ports/repositories.ts`, agrega `import type { ScreenCapture } from "./capture";` y el campo `capture: ScreenCapture;` al final de `Repositories`.

- [ ] **Step 3: Adaptador Tauri**

En `src/infrastructure/tauri/capture.ts`: agrega `import { listen } from "@tauri-apps/api/event";` como primera línea, cambia la importación de tipos a `import type { OverlayApi, ScreenCapture } from "../../ports/capture";`, agrega `import { capturedEventSchema } from "./dto";` y, después de `const nothing = z.null();`:

```ts
export const CAPTURED_EVENT = "evidence:captured";

export function tauriScreenCapture(): ScreenCapture {
  return {
    start: async () => {
      await call("start_capture", {}, nothing);
    },
    onCaptured: (handler) =>
      listen<unknown>(CAPTURED_EVENT, (event) => {
        const parsed = capturedEventSchema.safeParse(event.payload);
        if (parsed.success) handler(parsed.data.tmpPath);
      }),
  };
}
```

En `src/infrastructure/tauri/repositories.ts`, agrega `import { tauriScreenCapture } from "./capture";` y, después del bloque `evidence: { … },`:

```ts
    capture: tauriScreenCapture(),
```

- [ ] **Step 4: Adaptador en memoria**

En `src/infrastructure/memory/evidence.ts`, agrega `stagedCount(): number;` a `interface MemoryEvidence` y, dentro del objeto devuelto, después de `async discardStaged(path) { … },`:

```ts
    stagedCount: () => staged.size,
```

Crea `src/infrastructure/memory/capture.ts`:

```ts
import type { ScreenCapture } from "../../ports/capture";
import { AppError } from "../../ports/errors";
import type { EvidenceStore } from "../../ports/repositories";

export type MemoryCaptureMode = "image" | "cancel" | "error";

const PNG = new Uint8Array([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);

export interface MemoryCapture extends ScreenCapture {
  /** image: el usuario elige una región · cancel: pulsa Esc · error: falla xcap. */
  mode: MemoryCaptureMode;
  starts: number;
  /** Simula el evento `evidence:captured` (por ejemplo, desde el atajo global de F6). */
  emit(stagedPath: string): void;
  listeners(): number;
}

/** Replica el ciclo de `start_capture` para tests de aplicación y de UI. */
export function createMemoryCapture(evidence: EvidenceStore): MemoryCapture {
  const handlers = new Set<(stagedPath: string) => void>();
  const capture: MemoryCapture = {
    mode: "image",
    starts: 0,
    async start() {
      capture.starts += 1;
      if (capture.mode === "error") throw new AppError("capture", "No se pudo capturar la pantalla. Inténtalo nuevamente.");
      if (capture.mode === "cancel") return;
      const staged = await evidence.stage(PNG);
      capture.emit(staged.path);
    },
    async onCaptured(handler) {
      handlers.add(handler);
      return () => void handlers.delete(handler);
    },
    emit(stagedPath) {
      handlers.forEach((handler) => handler(stagedPath));
    },
    listeners: () => handlers.size,
  };
  return capture;
}
```

En `src/infrastructure/memory/repositories.ts`:
- Cambia `import { createMemoryEvidence } from "./evidence";` por `import { createMemoryEvidence, type MemoryEvidence } from "./evidence";` y agrega `import { createMemoryCapture, type MemoryCapture } from "./capture";`.
- Antes de la función, agrega:

```ts
export interface MemoryRepositories extends Repositories {
  evidence: MemoryEvidence;
  capture: MemoryCapture;
}
```

- Cambia la firma a `export function createMemoryRepositories(): MemoryRepositories {`.
- En el `return` final, reemplaza `    evidence,` por:

```ts
    evidence,
    capture: createMemoryCapture(evidence),
```

Run: `pnpm vitest run src/infrastructure`. Expected: PASS.

- [ ] **Step 5: Verificar y commit**

```bash
pnpm typecheck && pnpm test && pnpm build
git add src/ports src/infrastructure
git commit -m "feat(f3): puerto ScreenCapture con adaptadores Tauri y en memoria

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 8: Botón «Capturar», recepción del evento y editor de F2

**Files:**
- Modify: `src/ui/store/evidence-slice.ts`, `src/ui/store/evidence-slice.test.ts`
- Modify: `src/ui/screens/AppShell.tsx`, `src/ui/gallery/EvidenceGallery.tsx`, `src/ui/screens/Evidence.test.tsx`

**Interfaces:**
- Consumes: `Repositories.capture` (Task 7); `openEvidenceEditor`, `cancelEditor`, `pushToast` (F2).
- Produces (en `EvidenceSlice`):
  - `startCapture(): Promise<void>`: errores → toast.
  - `receiveCapture(stagedPath: string): Promise<void>`: sin casos → descarta el temporal y avisa `Crea un caso antes de capturar evidencias.`; si no, `openEvidenceEditor({ imagePath: stagedPath })` (caso activo preseleccionado; si el editor ya está abierto, queda en la cola).
  - `listenCaptures(): Promise<() => void>`.

- [ ] **Step 1: Tests que fallan — slice**

Agrega al final de `src/ui/store/evidence-slice.test.ts` (dentro del archivo, después del último `describe`):

```ts
describe("captura integrada", () => {
  it("la captura abre el editor con el caso activo y se guarda", async () => {
    const { store, repos, c1, save } = await setup();
    await store.getState().listenCaptures();
    await store.getState().startCapture();
    expect(repos.capture.starts).toBe(1);
    expect(store.getState().editor).toMatchObject({ kind: "new", caseId: c1.id });
    expect(await save()).toEqual({ ok: true });
    expect(store.getState().evidence).toHaveLength(1);
  });

  it("una captura con el editor abierto queda en la cola", async () => {
    const { store } = await setup();
    await store.getState().listenCaptures();
    await store.getState().intakeImages([PNG]);
    await store.getState().startCapture();
    expect(store.getState().editorQueue).toHaveLength(1);
  });

  it("cancelar el editor de una captura no deja temporales", async () => {
    const { store, repos } = await setup();
    await store.getState().listenCaptures();
    await store.getState().startCapture();
    expect(repos.evidence.stagedCount()).toBe(1);
    await store.getState().cancelEditor();
    expect(store.getState().editor).toBeNull();
    expect(repos.evidence.stagedCount()).toBe(0);
  });

  it("sin casos descarta la captura y avisa", async () => {
    const repos = createMemoryRepositories();
    const p = await seedProject(repos);
    const store = createWorkspaceStore(repos, memoryPreferences(p.id));
    await store.getState().init();
    await store.getState().listenCaptures();
    await store.getState().startCapture();
    await vi.waitFor(() => expect(repos.evidence.stagedCount()).toBe(0));
    expect(store.getState().editor).toBeNull();
    expect(store.getState().toasts.at(-1)?.message).toBe("Crea un caso antes de capturar evidencias.");
  });

  it("si la captura falla avisa, y si se cancela en el overlay no pasa nada", async () => {
    const { store, repos } = await setup();
    await store.getState().listenCaptures();
    repos.capture.mode = "error";
    await store.getState().startCapture();
    expect(store.getState().toasts.at(-1)?.message).toBe("No se pudo capturar la pantalla. Inténtalo nuevamente.");
    repos.capture.mode = "cancel";
    const toasts = store.getState().toasts.length;
    await store.getState().startCapture();
    expect(store.getState().editor).toBeNull();
    expect(store.getState().toasts).toHaveLength(toasts);
  });

  it("desuscribirse deja de recibir capturas", async () => {
    const { store, repos } = await setup();
    const off = await store.getState().listenCaptures();
    off();
    expect(repos.capture.listeners()).toBe(0);
  });
});
```

Run: `pnpm vitest run src/ui/store/evidence-slice.test.ts`. Expected: FAIL (`listenCaptures is not a function`).

- [ ] **Step 2: Implementar en el slice**

En `src/ui/store/evidence-slice.ts`, agrega a `interface EvidenceSlice`, después de `deleteEvidence(evidenceId: string): Promise<void>;`:

```ts
  startCapture(): Promise<void>;
  receiveCapture(stagedPath: string): Promise<void>;
  listenCaptures(): Promise<() => void>;
```

y en el objeto devuelto, después de `imageUrl: (path, version) => repos.evidence.imageUrl(path, version),`:

```ts
    async startCapture() {
      try {
        await repos.capture.start();
      } catch (error) {
        h.report(error);
      }
    },

    async receiveCapture(stagedPath) {
      if (get().cases.length === 0) {
        get().pushToast("error", "Crea un caso antes de capturar evidencias.");
        try {
          await repos.evidence.discardStaged(stagedPath);
        } catch {
          // Si falla, el próximo arranque vacía evidence/_staging.
        }
        return;
      }
      // El caso activo queda preseleccionado; si el editor está abierto, la captura espera en la cola.
      get().openEvidenceEditor({ imagePath: stagedPath });
    },

    listenCaptures: () => repos.capture.onCaptured((stagedPath) => void get().receiveCapture(stagedPath)),
```

Run: `pnpm vitest run src/ui/store/evidence-slice.test.ts`. Expected: PASS.

- [ ] **Step 3: Tests que fallan — UI**

Agrega al final de `src/ui/screens/Evidence.test.tsx`:

```tsx
describe("captura integrada", () => {
  it("«Capturar» abre el editor con la captura y el caso activo", async () => {
    const { user, repos } = await renderApp({ setup: async (r) => void (await seedCases(r)) });
    await user.click(screen.getByRole("button", { name: "Capturar" }));
    const dialog = await editor();
    expect(within(dialog).getByLabelText("Caso")).toHaveDisplayValue("EJ-001 · Recibos");
    expect(repos.capture.starts).toBe(1);
  });

  it("si la captura falla muestra el motivo", async () => {
    const { user, repos } = await renderApp({ setup: async (r) => void (await seedCases(r)) });
    repos.capture.mode = "error";
    await user.click(screen.getByRole("button", { name: "Capturar" }));
    expect(await screen.findByText("No se pudo capturar la pantalla. Inténtalo nuevamente.")).toBeInTheDocument();
    expect(screen.queryByRole("dialog", { name: "Editor de evidencia" })).not.toBeInTheDocument();
  });

  it("una captura que llega sin proyecto abierto se descarta y avisa", async () => {
    const { repos } = await renderApp();
    const staged = await repos.evidence.stage(new Uint8Array(PNG_BYTES));
    act(() => repos.capture.emit(staged.path));
    expect(await screen.findByText("Crea un caso antes de capturar evidencias.")).toBeInTheDocument();
    await waitFor(() => expect(repos.evidence.stagedCount()).toBe(0));
  });
});
```

Run: `pnpm vitest run src/ui/screens/Evidence.test.tsx`. Expected: FAIL (no hay botón «Capturar» ni suscripción).

- [ ] **Step 4: Suscripción y botón**

En `src/ui/screens/AppShell.tsx`, agrega después de `const [dialog, setDialog] = …`:

```tsx
  const listenCaptures = useWorkspace((s) => s.listenCaptures);
```

y después del `useEffect` de `init`:

```tsx
  useEffect(() => {
    // Siempre montado: la captura puede llegar sin proyecto abierto (F6 la dispara con el atajo global).
    // En StrictMode el efecto corre dos veces; si la suscripción llega tarde, se cancela al llegar.
    let active = true;
    let unlisten: (() => void) | undefined;
    void listenCaptures().then((off) => {
      if (active) unlisten = off;
      else off();
    });
    return () => {
      active = false;
      unlisten?.();
    };
  }, [listenCaptures]);
```

En `src/ui/gallery/EvidenceGallery.tsx`:
- Cambia `import { Upload } from "lucide-react";` por `import { Camera, Upload } from "lucide-react";`.
- Agrega `const startCapture = useWorkspace((s) => s.startCapture);` después de `const deleteEvidence = …`.
- Reemplaza el botón «Elegir imagen»:

```tsx
        <button type="button" className={btn.secondary} onClick={() => picker.current?.click()}>
          <Upload size={16} /> Elegir imagen
        </button>
```

por:

```tsx
        <div className="flex gap-2">
          <button type="button" className={btn.secondary} onClick={() => void startCapture()}>
            <Camera size={16} /> Capturar
          </button>
          <button type="button" className={btn.secondary} onClick={() => picker.current?.click()}>
            <Upload size={16} /> Elegir imagen
          </button>
        </div>
```

- En el párrafo del estado vacío, reemplaza `Pega una captura con` por `Usa «Capturar», pega una captura con`.

Run: `pnpm vitest run src/ui`. Expected: PASS.

- [ ] **Step 5: Smoke manual del flujo completo**

Run: `pnpm tauri dev`. Con un proyecto y un caso seleccionado:
1. «Capturar» → Qapture se oculta y aparece el overlay; al arrastrar una región se abre el editor con la captura y el caso activo preseleccionado. Guarda con descripción: aparece la miniatura y el contador sube.
2. «Capturar» → `Enter`: el editor recibe el monitor completo bajo el puntero.
3. «Capturar» → `Esc`: vuelve Qapture sin editor ni archivos nuevos en `evidence\_staging\`.
4. Con el editor abierto, pega una imagen (`Ctrl+V`) y luego captura: la captura queda en cola ("1 pendiente").

- [ ] **Step 6: Verificar y commit**

```bash
pnpm typecheck && pnpm test && pnpm build
git add src/ui
git commit -m "feat(f3): botón Capturar y apertura del editor con la captura

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 9: Verificación integral, versión `0.3.0` y cierre del contrato

**Files:**
- Modify: `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json` (versión `0.3.0`), `src-tauri/Cargo.lock`
- Modify: `docs/superpowers/contracts/F3-captura-integrada.md`, `docs/superpowers/contracts/README.md`

- [ ] **Step 1: Versión**

Cambia `"version": "0.2.0"` → `"0.3.0"` en `package.json` y `src-tauri/tauri.conf.json`, y `version = "0.2.0"` → `"0.3.0"` en `src-tauri/Cargo.toml`. Luego `cd src-tauri && cargo check` para actualizar `Cargo.lock`.

- [ ] **Step 2: Verificación completa**

```bash
cd src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test && cd ..
pnpm typecheck && pnpm test && pnpm build
pnpm tauri build
```

Expected: todo en verde. Instalador en `src-tauri/target/release/bundle/nsis/Qapture_0.3.0_x64-setup.exe`. Instálalo y repite el smoke de la Task 8 con el build de release: la CSP de producción debe permitir la imagen `blob:` del overlay, y la ACL de producción, los comandos.

- [ ] **Step 3: Criterios de aceptación (automáticos)**

En `docs/superpowers/contracts/F3-captura-integrada.md`, marca los dos criterios con los tests que los cubren:
- Recorte con escalas 1.0 / 1.25 / 1.5 y offset negativo → `capture::crop::tests::scales_css_pixels_to_physical_pixels_at_100_125_and_150_percent`, `capture::session::tests::finish_crops_the_selected_monitor_even_with_a_negative_offset` y `capture::screen::tests::finds_the_monitor_under_a_point_with_negative_offsets`.
- No quedan temporales después de cancelar → `capture::session::tests::cancel_leaves_no_temporary_files_and_restores_visibility` y `src/ui/store/evidence-slice.test.ts` («cancelar el editor de una captura no deja temporales»).

En la tabla **Tasks** del contrato agrega una columna `Estado` con `✅` en F3-T1…F3-T6. En `README.md`, cambia el estado de F3 a `🧪 En pruebas del equipo`.

- [ ] **Step 4: Commit**

```bash
git add -A package.json src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/tauri.conf.json docs
git commit -m "chore(f3): versión 0.3.0 y cierre técnico del contrato F3

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

- [ ] **Step 5: Entrega al equipo**

Pide a la persona usuaria que ejecute el **Checklist de pruebas locales** del contrato F3 (pruebas 1–5) con `Qapture_0.3.0_x64-setup.exe`, idealmente en un equipo con dos monitores a distinta escala (por ejemplo, el portátil al 150 % y un monitor externo al 100 % a la izquierda). Si la prueba 5 (video o app a pantalla completa) sale en negro, se evalúa activar la feature `wgc` de xcap (Windows Graphics Capture) en una corrección. La fase se cierra cuando el checklist está firmado. Push y PR, solo cuando lo pida.

---

## Probar un build

```bash
git pull && pnpm install
pnpm tauri build
# src-tauri/target/release/bundle/nsis/Qapture_0.3.0_x64-setup.exe
```

Datos de prueba: `%APPDATA%\com.teams4soft.qapture\` (`qapture.db`, `evidence\`, `backups\`). Las capturas sin guardar quedan en `evidence\_staging\` hasta que se guardan, se cancelan o se reinicia la app.

## Ejecución

Recomendado: **subagent-driven**. Las Tasks 1–3 (Rust puro) y la 5 (overlay TS) son independientes entre sí, pero la 6 une las cuatro y depende de nombres exactos (labels, comandos, payload del evento, ACL). Un error ahí solo se ve en el smoke manual, así que conviene un revisor por task. Las Tasks 1–3 se pueden delegar a Codex (`/codex:rescue`) indicando la task y este plan; Claude revisa cada diff contra su sección.
