import { trimDocVersion, validateDocVersion, type DocVersion, type DocVersionInput } from "../../domain/doc-versions";
import { AppError } from "../../ports/errors";
import type { DocVersionRepository } from "../../ports/export";

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
