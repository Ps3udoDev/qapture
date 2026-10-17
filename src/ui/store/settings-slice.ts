import type { StoreApi } from "zustand/vanilla";
import { failureMessage, validateSettings, type LinkTarget, type Settings, type SettingsSnapshot } from "../../domain/settings";
import { isValidationError } from "../../ports/errors";
import type { Repositories } from "../../ports/repositories";
import type { SliceHelpers } from "./evidence-slice";
import type { FormResult, WorkspaceState } from "./workspace";

export interface SettingsSlice {
  settings: SettingsSnapshot | null;
  /** El formulario de caso debe abrirse (atajo «Nuevo caso»); `caseId` `null` = caso nuevo. */
  caseFormRequest: { caseId: string | null } | null;
  loadSettings(): Promise<void>;
  saveSettings(settings: Settings): Promise<FormResult>;
  /** Guarda sin toast de éxito (migraciones internas); mismo resultado que `saveSettings`. */
  persistSettings(settings: Settings): Promise<FormResult>;
  /** Nombre del tester del perfil; reemplaza al que F4 recordaba en el equipo. */
  testerName(): string;
  openCaseForm(caseId?: string | null): void;
  closeCaseForm(): void;
  listenNewCase(): Promise<() => void>;
  /** Selector nativo de carpeta; `null` si se cierra sin elegir. */
  pickExportFolder(): Promise<string | null>;
  openDataFolder(): Promise<void>;
  hideWindow(): Promise<void>;
  openLink(target: LinkTarget): Promise<void>;
}

type Set = StoreApi<WorkspaceState>["setState"];
type Get = StoreApi<WorkspaceState>["getState"];

export function createSettingsSlice(repos: Repositories, set: Set, get: Get, h: SliceHelpers): SettingsSlice {
  return {
    settings: null,
    caseFormRequest: null,

    async loadSettings() {
      try {
        const settings = await repos.settings.load();
        set({ settings });
        for (const failure of settings.shortcutFailures) get().pushToast("error", failureMessage(failure));
      } catch (error) {
        h.report(error);
      }
    },

    async saveSettings(input) {
      const result = await get().persistSettings(input);
      if (result.ok) get().pushToast("success", "Ajustes guardados.");
      return result;
    },

    async persistSettings(input) {
      const errors = validateSettings(input);
      if (Object.keys(errors).length > 0) return { ok: false, errors };
      try {
        const settings = await repos.settings.save(input);
        set({ settings });
        return { ok: true };
      } catch (error) {
        if (isValidationError(error)) return { ok: false, errors: { _form: error.message } };
        h.report(error);
        return { ok: false, errors: {} };
      }
    },

    testerName: () => get().settings?.settings.profile.tester ?? "",

    // Un formulario abierto no se reemplaza: conserva lo que la persona está escribiendo.
    openCaseForm: (caseId = null) => set((s) => (s.caseFormRequest ? s : { caseFormRequest: { caseId } })),
    closeCaseForm: () => set({ caseFormRequest: null }),

    async pickExportFolder() {
      try {
        return await repos.settings.pickExportFolder();
      } catch (error) {
        h.report(error);
        return null;
      }
    },

    async openDataFolder() {
      try {
        await repos.settings.openDataFolder();
      } catch (error) {
        h.report(error);
      }
    },

    async hideWindow() {
      try {
        await repos.settings.hideWindow();
      } catch (error) {
        h.report(error);
      }
    },

    async openLink(target) {
      try {
        await repos.settings.openLink(target);
      } catch (error) {
        h.report(error);
      }
    },

    listenNewCase: () =>
      repos.settings.onNewCase(() => {
        if (get().project) get().openCaseForm(null);
        else get().pushToast("error", "Crea o abre una matriz para registrar casos.");
      }),
  };
}
