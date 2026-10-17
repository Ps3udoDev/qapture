import { describe, expect, it } from "vitest";
import type { CaseInput } from "../../domain/cases";
import { emptyDocVersion } from "../../domain/doc-versions";
import { defaultExportOptions } from "../../domain/export";
import { toSelection } from "../../domain/capture";
import type { EvidenceUpdateInput, NewEvidenceInput } from "../../domain/evidence";
import type { FieldInput, NewProjectInput } from "../../domain/project";
import fixture from "./__fixtures__/contracts.json";
import { appErrorSchema, backupSchema, capturedEventSchema, caseSchema, docVersionSchema, evidenceSchema, exportProgressSchema, exportResultSchema, fieldSchema, importSummarySchema, packageExportResultSchema, projectSchema, projectSummarySchema, restoreResultSchema, stagedImageSchema, startupStatusSchema, templateSchema } from "./dto";

describe("contrato Rust → TS", () => {
  it.each([
    ["field", fieldSchema],
    ["template", templateSchema],
    ["projectSummary", projectSummarySchema],
    ["project", projectSchema],
    ["case", caseSchema],
    ["evidence", evidenceSchema],
    ["stagedImage", stagedImageSchema],
    ["error", appErrorSchema],
    ["capturedEvent", capturedEventSchema],
    ["captureError", appErrorSchema],
    ["docVersion", docVersionSchema],
    ["exportResult", exportResultSchema],
    ["exportProgress", exportProgressSchema],
    ["lockedError", appErrorSchema],
    ["exportError", appErrorSchema],
    ["importSummary", importSummarySchema],
    ["packageExportResult", packageExportResultSchema],
    ["backup", backupSchema],
    ["restoreResult", restoreResultSchema],
    ["startupStatus", startupStatusSchema],
    ["corruptError", appErrorSchema],
  ] as const)("%s cumple el esquema estricto", (key, schema) => {
    expect(() => schema.parse(fixture[key])).not.toThrow();
  });
});

describe("contrato TS → Rust", () => {
  const sortedKeys = (o: object) => Object.keys(o).sort();

  it("los inputs de TS tienen exactamente las claves que Rust acepta", () => {
    const meta = { name: "", client: "", testType: "", releaseVersion: "", startDate: "", endDate: "" };
    const newProject: NewProjectInput = { meta, templateId: "" };
    const field: FieldInput = { id: null, label: "", type: "text", options: [], required: false, clearOnDuplicate: false, width: 8, archived: false };
    const caseInput: CaseInput = { title: "", status: "Aprobado", values: {} };
    expect(sortedKeys(newProject)).toEqual(sortedKeys(fixture.inputs.newProjectInput));
    expect(sortedKeys(meta)).toEqual(sortedKeys(fixture.inputs.projectMetaInput));
    expect(sortedKeys(field)).toEqual(sortedKeys(fixture.inputs.fieldInput));
    expect(sortedKeys(caseInput)).toEqual(sortedKeys(fixture.inputs.caseInput));
    const newEvidence: NewEvidenceInput = { caseId: "", stagedPath: "", description: "", annotations: "" };
    const evidenceUpdate: EvidenceUpdateInput = { evidenceId: "", description: "", annotations: "" };
    expect(sortedKeys(newEvidence)).toEqual(sortedKeys(fixture.inputs.newEvidenceMeta));
    expect(sortedKeys(evidenceUpdate)).toEqual(sortedKeys(fixture.inputs.evidenceUpdateMeta));
    const selection = toSelection({ x: 0, y: 0, width: 1, height: 1 }, { width: 1, height: 1 });
    expect(sortedKeys(selection)).toEqual(sortedKeys(fixture.inputs.selection));
    expect(sortedKeys(emptyDocVersion(""))).toEqual(sortedKeys(fixture.inputs.docVersionInput));
    expect(sortedKeys(defaultExportOptions(""))).toEqual(sortedKeys(fixture.inputs.exportOptions));
  });
});
