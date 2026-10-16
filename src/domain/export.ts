import type { FormErrors } from "./cases";
import { charLength } from "./fields";

/** Ancho máximo de las capturas en la hoja Evidencias. F6 lo llevará a Settings. */
export const IMAGE_WIDTH = { min: 400, max: 1600, default: 940 } as const;

export type ExportKind = "project" | "template";

export interface ExportOptions {
  tester: string;
  /** Obligatoria para el proyecto; la plantilla vacía no lleva versión en el nombre. */
  docVersionId: string | null;
  includeEvidence: boolean;
  includeSummary: boolean;
  imageMaxWidth: number;
}

export interface ExportResult {
  path: string;
  fileName: string;
  cases: number;
  evidence: number;
  missingImages: number;
}

export const EXPORT_STAGES = ["preparing", "images", "writing", "saving"] as const;
export type ExportStage = (typeof EXPORT_STAGES)[number];

export interface ExportProgress {
  stage: ExportStage;
  current: number;
  total: number;
}

export const defaultExportOptions = (tester: string): ExportOptions => ({
  tester,
  docVersionId: null,
  includeEvidence: true,
  includeSummary: true,
  imageMaxWidth: IMAGE_WIDTH.default,
});

/** Espejo de `excel::data::resolve_options` para avisar antes de abrir «Guardar como». */
export function validateExportOptions(options: ExportOptions, kind: ExportKind): FormErrors {
  const errors: FormErrors = {};
  const tester = options.tester.trim();
  if (!tester) errors.tester = "«Tester» es obligatorio.";
  else if (charLength(tester) > 100) errors.tester = "«Tester» supera 100 caracteres.";
  if (kind === "project" && !options.docVersionId) errors.docVersionId = "Elige la versión del documento.";
  const width = options.imageMaxWidth;
  if (!Number.isInteger(width) || width < IMAGE_WIDTH.min || width > IMAGE_WIDTH.max) {
    errors.imageMaxWidth = `El ancho máximo de imagen debe estar entre ${IMAGE_WIDTH.min} y ${IMAGE_WIDTH.max} px.`;
  }
  return errors;
}

export function progressLabel(progress: ExportProgress | null): string {
  if (!progress) return "";
  switch (progress.stage) {
    case "preparing":
      return "Preparando los datos…";
    case "images":
      return `Procesando imágenes ${progress.current} de ${progress.total}…`;
    case "writing":
      return "Escribiendo el libro…";
    case "saving":
      return "Guardando el archivo…";
  }
}

/** 0–100 para la barra de progreso; las etapas sin conteo avanzan por tramos. */
export function progressPercent(progress: ExportProgress | null): number {
  if (!progress) return 0;
  const base = { preparing: 0, images: 5, writing: 85, saving: 95 }[progress.stage];
  if (progress.stage !== "images" || progress.total === 0) return base;
  return Math.round(base + (80 * progress.current) / progress.total);
}
