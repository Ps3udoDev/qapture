import { describe, expect, it } from "vitest";
import { isUsable, MIN_SELECTION, rectFrom, sizeLabel, toSelection } from "./capture";

const viewport = { width: 1536, height: 864 };

describe("selección del overlay", () => {
  it("normaliza un arrastre en cualquier dirección", () => {
    expect(rectFrom({ x: 300, y: 200 }, { x: 100, y: 50 }, viewport)).toEqual({ x: 100, y: 50, width: 200, height: 150 });
  });

  it("limita la selección a la ventana", () => {
    expect(rectFrom({ x: -20, y: 800 }, { x: 2000, y: 900 }, viewport)).toEqual({ x: 0, y: 800, width: 1536, height: 64 });
  });

  it("un clic sin arrastre no es una selección", () => {
    expect(isUsable({ x: 10, y: 10, width: MIN_SELECTION - 1, height: 50 })).toBe(false);
    expect(isUsable({ x: 10, y: 10, width: MIN_SELECTION, height: MIN_SELECTION })).toBe(true);
  });

  it("muestra la medida en píxeles reales del monitor", () => {
    expect(sizeLabel({ x: 0, y: 0, width: 200, height: 100 }, 1.25)).toBe("250 × 125");
    expect(sizeLabel({ x: 0, y: 0, width: 33.3, height: 10 }, 1.5)).toBe("50 × 15");
  });

  it("agrega el tamaño de la ventana para que Rust calcule la escala", () => {
    expect(toSelection({ x: 1, y: 2, width: 3, height: 4 }, viewport)).toEqual({
      x: 1,
      y: 2,
      width: 3,
      height: 4,
      viewportWidth: 1536,
      viewportHeight: 864,
    });
  });
});
