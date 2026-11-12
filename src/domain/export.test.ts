import { describe, expect, it } from "vitest";
import {
  defaultExportOptions,
  defaultOriginExportOptions,
  formulasMessage,
  progressLabel,
  progressPercent,
  unmappedMessage,
  validateExportOptions,
  validateOriginExportOptions,
} from "./export";

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

describe("formato de origen", () => {
  it("mensajes de casos sin fila", () => {
    expect(unmappedMessage([])).toBeNull();
    expect(unmappedMessage(["CP99"])).toBe("1 caso no tiene fila en el guion y no se escribe: CP99.");
    expect(unmappedMessage(["CP99", "X"])).toBe("2 casos no tienen fila en el guion y no se escriben: CP99, X.");
  });

  it("mensajes de fórmulas quitadas", () => {
    expect(formulasMessage([])).toBeNull();
    expect(formulasMessage(["I16", "I17"])).toBe("Las celdas I16, I17 tenían fórmula: se escribe el valor y se quita la fórmula.");
  });

  it("valida opciones de exportación de origen", () => {
    const ok = defaultOriginExportOptions("Ana Pérez");
    expect(validateOriginExportOptions(ok)).toEqual({});

    expect(validateOriginExportOptions({ ...ok, tester: "   " })).toEqual({
      tester: "«Tester» es obligatorio.",
    });
    expect(validateOriginExportOptions({ ...ok, tester: "a".repeat(101) }).tester).toBe("«Tester» supera 100 caracteres.");

    expect(validateOriginExportOptions({ ...ok, observations: "a".repeat(501) })).toEqual({
      observations: "La observación supera 500 caracteres.",
    });

    expect(validateOriginExportOptions({ ...ok, imageMaxWidth: 200 }).imageMaxWidth).toBe(
      "El ancho máximo de imagen debe estar entre 400 y 1600 px.",
    );
  });
});
