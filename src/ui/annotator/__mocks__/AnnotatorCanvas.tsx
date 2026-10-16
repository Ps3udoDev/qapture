import { useImperativeHandle } from "react";
import { addShape, nextShapeId } from "../../../domain/annotations";
import type { AnnotatorCanvasProps } from "../types";

export const MOCK_RENDER = new Uint8Array([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);

/** jsdom no tiene <canvas>: en los tests el lienzo es un doble que dibuja con botones. */
export function AnnotatorCanvas({ ref, annotations, onCommit, onSelect }: AnnotatorCanvasProps) {
  useImperativeHandle(ref, () => ({ exportPng: async () => MOCK_RENDER }), []);
  const texts = annotations.shapes.flatMap((s) => (s.kind === "text" ? [s.text] : [])).join("|");
  return (
    <div data-testid="annotator-canvas" data-shapes={annotations.shapes.length} data-texts={texts} data-crop={annotations.crop ? "si" : "no"}>
      <button type="button" onClick={() => onCommit(addShape(annotations, { id: nextShapeId(annotations.shapes), kind: "marker", x: 10, y: 10, color: "#E02424", radius: 18 }))}>
        dibujar marcador
      </button>
      <button
        type="button"
        onClick={() => {
          const id = nextShapeId(annotations.shapes);
          onCommit(addShape(annotations, { id, kind: "text", x: 5, y: 5, text: "Texto", color: "#E02424", fontSize: 28 }));
          onSelect(id);
        }}
      >
        dibujar texto
      </button>
    </div>
  );
}
