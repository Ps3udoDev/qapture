import { DEFAULT_SETTINGS, SHORTCUT_ACTIONS, normalizeAccelerator, refusalMessage, validateSettings, type LinkTarget, type Settings, type SettingsSnapshot, type ShortcutFailure } from "../../domain/settings";
import { AppError } from "../../ports/errors";
import type { SettingsStore } from "../../ports/settings";

export interface MemorySettings extends SettingsStore {
  snapshot: SettingsSnapshot;
  /** Combinaciones que el «SO» rechaza al guardar. */
  refuse: Set<string>;
  /** Atajos que no se registraron al arrancar (los devuelve `load`). */
  failuresAtStart: ShortcutFailure[];
  /** Carpeta que devuelve el selector; `null` simula cerrarlo. */
  pickedFolder: string | null;
  /** Guardados aceptados. */
  saves: number;
  /** Veces que se ocultó la ventana. */
  hidden: number;
  /** Operaciones pedidas, en orden (`pickExportFolder`, `openDataFolder`, `openLink:docs`…). */
  calls: string[];
  /** Dispara el evento `shortcut:new-case` a quienes escuchan. */
  emitNewCase(): void;
}

const trimmed = (s: Settings): Settings => ({
  ...structuredClone(s),
  profile: { tester: s.profile.tester.trim(), role: s.profile.role.trim(), email: s.profile.email.trim() },
  export: { ...s.export, defaultFolder: s.export.defaultFolder.trim() },
});

/** Replica en memoria `get_settings` y `save_settings`. */
export function createMemorySettings(): MemorySettings {
  const handlers = new Set<() => void>();
  const view = (): SettingsSnapshot => ({ ...structuredClone(m.snapshot), shortcutFailures: structuredClone(m.failuresAtStart) });
  const m: MemorySettings = {
    snapshot: {
      settings: structuredClone(DEFAULT_SETTINGS),
      shortcutFailures: [],
      appVersion: "0.5.0",
      dataDir: "C:\\Users\\ana\\AppData\\Roaming\\com.teams4soft.qapture",
      portable: false,
    },
    refuse: new Set(),
    failuresAtStart: [],
    pickedFolder: null,
    saves: 0,
    hidden: 0,
    calls: [],
    async load() {
      return view();
    },
    async save(input) {
      const message = Object.values(validateSettings(input))[0];
      if (message) throw new AppError("validation", message);
      const next = trimmed(input);
      const current = m.snapshot.settings;
      for (const action of SHORTCUT_ACTIONS) {
        // Como Rust: se guarda y compara la forma canónica (validateSettings ya garantizó que existe).
        const accel = normalizeAccelerator(next.shortcuts[action]) ?? next.shortcuts[action];
        next.shortcuts[action] = accel;
        if (accel !== normalizeAccelerator(current.shortcuts[action]) && m.refuse.has(accel)) throw new AppError("validation", refusalMessage(accel, action));
      }
      m.saves += 1;
      m.snapshot = { ...m.snapshot, settings: next };
      // Como Rust: un atajo que falló al arrancar se reintenta si conserva su combinación
      // y solo sigue fallando si continúa ocupada; si cambió, ya se registró la nueva.
      m.failuresAtStart = m.failuresAtStart.filter((f) => next.shortcuts[f.action] === f.accelerator && m.refuse.has(f.accelerator));
      return view();
    },
    async pickExportFolder() {
      m.calls.push("pickExportFolder");
      return m.pickedFolder;
    },
    async openDataFolder() {
      m.calls.push("openDataFolder");
    },
    async hideWindow() {
      m.hidden += 1;
    },
    async openLink(target: LinkTarget) {
      m.calls.push(`openLink:${target}`);
    },
    async onNewCase(handler) {
      handlers.add(handler);
      return () => void handlers.delete(handler);
    },
    emitNewCase() {
      for (const h of [...handlers]) h();
    },
  };
  return m;
}
