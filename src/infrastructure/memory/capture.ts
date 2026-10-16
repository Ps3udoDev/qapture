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
