import { activeFields, charLength, normalizeValue, validateValue, type FieldDef } from "./fields";
import { DEFAULT_STATUS, isStatus, type Status } from "./status";

export interface CaseRecord {
  id: string;
  projectId: string;
  code: string;
  title: string;
  status: Status;
  values: Record<string, string>;
  createdAt: string;
  updatedAt: string;
}

export interface CaseInput {
  title: string;
  status: Status;
  values: Record<string, string>;
}

export type FormErrors = Record<string, string>;

export function validateCaseInput(fields: FieldDef[], input: CaseInput): FormErrors {
  const errors: FormErrors = {};
  const title = input.title.trim();
  if (!title) errors.title = "«Caso de prueba» es obligatorio.";
  else if (charLength(title) > 300) errors.title = "«Caso de prueba» supera 300 caracteres.";
  if (!isStatus(input.status)) errors.status = "Estado inválido.";
  for (const f of activeFields(fields)) {
    const message = validateValue(f, input.values[f.id] ?? "");
    if (message) errors[f.id] = message;
  }
  return errors;
}

export function normalizeCaseInput(fields: FieldDef[], input: CaseInput): CaseInput {
  const values: Record<string, string> = {};
  for (const f of activeFields(fields)) {
    const v = normalizeValue(f.type, input.values[f.id] ?? "");
    if (v !== "") values[f.id] = v;
  }
  return { title: input.title.trim(), status: input.status, values };
}

export function emptyCaseInput(fields: FieldDef[], defaults: Record<string, string> = {}): CaseInput {
  return {
    title: "",
    status: DEFAULT_STATUS,
    values: Object.fromEntries(activeFields(fields).map((f) => [f.id, defaults[f.key] ?? ""])),
  };
}

export function caseToInput(fields: FieldDef[], record: CaseRecord): CaseInput {
  return {
    title: record.title,
    status: record.status,
    values: Object.fromEntries(activeFields(fields).map((f) => [f.id, record.values[f.id] ?? ""])),
  };
}

const fold = (value: string) => value.normalize("NFD").replace(/\p{Diacritic}/gu, "").toLowerCase();

export function filterCases(
  cases: CaseRecord[],
  fields: FieldDef[],
  filter: { search: string; status: Status | "" },
): CaseRecord[] {
  const query = fold(filter.search.trim());
  const active = activeFields(fields);
  return cases.filter(
    (c) =>
      (!filter.status || c.status === filter.status) &&
      (!query ||
        fold([c.code, c.title, ...active.map((f) => c.values[f.id] ?? "")].join("\n")).includes(query)),
  );
}

export const formatCode = (sequence: number) => `EJ-${String(sequence).padStart(3, "0")}`;
