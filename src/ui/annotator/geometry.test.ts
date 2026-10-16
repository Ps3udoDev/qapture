import { describe, expect, it } from "vitest";
import { clampRect, exportRegion, fitScale, isMeaningful, pixelSizeFor, rectFromPoints } from "./geometry";

describe("geometría", () => {
  it("normaliza un arrastre en cualquier dirección y lo recorta a la imagen", () => {
    expect(rectFromPoints({ x: 50, y: 40 }, { x: 10, y: 0 })).toEqual({ x: 10, y: 0, width: 40, height: 40 });
    expect(clampRect({ x: -10, y: 90, width: 50, height: 50 }, 100, 100)).toEqual({ x: 0, y: 90, width: 40, height: 10 });
    expect(isMeaningful({ x: 0, y: 0, width: 3, height: 30 })).toBe(false);
  });

  it("ajusta la imagen al espacio sin agrandarla", () => {
    expect(fitScale(2400, 1200, 1200, 1000)).toBe(0.5);
    expect(fitScale(400, 300, 1200, 1000)).toBe(1);
    expect(fitScale(400, 300, 0, 0)).toBe(1);
  });

  it("exporta a resolución nativa la imagen completa o el recorte", () => {
    expect(exportRegion(null, 1920, 1080)).toEqual({ x: 0, y: 0, width: 1920, height: 1080 });
    expect(exportRegion({ x: 10.4, y: 20.6, width: 100.2, height: 50.5 }, 1920, 1080)).toEqual({ x: 10, y: 21, width: 100, height: 51 });
    expect(exportRegion({ x: 1900, y: 1000, width: 500, height: 500 }, 1920, 1080)).toEqual({ x: 1900, y: 1000, width: 20, height: 80 });
  });

  it("pixela con bloques grandes para que el texto no se lea", () => {
    expect(pixelSizeFor({ x: 0, y: 0, width: 200, height: 24 })).toBe(10);
    expect(pixelSizeFor({ x: 0, y: 0, width: 300, height: 90 })).toBe(30);
  });
});
