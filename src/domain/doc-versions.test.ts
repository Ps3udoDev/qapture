import { describe, expect, it } from "vitest";
import { emptyDocVersion, formatIsoDate, suggestNextVersion, todayIso, trimDocVersion, validateDocVersion, type DocVersion } from "./doc-versions";

const valid = { ...emptyDocVersion("2026-10-16", "Ana Pérez"), version: "1.0" };
const saved = (version: string): DocVersion => ({ ...valid, version, id: version, projectId: "p", createdAt: "" });

describe("versiones del documento", () => {
  it("valida obligatorios, fecha y largos como Rust", () => {
    expect(validateDocVersion(valid)).toEqual({});
    expect(validateDocVersion({ ...valid, version: " ", author: "" })).toEqual({
      version: "«Versión» es obligatorio.",
      author: "«Elaborado por» es obligatorio.",
    });
    expect(validateDocVersion({ ...valid, date: "2026-02-30" }).date).toBe("«Fecha» debe ser una fecha válida (AAAA-MM-DD).");
    expect(validateDocVersion({ ...valid, date: "" }).date).toBe("«Fecha» es obligatorio.");
    expect(validateDocVersion({ ...valid, version: "9".repeat(21) }).version).toBe("«Versión» supera 20 caracteres.");
    expect(validateDocVersion({ ...valid, changes: "🚀".repeat(1001) }).changes).toBe("«Cambios» supera 1000 caracteres.");
  });

  it("sugiere la siguiente versión menor", () => {
    expect(suggestNextVersion([])).toBe("1.0");
    expect(suggestNextVersion([saved("1.0"), saved("1.9")])).toBe("1.10");
    expect(suggestNextVersion([saved("v2.3")])).toBe("v2.4");
    expect(suggestNextVersion([saved("Final")])).toBe("");
  });

  it("usa la fecha local y recorta los textos", () => {
    expect(todayIso(new Date(2026, 9, 16, 23, 30))).toBe("2026-10-16");
    expect(formatIsoDate("2026-10-01")).toBe("01/10/2026");
    expect(formatIsoDate("ayer")).toBe("ayer");
    expect(trimDocVersion({ ...valid, version: " 1.1 ", changes: " Casos nuevos " })).toMatchObject({ version: "1.1", changes: "Casos nuevos" });
  });
});
