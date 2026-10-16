import { listen } from "@tauri-apps/api/event";
import { z } from "zod";
import type { DocVersionRepository, Exporter } from "../../ports/export";
import { call } from "./client";
import { docVersionSchema, exportProgressSchema, exportResultSchema } from "./dto";

const nothing = z.null();

export const EXPORT_PROGRESS_EVENT = "export:progress";

export function tauriDocVersions(): DocVersionRepository {
  return {
    list: (projectId) => call("list_doc_versions", { projectId }, z.array(docVersionSchema)),
    create: (projectId, input) => call("create_doc_version", { projectId, input }, docVersionSchema),
    update: (id, input) => call("update_doc_version", { id, input }, docVersionSchema),
    remove: async (id) => {
      await call("delete_doc_version", { id }, nothing);
    },
  };
}

export function tauriExporter(): Exporter {
  return {
    exportProject: (projectId, options) => call("export_project", { projectId, options }, exportResultSchema.nullable()),
    exportTemplate: (projectId, options) => call("export_template", { projectId, options }, exportResultSchema.nullable()),
    onProgress: (handler) =>
      listen<unknown>(EXPORT_PROGRESS_EVENT, (event) => {
        const parsed = exportProgressSchema.safeParse(event.payload);
        if (parsed.success) handler(parsed.data);
      }),
    openFile: async (path) => {
      await call("open_export", { path }, nothing);
    },
    revealFile: async (path) => {
      await call("reveal_export", { path }, nothing);
    },
  };
}
