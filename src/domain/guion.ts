import type { FieldType } from "./fields";
import { STATUSES, type Status } from "./status";

export type FieldRole = "executionDate" | "tester" | "conditions";
export type Target = { kind: "code" } | { kind: "title" } | { kind: "status" } | {
  kind: "field"; key: string; label: string; type: "text" | "longtext" | "list" | "date"; options: string[]; role: FieldRole | null;
};
export interface ColumnMap { column: string; header: string; target: Target }
export interface CoverCells { executionDate: string | null; responsible: string | null; observations: string | null }
export interface ProfileDefinition {
  version: number; sheet: string; headerRow: number; columns: ColumnMap[];
  statusMap: { import: Record<string, string>; export: Record<string, string> }; cover: CoverCells;
}
export interface GuionSheet { name: string; rows: string[][]; suggestedHeaderRow: number | null; cover: CoverCells }
export interface GuionInspection { token: string; fileName: string; sheets: GuionSheet[]; suggestedProfileId: string | null }
export interface GuionPreview {
  cases: { row: number; code: string; title: string; status: Status; conditioned: boolean }[];
  fields: { key: string; label: string; type: FieldType; options: string[] }[];
  warnings: string[]; errors: string[];
}
export interface ImportProfile { id: string; name: string; definition: ProfileDefinition; createdAt: string; updatedAt: string }
export interface GuionImportInput { token: string; profile: ProfileDefinition; projectName: string; saveProfileAs: string | null }

export const QAPTURE_STATUS_TARGETS = [...STATUSES.map((value) => ({ value, label: value })), { value: "Aprobado+condicionado", label: "Aprobado + Condicionado" }];
export const DEFAULT_STATUS_MAP: ProfileDefinition["statusMap"] = {
  import: { aceptado: "Aprobado", aprobado: "Aprobado", rechazado: "Fallido", condicionado: "Aprobado+condicionado", "": "No ejecutado" },
  export: { Aprobado: "Aceptado", "Aprobado+condicionado": "Condicionado", Fallido: "Rechazado" },
};

/** Mismo mapa de acentos que domain::text::fold en Rust. */
export function fold(value: string): string {
  return value.toLowerCase().replace(/[áàäâ]/g, "a").replace(/[éèëê]/g, "e").replace(/[íìïî]/g, "i").replace(/[óòöô]/g, "o").replace(/[úùüû]/g, "u").replace(/ñ/g, "n").trim().replace(/\s+/g, " ");
}
function slug(value: string): string {
  return fold(value).replace(/[^a-z0-9]+/g, "_").replace(/^_|_$/g, "").slice(0, 40).replace(/_$/, "") || "campo";
}
export function columnLetter(index: number): string {
  let value = index + 1;
  let result = "";
  while (value > 0) { result = String.fromCharCode(65 + (value - 1) % 26) + result; value = Math.floor((value - 1) / 26); }
  return result;
}
export function suggestTarget(header: string, values: string[]): Target | null {
  const key = fold(header);
  if (!key) return null;
  if (["id", "codigo", "cp"].includes(key)) return { kind: "code" };
  if (["escenario", "titulo", "caso de prueba"].some((part) => key.includes(part))) return { kind: "title" };
  if (["status", "estado"].includes(key)) return { kind: "status" };
  const present = values.filter((v) => v.trim());
  const distinct = [...new Set(present.map((v) => v.trim()))];
  const type = key.includes("fecha") ? "date" : present.some((v) => v.includes("\n") || [...v].length > 500) ? "longtext" : present.length >= 10 && distinct.length >= 2 && distinct.length <= 6 ? "list" : "text";
  const role = key.includes("fecha") && type === "date" ? "executionDate" : key.includes("tester") ? "tester" : key.includes("condicion") ? "conditions" : null;
  return { kind: "field", key: slug(header), label: header.trim(), type, options: type === "list" ? distinct : [], role };
}
export function buildProfile(sheet: GuionSheet, headerRow: number, targets: (Target | null)[], statusImport: Record<string, string>, cover: CoverCells): ProfileDefinition {
  return {
    version: 1, sheet: sheet.name, headerRow,
    columns: targets.flatMap((target, i) => target ? [{ column: columnLetter(i), header: sheet.rows[headerRow - 1]?.[i] ?? "", target: structuredClone(target) }] : []),
    statusMap: { import: { ...statusImport }, export: { ...DEFAULT_STATUS_MAP.export } }, cover: { ...cover },
  };
}
export function statusValues(sheet: GuionSheet, headerRow: number, column: number): string[] {
  return [...new Set(sheet.rows.slice(headerRow).filter((row) => row.some((v) => v.trim())).map((row) => row[column] ?? ""))];
}
