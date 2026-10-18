import type { UpdateInfo, UpdateProgress } from "../domain/update";

export type Unlisten = () => void;

/** Actualizaciones (F7): Rust busca, descarga y reinicia; el front decide cuándo avisar. */
export interface Updater {
  /** Revisión manual: `null` si Qapture está al día; rechaza sin red. */
  check(): Promise<UpdateInfo | null>;
  /** Descarga con progreso y reinicia; rechaza sin nada que instalar o en portable. */
  install(): Promise<void>;
  /** Evento `update:available` (revisión periódica); la misma versión puede repetirse. */
  onAvailable(handler: (info: UpdateInfo) => void): Promise<Unlisten>;
  /** Evento `update:show` (clic en la bandeja): abrir el diálogo aunque la versión se haya pospuesto. */
  onShow(handler: (info: UpdateInfo) => void): Promise<Unlisten>;
  /** Evento `update:progress` durante la descarga. */
  onProgress(handler: (progress: UpdateProgress) => void): Promise<Unlisten>;
}
