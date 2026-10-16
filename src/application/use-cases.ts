import { normalizeCaseInput, validateCaseInput, type CaseInput, type CaseRecord, type FormErrors } from "../domain/cases";
import type { FieldDef } from "../domain/fields";
import {
  normalizeFieldInput,
  validateFieldInputs,
  validateProjectMeta,
  type FieldInput,
  type NewProjectInput,
  type Project,
  type ProjectMeta,
  type Template,
} from "../domain/project";
import { isValidationError } from "../ports/errors";
import type { Repositories } from "../ports/repositories";

export type Outcome<T> = { ok: true; value: T } | { ok: false; errors: FormErrors };

const invalid = (errors: FormErrors) => Object.keys(errors).length > 0;

async function guard<T>(run: () => Promise<T>): Promise<Outcome<T>> {
  try {
    return { ok: true, value: await run() };
  } catch (error) {
    if (isValidationError(error)) return { ok: false, errors: { _form: error.message } };
    throw error;
  }
}

export async function createProject(repos: Repositories, input: NewProjectInput): Promise<Outcome<Project>> {
  const errors = validateProjectMeta(input.meta);
  if (invalid(errors)) return { ok: false, errors };
  return guard(() => repos.projects.create(input));
}

export async function updateProjectMeta(repos: Repositories, projectId: string, meta: ProjectMeta): Promise<Outcome<Project>> {
  const errors = validateProjectMeta(meta);
  if (invalid(errors)) return { ok: false, errors };
  return guard(() => repos.projects.updateMeta(projectId, meta));
}

export async function saveProjectFields(repos: Repositories, projectId: string, fields: FieldInput[]): Promise<Outcome<FieldDef[]>> {
  const errors = validateFieldInputs(fields);
  if (invalid(errors)) return { ok: false, errors };
  return guard(() => repos.projects.saveFields(projectId, fields.map(normalizeFieldInput)));
}

export async function saveCase(repos: Repositories, project: Project, input: CaseInput, caseId?: string): Promise<Outcome<CaseRecord>> {
  const errors = validateCaseInput(project.fields, input);
  if (invalid(errors)) return { ok: false, errors };
  const clean = normalizeCaseInput(project.fields, input);
  return guard(() => (caseId ? repos.cases.update(caseId, clean) : repos.cases.create(project.id, clean)));
}

export async function saveTemplate(repos: Repositories, projectId: string, name: string, testType: string): Promise<Outcome<Template>> {
  if (!name.trim()) return { ok: false, errors: { name: "«Nombre de la plantilla» es obligatorio." } };
  return guard(() => repos.templates.saveFromProject(projectId, name, testType));
}
