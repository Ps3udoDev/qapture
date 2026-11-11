import { z } from "zod";
import type { GuionImporter } from "../../ports/guion";
import { call } from "./client";
import { guionInspectSchema, guionPreviewSchema, importProfileSchema, importSummarySchema } from "./dto";

export function tauriGuion(): GuionImporter {
  return {
    inspect: () => call("guion_inspect", {}, guionInspectSchema.nullable()),
    preview: (token, profile) => call("guion_preview", { token, profile }, guionPreviewSchema),
    import: (input) => call("guion_import", { input }, importSummarySchema),
    listProfiles: () => call("import_profiles_list", {}, z.array(importProfileSchema)),
    deleteProfile: async (id) => { await call("import_profile_delete", { id }, z.null()); },
  };
}
