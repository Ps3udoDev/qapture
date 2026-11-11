import { describe, expect, it } from "vitest";
import type { CaseInput } from "../../domain/cases";
import { emptyDocVersion } from "../../domain/doc-versions";
import { defaultExportOptions } from "../../domain/export";
import { DEFAULT_SETTINGS } from "../../domain/settings";
import { toSelection } from "../../domain/capture";
import type { EvidenceUpdateInput, NewEvidenceInput } from "../../domain/evidence";
import type { FieldInput, NewProjectInput } from "../../domain/project";
import fixture from "./__fixtures__/contracts.json";
import { guionInspectSchema, guionPreviewSchema, importProfileSchema, profileDefinitionSchema, guionImportInputSchema, type GuionImportInputDto, originInfoSchema, originExportInputSchema, type OriginExportInputDto } from "./dto";
import { activitySchema, agentClientConfigSchema, appErrorSchema, backupSchema, capturedEventSchema, caseIssueLinkSchema, caseSchema, deviceCodeSchema, docVersionSchema, evidenceSchema, exportProgressSchema, exportResultSchema, fieldSchema, fixtureSchema, ghAccountSchema, ghConnectFailedSchema, ghRepoSchema, importSummarySchema, linkedRepoSchema, packageExportResultSchema, projectGithubInputSchema, projectGithubSchema, projectSchema, projectSummarySchema, repoMetaSchema, reportInputSchema, reportResultSchema, restoreResultSchema, settingsSnapshotSchema, stagedImageSchema, startupStatusSchema, templateSchema, updateInfoSchema, updateProgressSchema } from "./dto";

describe("contrato Rust → TS", () => {
  it("los ajustes antiguos sin agentes recuperan los permisos por defecto", () => {
    const settings = structuredClone(fixture.settings);
    Reflect.deleteProperty(settings.settings, "agents");
    expect(settingsSnapshotSchema.parse(settings).settings.agents).toEqual(DEFAULT_SETTINGS.agents);
    expect(settingsSnapshotSchema.parse({
      ...settings, settings: { ...settings.settings, agents: { read: true } },
    }).settings.agents).toEqual({ ...DEFAULT_SETTINGS.agents, read: true });
  });
  it.each([
    ["guionInspect", guionInspectSchema],
    ["guionPreview", guionPreviewSchema],
    ["importProfile", importProfileSchema],
    ["activity", activitySchema],
    ["agentClientConfig", agentClientConfigSchema],
    ["field", fieldSchema],
    ["template", templateSchema],
    ["projectSummary", projectSummarySchema],
    ["project", projectSchema],
    ["case", caseSchema],
    ["fixture", fixtureSchema],
    ["evidence", evidenceSchema],
    ["stagedImage", stagedImageSchema],
    ["error", appErrorSchema],
    ["capturedEvent", capturedEventSchema],
    ["captureError", appErrorSchema],
    ["docVersion", docVersionSchema],
    ["exportResult", exportResultSchema],
    ["exportProgress", exportProgressSchema],
    ["originInfo", originInfoSchema],
    ["lockedError", appErrorSchema],
    ["exportError", appErrorSchema],
    ["importSummary", importSummarySchema],
    ["packageExportResult", packageExportResultSchema],
    ["backup", backupSchema],
    ["restoreResult", restoreResultSchema],
    ["startupStatus", startupStatusSchema],
    ["corruptError", appErrorSchema],
    ["settings", settingsSnapshotSchema],
    ["updateInfo", updateInfoSchema],
    ["updateProgress", updateProgressSchema],
    ["ghAccount", ghAccountSchema],
    ["deviceCode", deviceCodeSchema],
    ["ghRepo", ghRepoSchema],
    ["linkedRepo", linkedRepoSchema],
    ["projectGithub", projectGithubSchema],
    ["ghConnectFailed", ghConnectFailedSchema],
    ["ghAuthError", appErrorSchema],
    ["caseIssueLink", caseIssueLinkSchema],
    ["repoMeta", repoMetaSchema],
    ["reportResult", reportResultSchema],
    ["commentResult", reportResultSchema],
  ] as const)("%s cumple el esquema estricto", (key, schema) => {
    expect(() => schema.parse(fixture[key])).not.toThrow();
  });
});

describe("contrato TS → Rust", () => {
  const sortedKeys = (o: object) => Object.keys(o).sort();

  it("los perfiles y las muestras de guion rechazan propiedades ajenas", () => {
    expect(() => profileDefinitionSchema.parse({ ...fixture.importProfile.definition, extra: 1 })).toThrow();
    const profile = structuredClone(fixture.importProfile.definition);
    Object.assign(profile.columns[0].target, { extra: 1 });
    expect(() => profileDefinitionSchema.parse(profile)).toThrow();
    expect(() => guionInspectSchema.parse({ ...fixture.guionInspect, extra: 1 })).toThrow();
    expect(() => guionPreviewSchema.parse({ ...fixture.guionPreview, extra: 1 })).toThrow();
    expect(() => importProfileSchema.parse({ ...fixture.importProfile, extra: 1 })).toThrow();
  });

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
    const nested = (o: object) => Object.fromEntries((Object.entries(o) as [string, object][]).map(([k, v]) => [k, sortedKeys(v)]));
    expect(sortedKeys(DEFAULT_SETTINGS)).toEqual(sortedKeys(fixture.inputs.saveSettings));
    expect(nested(DEFAULT_SETTINGS)).toEqual(nested(fixture.inputs.saveSettings));
    expect(() => projectGithubInputSchema.parse(fixture.inputs.projectGithubInput)).not.toThrow();
    expect(() => reportInputSchema.parse(fixture.inputs.reportInput)).not.toThrow();
    expect(sortedKeys(reportInputSchema.shape)).toEqual(sortedKeys(fixture.inputs.reportInput));
    const input: GuionImportInputDto = { token: "", profile: profileDefinitionSchema.parse(fixture.inputs.guionImportInput.profile), projectName: "", saveProfileAs: null };
    expect(sortedKeys(input)).toEqual(sortedKeys(fixture.inputs.guionImportInput));
    expect(() => guionImportInputSchema.parse(fixture.inputs.guionImportInput)).not.toThrow();
    const originExport: OriginExportInputDto = {
      tester: "",
      observations: "",
      includeEvidence: true,
      imageMaxWidth: 940,
    };
    expect(sortedKeys(originExport)).toEqual(sortedKeys(fixture.inputs.originExportInput));
    expect(() => originExportInputSchema.parse(fixture.inputs.originExportInput)).not.toThrow();
  });
});
