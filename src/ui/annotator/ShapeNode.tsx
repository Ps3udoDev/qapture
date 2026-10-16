import Konva from "konva";
import { useEffect, useRef } from "react";
import { Arrow, Circle, Ellipse, Group, Image as KImage, Label, Rect, Tag, Text } from "react-konva";
import type { Shape } from "../../domain/annotations";
import { pixelSizeFor } from "./geometry";
import { shapeOrigin } from "./shape-factory";

export interface Transform {
  x: number;
  y: number;
  scaleX: number;
  scaleY: number;
}

export interface ShapeNodeProps {
  shape: Shape;
  image: HTMLImageElement;
  number: number;
  draggable: boolean;
  preview?: boolean;
  onSelect?(): void;
  onMoved?(dx: number, dy: number): void;
  onTransformed?(t: Transform): void;
}

export function ShapeNode({ shape, image, number, draggable, preview = false, onSelect, onMoved, onTransformed }: ShapeNodeProps) {
  const origin = shapeOrigin(shape);
  const common = {
    id: preview ? undefined : shape.id,
    draggable,
    listening: !preview,
    onMouseDown: () => onSelect?.(),
    onDragEnd: (e: Konva.KonvaEventObject<DragEvent>) => {
      const node = e.target;
      const dx = node.x() - origin.x;
      const dy = node.y() - origin.y;
      node.position(origin);
      onMoved?.(dx, dy);
    },
    onTransformEnd: (e: Konva.KonvaEventObject<Event>) => {
      const node = e.target;
      const t = { x: node.x(), y: node.y(), scaleX: node.scaleX(), scaleY: node.scaleY() };
      node.scale({ x: 1, y: 1 });
      node.position(origin);
      onTransformed?.(t);
    },
  };

  switch (shape.kind) {
    case "arrow": {
      const head = shape.strokeWidth * 4;
      return (
        <Arrow {...common} x={0} y={0} points={shape.points} stroke={shape.color} fill={shape.color} strokeWidth={shape.strokeWidth} pointerLength={head} pointerWidth={head} hitStrokeWidth={Math.max(20, shape.strokeWidth * 3)} />
      );
    }
    case "marker":
      return (
        <Group {...common} x={shape.x} y={shape.y}>
          <Circle radius={shape.radius} fill={shape.color} stroke="#FFFFFF" strokeWidth={Math.max(2, shape.radius / 8)} />
          <Text text={String(number)} width={shape.radius * 2} height={shape.radius * 2} offsetX={shape.radius} offsetY={shape.radius} align="center" verticalAlign="middle" fontSize={shape.radius * 1.15} fontStyle="bold" fontFamily="Arial" fill="#FFFFFF" />
        </Group>
      );
    case "text":
      return (
        <Label {...common} x={shape.x} y={shape.y}>
          <Tag fill="rgba(255,255,255,0.85)" cornerRadius={shape.fontSize / 6} />
          <Text text={shape.text} fontSize={shape.fontSize} fontFamily="Arial" fill={shape.color} padding={shape.fontSize / 4} />
        </Label>
      );
    case "rect":
      return <Rect {...common} x={shape.x} y={shape.y} width={shape.width} height={shape.height} stroke={shape.color} strokeWidth={shape.strokeWidth} />;
    case "ellipse":
      return <Ellipse {...common} x={origin.x} y={origin.y} radiusX={shape.width / 2} radiusY={shape.height / 2} stroke={shape.color} strokeWidth={shape.strokeWidth} />;
    case "highlight":
      return <Rect {...common} x={shape.x} y={shape.y} width={shape.width} height={shape.height} fill="rgba(255,221,0,0.38)" />;
    case "blur":
      return preview ? (
        <Rect x={shape.x} y={shape.y} width={shape.width} height={shape.height} stroke="#FFFFFF" strokeWidth={2} dash={[6, 4]} strokeScaleEnabled={false} listening={false} />
      ) : (
        <PixelatedRegion shape={shape} image={image} common={common} />
      );
  }
}

/** Pixela la región del ORIGINAL que cubre la forma. El render final queda aplanado y no se puede revertir. */
function PixelatedRegion({ shape, image, common }: { shape: Extract<Shape, { kind: "blur" }>; image: HTMLImageElement; common: object }) {
  const ref = useRef<Konva.Image>(null);
  useEffect(() => {
    const node = ref.current;
    if (!node) return;
    node.cache();
    node.getLayer()?.batchDraw();
    return () => {
      node.clearCache();
    };
  }, [shape.x, shape.y, shape.width, shape.height, image]);
  return (
    <KImage
      ref={ref}
      {...common}
      x={shape.x}
      y={shape.y}
      width={shape.width}
      height={shape.height}
      image={image}
      crop={{ x: shape.x, y: shape.y, width: shape.width, height: shape.height }}
      filters={[Konva.Filters.Pixelate]}
      pixelSize={pixelSizeFor(shape)}
    />
  );
}
