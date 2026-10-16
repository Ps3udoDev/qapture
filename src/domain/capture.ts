export interface Point {
  x: number;
  y: number;
}

export interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface Viewport {
  width: number;
  height: number;
}

/** Lo que recibe `finish_capture` (`capture::crop::Selection` en Rust), en píxeles CSS del overlay. */
export interface CaptureSelection extends Rect {
  viewportWidth: number;
  viewportHeight: number;
}

/** Por debajo de esto (px CSS por lado) se toma como un clic sin arrastre. */
export const MIN_SELECTION = 4;

const clamp = (v: number, max: number) => Math.min(Math.max(v, 0), max);

/** Rectángulo entre el punto inicial y el actual, en cualquier dirección y limitado a la ventana. */
export function rectFrom(start: Point, current: Point, viewport: Viewport): Rect {
  const x1 = clamp(start.x, viewport.width);
  const x2 = clamp(current.x, viewport.width);
  const y1 = clamp(start.y, viewport.height);
  const y2 = clamp(current.y, viewport.height);
  return { x: Math.min(x1, x2), y: Math.min(y1, y2), width: Math.abs(x2 - x1), height: Math.abs(y2 - y1) };
}

export const isUsable = (rect: Rect) => rect.width >= MIN_SELECTION && rect.height >= MIN_SELECTION;

/** Medida junto a la selección: el overlay cubre el monitor, así que px CSS × devicePixelRatio = px de la captura. */
export function sizeLabel(rect: Rect, pixelRatio: number): string {
  return `${Math.round(rect.width * pixelRatio)} × ${Math.round(rect.height * pixelRatio)}`;
}

export const toSelection = (rect: Rect, viewport: Viewport): CaptureSelection => ({
  ...rect,
  viewportWidth: viewport.width,
  viewportHeight: viewport.height,
});
