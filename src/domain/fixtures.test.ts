import { describe, expect, it } from "vitest";
import {
  FIXTURE_ACCEPT,
  FIXTURE_EXTENSIONS,
  FIXTURE_NOT_ALLOWED,
  FIXTURE_TOO_BIG,
  MAX_FIXTURE_BYTES,
  fixtureProblem,
} from "./fixtures";

describe("reglas de archivos de prueba (fixtures)", () => {
  it("expone las constantes esperadas", () => {
    expect(FIXTURE_EXTENSIONS).toEqual(["xlsx", "xls", "csv", "txt", "pdf", "json", "xml", "zip"]);
    expect(MAX_FIXTURE_BYTES).toBe(25 * 1024 * 1024);
    expect(FIXTURE_ACCEPT).toBe(".xlsx,.xls,.csv,.txt,.pdf,.json,.xml,.zip");
    expect(FIXTURE_NOT_ALLOWED).toBe("Ese tipo de archivo no se acepta como archivo de prueba.");
    expect(FIXTURE_TOO_BIG).toBe("El archivo de prueba supera 25 MB.");
  });

  it("rechaza extensiones no permitidas", () => {
    expect(fixtureProblem("a.exe", 1)).toBe(FIXTURE_NOT_ALLOWED);
    expect(fixtureProblem("script.sh", 100)).toBe(FIXTURE_NOT_ALLOWED);
    expect(fixtureProblem("documento.docx", 500)).toBe(FIXTURE_NOT_ALLOWED);
  });

  it("rechaza archivos sin extensión", () => {
    expect(fixtureProblem("archivo_sin_extension", 10)).toBe(FIXTURE_NOT_ALLOWED);
    expect(fixtureProblem("puntofinal.", 10)).toBe(FIXTURE_NOT_ALLOWED);
  });

  it("rechaza archivos que superan 25 MB", () => {
    expect(fixtureProblem("a.csv", MAX_FIXTURE_BYTES + 1)).toBe(FIXTURE_TOO_BIG);
    expect(fixtureProblem("datos.xlsx", 30 * 1024 * 1024)).toBe(FIXTURE_TOO_BIG);
  });

  it("acepta todas las extensiones válidas en minúsculas y mayúsculas dentro del límite", () => {
    for (const ext of FIXTURE_EXTENSIONS) {
      expect(fixtureProblem(`prueba.${ext}`, 1024)).toBeNull();
      expect(fixtureProblem(`PRUEBA.${ext.toUpperCase()}`, 1024)).toBeNull();
    }
  });

  it("acepta archivos vacíos y archivos en el límite exacto de 25 MB", () => {
    expect(fixtureProblem("A.XLSX", 0)).toBeNull();
    expect(fixtureProblem("limite.zip", MAX_FIXTURE_BYTES)).toBeNull();
  });
});
