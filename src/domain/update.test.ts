import { describe, expect, it } from "vitest";
import { availableMessage, parseVersion, progressMessage, shouldPrompt, upToDateMessage, type UpdateInfo } from "./update";

const info: UpdateInfo = { version: "1.0.1", currentVersion: "1.0.0", notes: "", date: null, portable: false };

describe("textos del updater", () => {
  it("arma los mensajes fijos", () => {
    expect(availableMessage(info)).toBe("Qapture 1.0.1 está disponible (tienes 1.0.0).");
    expect(upToDateMessage("1.0.0")).toBe("Qapture está al día (versión 1.0.0).");
  });

  it("el progreso muestra el porcentaje solo con total conocido", () => {
    expect(progressMessage(50, 200)).toBe("Descargando actualización… 25 %");
    expect(progressMessage(500, 200)).toBe("Descargando actualización… 100 %");
    expect(progressMessage(50, null)).toBe("Descargando actualización…");
    expect(progressMessage(0, 0)).toBe("Descargando actualización…");
  });
});

describe("versión pospuesta (espejo de should_prompt en Rust)", () => {
  it("avisa si no hay versión pospuesta", () => {
    expect(shouldPrompt("1.0.1", null)).toBe(true);
  });
  it("compara X.Y.Z como números, no como texto", () => {
    expect(shouldPrompt("1.0.10", "1.0.9")).toBe(true);
    expect(shouldPrompt("1.0.9", "1.0.10")).toBe(false);
    expect(shouldPrompt("1.2.0", "1.1.9")).toBe(true);
    expect(shouldPrompt("2.0.0", "1.9.9")).toBe(true);
  });
  it("no avisa con la misma versión ni con una anterior", () => {
    expect(shouldPrompt("1.0.1", "1.0.1")).toBe(false);
    expect(shouldPrompt("1.0.0", "1.0.1")).toBe(false);
  });
  it("tolera la v inicial y, con texto no numérico, avisa solo si cambió", () => {
    expect(parseVersion("v1.0.1")).toEqual([1, 0, 1]);
    expect(parseVersion("1.0")).toBeNull();
    expect(parseVersion("1.0.1.2")).toBeNull();
    expect(shouldPrompt("v1.0.1", "1.0.1")).toBe(false);
    expect(shouldPrompt("1.0.1-rc", "1.0.1")).toBe(true);
    expect(shouldPrompt("1.0.1-rc", "1.0.1-rc")).toBe(false);
  });
});
