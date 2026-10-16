import type { FormErrors } from "./cases";
import { charLength, isIsoDate } from "./fields";

/** Una fila del control de versiones del documento (UC-05). */
export interface DocVersionInput {
  version: string;
  date: string;
  author: string;
  reviewer: string;
  approver: string;
  changes: string;
}

export interface DocVersion extends DocVersionInput {
  id: string;
  projectId: string;
  createdAt: string;
}

/** Fecha local de hoy en AAAA-MM-DD (no UTC: a las 21:00 en México sigue siendo hoy). */
export function todayIso(now = new Date()): string {
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`;
}

export const emptyDocVersion = (date: string, author = ""): DocVersionInput => ({
  version: "",
  date,
  author,
  reviewer: "",
  approver: "",
  changes: "",
});

const LIMITS: [keyof DocVersionInput, string, number][] = [
  ["version", "Versión", 20],
  ["author", "Elaborado por", 100],
  ["reviewer", "Revisado por", 100],
  ["approver", "Aprobado por", 100],
  ["changes", "Cambios", 1000],
];

/** Espejo de `repo/doc_versions.rs::validate` (la unicidad la revisa Rust). */
export function validateDocVersion(input: DocVersionInput): FormErrors {
  const errors: FormErrors = {};
  if (!input.version.trim()) errors.version = "«Versión» es obligatorio.";
  if (!input.date.trim()) errors.date = "«Fecha» es obligatorio.";
  else if (!isIsoDate(input.date.trim())) errors.date = "«Fecha» debe ser una fecha válida (AAAA-MM-DD).";
  if (!input.author.trim()) errors.author = "«Elaborado por» es obligatorio.";
  for (const [key, label, max] of LIMITS) {
    if (!errors[key] && charLength(input[key].trim()) > max) errors[key] = `«${label}» supera ${max} caracteres.`;
  }
  return errors;
}

export const trimDocVersion = (input: DocVersionInput): DocVersionInput => ({
  version: input.version.trim(),
  date: input.date.trim(),
  author: input.author.trim(),
  reviewer: input.reviewer.trim(),
  approver: input.approver.trim(),
  changes: input.changes.trim(),
});

/** AAAA-MM-DD → DD/MM/AAAA (como en el Excel); cualquier otro texto se muestra tal cual. */
export function formatIsoDate(value: string): string {
  if (!isIsoDate(value)) return value;
  const [y, m, d] = value.split("-");
  return `${d}/${m}/${y}`;
}

/** La lista llega ordenada por fecha (como la devuelve Rust): la última es la vigente. */
export const latestVersion = (versions: DocVersion[]): DocVersion | null => versions.at(-1) ?? null;

/** «1.0» si no hay versiones; si la última es `X.Y` (con o sin «v»), sugiere `X.(Y+1)`. */
export function suggestNextVersion(versions: DocVersion[]): string {
  const last = latestVersion(versions);
  if (!last) return "1.0";
  const match = /^(v?)(\d+)\.(\d+)$/i.exec(last.version.trim());
  return match ? `${match[1]}${match[2]}.${Number(match[3]) + 1}` : "";
}
