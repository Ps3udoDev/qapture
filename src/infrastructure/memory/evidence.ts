import { parseAnnotations } from "../../domain/annotations";
import { validateDescription, type Evidence } from "../../domain/evidence";
import { AppError } from "../../ports/errors";
import type { EvidenceStore } from "../../ports/repositories";

const startsWith = (b: Uint8Array, sig: number[], offset = 0) => sig.every((v, i) => b[offset + i] === v);
const isPng = (b: Uint8Array) => startsWith(b, [0x89, 0x50, 0x4e, 0x47]);
const isImage = (b: Uint8Array) =>
  isPng(b) || startsWith(b, [0xff, 0xd8, 0xff]) || (startsWith(b, [0x52, 0x49, 0x46, 0x46]) && startsWith(b, [0x57, 0x45, 0x42, 0x50], 8));

export interface MemoryEvidenceDeps {
  caseProject(caseId: string): string;
  touch(projectId: string): void;
  id(prefix: string): string;
  now(): string;
}

export interface MemoryEvidence extends EvidenceStore {
  countFor(caseId: string): number;
  removeForCase(caseId: string): void;
}

/** Replica en memoria las reglas de `repo/evidence.rs` para tests de aplicación y de UI. */
export function createMemoryEvidence(deps: MemoryEvidenceDeps): MemoryEvidence {
  const staged = new Map<string, Uint8Array>();
  const files = new Map<string, Uint8Array>();
  const items = new Map<string, Evidence>();
  const fail = (message: string): never => {
    throw new AppError("validation", message);
  };
  const get = (evidenceId: string) => {
    const found = items.get(evidenceId);
    if (!found) throw new AppError("not_found", "La evidencia no existe.");
    return found;
  };
  const inCase = (caseId: string) => [...items.values()].filter((e) => e.caseId === caseId).sort((a, b) => a.position - b.position);
  const compact = (caseId: string) => inCase(caseId).forEach((e, position) => (e.position = position));
  const check = (description: string, annotations: string, render: Uint8Array) => {
    const message = validateDescription(description);
    if (message) fail(message);
    if (!parseAnnotations(annotations)) fail("Las anotaciones no tienen un formato válido.");
    if (!isPng(render)) fail("El render de la evidencia debe ser un PNG de hasta 2400 px por lado.");
  };
  const copy = (e: Evidence): Evidence => ({ ...e });
  const removeOne = (e: Evidence) => {
    files.delete(e.originalPath);
    files.delete(e.renderedPath);
    items.delete(e.id);
  };

  return {
    async stage(image) {
      if (!isImage(image)) fail("El archivo no es una imagen PNG, JPEG o WebP.");
      const path = `mem/evidence/_staging/${deps.id("stg")}.png`;
      staged.set(path, image);
      return { path, width: 1280, height: 720 };
    },
    async discardStaged(path) {
      staged.delete(path);
    },
    async readImage(path) {
      const bytes = staged.get(path) ?? files.get(path);
      if (!bytes) throw new AppError("not_found", "La imagen no existe.");
      return bytes;
    },
    imageUrl: (path, version) => `memory://${path}?v=${encodeURIComponent(version)}`,
    async list(caseId) {
      deps.caseProject(caseId);
      return inCase(caseId).map(copy);
    },
    async add(input, render) {
      check(input.description, input.annotations, render);
      const projectId = deps.caseProject(input.caseId);
      const original = staged.get(input.stagedPath) ?? fail("La ruta de la imagen no es válida.");
      const evidenceId = deps.id("ev");
      const ts = deps.now();
      const e: Evidence = {
        id: evidenceId,
        caseId: input.caseId,
        position: inCase(input.caseId).length,
        description: input.description.trim(),
        originalPath: `mem/evidence/${projectId}/${evidenceId}.orig.png`,
        renderedPath: `mem/evidence/${projectId}/${evidenceId}.render.png`,
        annotations: input.annotations,
        width: 1280,
        height: 720,
        capturedAt: ts,
        updatedAt: ts,
      };
      files.set(e.originalPath, original);
      files.set(e.renderedPath, render);
      staged.delete(input.stagedPath);
      items.set(e.id, e);
      deps.touch(projectId);
      return copy(e);
    },
    async update(input, render) {
      const e = get(input.evidenceId);
      check(input.description, input.annotations, render);
      files.set(e.renderedPath, render);
      Object.assign(e, { description: input.description.trim(), annotations: input.annotations, updatedAt: deps.now() });
      return copy(e);
    },
    async updateDescription(evidenceId, description) {
      const e = get(evidenceId);
      const message = validateDescription(description);
      if (message) fail(message);
      Object.assign(e, { description: description.trim(), updatedAt: deps.now() });
      return copy(e);
    },
    async move(evidenceId, targetCaseId) {
      const e = get(evidenceId);
      if (deps.caseProject(targetCaseId) !== deps.caseProject(e.caseId)) fail("Solo puedes mover evidencias entre casos de la misma matriz.");
      if (targetCaseId !== e.caseId) {
        const source = e.caseId;
        Object.assign(e, { caseId: targetCaseId, position: inCase(targetCaseId).length, updatedAt: deps.now() });
        compact(source);
      }
      return copy(e);
    },
    async reorder(caseId, orderedIds) {
      deps.caseProject(caseId);
      const current = inCase(caseId).map((e) => e.id).sort();
      if (JSON.stringify(current) !== JSON.stringify([...orderedIds].sort())) fail("La lista de evidencias cambió. Recarga el caso.");
      orderedIds.forEach((evidenceId, position) => (get(evidenceId).position = position));
      return inCase(caseId).map(copy);
    },
    async remove(evidenceId) {
      const e = get(evidenceId);
      removeOne(e);
      compact(e.caseId);
    },
    countFor: (caseId) => inCase(caseId).length,
    removeForCase: (caseId) => inCase(caseId).forEach(removeOne),
  };
}
