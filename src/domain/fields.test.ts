import { describe, expect, it } from "vitest";
import { charLength, isIsoDate, normalizeValue, validateValue, type FieldDef } from "./fields";

const field = (over: Partial<FieldDef> = {}): FieldDef => ({
  id: "f1",
  key: "campo",
  label: "Campo",
  type: "text",
  options: [],
  required: false,
  clearOnDuplicate: false,
  width: 20,
  position: 0,
  archived: false,
  ...over,
});

describe("validateValue", () => {
  it("exige obligatorios ignorando espacios", () => {
    expect(validateValue(field({ required: true, label: "Entorno" }), "   ")).toBe("«Entorno» es obligatorio.");
    expect(validateValue(field(), "")).toBeNull();
  });

  it("valida listas", () => {
    const f = field({ type: "list", options: ["Alta", "Media"] });
    expect(validateValue(f, "Media")).toBeNull();
    expect(validateValue(f, "Baja")).toMatch(/opciones de la lista/);
  });

  it("valida fechas reales", () => {
    const f = field({ type: "date" });
    expect(validateValue(f, "2026-02-28")).toBeNull();
    expect(validateValue(f, "2026-02-30")).toMatch(/fecha válida/);
    expect(validateValue(f, "2026-2-1")).toMatch(/fecha válida/);
  });

  it("acepta el año 0000 como chrono (Date.UTC lo mapearía a 1900)", () => {
    expect(isIsoDate("0000-01-01")).toBe(true);
    expect(isIsoDate("0099-12-31")).toBe(true);
    expect(isIsoDate("0000-02-29")).toBe(true); // el año 0 es bisiesto en el calendario proléptico
    expect(isIsoDate("0001-02-29")).toBe(false);
  });

  it("acepta coma decimal y rechaza formatos ambiguos", () => {
    const f = field({ type: "number" });
    expect(validateValue(f, "1,5")).toBeNull();
    expect(normalizeValue("number", " 1,5 ")).toBe("1.5");
    expect(validateValue(f, "-3.25e2")).toBeNull();
    for (const ok of ["1.", ".5", "+1", "1e5", "-3.25e2", "1,5"]) {
      expect(validateValue(f, ok)).toBeNull();
    }
    for (const bad of ["1.234,5", "0x10", "Infinity", "NaN", "abc", "1e999", "1_0", "1 0"]) {
      expect(validateValue(f, bad)).toMatch(/número/);
    }
  });

  it("valida sí/no", () => {
    const f = field({ type: "boolean" });
    expect(validateValue(f, "true")).toBeNull();
    expect(validateValue(f, "sí")).toMatch(/Sí o No/);
  });

  it("cuenta caracteres como Rust (un emoji = 1)", () => {
    expect(charLength("✅🚀")).toBe(2);
    expect(validateValue(field(), "🚀".repeat(500))).toBeNull();
    expect(validateValue(field(), "🚀".repeat(501))).toBe("«Campo» supera 500 caracteres.");
  });

  it("conserva saltos de línea en texto largo", () => {
    expect(normalizeValue("longtext", "línea 1\nlínea 2\n")).toBe("línea 1\nlínea 2\n");
    expect(normalizeValue("longtext", " \n ")).toBe("");
  });
});
