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

/** Captura integrada (UC-11). F6 la dispara también desde el atajo global. */
export interface ScreenCapture {
  /** Oculta Qapture y abre el overlay; termina cuando el overlay está abierto, no cuando el usuario elige. */
  start(): Promise<void>;
  /** Suscribe al evento `evidence:captured`; devuelve la función para desuscribirse. */
  onCaptured(handler: (stagedPath: string) => void): Promise<() => void>;
}
