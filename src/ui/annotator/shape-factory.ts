import type { Shape } from "../../domain/annotations";
import { isMeaningful, MIN_SIZE, rectFromPoints, type Point } from "./geometry";

export type Tool = "select" | "arrow" | "marker" | "text" | "rect" | "ellipse" | "highlight" | "blur" | "crop";
export type DragTool = "arrow" | "rect" | "ellipse" | "highlight" | "blur";
export type SizeLevel = "s" | "m" | "l";

export interface ToolStyle {
  color: string;
  strokeWidth: number;
  fontSize: number;
  markerRadius: number;
}

const DRAG_TOOLS: readonly Tool[] = ["arrow", "rect", "ellipse", "highlight", "blur"];
const FACTOR: Record<SizeLevel, number> = { s: 0.6, m: 1, l: 1.6 };
const clamp = (value: number, min: number, max: number) => Math.min(Math.max(value, min), max);

export const isDragTool = (tool: Tool): tool is DragTool => DRAG_TOOLS.includes(tool);

/** Grosores relativos al tamaño de la imagen, para que se vean igual en una captura chica o en una 4K. */
export function styleFor(imageW: number, imageH: number, color: string, level: SizeLevel): ToolStyle {
  const base = Math.max(imageW, imageH, 1);
  const f = FACTOR[level];
  return {
    color,
    strokeWidth: Math.max(2, Math.round((base / 400) * f)),
    fontSize: Math.max(14, Math.round((base / 60) * f)),
    markerRadius: Math.max(10, Math.round((base / 90) * f)),
  };
}

export function placeShape(tool: "marker" | "text", id: string, p: Point, style: ToolStyle): Shape {
  if (tool === "marker") return { id, kind: "marker", x: p.x, y: p.y, color: style.color, radius: style.markerRadius };
  return { id, kind: "text", x: p.x, y: p.y, text: "Texto", color: style.color, fontSize: style.fontSize };
}

export function dragShape(tool: DragTool, id: string, start: Point, end: Point, style: ToolStyle): Shape {
  if (tool === "arrow") return { id, kind: "arrow", points: [start.x, start.y, end.x, end.y], color: style.color, strokeWidth: style.strokeWidth };
  const r = rectFromPoints(start, end);
  if (tool === "highlight" || tool === "blur") return { id, kind: tool, ...r };
  return { id, kind: tool, ...r, color: style.color, strokeWidth: style.strokeWidth };
}

export function isShapeMeaningful(s: Shape): boolean {
  if (s.kind === "arrow") return Math.hypot(s.points[2] - s.points[0], s.points[3] - s.points[1]) >= MIN_SIZE;
  if (s.kind === "marker" || s.kind === "text") return true;
  return isMeaningful(s);
}

/** Posición del nodo Konva de la forma: las flechas usan sus puntos y las elipses su centro. */
export function shapeOrigin(s: Shape): Point {
  if (s.kind === "arrow") return { x: 0, y: 0 };
  if (s.kind === "ellipse") return { x: s.x + s.width / 2, y: s.y + s.height / 2 };
  return { x: s.x, y: s.y };
}

export function moveShape(s: Shape, dx: number, dy: number): Shape {
  if (s.kind === "arrow") {
    const [x1, y1, x2, y2] = s.points;
    return { ...s, points: [x1 + dx, y1 + dy, x2 + dx, y2 + dy] };
  }
  return { ...s, x: s.x + dx, y: s.y + dy };
}

export function transformShape(s: Shape, t: { x: number; y: number; scaleX: number; scaleY: number }): Shape {
  switch (s.kind) {
    case "arrow":
      return s;
    case "marker":
      return { ...s, x: t.x, y: t.y, radius: clamp(s.radius * Math.max(t.scaleX, t.scaleY), 8, 120) };
    case "text":
      return { ...s, x: t.x, y: t.y, fontSize: clamp(Math.round(s.fontSize * t.scaleY), 10, 200) };
    case "ellipse": {
      const width = Math.max(MIN_SIZE, s.width * t.scaleX);
      const height = Math.max(MIN_SIZE, s.height * t.scaleY);
      return { ...s, x: t.x - width / 2, y: t.y - height / 2, width, height };
    }
    default:
      return { ...s, x: t.x, y: t.y, width: Math.max(MIN_SIZE, s.width * t.scaleX), height: Math.max(MIN_SIZE, s.height * t.scaleY) };
  }
}
