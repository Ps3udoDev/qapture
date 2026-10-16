import { describe, expect, it } from "vitest";
import { defaultExportOptions, progressLabel, progressPercent, validateExportOptions } from "./export";

describe("opciones de exportación", () => {
  const ok = { ...defaultExportOptions("Ana Pérez"), docVersionId: "ver-1" };

  it("exigen tester y, para el proyecto, versión", () => {
    expect(validateExportOptions(ok, "project")).toEqual({});
    expect(validateExportOptions({ ...ok, tester: "  ", docVersionId: null }, "project")).toEqual({
      tester: "«Tester» es obligatorio.",
      docVersionId: "Elige la versión del documento.",
    });
    expect(validateExportOptions({ ...ok, docVersionId: null }, "template")).toEqual({});
    expect(validateExportOptions({ ...ok, tester: "a".repeat(101) }, "project").tester).toBe("«Tester» supera 100 caracteres.");
  });

  it("acotan el ancho de imagen", () => {
    expect(ok.imageMaxWidth).toBe(940);
    for (const bad of [399, 1601, 900.5]) {
      expect(validateExportOptions({ ...ok, imageMaxWidth: bad }, "project").imageMaxWidth).toBe(
        "El ancho máximo de imagen debe estar entre 400 y 1600 px.",
      );
    }
  });

  it("describen el avance", () => {
    expect(progressLabel(null)).toBe("");
    expect(progressLabel({ stage: "images", current: 120, total: 300 })).toBe("Procesando imágenes 120 de 300…");
    expect(progressLabel({ stage: "saving", current: 0, total: 0 })).toBe("Guardando el archivo…");
    expect(progressPercent({ stage: "images", current: 150, total: 300 })).toBe(45);
    expect(progressPercent({ stage: "images", current: 0, total: 0 })).toBe(5);
    expect(progressPercent({ stage: "writing", current: 0, total: 0 })).toBe(85);
  });
});
