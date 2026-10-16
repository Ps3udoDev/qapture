import { z } from "zod";
import type { Repositories } from "../../ports/repositories";
import { call } from "./client";
import { caseSchema, fieldSchema, projectSchema, projectSummarySchema, templateSchema } from "./dto";

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
  };
}
