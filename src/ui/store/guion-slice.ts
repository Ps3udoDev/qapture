import type { StoreApi } from "zustand/vanilla";
import * as useCases from "../../application/use-cases";
import { importSummaryMessage } from "../../domain/data";
import type { GuionImportInput, GuionInspection, GuionPreview, ImportProfile, ProfileDefinition } from "../../domain/guion";
import type { Repositories } from "../../ports/repositories";
import type { SliceHelpers } from "./evidence-slice";
import type { FormResult, WorkspaceState } from "./workspace";

export interface GuionSlice {
  guionWizard: { inspection: GuionInspection; preview: GuionPreview | null; profiles: ImportProfile[] } | null;
  openGuionImport(): Promise<void>;
  previewGuion(profile: ProfileDefinition): Promise<void>;
  importGuion(input: GuionImportInput): Promise<FormResult>;
  closeGuionImport(): void;
  deleteImportProfile(id: string): Promise<void>;
}
type Set = StoreApi<WorkspaceState>["setState"];
type Get = StoreApi<WorkspaceState>["getState"];
const BUSY: FormResult = { ok: false, errors: { _form: "Hay una operación de datos en curso." } };

export function createGuionSlice(repos: Repositories, set: Set, get: Get, h: SliceHelpers): GuionSlice {
  // Una sesión distinta invalida respuestas pendientes, incluso si aún no había asistente.
  let session = 0;
  const exclusive = async <T>(busy: T, run: () => Promise<T>): Promise<T> => {
    if (get().dataBusy) return busy;
    set({ dataBusy: true });
    try { return await run(); } finally { set({ dataBusy: false }); }
  };
  return {
    guionWizard: null,
    openGuionImport: () => exclusive(undefined, async () => {
      const current = ++session;
      try {
        const inspection = await repos.guion.inspect();
        if (!inspection || current !== session) return;
        const profiles = await repos.guion.listProfiles();
        if (current === session) set({ guionWizard: { inspection, preview: null, profiles } });
      } catch (error) { h.report(error); }
    }),
    previewGuion: (profile) => exclusive(undefined, async () => {
      const wizard = get().guionWizard;
      if (!wizard) return;
      const current = session;
      try {
        const preview = await repos.guion.preview(wizard.inspection.token, profile);
        if (current === session && get().guionWizard?.inspection.token === wizard.inspection.token) {
          set({ guionWizard: { ...get().guionWizard!, preview } });
        }
      } catch (error) { h.report(error); }
    }),
    importGuion: (input) => exclusive<FormResult>(BUSY, async () => {
      const wizard = get().guionWizard;
      if (!wizard || wizard.inspection.token !== input.token) return { ok: false, errors: { _form: "Vuelve a elegir el archivo del guion." } };
      try {
        const outcome = await useCases.importGuion(repos, input);
        if (!outcome.ok) return outcome;
        session += 1;
        set({ guionWizard: null });
        await h.reload(outcome.value.projects[0]?.id ?? null);
        get().pushToast("success", importSummaryMessage(outcome.value));
        for (const warning of outcome.value.warnings) get().pushToast("error", warning);
        return { ok: true };
      } catch (error) { h.report(error); return { ok: false, errors: {} }; }
    }),
    closeGuionImport() { session += 1; set({ guionWizard: null }); },
    deleteImportProfile: (id) => exclusive(undefined, async () => {
      const current = session;
      try {
        await repos.guion.deleteProfile(id);
        const profiles = await repos.guion.listProfiles();
        const wizard = get().guionWizard;
        if (wizard && current === session) set({ guionWizard: { ...wizard, profiles } });
      } catch (error) { h.report(error); }
    }),
  };
}
