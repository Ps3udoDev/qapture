import { z } from "zod";
import { FIELD_TYPES } from "../../domain/fields";
import { BACKUP_KINDS } from "../../domain/data";
import { EXPORT_STAGES } from "../../domain/export";
import { SHORTCUT_ACTIONS } from "../../domain/settings";
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
  code: z.enum(["validation", "not_found", "db", "io", "capture", "file_locked", "export", "corrupt", "internal",
    "gh_auth", "gh_forbidden", "gh_not_found", "gh_rate_limited", "gh_offline", "gh_keyring", "gh_validation", "gh_conflict"]),
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

export const settingsSchema = z.strictObject({
  profile: z.strictObject({ tester: z.string(), role: z.string(), email: z.string() }),
  shortcuts: z.strictObject({ toggle: z.string(), capture: z.string(), paste: z.string(), newCase: z.string() }),
  system: z.strictObject({ autostart: z.boolean(), startHidden: z.boolean(), closeToTray: z.boolean(), escHides: z.boolean() }),
  export: z.strictObject({ defaultFolder: z.string(), imageMaxWidth: z.number().int(), includeSummary: z.boolean() }),
});

export const settingsSnapshotSchema = z.strictObject({
  settings: settingsSchema,
  shortcutFailures: z.array(z.strictObject({ action: z.enum(SHORTCUT_ACTIONS), accelerator: z.string() })),
  appVersion: z.string(),
  dataDir: z.string(),
  portable: z.boolean(),
});

export const updateInfoSchema = z.strictObject({
  version: z.string(),
  currentVersion: z.string(),
  notes: z.string(),
  date: z.string().nullable(),
  portable: z.boolean(),
});

export const updateProgressSchema = z.strictObject({
  downloaded: z.number().int().nonnegative(),
  total: z.number().int().nonnegative().nullable(),
});

export const ghAccountSchema = z.strictObject({
  login: z.string(),
  name: z.string().nullable(),
  avatarDataUrl: z.string().nullable(),
  connectedAt: z.string(),
  needsReconnect: z.boolean(),
});

export const deviceCodeSchema = z.strictObject({
  userCode: z.string(),
  verificationUri: z.string(),
  expiresIn: z.number().int().nonnegative(),
  interval: z.number().int().nonnegative(),
});

export const ghRepoSchema = z.strictObject({
  owner: z.string(),
  name: z.string(),
  fullName: z.string(),
  private: z.boolean(),
  canWriteIssues: z.boolean(),
  canWriteContents: z.boolean(),
});

export const linkedRepoSchema = z.strictObject({
  fullName: z.string(),
  isDefault: z.boolean(),
  defaultLabels: z.array(z.string()),
});

export const projectGithubSchema = z.strictObject({
  repos: z.array(linkedRepoSchema),
  routingFieldId: z.string().nullable(),
  routingMap: z.record(z.string(), z.string()),
  evidenceRepo: z.string().nullable(),
  routingStale: z.boolean(),
});

/** Lo que acepta `gh_project_github_set`: igual que `projectGithubSchema` sin `routingStale`. */
export const projectGithubInputSchema = projectGithubSchema.omit({ routingStale: true });

export const ghConnectFailedSchema = z.strictObject({ message: z.string() });
