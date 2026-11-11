export interface Fixture {
  id: string;
  caseId: string;
  fileName: string;
  size: number;
  sha256: string;
  note: string;
  position: number;
  addedAt: string;
}

export const FIXTURE_EXTENSIONS = ["xlsx", "xls", "csv", "txt", "pdf", "json", "xml", "zip"] as const;
export const MAX_FIXTURE_BYTES = 25 * 1024 * 1024;
export const FIXTURE_ACCEPT = ".xlsx,.xls,.csv,.txt,.pdf,.json,.xml,.zip";
export const FIXTURE_TOO_BIG = "El archivo de prueba supera 25 MB.";
export const FIXTURE_NOT_ALLOWED = "Ese tipo de archivo no se acepta como archivo de prueba.";

export function fixtureProblem(name: string, size: number): string | null {
  const dot = name.lastIndexOf(".");
  if (dot === -1) return FIXTURE_NOT_ALLOWED;
  const ext = name.slice(dot + 1).toLowerCase();
  if (!(FIXTURE_EXTENSIONS as readonly string[]).includes(ext)) {
    return FIXTURE_NOT_ALLOWED;
  }
  if (size > MAX_FIXTURE_BYTES) {
    return FIXTURE_TOO_BIG;
  }
  return null;
}
