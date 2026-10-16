import type { CaseInput, CaseRecord } from "../domain/cases";
import type { Evidence, EvidenceUpdateInput, NewEvidenceInput, StagedImage } from "../domain/evidence";
import type { FieldDef } from "../domain/fields";
import type { FieldInput, NewProjectInput, Project, ProjectMeta, ProjectSummary, Template } from "../domain/project";
import type { ScreenCapture } from "./capture";
import type { DocVersionRepository, Exporter } from "./export";

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

export interface EvidenceStore {
  stage(image: Uint8Array): Promise<StagedImage>;
  discardStaged(path: string): Promise<void>;
  readImage(path: string): Promise<Uint8Array>;
  imageUrl(path: string, version: string): string;
  list(caseId: string): Promise<Evidence[]>;
  add(input: NewEvidenceInput, render: Uint8Array): Promise<Evidence>;
  update(input: EvidenceUpdateInput, render: Uint8Array): Promise<Evidence>;
  updateDescription(evidenceId: string, description: string): Promise<Evidence>;
  move(evidenceId: string, targetCaseId: string): Promise<Evidence>;
  reorder(caseId: string, orderedIds: string[]): Promise<Evidence[]>;
  remove(evidenceId: string): Promise<void>;
}

export interface Repositories {
  templates: TemplateRepository;
  projects: ProjectRepository;
  cases: CaseRepository;
  evidence: EvidenceStore;
  capture: ScreenCapture;
  docVersions: DocVersionRepository;
  exporter: Exporter;
}
