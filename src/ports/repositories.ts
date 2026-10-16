import type { CaseInput, CaseRecord } from "../domain/cases";
import type { FieldDef } from "../domain/fields";
import type { FieldInput, NewProjectInput, Project, ProjectMeta, ProjectSummary, Template } from "../domain/project";

export interface TemplateRepository {
  list(): Promise<Template[]>;
  saveFromProject(projectId: string, name: string, testType: string): Promise<Template>;
  remove(id: string): Promise<void>;
}

export interface ProjectRepository {
  list(): Promise<ProjectSummary[]>;
  get(id: string): Promise<Project>;
  create(input: NewProjectInput): Promise<Project>;
  updateMeta(id: string, meta: ProjectMeta): Promise<Project>;
  remove(id: string): Promise<void>;
  saveFields(projectId: string, fields: FieldInput[]): Promise<FieldDef[]>;
}

export interface CaseRepository {
  list(projectId: string): Promise<CaseRecord[]>;
  create(projectId: string, input: CaseInput): Promise<CaseRecord>;
  update(caseId: string, input: CaseInput): Promise<CaseRecord>;
  duplicate(caseId: string): Promise<CaseRecord>;
  remove(caseId: string): Promise<void>;
}

export interface Repositories {
  templates: TemplateRepository;
  projects: ProjectRepository;
  cases: CaseRepository;
}
