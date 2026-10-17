import { z } from "zod";
import type { DataTransfer } from "../../ports/data";
import { call } from "./client";
import { backupSchema, importSummarySchema, packageExportResultSchema, restoreResultSchema, startupStatusSchema } from "./dto";

export function tauriData(): DataTransfer {
  return {
    startupStatus: () => call("get_startup_status", {}, startupStatusSchema),
    importLegacy: () => call("import_legacy", {}, importSummarySchema.nullable()),
    importPackage: () => call("import_package", {}, importSummarySchema.nullable()),
    exportPackage: (projectId) => call("export_package", { projectId }, packageExportResultSchema.nullable()),
    listBackups: () => call("list_backups", {}, z.array(backupSchema)),
    createBackup: () => call("create_backup", {}, backupSchema),
    restoreBackup: (fileName) => call("restore_backup", { fileName }, restoreResultSchema),
    recover: async (fileName) => {
      await call("recover_database", { fileName }, z.null());
    },
  };
}
