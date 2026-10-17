import { z } from "zod";
import { FIELD_TYPES } from "../../domain/fields";
import { BACKUP_KINDS } from "../../domain/data";
import { EXPORT_STAGES } from "../../domain/export";
import { STATUSES } from "../../domain/status";

export const fieldSchema = z.strictObject({
  id: z.string(),
  key: z.string(),
  label: z.string(),
  type: z.enum(FIELD_TYPES),
  options: z.array(z.string()),
  required: z.boolean(),
  clearOnDuplicate: z.boolean(),
  width: z.number().int(),
  position: z.number().int(),
  archived: z.boolean(),
});

export const templateSchema = z.strictObject({
  id: z.string(),
  name: z.string(),
  testType: z.string(),
  builtin: z.boolean(),
  fields: z.array(fieldSchema),
});

export const projectSummarySchema = z.strictObject({
  id: z.string(),
  name: z.string(),
  client: z.string(),
  caseCount: z.number().int(),
  updatedAt: z.string(),
});

export const projectSchema = z.strictObject({
  id: z.string(),
  name: z.string(),
  client: z.string(),
  testType: z.string(),
  releaseVersion: z.string(),
  startDate: z.string(),
  endDate: z.string(),
  nextSequence: z.number().int(),
  sourceTemplateId: z.string().nullable(),
  createdAt: z.string(),
  updatedAt: z.string(),
  fields: z.array(fieldSchema),
});

export const caseSchema = z.strictObject({
  id: z.string(),
  projectId: z.string(),
  code: z.string(),
  title: z.string(),
  status: z.enum(STATUSES),
  values: z.record(z.string(), z.string()),
  evidenceCount: z.number().int(),
  createdAt: z.string(),
  updatedAt: z.string(),
});

export const appErrorSchema = z.strictObject({
  code: z.enum(["validation", "not_found", "db", "io", "capture", "file_locked", "export", "corrupt", "internal"]),
  message: z.string(),
  detail: z.string(),
});

export const evidenceSchema = z.strictObject({
  id: z.string(),
  caseId: z.string(),
  position: z.number().int(),
  description: z.string(),
  originalPath: z.string(),
  renderedPath: z.string(),
  annotations: z.string(),
  width: z.number().int(),
  height: z.number().int(),
  capturedAt: z.string(),
  updatedAt: z.string(),
});

export const stagedImageSchema = z.strictObject({
  path: z.string(),
  width: z.number().int(),
  height: z.number().int(),
});

export const capturedEventSchema = z.strictObject({
  tmpPath: z.string(),
});

export const docVersionSchema = z.strictObject({
  id: z.string(),
  projectId: z.string(),
  version: z.string(),
  date: z.string(),
  author: z.string(),
  reviewer: z.string(),
  approver: z.string(),
  changes: z.string(),
  createdAt: z.string(),
});

export const exportResultSchema = z.strictObject({
  path: z.string(),
  fileName: z.string(),
  cases: z.number().int(),
  evidence: z.number().int(),
  missingImages: z.number().int(),
});

export const exportProgressSchema = z.strictObject({
  stage: z.enum(EXPORT_STAGES),
  current: z.number().int(),
  total: z.number().int(),
});

export const importSummarySchema = z.strictObject({
  projects: z.array(z.strictObject({ id: z.string(), name: z.string() })),
  cases: z.number().int(),
  evidence: z.number().int(),
});

export const packageExportResultSchema = z.strictObject({
  path: z.string(),
  fileName: z.string(),
  cases: z.number().int(),
  evidence: z.number().int(),
  missingImages: z.number().int(),
});

export const backupSchema = z.strictObject({
  fileName: z.string(),
  kind: z.enum(BACKUP_KINDS),
  createdAt: z.string(),
  sizeBytes: z.number().int(),
});

export const restoreResultSchema = z.strictObject({
  restored: backupSchema,
  safetyCopy: z.string(),
  missingImages: z.number().int(),
  setAsideFiles: z.number().int(),
});

export const startupStatusSchema = z.strictObject({
  recovery: z.boolean(),
  detail: z.string(),
});
