import type { DocVersion, DocVersionInput } from "../domain/doc-versions";
import type { ExportOptions, ExportProgress, ExportResult } from "../domain/export";

/** Control de versiones del documento (UC-05). */
export interface DocVersionRepository {
  list(projectId: string): Promise<DocVersion[]>;
  create(projectId: string, input: DocVersionInput): Promise<DocVersion>;
  update(id: string, input: DocVersionInput): Promise<DocVersion>;
  remove(id: string): Promise<void>;
}

/** Excel con carátula (UC-16) y plantilla vacía (UC-17). Rust abre «Guardar como». */
export interface Exporter {
  /** `null`: el usuario cerró «Guardar como» sin elegir archivo. */
  exportProject(projectId: string, options: ExportOptions): Promise<ExportResult | null>;
  exportTemplate(projectId: string, options: ExportOptions): Promise<ExportResult | null>;
  /** Suscribe al evento `export:progress`; devuelve la función para desuscribirse. */
  onProgress(handler: (progress: ExportProgress) => void): Promise<() => void>;
  /** Solo acepta archivos exportados en esta sesión. */
  openFile(path: string): Promise<void>;
  revealFile(path: string): Promise<void>;
}
