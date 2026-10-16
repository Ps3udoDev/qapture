import type { Ref } from "react";
import type { Annotations } from "../../domain/annotations";
import type { SizeLevel, Tool } from "./shape-factory";

export interface AnnotatorHandle {
  /** PNG de la región exportada (todo o el recorte) a resolución nativa del original. */
  exportPng(): Promise<Uint8Array>;
}

export interface AnnotatorCanvasProps {
  ref?: Ref<AnnotatorHandle>;
  imagePath: string;
  loadImage(path: string): Promise<Uint8Array>;
  annotations: Annotations;
  tool: Tool;
  color: string;
  sizeLevel: SizeLevel;
  selectedId: string | null;
  onSelect(id: string | null): void;
  onCommit(next: Annotations): void;
  onToolDone?(): void;
}
