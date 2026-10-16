import { z } from "zod";
import type { DocVersionRepository } from "../../ports/export";
import { call } from "./client";
import { docVersionSchema } from "./dto";

const nothing = z.null();

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
