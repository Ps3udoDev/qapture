import type { FormErrors } from "./cases";
import { charLength, isIsoDate, type FieldDef, type FieldType } from "./fields";

export interface ProjectMeta {
  name: string;
  client: string;
  testType: string;
  releaseVersion: string;
  startDate: string;
  endDate: string;
}

export interface Project extends ProjectMeta {
  id: string;
  nextSequence: number;
  sourceTemplateId: string | null;
  createdAt: string;
  updatedAt: string;
  fields: FieldDef[];
}

export interface ProjectSummary {
  id: string;
  name: string;
  client: string;
  caseCount: number;
  updatedAt: string;
}

export interface Template {
  id: string;
  name: string;
  testType: string;
  builtin: boolean;
  fields: FieldDef[];
}

export interface NewProjectInput {
  meta: ProjectMeta;
  templateId: string;
}

export interface FieldInput {
  id: string | null;
  label: string;
  type: FieldType;
  options: string[];
  required: boolean;
  clearOnDuplicate: boolean;
  width: number;
  archived: boolean;
}

export const emptyMeta = (): ProjectMeta => ({ name: "", client: "", testType: "", releaseVersion: "", startDate: "", endDate: "" });

export const projectToMeta = (p: ProjectMeta): ProjectMeta => ({
  name: p.name,
  client: p.client,
  testType: p.testType,
  releaseVersion: p.releaseVersion,
  startDate: p.startDate,
  endDate: p.endDate,
});

const LIMITS: [keyof ProjectMeta, string, number][] = [
  ["name", "Nombre del proyecto", 150],
  ["client", "Cliente", 150],
  ["testType", "Tipo de prueba", 100],
  ["releaseVersion", "Versión / release", 100],
];

export function validateProjectMeta(meta: ProjectMeta): FormErrors {
  const errors: FormErrors = {};
  if (!meta.name.trim()) errors.name = "«Nombre del proyecto» es obligatorio.";
  for (const [key, label, max] of LIMITS) {
    if (!errors[key] && charLength(meta[key].trim()) > max) errors[key] = `«${label}» supera ${max} caracteres.`;
  }
  const start = meta.startDate.trim();
  const end = meta.endDate.trim();
  if (start && !isIsoDate(start)) errors.startDate = "«Fecha de inicio» debe ser una fecha válida (AAAA-MM-DD).";
  if (end && !isIsoDate(end)) errors.endDate = "«Fecha de fin» debe ser una fecha válida (AAAA-MM-DD).";
  if (!errors.startDate && !errors.endDate && start && end && start > end) {
    errors.endDate = "La fecha de fin no puede ser anterior a la de inicio.";
  }
  return errors;
}

export const fieldToInput = (f: FieldDef): FieldInput => ({
  id: f.id,
  label: f.label,
  type: f.type,
  options: [...f.options],
  required: f.required,
  clearOnDuplicate: f.clearOnDuplicate,
  width: f.width,
  archived: f.archived,
});

export const newFieldInput = (): FieldInput => ({
  id: null,
  label: "",
  type: "text",
  options: [],
  required: false,
  clearOnDuplicate: false,
  width: 20,
  archived: false,
});

export function normalizeFieldInput(input: FieldInput): FieldInput {
  const options = input.type === "list" ? [...new Set(input.options.map((o) => o.trim()).filter(Boolean))] : [];
  return { ...input, label: input.label.trim(), options };
}

export function validateFieldInputs(inputs: FieldInput[]): FormErrors {
  const errors: FormErrors = {};
  if (inputs.length > 60) errors._form = "Un proyecto admite como máximo 60 columnas.";
  const seen = new Set<string>();
  inputs.forEach((raw, index) => {
    const input = normalizeFieldInput(raw);
    const key = String(index);
    if (!input.label) {
      errors[key] = `La columna ${index + 1} necesita un nombre.`;
      return;
    }
    if (!input.archived) {
      const folded = input.label.toLowerCase();
      if (seen.has(folded)) {
        errors[key] = `La columna «${input.label}» está repetida.`;
        return;
      }
      seen.add(folded);
    }
    if (input.type === "list" && input.options.length === 0) {
      errors[key] = `La lista «${input.label}» necesita al menos una opción.`;
    } else if (input.width < 8 || input.width > 100) {
      errors[key] = `El ancho de «${input.label}» debe estar entre 8 y 100.`;
    }
  });
  return errors;
}
