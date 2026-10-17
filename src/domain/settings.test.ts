import { describe, expect, it } from "vitest";
import { DEFAULT_SETTINGS, formatAccelerator, validateSettings, type Settings } from "./settings";

const key = (code: string, mods: Partial<Record<"ctrlKey" | "altKey" | "shiftKey" | "metaKey", boolean>> = {}) => ({
  ctrlKey: false,
  altKey: false,
  shiftKey: false,
  metaKey: false,
  code,
  ...mods,
});

const with_ = (f: (s: Settings) => void): Settings => {
  const s = structuredClone(DEFAULT_SETTINGS);
  f(s);
  return s;
};

describe("ajustes", () => {
  it("formatAccelerator da la forma canónica", () => {
    expect(formatAccelerator(key("KeyQ", { ctrlKey: true, altKey: true }))).toBe("Ctrl+Alt+Q");
    expect(formatAccelerator(key("F12", { ctrlKey: true, shiftKey: true }))).toBe("Ctrl+Shift+F12");
    expect(formatAccelerator(key("Digit1", { altKey: true, metaKey: true }))).toBe("Alt+Win+1");
    expect(formatAccelerator(key("ControlLeft", { ctrlKey: true }))).toBeNull();
    expect(formatAccelerator(key("BracketLeft", { ctrlKey: true }))).toBeNull();
  });

  it("validateSettings replica las reglas de Rust", () => {
    expect(validateSettings(DEFAULT_SETTINGS)).toEqual({});
    expect(validateSettings(with_((s) => (s.shortcuts.capture = "Shift+S")))["shortcuts.capture"]).toBe("«Capturar pantalla»: usa al menos Ctrl, Alt o Win.");
    expect(validateSettings(with_((s) => (s.shortcuts.paste = s.shortcuts.capture)))["shortcuts.paste"]).toBe(
      "«Capturar pantalla» y «Pegar evidencia» usan la misma combinación (Ctrl+Alt+S).",
    );
    expect(validateSettings(with_((s) => (s.shortcuts.toggle = "Alt+F4")))["shortcuts.toggle"]).toBe("«Mostrar/ocultar»: Windows reserva Alt+F4.");
    expect(validateSettings(with_((s) => (s.shortcuts.newCase = "Ctrl+[")))["shortcuts.newCase"]).toBe("«Nuevo caso»: combinación no válida.");
    expect(validateSettings(with_((s) => (s.export.imageMaxWidth = 300)))["export.imageMaxWidth"]).toBe(
      "El ancho máximo de imagen debe estar entre 400 y 1600 px.",
    );
    expect(validateSettings(with_((s) => (s.profile.email = "ana@")))["profile.email"]).toBe("«Correo» no es válido.");
    expect(validateSettings(with_((s) => (s.profile.tester = "a".repeat(101))))["profile.tester"]).toBe("«Nombre del tester» supera 100 caracteres.");
  });
});
