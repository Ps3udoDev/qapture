import { formatCode, normalizeCaseInput, validateCaseInput, type CaseInput, type CaseRecord } from "../../domain/cases";
import { activeFields, type FieldDef } from "../../domain/fields";
import { emptyMeta, normalizeFieldInput, validateFieldInputs, validateProjectMeta, type Project, type ProjectMeta, type Template } from "../../domain/project";
import { DEFAULT_STATUS } from "../../domain/status";
import { AppError } from "../../ports/errors";
import type { Repositories } from "../../ports/repositories";
import { createMemoryCapture, type MemoryCapture } from "./capture";
import { createMemoryData, type MemoryData } from "./data";
import { createMemoryEvidence, type MemoryEvidence } from "./evidence";
import { createMemoryDocVersions, createMemoryExporter, type MemoryDocVersions, type MemoryExporter } from "./export";
import { builtinTemplates } from "./fixtures";
import { createMemoryGithub, type MemoryGithub } from "./github";
import { createMemorySettings, type MemorySettings } from "./settings";
import { createMemoryUpdater, type MemoryUpdater } from "./updater";

export interface MemoryRepositories extends Repositories {
  evidence: MemoryEvidence;
  capture: MemoryCapture;
  docVersions: MemoryDocVersions;
  exporter: MemoryExporter;
  data: MemoryData;
  settings: MemorySettings;
  updater: MemoryUpdater;
  github: MemoryGithub;
}

/** Implementación en memoria para tests de la aplicación y de la UI. Replica las reglas principales de Rust. */
export function createMemoryRepositories(): MemoryRepositories {
  let seq = 0;
  const id = (prefix: string) => `${prefix}-${++seq}`;
  let tick = 0;
  const now = () => new Date(Date.UTC(2026, 9, 15, 12, 0, 0, tick++)).toISOString();
  const copy = <T>(value: T): T => structuredClone(value);
  const fail = (message: string): never => {
    throw new AppError("validation", message);
  };
  const firstError = (errors: Record<string, string>) => {
    const message = Object.values(errors)[0];
    if (message) fail(message);
  };

  const templates: Template[] = copy(builtinTemplates);
  const projects = new Map<string, Project>();
  const cases = new Map<string, CaseRecord>();

  const getProject = (projectId: string): Project => {
    const p = projects.get(projectId);
    if (!p) throw new AppError("not_found", "El proyecto no existe.");
    return p;
  };
  const getCase = (caseId: string): CaseRecord => {
    const c = cases.get(caseId);
    if (!c) throw new AppError("not_found", "El caso no existe.");
    return c;
  };
  const touch = (p: Project) => {
    p.updatedAt = now();
  };
  const evidence = createMemoryEvidence({
    caseProject: (caseId) => getCase(caseId).projectId,
    touch: (projectId) => touch(getProject(projectId)),
    id,
    now,
  });
  const docVersions = createMemoryDocVersions({
    ensureProject: (projectId) => void getProject(projectId),
    touch: (projectId) => touch(getProject(projectId)),
    id,
    now,
  });
  const withCount = (c: CaseRecord): CaseRecord => ({ ...copy(c), evidenceCount: evidence.countFor(c.id) });
  const trimMeta = (m: ProjectMeta): ProjectMeta => ({
    name: m.name.trim(),
    client: m.client.trim(),
    testType: m.testType.trim(),
    releaseVersion: m.releaseVersion.trim(),
    startDate: m.startDate.trim(),
    endDate: m.endDate.trim(),
  });
  const checkCase = (p: Project, input: CaseInput) => {
    const active = new Set(activeFields(p.fields).map((f) => f.id));
    if (Object.keys(input.values).some((k) => !active.has(k))) fail("La columna enviada no pertenece al proyecto. Recarga el proyecto.");
    firstError(validateCaseInput(p.fields, input));
    return normalizeCaseInput(p.fields, input);
  };
  const projectCases = (projectId: string) => [...cases.values()].filter((c) => c.projectId === projectId);

  const repos: MemoryRepositories = {
    templates: {
      async list() {
        return copy([...templates].sort((a, b) => Number(b.builtin) - Number(a.builtin) || a.name.localeCompare(b.name)));
      },
      async saveFromProject(projectId, name, testType) {
        const p = getProject(projectId);
        const clean = name.trim();
        if (!clean) fail("«Nombre de la plantilla» es obligatorio.");
        if (templates.some((t) => t.name.toLowerCase() === clean.toLowerCase())) fail("Ya existe una plantilla con ese nombre.");
        const template: Template = {
          id: id("tpl"),
          name: clean,
          testType: testType.trim(),
          builtin: false,
          fields: activeFields(p.fields).map((f, position) => ({ ...f, id: id("tf"), position, archived: false })),
        };
        templates.push(template);
        return copy(template);
      },
      async remove(templateId) {
        const t = templates.find((x) => x.id === templateId);
        if (!t) throw new AppError("not_found", "La plantilla no existe.");
        if (t.builtin) fail("Las plantillas incluidas no se pueden eliminar.");
        templates.splice(templates.indexOf(t), 1);
      },
    },
    projects: {
      async list() {
        return [...projects.values()]
          .sort((a, b) => b.updatedAt.localeCompare(a.updatedAt))
          .map((p) => ({ id: p.id, name: p.name, client: p.client, caseCount: projectCases(p.id).length, updatedAt: p.updatedAt }));
      },
      async get(projectId) {
        return copy(getProject(projectId));
      },
      async create(input) {
        firstError(validateProjectMeta(input.meta));
        const t = templates.find((x) => x.id === input.templateId);
        if (!t) throw new AppError("not_found", "La plantilla no existe.");
        const ts = now();
        const p: Project = {
          id: id("prj"),
          ...trimMeta(input.meta),
          nextSequence: 1,
          sourceTemplateId: t.id,
          createdAt: ts,
          updatedAt: ts,
          fields: t.fields.map((f) => ({ ...f, id: id("fld"), archived: false })),
        };
        projects.set(p.id, p);
        return copy(p);
      },
      async updateMeta(projectId, meta) {
        const p = getProject(projectId);
        firstError(validateProjectMeta(meta));
        Object.assign(p, trimMeta(meta));
        touch(p);
        return copy(p);
      },
      async remove(projectId) {
        getProject(projectId);
        projects.delete(projectId);
        docVersions.removeForProject(projectId);
        for (const c of projectCases(projectId)) {
          evidence.removeForCase(c.id);
          cases.delete(c.id);
        }
      },
      async saveFields(projectId, inputs) {
        const p = getProject(projectId);
        firstError(validateFieldInputs(inputs));
        const hasData = (fieldId: string) => projectCases(projectId).some((c) => (c.values[fieldId] ?? "") !== "");
        const next: FieldDef[] = inputs.map((raw, position) => {
          const input = normalizeFieldInput(raw);
          const existing = input.id ? p.fields.find((f) => f.id === input.id) : undefined;
          if (input.id && !existing) fail("La columna ya no existe. Recarga el proyecto.");
          return {
            id: existing?.id ?? id("fld"),
            key: existing?.key ?? id("campo"),
            label: input.label,
            type: input.type,
            options: input.options,
            required: input.required,
            clearOnDuplicate: input.clearOnDuplicate,
            width: input.width,
            position,
            archived: input.archived,
          };
        });
        for (const f of p.fields) {
          if (!next.some((n) => n.id === f.id) && hasData(f.id)) next.push({ ...f, archived: true, position: next.length });
        }
        p.fields = next;
        touch(p);
        return copy([...activeFields(next), ...next.filter((f) => f.archived)]);
      },
    },
    cases: {
      async list(projectId) {
        getProject(projectId);
        return projectCases(projectId).map(withCount);
      },
      async create(projectId, input) {
        const p = getProject(projectId);
        const clean = checkCase(p, input);
        const ts = now();
        const record: CaseRecord = { id: id("case"), projectId, code: formatCode(p.nextSequence++), ...clean, evidenceCount: 0, createdAt: ts, updatedAt: ts };
        cases.set(record.id, record);
        touch(p);
        return withCount(record);
      },
      async update(caseId, input) {
        const c = getCase(caseId);
        const p = getProject(c.projectId);
        const clean = checkCase(p, input);
        const archived = Object.fromEntries(Object.entries(c.values).filter(([fid]) => p.fields.find((f) => f.id === fid)?.archived));
        Object.assign(c, { title: clean.title, status: clean.status, values: { ...archived, ...clean.values }, updatedAt: now() });
        touch(p);
        return withCount(c);
      },
      async duplicate(caseId) {
        const src = getCase(caseId);
        const p = getProject(src.projectId);
        const values = Object.fromEntries(
          Object.entries(src.values).filter(([fid]) => !p.fields.find((f) => f.id === fid)?.clearOnDuplicate),
        );
        const ts = now();
        const record: CaseRecord = {
          id: id("case"),
          projectId: p.id,
          code: formatCode(p.nextSequence++),
          title: src.title,
          status: DEFAULT_STATUS,
          values,
          evidenceCount: 0,
          createdAt: ts,
          updatedAt: ts,
        };
        cases.set(record.id, record);
        touch(p);
        return withCount(record);
      },
      async remove(caseId) {
        const c = getCase(caseId);
        evidence.removeForCase(caseId);
        cases.delete(caseId);
        touch(getProject(c.projectId));
      },
    },
    evidence,
    capture: createMemoryCapture(evidence),
    docVersions,
    exporter: createMemoryExporter((projectId) => {
      const list = projectCases(projectId);
      return { cases: list.length, evidence: list.reduce((n, c) => n + evidence.countFor(c.id), 0) };
    }),
    // `repos` se usa dentro de funciones que corren después de inicializarse.
    data: createMemoryData({
      importProject: async (name) => {
        const project = await repos.projects.create({ meta: { ...emptyMeta(), name }, templateId: "tpl-funcional" });
        return { id: project.id, name: project.name };
      },
      remember: (path) => repos.exporter.remember(path),
    }),
    settings: createMemorySettings(),
    updater: createMemoryUpdater(),
    github: createMemoryGithub((projectId) => projects.get(projectId)?.fields ?? []),
  };
  return repos;
}
