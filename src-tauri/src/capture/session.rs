use std::{
    sync::{Condvar, Mutex, MutexGuard},
    thread,
    time::Duration,
};

use super::{
    crop::{self, Selection},
    screen::{CapturedMonitor, MonitorBounds, Screen, monitor_at, preview_png},
};
use crate::{
    dto::StagedImageDto,
    error::{AppError, AppResult, validation},
    media::storage::EvidenceFiles,
    repo::evidence,
};

/// Tiempo para que Windows termine de ocultar Qapture antes de capturar.
pub const HIDE_DELAY: Duration = Duration::from_millis(200);
/// Tiempo máximo para que algún overlay pinte su imagen; si no, la captura se aborta.
pub const READY_TIMEOUT: Duration = Duration::from_secs(5);

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
    /// Distingue esta sesión de una posterior: quien la empezó o recortó solo termina la suya.
    generation: u64,
    monitors: Vec<CapturedMonitor>,
    main_was_visible: bool,
    /// Algún overlay ya pintó su imagen.
    ready: bool,
}

enum Phase {
    Idle,
    Selecting(Session),
}

struct State {
    phase: Phase,
    /// Una llamada a `start` sigue en curso (capturando, creando overlays o esperándolos).
    starting: bool,
    generation: u64,
}

/// Ciclo de una captura. Las imágenes congeladas viven solo en memoria mientras los overlays
/// están abiertos. El candado nunca se mantiene mientras se llama al host, porque cerrar un
/// overlay dispara `overlay_closed` desde el hilo de eventos.
pub struct CaptureController {
    state: Mutex<State>,
    /// Avisa a `start` cuando un overlay está listo o la sesión terminó.
    changed: Condvar,
    hide_delay: Duration,
    ready_timeout: Option<Duration>,
}

impl CaptureController {
    /// `ready_timeout: None` no espera a los overlays (tests de un solo hilo).
    pub fn new(hide_delay: Duration, ready_timeout: Option<Duration>) -> Self {
        Self {
            state: Mutex::new(State {
                phase: Phase::Idle,
                starting: false,
                generation: 0,
            }),
            changed: Condvar::new(),
            hide_delay,
            ready_timeout,
        }
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Termina la sesión abierta; con `generation`, solo si es esa.
    fn take_session(&self, generation: Option<u64>) -> Option<Session> {
        let mut state = self.state();
        let matches = match &state.phase {
            Phase::Selecting(session) => generation.is_none_or(|g| g == session.generation),
            Phase::Idle => false,
        };
        if !matches {
            return None;
        }
        let Phase::Selecting(session) = std::mem::replace(&mut state.phase, Phase::Idle) else {
            unreachable!("comprobado arriba");
        };
        self.changed.notify_all();
        Some(session)
    }

    fn is_current(&self, generation: u64) -> bool {
        matches!(&self.state().phase, Phase::Selecting(s) if s.generation == generation)
    }

    fn bounds(&self) -> AppResult<Vec<MonitorBounds>> {
        match &self.state().phase {
            Phase::Selecting(session) => Ok(session.monitors.iter().map(|m| m.bounds).collect()),
            Phase::Idle => Err(validation(NO_SESSION)),
        }
    }

    /// Vuelve cuando algún overlay pintó su imagen, o con error si ninguno lo hace a tiempo
    /// (overlay.html no cargó, la imagen no se decodificó, el WebView murió…).
    pub fn start(&self, host: &impl CaptureHost, screen: &impl Screen) -> AppResult<()> {
        {
            let mut state = self.state();
            if state.starting || !matches!(state.phase, Phase::Idle) {
                return Err(validation(BUSY));
            }
            state.starting = true;
        }
        let result = self.run_start(host, screen);
        self.state().starting = false;
        result
    }

    fn run_start(&self, host: &impl CaptureHost, screen: &impl Screen) -> AppResult<()> {
        let main_was_visible = host.hide_main();
        if main_was_visible {
            thread::sleep(self.hide_delay);
        }
        let monitors = match screen.capture_all() {
            Ok(monitors) => monitors,
            Err(error) => {
                // Siempre se muestra: si no, el mensaje de error no tendría dónde verse.
                host.show_main();
                return Err(error);
            }
        };
        let bounds: Vec<MonitorBounds> = monitors.iter().map(|m| m.bounds).collect();
        let generation = {
            let mut state = self.state();
            state.generation += 1;
            state.phase = Phase::Selecting(Session {
                generation: state.generation,
                monitors,
                main_was_visible,
                ready: false,
            });
            state.generation
        };
        if let Err(error) = host.open_overlays(&bounds) {
            self.take_session(Some(generation));
            host.close_overlays();
            host.show_main();
            return Err(error);
        }
        if !self.is_current(generation) {
            // La sesión terminó (cancelar, Alt+F4) mientras se creaban los overlays: quien la
            // terminó ya restauró Qapture, pero los overlays creados después quedaron huérfanos.
            host.close_overlays();
            return Ok(());
        }
        let Some(timeout) = self.ready_timeout else {
            return Ok(());
        };
        if self.wait_until_ready(generation, timeout) {
            return Ok(());
        }
        if self.take_session(Some(generation)).is_some() {
            host.close_overlays();
            // Siempre se muestra: el error tiene que verse aunque Qapture estuviera oculto.
            host.show_main();
            return Err(AppError::Capture("el overlay no respondió".into()));
        }
        Ok(())
    }

    /// `true` si un overlay quedó listo o la sesión ya terminó por otro camino.
    fn wait_until_ready(&self, generation: u64, timeout: Duration) -> bool {
        let pending = |state: &mut State| matches!(&state.phase, Phase::Selecting(s) if s.generation == generation && !s.ready);
        // La espera suelta el candado: `overlay_ready` y `cancel` pueden entrar.
        let (mut state, _) = self
            .changed
            .wait_timeout_while(self.state(), timeout, pending)
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        !pending(&mut state)
    }

    /// Imagen congelada del monitor `index`, para el overlay que lo cubre.
    pub fn frame(&self, index: usize) -> AppResult<Vec<u8>> {
        let state = self.state();
        let Phase::Selecting(session) = &state.phase else {
            return Err(validation(NO_SESSION));
        };
        let monitor = session
            .monitors
            .get(index)
            .ok_or_else(|| validation(NO_SESSION))?;
        preview_png(&monitor.image)
    }

    /// El overlay ya pintó su imagen: se muestra, y toma el foco si el puntero está en su monitor.
    pub fn overlay_ready(&self, host: &impl CaptureHost, index: usize) -> AppResult<()> {
        let bounds: Vec<MonitorBounds> = {
            let mut state = self.state();
            let Phase::Selecting(session) = &mut state.phase else {
                return Err(validation(NO_SESSION));
            };
            if index >= session.monitors.len() {
                return Err(validation(NO_SESSION));
            }
            session.ready = true;
            self.changed.notify_all();
            session.monitors.iter().map(|m| m.bounds).collect()
        };
        let under = host
            .cursor()
            .and_then(|(x, y)| monitor_at(&bounds, x, y))
            .unwrap_or(0);
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
        let (generation, png) = self.crop(host, index, selection)?;
        self.commit(host, files, generation, &png)
    }

    /// Recorta la sesión actual y devuelve su generación junto al PNG.
    fn crop(
        &self,
        host: &impl CaptureHost,
        index: usize,
        selection: Option<Selection>,
    ) -> AppResult<(u64, Vec<u8>)> {
        let bounds = self.bounds()?;
        let index = match selection {
            Some(_) => index,
            None => host
                .cursor()
                .and_then(|(x, y)| monitor_at(&bounds, x, y))
                .unwrap_or(index),
        };
        let state = self.state();
        let Phase::Selecting(session) = &state.phase else {
            return Err(validation(NO_SESSION));
        };
        let monitor = session
            .monitors
            .get(index)
            .ok_or_else(|| validation(NO_SESSION))?;
        let (width, height) = monitor.image.dimensions();
        let rect = match &selection {
            Some(s) => crop::to_pixels(s, width, height)?,
            None => crop::full(width, height),
        };
        Ok((session.generation, crop::crop_png(&monitor.image, rect)?))
    }

    /// Deja el recorte en `_staging` y cierra la sesión **de la que salió** el recorte.
    fn commit(
        &self,
        host: &impl CaptureHost,
        files: &EvidenceFiles,
        generation: u64,
        png: &[u8],
    ) -> AppResult<StagedImageDto> {
        let staged = evidence::stage(files, png)?;
        if self.take_session(Some(generation)).is_none() {
            // Se canceló (o empezó otra) mientras se guardaba: no debe quedar el temporal.
            let _ = files.discard_staged(&staged.path);
            return Err(validation(NO_SESSION));
        }
        host.close_overlays();
        host.show_main();
        host.emit_captured(&staged.path);
        Ok(staged)
    }

    /// Lo pide un overlay (Esc, imagen que no carga). Sin sesión, igual cierra los overlays:
    /// puede quedar alguno huérfano de una sesión que ya terminó.
    pub fn cancel(&self, host: &impl CaptureHost) {
        match self.take_session(None) {
            Some(session) => Self::end_cancelled(host, &session),
            None => host.close_overlays(),
        }
    }

    /// Un overlay se cerró sin pasar por `finish` o `cancel` (Alt+F4, WebView caído).
    /// Si la sesión ya terminó (los overlays se cierran al terminar), no hace nada.
    pub fn overlay_closed(&self, host: &impl CaptureHost) {
        if let Some(session) = self.take_session(None) {
            Self::end_cancelled(host, &session);
        }
    }

    fn end_cancelled(host: &impl CaptureHost, session: &Session) {
        host.close_overlays();
        if session.main_was_visible {
            host.show_main();
        }
    }
}

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
            Self {
                visible: Cell::new(true),
                cursor: Cell::new(None),
                fail_overlays: false,
                log: RefCell::new(vec![]),
            }
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
            if self.fail_overlays {
                Err(AppError::Capture("webview".into()))
            } else {
                Ok(())
            }
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
            let list = self
                .0
                .as_ref()
                .ok_or_else(|| AppError::Capture("xcap".into()))?;
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
        MonitorBounds {
            x,
            y,
            width,
            height,
        }
    }

    /// Izquierda (offset negativo) al 100 % y principal al 150 %, a escala reducida para que el test sea rápido.
    fn two_monitors() -> FakeScreen {
        FakeScreen(Some(vec![
            (bounds(-192, 0, 192, 108), RED),
            (bounds(0, 0, 288, 162), BLUE),
        ]))
    }

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

    fn controller() -> CaptureController {
        CaptureController::new(Duration::ZERO, None)
    }

    fn staged_files(files: &EvidenceFiles) -> usize {
        std::fs::read_dir(files.staging_dir())
            .map(|d| d.count())
            .unwrap_or(0)
    }

    fn pixels(path: &str) -> image::RgbImage {
        image::open(path).unwrap().to_rgb8()
    }

    #[test]
    fn start_hides_qapture_and_opens_one_overlay_per_monitor() {
        let (host, capture) = (FakeHost::new(), controller());
        capture.start(&host, &two_monitors()).unwrap();
        assert_eq!(host.log(), ["hide_main", "open_overlays:2"]);
        assert_eq!(
            crate::media::raster::dimensions(&capture.frame(0).unwrap()).unwrap(),
            (192, 108)
        );
        assert_eq!(
            crate::media::raster::dimensions(&capture.frame(1).unwrap()).unwrap(),
            (288, 162)
        );
        assert_eq!(
            capture.frame(2).unwrap_err().to_string(),
            "La captura ya terminó."
        );
    }

    #[test]
    fn a_second_start_is_rejected_until_the_session_ends() {
        let (host, capture) = (FakeHost::new(), controller());
        capture.start(&host, &two_monitors()).unwrap();
        assert_eq!(
            capture
                .start(&host, &two_monitors())
                .unwrap_err()
                .to_string(),
            "Ya hay una captura en curso."
        );
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
        let host = FakeHost {
            fail_overlays: true,
            ..FakeHost::new()
        };
        let capture = controller();
        assert!(capture.start(&host, &two_monitors()).is_err());
        assert_eq!(
            host.log(),
            [
                "hide_main",
                "open_overlays:2",
                "close_overlays",
                "show_main"
            ]
        );
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
        let staged = capture
            .finish(
                &host,
                &files,
                0,
                Some(sel(10.0, 10.0, 50.0, 20.0, 192.0, 108.0)),
            )
            .unwrap();
        let img = pixels(&staged.path);
        assert_eq!((img.width(), img.height()), (50, 20));
        assert!(img.pixels().all(|p| p.0 == RED));
        assert_eq!((staged.width, staged.height), (50, 20));
        assert_eq!(
            host.log(),
            [
                "close_overlays".to_string(),
                "show_main".into(),
                format!("emit:{}", staged.path)
            ]
        );
        assert!(capture.frame(0).is_err(), "la sesión terminó");

        // Monitor al 150 %: su ventana mide 192×108 px CSS para una imagen de 288×162.
        capture.start(&host, &two_monitors()).unwrap();
        let staged = capture
            .finish(
                &host,
                &files,
                1,
                Some(sel(10.0, 10.0, 20.0, 20.0, 192.0, 108.0)),
            )
            .unwrap();
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
        assert_eq!(
            (staged.width, staged.height),
            (192, 108),
            "el del puntero, no el que recibió la tecla"
        );

        capture.start(&host, &two_monitors()).unwrap();
        host.cursor.set(None);
        let staged = capture.finish(&host, &files, 1, None).unwrap();
        assert_eq!(
            (staged.width, staged.height),
            (288, 162),
            "sin puntero: el que recibió la tecla"
        );
    }

    #[test]
    fn a_large_capture_is_staged_with_its_longest_side_at_2400_px() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let (host, capture) = (FakeHost::new(), controller());
        capture
            .start(
                &host,
                &FakeScreen(Some(vec![(bounds(0, 0, 4800, 100), BLUE)])),
            )
            .unwrap();
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
        let error = capture
            .finish(
                &host,
                &files,
                0,
                Some(sel(10.0, 10.0, 0.0, 5.0, 192.0, 108.0)),
            )
            .unwrap_err();
        assert_eq!(error.to_string(), "La selección es demasiado pequeña.");
        assert!(host.log().is_empty(), "no cierra nada");
        assert_eq!(staged_files(&files), 0);
        assert!(
            capture
                .finish(
                    &host,
                    &files,
                    0,
                    Some(sel(0.0, 0.0, 10.0, 10.0, 192.0, 108.0))
                )
                .is_ok()
        );
    }

    #[test]
    fn cancel_leaves_no_temporary_files_and_restores_visibility() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let (host, capture) = (FakeHost::new(), controller());
        capture.start(&host, &two_monitors()).unwrap();
        capture.cancel(&host);
        assert_eq!(
            host.log(),
            [
                "hide_main",
                "open_overlays:2",
                "close_overlays",
                "show_main"
            ]
        );
        assert_eq!(staged_files(&files), 0);
        assert!(capture.frame(0).is_err());

        // Qapture oculto antes de capturar (atajo global de F6): al cancelar sigue oculto.
        host.visible.set(false);
        host.clear();
        capture.start(&host, &two_monitors()).unwrap();
        capture.cancel(&host);
        assert_eq!(
            host.log(),
            ["hide_main", "open_overlays:2", "close_overlays"]
        );
        capture.cancel(&host);
        assert_eq!(
            host.log().len(),
            4,
            "cancelar sin sesión solo cierra overlays huérfanos"
        );
    }

    #[test]
    fn cancel_without_a_session_still_closes_orphan_overlays() {
        let (host, capture) = (FakeHost::new(), controller());
        capture.cancel(&host);
        assert_eq!(host.log(), ["close_overlays"]);
        host.clear();
        // El evento Destroyed de un overlay ya cerrado no vuelve a cerrar nada.
        capture.overlay_closed(&host);
        assert!(host.log().is_empty());
    }

    #[test]
    fn overlays_that_never_become_ready_abort_the_capture() {
        let host = FakeHost::new();
        let capture = CaptureController::new(Duration::ZERO, Some(Duration::from_millis(50)));
        let error = capture.start(&host, &two_monitors()).unwrap_err();
        assert_eq!(error.code(), "capture");
        assert_eq!(
            host.log(),
            [
                "hide_main",
                "open_overlays:2",
                "close_overlays",
                "show_main"
            ]
        );
        assert!(capture.frame(0).is_err(), "la sesión se liberó");

        // Aunque Qapture estuviera oculto, se muestra: el error tiene que verse.
        host.visible.set(false);
        host.clear();
        let error = capture.start(&host, &two_monitors()).unwrap_err();
        assert_eq!(error.code(), "capture", "no queda «en curso»");
        assert_eq!(host.log().last().unwrap(), "show_main");
    }

    /// Doble `Sync` para los tests con varios hilos.
    #[derive(Default)]
    struct SyncHost(std::sync::Mutex<Vec<String>>);

    impl SyncHost {
        fn push(&self, entry: impl Into<String>) {
            self.0.lock().unwrap().push(entry.into());
        }
    }

    impl CaptureHost for SyncHost {
        fn hide_main(&self) -> bool {
            self.push("hide_main");
            true
        }
        fn show_main(&self) {
            self.push("show_main");
        }
        fn open_overlays(&self, monitors: &[MonitorBounds]) -> AppResult<()> {
            self.push(format!("open_overlays:{}", monitors.len()));
            Ok(())
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
            None
        }
    }

    #[test]
    fn start_returns_as_soon_as_an_overlay_is_ready() {
        let host = SyncHost::default();
        let capture = CaptureController::new(Duration::ZERO, Some(Duration::from_secs(5)));
        let begun = std::time::Instant::now();
        let result = thread::scope(|s| {
            let starting = s.spawn(|| capture.start(&host, &two_monitors()));
            while capture.overlay_ready(&host, 1).is_err() {
                thread::yield_now();
            }
            starting.join().unwrap()
        });
        result.unwrap();
        assert!(
            begun.elapsed() < Duration::from_secs(4),
            "no esperó el plazo"
        );
        assert!(capture.frame(0).is_ok(), "la sesión sigue abierta");
        assert_eq!(
            *host.0.lock().unwrap(),
            ["hide_main", "open_overlays:2", "show_overlay:1:false"]
        );
    }

    /// Un overlay cancela mientras `open_overlays` todavía crea los demás.
    struct CancelWhileOpening<'a> {
        inner: &'a FakeHost,
        capture: &'a CaptureController,
    }

    impl CaptureHost for CancelWhileOpening<'_> {
        fn hide_main(&self) -> bool {
            self.inner.hide_main()
        }
        fn show_main(&self) {
            self.inner.show_main()
        }
        fn open_overlays(&self, monitors: &[MonitorBounds]) -> AppResult<()> {
            self.inner.push(format!("open_overlays:{}", monitors.len()));
            self.capture.cancel(self);
            let again = self.capture.start(self.inner, &two_monitors());
            self.inner.push(format!("restart:{}", again.unwrap_err()));
            Ok(())
        }
        fn close_overlays(&self) {
            self.inner.close_overlays()
        }
        fn show_overlay(&self, index: usize, focus: bool) {
            self.inner.show_overlay(index, focus)
        }
        fn emit_captured(&self, tmp_path: &str) {
            self.inner.emit_captured(tmp_path)
        }
        fn cursor(&self) -> Option<(f64, f64)> {
            self.inner.cursor()
        }
    }

    #[test]
    fn overlays_built_after_the_session_ended_are_closed() {
        let inner = FakeHost::new();
        let capture = CaptureController::new(Duration::ZERO, Some(Duration::from_secs(5)));
        let host = CancelWhileOpening {
            inner: &inner,
            capture: &capture,
        };
        for _ in 0..2 {
            // La segunda vuelta prueba que la sesión no quedó «en curso».
            inner.clear();
            let begun = std::time::Instant::now();
            capture.start(&host, &two_monitors()).unwrap();
            assert!(
                begun.elapsed() < Duration::from_secs(4),
                "no esperó el plazo"
            );
            assert_eq!(
                inner.log(),
                [
                    "hide_main",
                    "open_overlays:2",
                    "close_overlays",
                    "show_main",
                    "restart:Ya hay una captura en curso.",
                    "close_overlays"
                ]
            );
            assert!(capture.frame(0).is_err());
        }
    }

    #[test]
    fn finish_only_completes_the_session_it_cropped_from() {
        let tmp = TempDir::new();
        let files = tmp.files();
        let (host, capture) = (FakeHost::new(), controller());
        capture.start(&host, &two_monitors()).unwrap();
        let (generation, png) = capture.crop(&host, 0, None).unwrap();
        // Mientras se guardaba el recorte, la sesión se canceló y empezó otra.
        capture.cancel(&host);
        capture.start(&host, &two_monitors()).unwrap();
        host.clear();
        let error = capture.commit(&host, &files, generation, &png).unwrap_err();
        assert_eq!(error.to_string(), "La captura ya terminó.");
        assert!(host.log().is_empty(), "no cierra los overlays de la nueva");
        assert_eq!(staged_files(&files), 0);
        assert!(capture.frame(0).is_ok(), "la nueva sesión sigue abierta");
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
