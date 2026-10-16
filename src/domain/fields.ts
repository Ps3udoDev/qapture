export const FIELD_TYPES = ["text", "longtext", "list", "date", "number", "boolean"] as const;
export type FieldType = (typeof FIELD_TYPES)[number];

export const FIELD_TYPE_LABELS: Record<FieldType, string> = {
  text: "Texto",
  longtext: "Texto largo",
  list: "Lista",
  date: "Fecha",
  number: "Número",
  boolean: "Sí / No",
};

export interface FieldDef {
  id: string;
  key: string;
  label: string;
  type: FieldType;
  options: string[];
  required: boolean;
  clearOnDuplicate: boolean;
  width: number;
  position: number;
  archived: boolean;
}

export const MAX_LENGTH: Record<FieldType, number> = {
  text: 500,
  longtext: 16000,
  list: 100,
  date: 10,
  number: 40,
  boolean: 5,
};

const NUMBER_RE = /^[+-]?(\d+(\.\d*)?|\.\d+)([eE][+-]?\d+)?$/;
const COMMA_DECIMAL_RE = /^[+-]?\d+,\d+$/;

/** Igual que `chars().count()` en Rust: cuenta puntos de código, no unidades UTF-16. */
export const charLength = (value: string) => [...value].length;

export function normalizeValue(type: FieldType, raw: string): string {
  if (type === "text" || type === "longtext") return raw.trim() === "" ? "" : raw;
  const v = raw.trim();
  if (type === "number" && COMMA_DECIMAL_RE.test(v)) return v.replace(",", ".");
  return v;
}

export function isIsoDate(value: string): boolean {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(value)) return false;
  const [y, m, d] = value.split("-").map(Number);
  // `Date.UTC` mapea los años 0-99 a 1900-1999; chrono (Rust) los acepta tal cual.
  // `setUTCFullYear` no tiene ese comportamiento.
  const date = new Date(0);
  date.setUTCFullYear(y, m - 1, d);
  return date.getUTCFullYear() === y && date.getUTCMonth() === m - 1 && date.getUTCDate() === d;
}

export function validateValue(field: Pick<FieldDef, "label" | "type" | "options" | "required">, raw: string): string | null {
  const v = normalizeValue(field.type, raw);
  const label = field.label;
  if (v.trim() === "") return field.required ? `«${label}» es obligatorio.` : null;
  const max = MAX_LENGTH[field.type];
  if (charLength(v) > max) return `«${label}» supera ${max} caracteres.`;
  switch (field.type) {
    case "text":
    case "longtext":
      return null;
    case "list":
      return field.options.includes(v) ? null : `«${label}» debe ser una de las opciones de la lista.`;
    case "date":
      return isIsoDate(v) ? null : `«${label}» debe ser una fecha válida (AAAA-MM-DD).`;
    case "number":
      return NUMBER_RE.test(v) && Number.isFinite(Number(v)) ? null : `«${label}» debe ser un número.`;
    case "boolean":
      return v === "true" || v === "false" ? null : `«${label}» debe ser Sí o No.`;
  }
}

const byPosition = (a: FieldDef, b: FieldDef) => a.position - b.position;
export const activeFields = (fields: FieldDef[]) => fields.filter((f) => !f.archived).sort(byPosition);
export const archivedFields = (fields: FieldDef[]) => fields.filter((f) => f.archived).sort(byPosition);
