import { describe, expect, it } from "vitest";
import { dragShape, isShapeMeaningful, moveShape, placeShape, shapeOrigin, styleFor, transformShape } from "./shape-factory";

const style = styleFor(2400, 1200, "#E02424", "m");

describe("fábrica de formas", () => {
  it("escala el estilo con el tamaño de la imagen", () => {
    expect(style).toEqual({ color: "#E02424", strokeWidth: 6, fontSize: 40, markerRadius: 27 });
    expect(styleFor(300, 200, "#E02424", "s")).toEqual({ color: "#E02424", strokeWidth: 2, fontSize: 14, markerRadius: 10 });
  });

  it("crea formas por clic y por arrastre", () => {
    expect(placeShape("marker", "s1", { x: 5, y: 6 }, style)).toEqual({ id: "s1", kind: "marker", x: 5, y: 6, color: "#E02424", radius: 27 });
    expect(placeShape("text", "s2", { x: 5, y: 6 }, style)).toMatchObject({ kind: "text", text: "Texto", fontSize: 40 });
    expect(dragShape("arrow", "s3", { x: 1, y: 2 }, { x: 30, y: 40 }, style)).toMatchObject({ points: [1, 2, 30, 40], strokeWidth: 6 });
    expect(dragShape("rect", "s4", { x: 50, y: 50 }, { x: 10, y: 20 }, style)).toMatchObject({ x: 10, y: 20, width: 40, height: 30 });
    expect(dragShape("highlight", "s5", { x: 0, y: 0 }, { x: 9, y: 9 }, style)).toEqual({ id: "s5", kind: "highlight", x: 0, y: 0, width: 9, height: 9 });
  });

  it("descarta formas diminutas", () => {
    expect(isShapeMeaningful(dragShape("rect", "x", { x: 0, y: 0 }, { x: 2, y: 2 }, style))).toBe(false);
    expect(isShapeMeaningful(dragShape("arrow", "x", { x: 0, y: 0 }, { x: 1, y: 1 }, style))).toBe(false);
    expect(isShapeMeaningful(placeShape("marker", "x", { x: 0, y: 0 }, style))).toBe(true);
  });

  it("mueve y transforma según el origen de cada forma", () => {
    const arrow = dragShape("arrow", "a", { x: 0, y: 0 }, { x: 10, y: 10 }, style);
    expect(shapeOrigin(arrow)).toEqual({ x: 0, y: 0 });
    expect(moveShape(arrow, 5, -5)).toMatchObject({ points: [5, -5, 15, 5] });
    const ellipse = dragShape("ellipse", "e", { x: 10, y: 10 }, { x: 50, y: 30 }, style);
    expect(shapeOrigin(ellipse)).toEqual({ x: 30, y: 20 });
    expect(transformShape(ellipse, { x: 30, y: 20, scaleX: 2, scaleY: 1 })).toMatchObject({ x: -10, y: 10, width: 80, height: 20 });
    const rect = dragShape("rect", "r", { x: 0, y: 0 }, { x: 10, y: 10 }, style);
    expect(transformShape(rect, { x: 3, y: 4, scaleX: 3, scaleY: 0.1 })).toMatchObject({ x: 3, y: 4, width: 30, height: 4 });
    const text = placeShape("text", "t", { x: 0, y: 0 }, style);
    expect(transformShape(text, { x: 1, y: 1, scaleX: 9, scaleY: 9 })).toMatchObject({ fontSize: 200 });
  });
});
