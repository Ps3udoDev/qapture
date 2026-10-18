import type { UpdateInfo, UpdateProgress } from "../../domain/update";
import type { AppError } from "../../ports/errors";
import type { Updater } from "../../ports/updater";

export interface MemoryUpdater extends Updater {
  /** Lo que devuelve `check`: una versión, `null` (al día) o un error (sin red). */
  next: UpdateInfo | null | AppError;
  /** Error con el que falla `install`. */
  installError: AppError | null;
  /** Si es `true`, `install` queda pendiente hasta `release()` (para ver el progreso). */
  hold: boolean;
  /** Si es `true`, `check` queda pendiente hasta `releaseCheck()`. */
  holdCheck: boolean;
  /** Veces que se pidió instalar. */
  installs: number;
  emitAvailable(info: UpdateInfo): void;
  emitShow(info: UpdateInfo): void;
  emitProgress(progress: UpdateProgress): void;
  release(): void;
  releaseCheck(): void;
}

/** Replica en memoria `check_update` e `install_update` y sus eventos. */
export function createMemoryUpdater(): MemoryUpdater {
  const available = new Set<(info: UpdateInfo) => void>();
  const show = new Set<(info: UpdateInfo) => void>();
  const progress = new Set<(p: UpdateProgress) => void>();
  let release: () => void = () => undefined;
  let releaseCheck: () => void = () => undefined;
  const m: MemoryUpdater = {
    next: null,
    installError: null,
    hold: false,
    holdCheck: false,
    installs: 0,
    async check() {
      if (m.holdCheck) await new Promise<void>((resolve) => (releaseCheck = resolve));
      if (m.next instanceof Error) throw m.next;
      return m.next ? structuredClone(m.next) : null;
    },
    async install() {
      m.installs += 1;
      if (m.installError) throw m.installError;
      if (m.hold) await new Promise<void>((resolve) => (release = resolve));
    },
    async onAvailable(handler) {
      available.add(handler);
      return () => void available.delete(handler);
    },
    async onShow(handler) {
      show.add(handler);
      return () => void show.delete(handler);
    },
    async onProgress(handler) {
      progress.add(handler);
      return () => void progress.delete(handler);
    },
    emitAvailable(info) {
      for (const h of [...available]) h(structuredClone(info));
    },
    emitShow(info) {
      for (const h of [...show]) h(structuredClone(info));
    },
    emitProgress(p) {
      for (const h of [...progress]) h({ ...p });
    },
    release: () => release(),
    releaseCheck: () => releaseCheck(),
  };
  return m;
}
