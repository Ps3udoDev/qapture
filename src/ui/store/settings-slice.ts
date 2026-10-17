import type { StoreApi } from "zustand/vanilla";
import { failureMessage, validateSettings, type Settings, type SettingsSnapshot } from "../../domain/settings";
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
  /** Nombre del tester del perfil; reemplaza al que F4 recordaba en el equipo. */
  testerName(): string;
  openCaseForm(caseId?: string | null): void;
  closeCaseForm(): void;
  listenNewCase(): Promise<() => void>;
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
      const errors = validateSettings(input);
      if (Object.keys(errors).length > 0) return { ok: false, errors };
      try {
        const settings = await repos.settings.save(input);
        set({ settings });
        get().pushToast("success", "Ajustes guardados.");
        return { ok: true };
      } catch (error) {
        if (isValidationError(error)) return { ok: false, errors: { _form: error.message } };
        h.report(error);
        return { ok: false, errors: {} };
      }
    },

    testerName: () => get().settings?.settings.profile.tester ?? "",

    openCaseForm: (caseId = null) => set({ caseFormRequest: { caseId } }),
    closeCaseForm: () => set({ caseFormRequest: null }),

    listenNewCase: () =>
      repos.settings.onNewCase(() => {
        if (get().project) get().openCaseForm(null);
        else get().pushToast("error", "Crea o abre una matriz para registrar casos.");
      }),
  };
}
