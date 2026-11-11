import { describe, expect, it } from "vitest";
import fixture from "../infrastructure/tauri/__fixtures__/contracts.json";
import { buildProfile, columnLetter, DEFAULT_STATUS_MAP, fold, statusValues, suggestTarget, type GuionSheet } from "./guion";

const HEADERS = ["Módulo", "SubMódulo", "ID", "Opción", "RN", "Escenario de Prueba", "Resultado Esperado", "Criticidad", "Fecha de Prueba", "Version", "Status", "Nombre del tester", "Condiciones"];
describe("sugerencias del guion", () => {
  it("sugiere destinos y tipos de las 13 columnas", () => {
    const values = Array.from({ length: 36 }, (_, i) => i % 2 ? "Importante" : "Crítico");
    const targets = HEADERS.map((header) => suggestTarget(header, values));
    expect(targets[2]).toEqual({ kind: "code" });
    expect(targets[5]).toEqual({ kind: "title" });
    expect(targets[10]).toEqual({ kind: "status" });
    expect(targets[7]).toMatchObject({ kind: "field", key: "criticidad", type: "list", options: ["Crítico", "Importante"] });
    expect(targets[8]).toMatchObject({ kind: "field", type: "date", role: "executionDate" });
    expect(targets[11]).toMatchObject({ kind: "field", role: "tester" });
    expect(targets[12]).toMatchObject({ kind: "field", role: "conditions" });
    expect(suggestTarget(" ", [])).toBeNull();
    expect(suggestTarget("Notas", ["a\nb"])).toMatchObject({ type: "longtext" });
    expect(fold("  Crìtico   Alto ")).toBe("critico alto");
    expect(columnLetter(26)).toBe("AA");
  });

  it("arma el perfil con la forma del contrato y conserva los estados originales", () => {
    const sheet: GuionSheet = { name: "Guión", rows: [HEADERS, ["", "", "CP01", "", "", "Caso", "", "", "", "", " ACEPTADO "]], suggestedHeaderRow: 1, cover: { executionDate: null, responsible: null, observations: null } };
    const profile = buildProfile(sheet, 1, HEADERS.map((h) => suggestTarget(h, [])), DEFAULT_STATUS_MAP.import, sheet.cover);
    expect(Object.keys(profile).sort()).toEqual(Object.keys(fixture.inputs.guionImportInput.profile).sort());
    expect(Object.keys(profile.columns[0].target).sort()).toEqual(Object.keys(fixture.inputs.guionImportInput.profile.columns[0].target).sort());
    expect(profile.columns[2]).toEqual({ column: "C", header: "ID", target: { kind: "code" } });
    expect(profile.statusMap.export).toEqual(DEFAULT_STATUS_MAP.export);
    expect(statusValues(sheet, 1, 10)).toEqual([" ACEPTADO "]);
  });
});
