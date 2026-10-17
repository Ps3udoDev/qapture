import type { LinkTarget, Settings, SettingsSnapshot } from "../domain/settings";

/** Ajustes de la aplicación y acciones del sistema que dependen de ellos (F6). */
export interface SettingsStore {
  load(): Promise<SettingsSnapshot>;
  /** Rust valida, registra los atajos y guarda; si el SO rechaza uno, lanza `validation` y no cambia nada. */
  save(settings: Settings): Promise<SettingsSnapshot>;
  /** `null`: el usuario cerró el diálogo sin elegir carpeta. */
  pickExportFolder(): Promise<string | null>;
  openDataFolder(): Promise<void>;
  hideWindow(): Promise<void>;
  openLink(target: LinkTarget): Promise<void>;
  /** Suscribe al evento `shortcut:new-case` (atajo o menú de la bandeja). */
  onNewCase(handler: () => void): Promise<() => void>;
}
