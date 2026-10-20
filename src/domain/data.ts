/** UC-18 / UC-19 / UC-20: importación, paquetes `.qapture` y respaldos. */

export interface ImportedProject {
  id: string;
  name: string;
}

export interface ImportSummary {
  projects: ImportedProject[];
  cases: number;
  evidence: number;
  /** Avisos no bloqueantes de la importación (se muestran como avisos). */
  warnings: string[];
}

export interface PackageExportResult {
  path: string;
  fileName: string;
  cases: number;
  evidence: number;
  missingImages: number;
}

export const BACKUP_KINDS = ["daily", "manual", "before-restore", "before-migration"] as const;
export type BackupKind = (typeof BACKUP_KINDS)[number];

export interface Backup {
  fileName: string;
  kind: BackupKind;
  /** Hora local `AAAA-MM-DDTHH:MM:SS`. */
  createdAt: string;
  sizeBytes: number;
}

export interface RestoreResult {
  restored: Backup;
  safetyCopy: string;
  missingImages: number;
  setAsideFiles: number;
}

export interface StartupStatus {
  recovery: boolean;
  detail: string;
}

export interface ImportIssues {
  title: string;
  items: string[];
}

export const BACKUP_KIND_LABEL: Record<BackupKind, string> = {
  daily: "Diario",
  manual: "Manual",
  "before-restore": "Antes de restaurar",
  "before-migration": "Antes de actualizar",
};

const count = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;

/** Nombra el proyecto creado: una copia «(importado)» puede quedar lejos en la lista. */
export function importSummaryMessage(summary: ImportSummary): string {
  const totals = `${count(summary.cases, "caso", "casos")} y ${count(summary.evidence, "captura", "capturas")}`;
  const names = summary.projects.map((p) => `«${p.name}»`);
  if (names.length === 1) return `Importado ${names[0]}: ${totals}.`;
  const listed = names.length > 1 ? `${names.slice(0, -1).join(", ")} y ${names[names.length - 1]}` : "";
  return `Importado: ${count(names.length, "proyecto", "proyectos")}${listed ? ` (${listed})` : ""}, ${totals}.`;
}

/** Rust envía «encabezado\n• problema\n• problema…» (máximo 10 + resto). */
export function parseImportIssues(message: string): ImportIssues {
  const [title = "", ...rest] = message.split("\n");
  return { title, items: rest.filter((line) => line.startsWith("• ")).map((line) => line.slice(2)) };
}

/** `2026-10-15T09:00:05` → `15/10/2026 09:00`. */
export function formatBackupDate(iso: string): string {
  const m = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2})/.exec(iso);
  return m ? `${m[3]}/${m[2]}/${m[1]} ${m[4]}:${m[5]}` : iso;
}

export function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const kb = bytes / 1024;
  if (kb < 1024) return `${Math.round(kb)} KB`;
  return `${(kb / 1024).toFixed(1).replace(".", ",")} MB`;
}

export function restoreMessage(result: RestoreResult): string {
  const base = `Se restauró el respaldo del ${formatBackupDate(result.restored.createdAt)}.`;
  const n = result.missingImages;
  if (n === 0) return base;
  return `${base} ${count(n, "captura", "capturas")} del respaldo ya no ${n === 1 ? "tiene" : "tienen"} imagen.`;
}
