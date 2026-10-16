import Konva from "konva";
import { useEffect, useImperativeHandle, useMemo, useRef, useState } from "react";
import { Group, Image as KImage, Layer, Stage, Transformer } from "react-konva";
import { addShape, markerNumbers, nextShapeId, setCrop, updateShape, type Rect, type Shape } from "../../domain/annotations";
import { CropMask } from "./CropMask";
import { clampPoint, clampRect, exportRegion, fitScale, isMeaningful, rectFromPoints, type Point } from "./geometry";
import { useBoxSize, useLoadedImage } from "./hooks";
import { dragShape, isDragTool, isShapeMeaningful, moveShape, placeShape, styleFor, transformShape } from "./shape-factory";
import { ShapeNode } from "./ShapeNode";
import type { AnnotatorCanvasProps } from "./types";

const TRANSFORMABLE = new Set<Shape["kind"]>(["rect", "ellipse", "highlight", "blur", "text", "marker"]);

interface Draft {
  start: Point;
  shape: Shape | null;
  crop: Rect | null;
}

export function AnnotatorCanvas({ ref, imagePath, loadImage, annotations, tool, color, sizeLevel, selectedId, onSelect, onCommit, onToolDone }: AnnotatorCanvasProps) {
  const box = useRef<HTMLDivElement>(null);
  const size = useBoxSize(box);
  const { image, error } = useLoadedImage(imagePath, loadImage);
  const content = useRef<Konva.Layer>(null);
  const transformer = useRef<Konva.Transformer>(null);
  const [draft, setDraft] = useState<Draft | null>(null);
  const scale = image ? fitScale(image.width, image.height, size.width, size.height) : 1;
  const style = image ? styleFor(image.width, image.height, color, sizeLevel) : null;
  const numbers = useMemo(() => markerNumbers(annotations.shapes), [annotations.shapes]);

  useImperativeHandle(
    ref,
    () => ({
      async exportPng() {
        const layer = content.current;
        if (!layer || !image) throw new Error("La imagen aún no está lista.");
        const region = exportRegion(annotations.crop, image.width, image.height);
        // Se exporta solo la capa de contenido (sin selección ni máscara de recorte) a escala 1:1 del original.
        const raw = layer.toCanvas({
          x: region.x * scale,
          y: region.y * scale,
          // +1: Konva redondea hacia abajo (floor) el tamaño del canvas; así nunca queda 1 px más chico.
          width: region.width * scale + 1,
          height: region.height * scale + 1,
          pixelRatio: 1 / scale,
        });
        const out = document.createElement("canvas");
        out.width = region.width;
        out.height = region.height;
        const ctx = out.getContext("2d");
        if (!ctx) throw new Error("No se pudo preparar el render.");
        ctx.drawImage(raw, 0, 0, region.width, region.height, 0, 0, region.width, region.height);
        const blob = await new Promise<Blob>((resolve, reject) =>
          out.toBlob((b) => (b ? resolve(b) : reject(new Error("No se pudo generar el PNG."))), "image/png"),
        );
        return new Uint8Array(await blob.arrayBuffer());
      },
    }),
    [annotations.crop, image, scale],
  );

  useEffect(() => {
    const tr = transformer.current;
    const layer = content.current;
    if (!tr || !layer) return;
    const shape = annotations.shapes.find((s) => s.id === selectedId);
    const node = shape && tool === "select" && TRANSFORMABLE.has(shape.kind) ? layer.findOne(`#${shape.id}`) : undefined;
    tr.keepRatio(shape?.kind === "text" || shape?.kind === "marker");
    tr.nodes(node ? [node] : []);
    tr.getLayer()?.batchDraw();
  }, [annotations, selectedId, tool, scale]);

  const pointer = (e: Konva.KonvaEventObject<MouseEvent>): Point | null => {
    const p = e.target.getStage()?.getPointerPosition();
    if (!p || !image) return null;
    return clampPoint({ x: p.x / scale, y: p.y / scale }, image.width, image.height);
  };

  const onDown = (e: Konva.KonvaEventObject<MouseEvent>) => {
    if (!image || !style) return;
    if (tool === "select") {
      if (e.target === e.target.getStage() || e.target.name() === "background") onSelect(null);
      return;
    }
    const p = pointer(e);
    if (!p) return;
    if (tool === "marker" || tool === "text") {
      const shape = placeShape(tool, nextShapeId(annotations.shapes), p, style);
      onCommit(addShape(annotations, shape));
      if (tool === "text") {
        onSelect(shape.id);
        onToolDone?.();
      }
      return;
    }
    setDraft({ start: p, shape: null, crop: null });
  };

  const onMove = (e: Konva.KonvaEventObject<MouseEvent>) => {
    if (!draft || !style) return;
    const p = pointer(e);
    if (!p) return;
    if (tool === "crop") setDraft({ ...draft, crop: rectFromPoints(draft.start, p) });
    else if (isDragTool(tool)) setDraft({ ...draft, shape: dragShape(tool, "draft", draft.start, p, style) });
  };

  const onUp = () => {
    if (!draft || !image) return;
    const d = draft;
    setDraft(null);
    if (tool === "crop" && d.crop && isMeaningful(d.crop)) {
      onCommit(setCrop(annotations, clampRect(d.crop, image.width, image.height)));
    } else if (d.shape && isShapeMeaningful(d.shape)) {
      onCommit(addShape(annotations, { ...d.shape, id: nextShapeId(annotations.shapes) }));
    }
  };

  return (
    <div ref={box} className="relative h-full w-full overflow-hidden bg-t4s-charcoal/90">
      {error && (
        <p role="alert" className="absolute inset-0 grid place-items-center text-sm text-white">
          {error}
        </p>
      )}
      {!image && !error && <p className="absolute inset-0 grid place-items-center text-sm text-white/80">Cargando imagen…</p>}
      {image && size.width > 0 && size.height > 0 && (
        <Stage
          width={Math.round(image.width * scale)}
          height={Math.round(image.height * scale)}
          onMouseDown={onDown}
          onMouseMove={onMove}
          onMouseUp={onUp}
          onMouseLeave={onUp}
          className="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 shadow-lg"
          style={{ cursor: tool === "select" ? "default" : "crosshair" }}
        >
          <Layer ref={content} scaleX={scale} scaleY={scale}>
            <KImage image={image} name="background" />
            {annotations.shapes.map((s) => (
              <ShapeNode
                key={s.id}
                shape={s}
                image={image}
                number={numbers.get(s.id) ?? 0}
                draggable={tool === "select"}
                onSelect={() => tool === "select" && onSelect(s.id)}
                onMoved={(dx, dy) => onCommit(updateShape(annotations, s.id, moveShape(s, dx, dy)))}
                onTransformed={(t) => onCommit(updateShape(annotations, s.id, transformShape(s, t)))}
              />
            ))}
          </Layer>
          <Layer>
            <Group scaleX={scale} scaleY={scale} listening={false}>
              {draft?.shape && <ShapeNode shape={draft.shape} image={image} number={0} draggable={false} preview />}
              <CropMask crop={draft?.crop ?? annotations.crop} width={image.width} height={image.height} />
            </Group>
            <Transformer ref={transformer} rotateEnabled={false} flipEnabled={false} ignoreStroke />
          </Layer>
        </Stage>
      )}
    </div>
  );
}
