import { z } from "zod";

export const ANNOTATIONS_VERSION = 1 as const;
export const DEFAULT_COLOR = "#E02424";
export const ANNOTATION_COLORS = [
  { name: "Rojo", value: "#E02424" },
  { name: "Azul T4S", value: "#2B629E" },
  { name: "Lima T4S", value: "#BAC82D" },
  { name: "Amarillo", value: "#F5B700" },
  { name: "Carbón", value: "#292B2D" },
  { name: "Blanco", value: "#FFFFFF" },
] as const;

const num = z.number();
const size = z.number().positive();
const color = z.string().regex(/^#[0-9A-Fa-f]{6}$/);
const id = z.string().min(1).max(40);
const rect = { x: num, y: num, width: size, height: size };

const shapeSchema = z.discriminatedUnion("kind", [
  z.strictObject({ id, kind: z.literal("arrow"), points: z.tuple([num, num, num, num]), color, strokeWidth: size }),
  z.strictObject({ id, kind: z.literal("marker"), x: num, y: num, color, radius: size }),
  z.strictObject({ id, kind: z.literal("text"), x: num, y: num, text: z.string().max(500), color, fontSize: size }),
  z.strictObject({ id, kind: z.literal("rect"), ...rect, color, strokeWidth: size }),
  z.strictObject({ id, kind: z.literal("ellipse"), ...rect, color, strokeWidth: size }),
  z.strictObject({ id, kind: z.literal("highlight"), ...rect }),
  z.strictObject({ id, kind: z.literal("blur"), ...rect }),
]);

/** Capa editable sobre el PNG original inmutable (spec §6.4). Coordenadas en píxeles del original. */
export const annotationsSchema = z.strictObject({
  version: z.literal(ANNOTATIONS_VERSION),
  crop: z.strictObject(rect).nullable(),
  shapes: z.array(shapeSchema).max(500),
});

export type Shape = z.infer<typeof shapeSchema>;
export type ShapeKind = Shape["kind"];
export type Annotations = z.infer<typeof annotationsSchema>;
export interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export const emptyAnnotations = (): Annotations => ({ version: ANNOTATIONS_VERSION, crop: null, shapes: [] });

export const serializeAnnotations = (a: Annotations): string => JSON.stringify(a);

export function parseAnnotations(text: string): Annotations | null {
  try {
    const result = annotationsSchema.safeParse(JSON.parse(text));
    return result.success ? result.data : null;
  } catch {
    return null;
  }
}

/** Los marcadores ①②③ se numeran por orden de creación; al borrar uno, los siguientes se renumeran. */
export function markerNumbers(shapes: Shape[]): Map<string, number> {
  const numbers = new Map<string, number>();
  for (const s of shapes) if (s.kind === "marker") numbers.set(s.id, numbers.size + 1);
  return numbers;
}

export function nextShapeId(shapes: Shape[]): string {
  const max = shapes.reduce((m, s) => {
    const n = Number(s.id.slice(1));
    return s.id.startsWith("s") && Number.isInteger(n) && n > m ? n : m;
  }, 0);
  return `s${max + 1}`;
}

export const addShape = (a: Annotations, shape: Shape): Annotations => ({ ...a, shapes: [...a.shapes, shape] });
export const updateShape = (a: Annotations, shapeId: string, next: Shape): Annotations => ({
  ...a,
  shapes: a.shapes.map((s) => (s.id === shapeId ? next : s)),
});
export const removeShape = (a: Annotations, shapeId: string): Annotations => ({ ...a, shapes: a.shapes.filter((s) => s.id !== shapeId) });
export const setCrop = (a: Annotations, crop: Rect | null): Annotations => ({ ...a, crop });
export const findShape = (a: Annotations, shapeId: string): Shape | null => a.shapes.find((s) => s.id === shapeId) ?? null;
