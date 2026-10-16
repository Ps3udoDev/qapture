import { Rect } from "react-konva";
import type { Rect as Area } from "../../domain/annotations";
import { clampRect } from "./geometry";

const SHADE = "rgba(0,0,0,0.55)";

/** Oscurece lo que queda fuera del recorte. Solo se ve en el editor: no forma parte del render. */
export function CropMask({ crop, width, height }: { crop: Area | null; width: number; height: number }) {
  if (!crop) return null;
  const c = clampRect(crop, width, height);
  return (
    <>
      <Rect x={0} y={0} width={width} height={c.y} fill={SHADE} />
      <Rect x={0} y={c.y + c.height} width={width} height={height - c.y - c.height} fill={SHADE} />
      <Rect x={0} y={c.y} width={c.x} height={c.height} fill={SHADE} />
      <Rect x={c.x + c.width} y={c.y} width={width - c.x - c.width} height={c.height} fill={SHADE} />
      <Rect x={c.x} y={c.y} width={c.width} height={c.height} stroke="#FFFFFF" strokeWidth={2} dash={[8, 6]} strokeScaleEnabled={false} />
    </>
  );
}
