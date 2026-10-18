import { convertFileSrc } from "@tauri-apps/api/core";
import { z } from "zod";
import type { Repositories } from "../../ports/repositories";
import { tauriScreenCapture } from "./capture";
import { call, callBytes } from "./client";
import { caseSchema, evidenceSchema, fieldSchema, projectSchema, projectSummarySchema, stagedImageSchema, templateSchema } from "./dto";
import { tauriData } from "./data";
import { tauriDocVersions, tauriExporter } from "./export";
import { encodeFrame } from "./frame";
import { tauriSettings } from "./settings";

const nothing = z.null();

export function tauriRepositories(): Repositories {
  return {
    templates: {
      list: () => call("list_templates", {}, z.array(templateSchema)),
      saveFromProject: (projectId, name, testType) =>
        call("save_template_from_project", { projectId, name, testType }, templateSchema),
      remove: async (id) => {
        await call("delete_template", { id }, nothing);
      },
    },
    projects: {
      list: () => call("list_projects", {}, z.array(projectSummarySchema)),
      get: (id) => call("get_project", { id }, projectSchema),
      create: (input) => call("create_project", { input }, projectSchema),
      updateMeta: (id, input) => call("update_project", { id, input }, projectSchema),
      remove: async (id) => {
        await call("delete_project", { id }, nothing);
      },
      saveFields: (projectId, fields) => call("save_project_fields", { projectId, fields }, z.array(fieldSchema)),
    },
    cases: {
      list: (projectId) => call("list_cases", { projectId }, z.array(caseSchema)),
      create: (projectId, input) => call("create_case", { projectId, input }, caseSchema),
      update: (caseId, input) => call("update_case", { caseId, input }, caseSchema),
      duplicate: (caseId) => call("duplicate_case", { caseId }, caseSchema),
      remove: async (caseId) => {
        await call("delete_case", { caseId }, nothing);
      },
    },
    evidence: {
      stage: (image) => call("stage_evidence_image", image, stagedImageSchema),
      discardStaged: async (path) => {
        await call("discard_staged_image", { path }, nothing);
      },
      readImage: (path) => callBytes("read_evidence_image", { path }),
      // `?v=` evita que el webview muestre un render viejo tras re-anotar.
      imageUrl: (path, version) => `${convertFileSrc(path)}?v=${encodeURIComponent(version)}`,
      list: (caseId) => call("list_evidence", { caseId }, z.array(evidenceSchema)),
      add: (input, render) => call("add_evidence", encodeFrame(input, render), evidenceSchema),
      update: (input, render) => call("update_evidence", encodeFrame(input, render), evidenceSchema),
      updateDescription: (evidenceId, description) => call("update_evidence_description", { evidenceId, description }, evidenceSchema),
      move: (evidenceId, targetCaseId) => call("move_evidence", { evidenceId, targetCaseId }, evidenceSchema),
      reorder: (caseId, orderedIds) => call("reorder_evidence", { caseId, orderedIds }, z.array(evidenceSchema)),
      remove: async (evidenceId) => {
        await call("delete_evidence", { evidenceId }, nothing);
      },
    },
    capture: tauriScreenCapture(),
    docVersions: tauriDocVersions(),
    exporter: tauriExporter(),
    data: tauriData(),
    settings: tauriSettings(),
  };
}
