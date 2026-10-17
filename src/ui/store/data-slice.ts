import type { StoreApi } from "zustand/vanilla";
import { importSummaryMessage, parseImportIssues, restoreMessage, type Backup, type ImportIssues, type ImportSummary, type StartupStatus } from "../../domain/data";
import { isValidationError } from "../../ports/errors";
import type { Repositories } from "../../ports/repositories";
import type { SliceHelpers } from "./evidence-slice";
import type { WorkspaceState } from "./workspace";

export interface DataSlice {
  /** Con la base dañada, el estado del arranque; `null` en uso normal. */
  recovery: StartupStatus | null;
  backups: Backup[];
  /** Problemas de la última importación rechazada, para el diálogo «No se pudo importar». */
  importIssues: ImportIssues | null;
  /** Hay una importación, exportación, respaldo o restauración en curso. */
  dataBusy: boolean;
  loadBackups(): Promise<void>;
  importLegacy(): Promise<void>;
  importPackage(): Promise<void>;
  exportPackage(): Promise<void>;
  createBackup(): Promise<void>;
  /** `true` si se restauró (para cerrar el diálogo). */
  restoreBackup(fileName: string): Promise<boolean>;
  recover(fileName: string | null, thenImportPackage?: boolean): Promise<void>;
  dismissImportIssues(): void;
}

type Set = StoreApi<WorkspaceState>["setState"];
type Get = StoreApi<WorkspaceState>["getState"];

const missingMessage = (n: number) =>
  n === 1 ? "1 captura no se incluyó porque falta su imagen." : `${n} capturas no se incluyeron porque falta su imagen.`;

export function createDataSlice(repos: Repositories, set: Set, get: Get, h: SliceHelpers): DataSlice {
  /** Una operación de datos a la vez; `false` si ya había otra en curso. */
  const exclusive = async (run: () => Promise<void>): Promise<boolean> => {
    if (get().dataBusy) return false;
    set({ dataBusy: true });
    try {
      await run();
      return true;
    } finally {
      set({ dataBusy: false });
    }
  };

  const runImport = async (pick: () => Promise<ImportSummary | null>) => {
    await exclusive(async () => {
      try {
        const summary = await pick();
        if (!summary) return;
        await h.reload(summary.projects[0]?.id ?? null);
        get().pushToast("success", importSummaryMessage(summary));
      } catch (error) {
        if (isValidationError(error)) set({ importIssues: parseImportIssues(error.message) });
        else h.report(error);
      }
    });
  };

  return {
    recovery: null,
    backups: [],
    importIssues: null,
    dataBusy: false,

    async loadBackups() {
      try {
        set({ backups: await repos.data.listBackups() });
      } catch (error) {
        h.report(error);
      }
    },

    importLegacy: () => runImport(() => repos.data.importLegacy()),
    importPackage: () => runImport(() => repos.data.importPackage()),

    async exportPackage() {
      const current = get().project;
      if (!current) return;
      await exclusive(async () => {
        try {
          const result = await repos.data.exportPackage(current.id);
          if (!result) return;
          get().pushToast("success", `Paquete guardado: ${result.fileName}`, [
            { label: "Abrir carpeta", run: () => void repos.exporter.revealFile(result.path).catch(h.report) },
          ]);
          if (result.missingImages > 0) get().pushToast("error", missingMessage(result.missingImages));
        } catch (error) {
          h.report(error);
        }
      });
    },

    async createBackup() {
      await exclusive(async () => {
        try {
          const created = await repos.data.createBackup();
          get().pushToast("success", `Respaldo creado: ${created.fileName}`);
        } catch (error) {
          h.report(error);
          return;
        }
        await get().loadBackups();
      });
    },

    async restoreBackup(fileName) {
      let restored = false;
      await exclusive(async () => {
        try {
          const result = await repos.data.restoreBackup(fileName);
          restored = true;
          await h.reload(null);
          await get().loadBackups();
          get().pushToast("success", restoreMessage(result));
        } catch (error) {
          h.report(error);
        }
      });
      return restored;
    },

    async recover(fileName, thenImportPackage = false) {
      const ran = await exclusive(async () => {
        try {
          await repos.data.recover(fileName);
          set({ recovery: null });
          await h.reload(null);
          get().pushToast("success", fileName ? "Base recuperada desde el respaldo." : "Qapture empezó con una base vacía.");
        } catch (error) {
          h.report(error);
        }
      });
      if (ran && thenImportPackage && !get().recovery) await get().importPackage();
    },

    dismissImportIssues: () => set({ importIssues: null }),
  };
}
