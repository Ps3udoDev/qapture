import { describe, expect, it } from "vitest";
import { evidenceLabel, isAcceptedImage, moveItem, validateDescription } from "./evidence";

describe("evidencias", () => {
  it("exige descripción y limita a 2000 caracteres Unicode", () => {
    expect(validateDescription("  ")).toBe("«Descripción» es obligatorio.");
    expect(validateDescription("✅".repeat(2000))).toBeNull();
    expect(validateDescription("a".repeat(2001))).toBe("«Descripción» supera 2000 caracteres.");
  });

  it("acepta solo PNG, JPEG y WebP", () => {
    expect(["image/png", "image/jpeg", "image/webp"].every((type) => isAcceptedImage({ type }))).toBe(true);
    expect(isAcceptedImage({ type: "application/pdf" })).toBe(false);
    expect(isAcceptedImage({ type: "image/gif" })).toBe(false);
  });

  it("numera desde 1 y mueve elementos sin mutar", () => {
    expect(evidenceLabel({ position: 0 })).toBe("Evidencia 1");
    const ids = ["a", "b", "c"];
    expect(moveItem(ids, 2, 0)).toEqual(["c", "a", "b"]);
    expect(moveItem(ids, 0, 1)).toEqual(["b", "a", "c"]);
    expect(moveItem(ids, 0, 9)).toEqual(ids);
    expect(ids).toEqual(["a", "b", "c"]);
  });
});
