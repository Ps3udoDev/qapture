import type { Rect } from "../../domain/annotations";

export interface Point {
  x: number;
  y: number;
}

export const MIN_SIZE = 4;

const clamp = (value: number, min: number, max: number) => Math.min(Math.max(value, min), max);

export const clampPoint = (p: Point, width: number, height: number): Point => ({ x: clamp(p.x, 0, width), y: clamp(p.y, 0, height) });

export const rectFromPoints = (a: Point, b: Point): Rect => ({
  x: Math.min(a.x, b.x),
  y: Math.min(a.y, b.y),
  width: Math.abs(b.x - a.x),
  height: Math.abs(b.y - a.y),
});

export function clampRect(r: Rect, width: number, height: number): Rect {
  const x = clamp(r.x, 0, width);
  const y = clamp(r.y, 0, height);
  const fitsX = r.x >= 0 && r.x + r.width <= width;
  const fitsY = r.y >= 0 && r.y + r.height <= height;
  return {
    x,
    y,
    width: fitsX ? r.width : clamp(r.x + r.width, 0, width) - x,
    height: fitsY ? r.height : clamp(r.y + r.height, 0, height) - y,
  };
}

export const isMeaningful = (r: Rect) => r.width >= MIN_SIZE && r.height >= MIN_SIZE;

/** Escala para mostrar la imagen completa en el espacio disponible, sin agrandarla. */
export function fitScale(imageW: number, imageH: number, boxW: number, boxH: number): number {
  if (imageW <= 0 || imageH <= 0 || boxW <= 0 || boxH <= 0) return 1;
  return Math.min(1, boxW / imageW, boxH / imageH);
}

/** Región a exportar en píxeles enteros del original: todo, o el recorte no destructivo. */
export function exportRegion(crop: Rect | null, width: number, height: number): Rect {
  if (!crop) return { x: 0, y: 0, width, height };
  const c = clampRect(crop, width, height);
  const x = Math.min(Math.round(c.x), width - 1);
  const y = Math.min(Math.round(c.y), height - 1);
  return {
    x,
    y,
    width: Math.max(1, Math.min(width - x, Math.round(c.width))),
    height: Math.max(1, Math.min(height - y, Math.round(c.height))),
  };
}

/** Bloques de al menos 10 px y de un tercio del lado menor: un número de póliza no se puede leer. */
export const pixelSizeFor = (r: Rect) => Math.max(10, Math.round(Math.min(r.width, r.height) / 3));
