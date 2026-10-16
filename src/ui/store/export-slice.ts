import type { StoreApi } from "zustand/vanilla";
import * as useCases from "../../application/use-cases";
import type { DocVersion, DocVersionInput } from "../../domain/doc-versions";
import type { ExportProgress } from "../../domain/export";
import type { Repositories } from "../../ports/repositories";
import type { SliceHelpers } from "./evidence-slice";
import type { Preferences } from "./preferences";
import type { FormResult, WorkspaceState } from "./workspace";

export interface ExportSlice {
  docVersions: DocVersion[];
  /** Avance del export en curso; `null` si no hay ninguno. */
  exporting: ExportProgress | null;
  loadDocVersions(): Promise<void>;
  saveDocVersion(input: DocVersionInput, id?: string): Promise<FormResult>;
  deleteDocVersion(id: string): Promise<void>;
  rememberedTester(): string;
  exportExcel(request: useCases.ExportRequest): Promise<FormResult>;
}

type Set = StoreApi<WorkspaceState>["setState"];
type Get = StoreApi<WorkspaceState>["getState"];

const NO_PROJECT: FormResult = { ok: false, errors: { _form: "No hay un proyecto abierto." } };
const BUSY: FormResult = { ok: false, errors: { _form: "Ya hay una exportación en curso." } };

const missingMessage = (n: number) =>
  n === 1
    ? "1 imagen no se pudo incluir: en la hoja Evidencias aparece un aviso en su lugar."
    : `${n} imágenes no se pudieron incluir: en la hoja Evidencias aparece un aviso en su lugar.`;

export function createExportSlice(repos: Repositories, prefs: Preferences, set: Set, get: Get, h: SliceHelpers): ExportSlice {
  return {
    docVersions: [],
    exporting: null,

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

    async exportExcel(request) {
      const current = get().project;
      if (!current) return NO_PROJECT;
      if (get().exporting) return BUSY;
      set({ exporting: { stage: "preparing", current: 0, total: 0 } });
      try {
        const outcome = await useCases.exportWorkbook(repos, current.id, request, (exporting) => set({ exporting }));
        // Una versión nueva queda registrada aunque luego se cancele «Guardar como».
        if (request.newVersion) await get().loadDocVersions();
        if (!outcome.ok) return outcome;
        prefs.setTester(request.options.tester.trim());
        const result = outcome.value;
        if (!result) return { ok: false, errors: {} };
        const open = (run: (path: string) => Promise<void>) => () => void run(result.path).catch(h.report);
        get().pushToast("success", `Excel guardado: ${result.fileName}`, [
          { label: "Abrir archivo", run: open(repos.exporter.openFile) },
          { label: "Abrir carpeta", run: open(repos.exporter.revealFile) },
        ]);
        if (result.missingImages > 0) get().pushToast("error", missingMessage(result.missingImages));
        return { ok: true };
      } catch (error) {
        h.report(error);
        return { ok: false, errors: {} };
      } finally {
        set({ exporting: null });
      }
    },
  };
}
