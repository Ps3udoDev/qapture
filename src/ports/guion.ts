import type { ImportSummary } from "../domain/data";
import type { GuionImportInput, GuionInspection, GuionPreview, ImportProfile, ProfileDefinition } from "../domain/guion";

export interface GuionImporter {
  inspect(): Promise<GuionInspection | null>;
  preview(token: string, profile: ProfileDefinition): Promise<GuionPreview>;
  import(input: GuionImportInput): Promise<ImportSummary>;
  listProfiles(): Promise<ImportProfile[]>;
  deleteProfile(id: string): Promise<void>;
}
