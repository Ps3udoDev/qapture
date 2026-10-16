import { describe, expect, it } from "vitest";
import {
  caseToInput,
  emptyCaseInput,
  filterCases,
  formatCode,
  normalizeCaseInput,
  validateCaseInput,
  type CaseRecord,
} from "./cases";
import type { FieldDef } from "./fields";

const fields: FieldDef[] = [
  { id: "env", key: "entorno", label: "Entorno", type: "text", options: [], required: true, clearOnDuplicate: false, width: 21, position: 0, archived: false },
  { id: "mod", key: "modulo", label: "Módulo", type: "text", options: [], required: false, clearOnDuplicate: false, width: 22, position: 1, archived: true },
  { id: "obs", key: "observaciones", label: "Observaciones", type: "longtext", options: [], required: false, clearOnDuplicate: true, width: 40, position: 2, archived: false },
];

const record = (over: Partial<CaseRecord>): CaseRecord => ({
  id: "c1",
  projectId: "p1",
  code: "EJ-001",
  title: "Aplicación de recibos",
  status: "Aprobado",
  values: {},
  createdAt: "",
  updatedAt: "",
  ...over,
});

describe("validateCaseInput", () => {
  it("valida título, estado y campos activos", () => {
    const errors = validateCaseInput(fields, { title: " ", status: "Listo" as never, values: { mod: "" } });
    expect(errors).toEqual({
      title: "«Caso de prueba» es obligatorio.",
      status: "Estado inválido.",
      env: "«Entorno» es obligatorio.",
    });
  });

  it("acepta un caso completo", () => {
    expect(validateCaseInput(fields, { title: "Caso", status: "Aprobado", values: { env: "QA" } })).toEqual({});
  });
});

describe("normalizeCaseInput", () => {
  it("recorta el título, descarta vacíos y columnas archivadas", () => {
    expect(
      normalizeCaseInput(fields, { title: "  Caso ", status: "Aprobado", values: { env: "QA", mod: "x", obs: "  " } }),
    ).toEqual({ title: "Caso", status: "Aprobado", values: { env: "QA" } });
  });
});

describe("formularios", () => {
  it("emptyCaseInput usa defaults por clave", () => {
    expect(emptyCaseInput(fields, { entorno: "QA" })).toEqual({
      title: "",
      status: "No ejecutado",
      values: { env: "QA", obs: "" },
    });
  });

  it("caseToInput solo toma columnas activas", () => {
    expect(caseToInput(fields, record({ values: { env: "QA", mod: "Cobranzas" } })).values).toEqual({ env: "QA", obs: "" });
  });
});

describe("filterCases", () => {
  const cases = [
    record({ id: "a", code: "EJ-001", title: "Aplicación de RECIBOS", status: "Aprobado" }),
    record({ id: "b", code: "EJ-002", title: "Emisión", status: "Fallido", values: { env: "Máquina Virtual" } }),
  ];

  it("busca sin distinguir mayúsculas ni acentos, también en valores", () => {
    expect(filterCases(cases, { search: "recíb", status: "" }).map((c) => c.id)).toEqual(["a"]);
    expect(filterCases(cases, { search: "maquina", status: "" }).map((c) => c.id)).toEqual(["b"]);
    expect(filterCases(cases, { search: "ej-002", status: "" }).map((c) => c.id)).toEqual(["b"]);
  });

  it("filtra por estado", () => {
    expect(filterCases(cases, { search: "", status: "Fallido" }).map((c) => c.id)).toEqual(["b"]);
  });
});

it("formatCode rellena a 3 dígitos", () => {
  expect(formatCode(7)).toBe("EJ-007");
  expect(formatCode(1234)).toBe("EJ-1234");
});
