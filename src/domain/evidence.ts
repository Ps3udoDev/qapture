import { charLength } from "./fields";

export interface Evidence {
  id: string;
  caseId: string;
  position: number;
  description: string;
  originalPath: string;
  renderedPath: string;
  annotations: string;
  width: number;
  height: number;
  capturedAt: string;
  updatedAt: string;
}

export interface StagedImage {
  path: string;
  width: number;
  height: number;
}

export interface NewEvidenceInput {
  caseId: string;
  stagedPath: string;
  description: string;
  annotations: string;
}

export interface EvidenceUpdateInput {
  evidenceId: string;
  description: string;
  annotations: string;
}

export const MAX_DESCRIPTION = 2000;
export const ACCEPTED_IMAGE_TYPES = ["image/png", "image/jpeg", "image/webp"] as const;

export const isAcceptedImage = (file: { type: string }) => (ACCEPTED_IMAGE_TYPES as readonly string[]).includes(file.type);

export function validateDescription(raw: string): string | null {
  const value = raw.trim();
  if (!value) return "«Descripción» es obligatorio.";
  if (charLength(value) > MAX_DESCRIPTION) return `«Descripción» supera ${MAX_DESCRIPTION} caracteres.`;
  return null;
}

export const evidenceLabel = (e: Pick<Evidence, "position">) => `Evidencia ${e.position + 1}`;

export function moveItem<T>(items: T[], from: number, to: number): T[] {
  if (from < 0 || from >= items.length || to < 0 || to >= items.length || from === to) return [...items];
  const next = [...items];
  const [item] = next.splice(from, 1);
  next.splice(to, 0, item);
  return next;
}
