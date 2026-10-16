import type { DocVersion, DocVersionInput } from "../domain/doc-versions";

/** Control de versiones del documento (UC-05). */
export interface DocVersionRepository {
  list(projectId: string): Promise<DocVersion[]>;
  create(projectId: string, input: DocVersionInput): Promise<DocVersion>;
  update(id: string, input: DocVersionInput): Promise<DocVersion>;
  remove(id: string): Promise<void>;
}
