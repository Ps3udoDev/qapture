import type { Backup, ImportSummary, PackageExportResult, RestoreResult, StartupStatus } from "../domain/data";

/** Importar (v1 y paquetes), exportar paquetes y respaldos. Los diálogos de archivo los abre Rust. */
export interface DataTransfer {
  startupStatus(): Promise<StartupStatus>;
  /** `null`: el usuario cerró el diálogo sin elegir archivo. */
  importLegacy(): Promise<ImportSummary | null>;
  importPackage(): Promise<ImportSummary | null>;
  exportPackage(projectId: string): Promise<PackageExportResult | null>;
  listBackups(): Promise<Backup[]>;
  createBackup(): Promise<Backup>;
  restoreBackup(fileName: string): Promise<RestoreResult>;
  /** Sale del modo recuperación con ese respaldo o, con `null`, con una base vacía. */
  recover(fileName: string | null): Promise<void>;
}
