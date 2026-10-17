import type { Backup, BackupKind, ImportedProject, ImportSummary, StartupStatus } from "../../domain/data";
import type { DataTransfer } from "../../ports/data";
import { AppError } from "../../ports/errors";

export interface MemoryDataDeps {
  /** Crea el proyecto que «trae» la importación. */
  importProject(name: string): Promise<ImportedProject>;
  /** Registra el paquete como exportado en la sesión (para «Abrir carpeta»). */
  remember(path: string): void;
}

/** ok: importa/exporta · cancel: el usuario cierra el diálogo · invalid: Rust rechaza el archivo. */
export type MemoryDataMode = "ok" | "cancel" | "invalid";

export interface MemoryData extends DataTransfer {
  mode: MemoryDataMode;
  invalidMessage: string;
  status: StartupStatus;
  backups: Backup[];
  /** Operaciones pedidas, en orden (`importLegacy`, `restore:<archivo>`, `recover:vacía`…). */
  calls: string[];
}

export const MEMORY_INVALID_IMPORT =
  "No se importó nada. Revisa estos problemas:\n• Proyecto «Aplicación de recibos», EJ-001: Completa caso y entorno.\n• Proyecto «Aplicación de recibos», EJ-002: Fecha inválida.";

const backup = (fileName: string, kind: BackupKind, createdAt: string): Backup => ({ fileName, kind, createdAt, sizeBytes: 98304 });
const pad = (n: number) => String(n).padStart(2, "0");

/** Replica en memoria lo que devuelven los comandos de `commands/data.rs`. */
export function createMemoryData(deps: MemoryDataDeps): MemoryData {
  let manualSeq = 0;
  const pick = async (call: string, name: string, counts: { cases: number; evidence: number }): Promise<ImportSummary | null> => {
    data.calls.push(call);
    if (data.mode === "cancel") return null;
    if (data.mode === "invalid") throw new AppError("validation", data.invalidMessage);
    return { projects: [await deps.importProject(name)], ...counts };
  };
  const data: MemoryData = {
    mode: "ok",
    invalidMessage: MEMORY_INVALID_IMPORT,
    status: { recovery: false, detail: "" },
    backups: [backup("qapture-diario-20261016.db", "daily", "2026-10-16T09:00:00"), backup("qapture-diario-20261015.db", "daily", "2026-10-15T09:00:00")],
    calls: [],
    async startupStatus() {
      return { ...data.status };
    },
    importLegacy: () => pick("importLegacy", "Aplicación de recibos", { cases: 1, evidence: 2 }),
    importPackage: () => pick("importPackage", "Recibos (importado)", { cases: 3, evidence: 4 }),
    async exportPackage(projectId) {
      data.calls.push(`exportPackage:${projectId}`);
      if (data.mode === "cancel") return null;
      const fileName = "T4S_ACME_Recibos_20261016.qapture";
      const path = `C:\\Users\\ana\\Documents\\${fileName}`;
      deps.remember(path);
      return { path, fileName, cases: 2, evidence: 2, missingImages: 0 };
    },
    async listBackups() {
      return structuredClone(data.backups);
    },
    async createBackup() {
      data.calls.push("createBackup");
      manualSeq += 1;
      const created = backup(`qapture-manual-20261016-1000${pad(manualSeq)}.db`, "manual", `2026-10-16T10:00:${pad(manualSeq)}`);
      data.backups = [created, ...data.backups];
      return structuredClone(created);
    },
    async restoreBackup(fileName) {
      data.calls.push(`restore:${fileName}`);
      const restored = data.backups.find((b) => b.fileName === fileName);
      if (!restored) throw new AppError("not_found", "El respaldo no existe.");
      const safety = backup("qapture-antes-de-restaurar-20261016-110000.db", "before-restore", "2026-10-16T11:00:00");
      data.backups = [safety, ...data.backups];
      return { restored: structuredClone(restored), safetyCopy: safety.fileName, missingImages: 0, setAsideFiles: 0 };
    },
    async recover(fileName) {
      data.calls.push(`recover:${fileName ?? "vacía"}`);
      if (!data.status.recovery) throw new AppError("validation", "Qapture no está en modo recuperación.");
      data.status = { recovery: false, detail: "" };
    },
  };
  return data;
}
