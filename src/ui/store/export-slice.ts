import type { StoreApi } from "zustand/vanilla";
import * as useCases from "../../application/use-cases";
import type { DocVersion, DocVersionInput } from "../../domain/doc-versions";
import type { Repositories } from "../../ports/repositories";
import type { SliceHelpers } from "./evidence-slice";
import type { Preferences } from "./preferences";
import type { FormResult, WorkspaceState } from "./workspace";

export interface ExportSlice {
  docVersions: DocVersion[];
  loadDocVersions(): Promise<void>;
  saveDocVersion(input: DocVersionInput, id?: string): Promise<FormResult>;
  deleteDocVersion(id: string): Promise<void>;
  rememberedTester(): string;
}

type Set = StoreApi<WorkspaceState>["setState"];
type Get = StoreApi<WorkspaceState>["getState"];

const NO_PROJECT: FormResult = { ok: false, errors: { _form: "No hay un proyecto abierto." } };

export function createExportSlice(repos: Repositories, prefs: Preferences, set: Set, get: Get, h: SliceHelpers): ExportSlice {
  return {
    docVersions: [],

    async loadDocVersions() {
      const projectId = get().project?.id;
      if (!projectId) {
        set({ docVersions: [] });
        return;
      }
      try {
        const docVersions = await repos.docVersions.list(projectId);
        if (get().project?.id === projectId) set({ docVersions });
      } catch (error) {
        h.report(error);
      }
    },

    async saveDocVersion(input, id) {
      const current = get().project;
      if (!current) return NO_PROJECT;
      return h.form(
        () => useCases.saveDocVersion(repos, current.id, input, id),
        async (saved) => {
          await get().loadDocVersions();
          get().pushToast("success", id ? `Versión ${saved.version} actualizada.` : `Versión ${saved.version} registrada.`);
        },
      );
    },

    async deleteDocVersion(id) {
      const removed = get().docVersions.find((v) => v.id === id);
      try {
        await repos.docVersions.remove(id);
        await get().loadDocVersions();
        if (removed) get().pushToast("success", `Versión ${removed.version} eliminada.`);
      } catch (error) {
        h.report(error);
      }
    },

    rememberedTester: () => prefs.getTester(),
  };
}
