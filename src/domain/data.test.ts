import { describe, expect, it } from "vitest";
import { formatBackupDate, formatSize, importSummaryMessage, parseImportIssues, restoreMessage, type Backup } from "./data";

const backup: Backup = { fileName: "qapture-diario-20261015.db", kind: "daily", createdAt: "2026-10-15T09:00:05", sizeBytes: 98304 };

describe("importSummaryMessage", () => {
  it("usa singular y plural", () => {
    expect(importSummaryMessage({ projects: [{ id: "p1", name: "A" }], cases: 1, evidence: 1 })).toBe("Importado «A»: 1 caso y 1 captura.");
    expect(importSummaryMessage({ projects: [{ id: "p1", name: "A" }, { id: "p2", name: "B" }], cases: 3, evidence: 0 })).toBe(
      "Importado: 2 proyectos («A» y «B»), 3 casos y 0 capturas.",
    );
  });
});

describe("parseImportIssues", () => {
  it("separa el encabezado de las viñetas", () => {
    const issues = parseImportIssues("No se importó nada. Revisa estos problemas:\n• EJ-001: Fecha inválida.\n• … y 3 problemas más.");
    expect(issues).toEqual({ title: "No se importó nada. Revisa estos problemas:", items: ["EJ-001: Fecha inválida.", "… y 3 problemas más."] });
  });

  it("un mensaje de una línea queda como título", () => {
    expect(parseImportIssues("El archivo no es un paquete de Qapture válido.")).toEqual({ title: "El archivo no es un paquete de Qapture válido.", items: [] });
  });
});

describe("formatos", () => {
  it("fecha del respaldo en formato local", () => {
    expect(formatBackupDate("2026-10-15T09:00:05")).toBe("15/10/2026 09:00");
    expect(formatBackupDate("raro")).toBe("raro");
  });

  it("tamaño legible", () => {
    expect(formatSize(512)).toBe("512 B");
    expect(formatSize(98304)).toBe("96 KB");
    expect(formatSize(1.5 * 1024 * 1024)).toBe("1,5 MB");
  });

  it("mensaje al restaurar", () => {
    const result = { restored: backup, safetyCopy: "qapture-antes-de-restaurar-20261016-100000.db", missingImages: 0, setAsideFiles: 0 };
    expect(restoreMessage(result)).toBe("Se restauró el respaldo del 15/10/2026 09:00.");
    expect(restoreMessage({ ...result, missingImages: 2 })).toBe("Se restauró el respaldo del 15/10/2026 09:00. 2 capturas del respaldo ya no tienen imagen.");
    expect(restoreMessage({ ...result, missingImages: 1 })).toBe("Se restauró el respaldo del 15/10/2026 09:00. 1 captura del respaldo ya no tiene imagen.");
  });
});
