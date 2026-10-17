import { trimDocVersion, validateDocVersion, type DocVersion, type DocVersionInput } from "../../domain/doc-versions";
import type { ExportOptions, ExportProgress, ExportResult } from "../../domain/export";
import { AppError } from "../../ports/errors";
import type { DocVersionRepository, Exporter } from "../../ports/export";

export interface MemoryDocVersionDeps {
  ensureProject(projectId: string): void;
  touch(projectId: string): void;
  id(prefix: string): string;
  now(): string;
}

export interface MemoryDocVersions extends DocVersionRepository {
  removeForProject(projectId: string): void;
}

/** Replica en memoria las reglas de `repo/doc_versions.rs`. */
export function createMemoryDocVersions(deps: MemoryDocVersionDeps): MemoryDocVersions {
  const items = new Map<string, DocVersion>();
  const fail = (message: string): never => {
    throw new AppError("validation", message);
  };
  const get = (id: string) => {
    const found = items.get(id);
    if (!found) throw new AppError("not_found", "La versión del documento no existe.");
    return found;
  };
  const check = (projectId: string, raw: DocVersionInput, except?: string) => {
    const first = Object.values(validateDocVersion(raw))[0];
    if (first) fail(first);
    const input = trimDocVersion(raw);
    const taken = [...items.values()].some(
      (v) => v.projectId === projectId && v.id !== except && v.version.toLowerCase() === input.version.toLowerCase(),
    );
    if (taken) fail(`La versión «${input.version}» ya existe en este proyecto.`);
    return input;
  };
  const sorted = (projectId: string) =>
    [...items.values()]
      .filter((v) => v.projectId === projectId)
      .sort((a, b) => a.date.localeCompare(b.date) || a.createdAt.localeCompare(b.createdAt));

  return {
    async list(projectId) {
      deps.ensureProject(projectId);
      return structuredClone(sorted(projectId));
    },
    async create(projectId, raw) {
      deps.ensureProject(projectId);
      const input = check(projectId, raw);
      const version: DocVersion = { ...input, id: deps.id("ver"), projectId, createdAt: deps.now() };
      items.set(version.id, version);
      deps.touch(projectId);
      return structuredClone(version);
    },
    async update(id, raw) {
      const current = get(id);
      Object.assign(current, check(current.projectId, raw, id));
      deps.touch(current.projectId);
      return structuredClone(current);
    },
    async remove(id) {
      const current = get(id);
      items.delete(id);
      deps.touch(current.projectId);
    },
    removeForProject(projectId) {
      for (const v of sorted(projectId)) items.delete(v.id);
    },
  };
}

/** save: guarda · cancel: el usuario cierra «Guardar como» · locked: el archivo está abierto en Excel. */
export type MemoryExportMode = "save" | "cancel" | "locked";

export interface MemoryExporter extends Exporter {
  mode: MemoryExportMode;
  calls: { kind: "project" | "template"; projectId: string; options: ExportOptions }[];
  opened: string[];
  revealed: string[];
  /** Imágenes que el próximo export no pudo incluir. */
  missingImages: number;
  emit(progress: ExportProgress): void;
  /** Un paquete exportado en esta sesión también se puede mostrar en su carpeta. */
  remember(path: string): void;
}

export function createMemoryExporter(countFor: (projectId: string) => { cases: number; evidence: number }): MemoryExporter {
  const handlers = new Set<(progress: ExportProgress) => void>();
  const exported = new Set<string>();
  const run = async (kind: "project" | "template", projectId: string, options: ExportOptions): Promise<ExportResult | null> => {
    exporter.calls.push({ kind, projectId, options: structuredClone(options) });
    if (exporter.mode === "cancel") return null;
    if (exporter.mode === "locked") throw new AppError("file_locked", "Cierra el archivo en Excel y vuelve a intentar.");
    exporter.emit({ stage: "preparing", current: 0, total: 0 });
    exporter.emit({ stage: "saving", current: 0, total: 0 });
    const fileName = kind === "project" ? "T4S_ACME_Recibos_Evidencias_20261016_v1.0.xlsx" : "T4S_ACME_Recibos_Plantilla_20261016.xlsx";
    const path = `C:\\Users\\ana\\Documents\\${fileName}`;
    exported.add(path);
    const counts = kind === "project" ? countFor(projectId) : { cases: 0, evidence: 0 };
    return { path, fileName, ...counts, missingImages: exporter.missingImages };
  };
  const known = (path: string) => {
    if (!exported.has(path)) throw new AppError("validation", "Solo se pueden abrir archivos exportados en esta sesión.");
  };
  const exporter: MemoryExporter = {
    mode: "save",
    calls: [],
    opened: [],
    revealed: [],
    missingImages: 0,
    exportProject: (projectId, options) => run("project", projectId, options),
    exportTemplate: (projectId, options) => run("template", projectId, options),
    async onProgress(handler) {
      handlers.add(handler);
      return () => void handlers.delete(handler);
    },
    async openFile(path) {
      known(path);
      exporter.opened.push(path);
    },
    async revealFile(path) {
      known(path);
      exporter.revealed.push(path);
    },
    remember(path) {
      exported.add(path);
    },
    emit(progress) {
      handlers.forEach((handler) => handler(progress));
    },
  };
  return exporter;
}
