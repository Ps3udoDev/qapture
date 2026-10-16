import { describe, expect, it } from "vitest";
import {
  addShape,
  emptyAnnotations,
  markerNumbers,
  nextShapeId,
  parseAnnotations,
  removeShape,
  serializeAnnotations,
  setCrop,
  updateShape,
  type Annotations,
  type Shape,
} from "./annotations";

const marker = (id: string): Shape => ({ id, kind: "marker", x: 10, y: 10, color: "#E02424", radius: 18 });

const full: Annotations = {
  version: 1,
  crop: { x: 5, y: 5, width: 300, height: 200 },
  shapes: [
    { id: "s1", kind: "arrow", points: [0, 0, 50, 60], color: "#E02424", strokeWidth: 6 },
    marker("s2"),
    { id: "s3", kind: "text", x: 1, y: 2, text: "Monto ✅", color: "#2B629E", fontSize: 28 },
    { id: "s4", kind: "rect", x: 1, y: 1, width: 20, height: 10, color: "#BAC82D", strokeWidth: 4 },
    { id: "s5", kind: "ellipse", x: 1, y: 1, width: 20, height: 10, color: "#BAC82D", strokeWidth: 4 },
    { id: "s6", kind: "highlight", x: 1, y: 1, width: 20, height: 10 },
    { id: "s7", kind: "blur", x: 1, y: 1, width: 20, height: 10 },
  ],
};

describe("anotaciones", () => {
  it("serializar y deserializar da el mismo resultado", () => {
    expect(parseAnnotations(serializeAnnotations(full))).toEqual(full);
    expect(parseAnnotations(serializeAnnotations(emptyAnnotations()))).toEqual(emptyAnnotations());
  });

  it("rechaza JSON inválido, otra versión o formas desconocidas", () => {
    expect(parseAnnotations("no es json")).toBeNull();
    expect(parseAnnotations(JSON.stringify({ ...full, version: 2 }))).toBeNull();
    expect(parseAnnotations(JSON.stringify({ ...full, shapes: [{ id: "x", kind: "estrella" }] }))).toBeNull();
    expect(parseAnnotations(JSON.stringify({ ...full, extra: 1 }))).toBeNull();
  });

  it("numera los marcadores en orden y se reordena al borrar uno", () => {
    let a = emptyAnnotations();
    for (const id of ["m1", "m2", "m3"]) a = addShape(a, marker(id));
    a = addShape(a, full.shapes[0]);
    expect([...markerNumbers(a.shapes)]).toEqual([["m1", 1], ["m2", 2], ["m3", 3]]);
    a = removeShape(a, "m2");
    expect([...markerNumbers(a.shapes)]).toEqual([["m1", 1], ["m3", 2]]);
  });

  it("genera ids únicos y actualiza sin mutar", () => {
    expect(nextShapeId([])).toBe("s1");
    expect(nextShapeId(full.shapes)).toBe("s8");
    const moved = updateShape(full, "s2", { ...(marker("s2") as Extract<Shape, { kind: "marker" }>), x: 99 });
    expect(moved.shapes[1]).toMatchObject({ x: 99 });
    expect(full.shapes[1]).toMatchObject({ x: 10 });
    expect(setCrop(full, null).crop).toBeNull();
    expect(full.crop).not.toBeNull();
  });
});
