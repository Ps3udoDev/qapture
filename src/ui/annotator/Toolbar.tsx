import { Circle, Crop, Grid3x3, Hash, Highlighter, MousePointer2, MoveUpRight, Redo2, Square, Trash2, Type, Undo2, X, type LucideIcon } from "lucide-react";
import { ANNOTATION_COLORS } from "../../domain/annotations";
import { iconButton, inputCls } from "../components/styles";
import type { SizeLevel, Tool } from "./shape-factory";

const TOOLS: { id: Tool; label: string; key: string; icon: LucideIcon }[] = [
  { id: "select", label: "Seleccionar", key: "V", icon: MousePointer2 },
  { id: "arrow", label: "Flecha", key: "A", icon: MoveUpRight },
  { id: "marker", label: "Número", key: "N", icon: Hash },
  { id: "text", label: "Texto", key: "T", icon: Type },
  { id: "rect", label: "Recuadro", key: "R", icon: Square },
  { id: "ellipse", label: "Elipse", key: "E", icon: Circle },
  { id: "highlight", label: "Resaltado", key: "H", icon: Highlighter },
  { id: "blur", label: "Desenfoque", key: "B", icon: Grid3x3 },
  { id: "crop", label: "Recortar", key: "C", icon: Crop },
];

export const TOOL_BY_KEY: Record<string, Tool> = Object.fromEntries(TOOLS.map((t) => [t.key.toLowerCase(), t.id]));

export interface ToolbarProps {
  tool: Tool;
  onTool(tool: Tool): void;
  color: string;
  onColor(color: string): void;
  sizeLevel: SizeLevel;
  onSizeLevel(level: SizeLevel): void;
  canUndo: boolean;
  canRedo: boolean;
  onUndo(): void;
  onRedo(): void;
  hasSelection: boolean;
  onDelete(): void;
  hasCrop: boolean;
  onClearCrop(): void;
}

export function Toolbar(p: ToolbarProps) {
  const pressed = (on: boolean) => `${iconButton} ${on ? "bg-t4s-blue/15 text-t4s-blue" : ""}`;
  return (
    <div role="toolbar" aria-label="Herramientas de anotación" className="flex flex-wrap items-center gap-1 border-b border-t4s-line bg-white px-3 py-2">
      {TOOLS.map((t) => (
        <button key={t.id} type="button" aria-label={t.label} title={`${t.label} (${t.key})`} aria-pressed={p.tool === t.id} className={pressed(p.tool === t.id)} onClick={() => p.onTool(t.id)}>
          <t.icon size={18} />
        </button>
      ))}
      <span className="mx-2 h-6 w-px bg-t4s-line" />
      {ANNOTATION_COLORS.map((c) => (
        <button
          key={c.value}
          type="button"
          aria-label={`Color ${c.name}`}
          aria-pressed={p.color === c.value}
          className={`h-6 w-6 rounded-full border-2 ${p.color === c.value ? "border-t4s-charcoal" : "border-t4s-line"}`}
          style={{ backgroundColor: c.value }}
          onClick={() => p.onColor(c.value)}
        />
      ))}
      <select aria-label="Grosor" className={`${inputCls} ml-2 w-28 py-1`} value={p.sizeLevel} onChange={(e) => p.onSizeLevel(e.target.value as SizeLevel)}>
        <option value="s">Fino</option>
        <option value="m">Medio</option>
        <option value="l">Grueso</option>
      </select>
      <span className="mx-2 h-6 w-px bg-t4s-line" />
      <button type="button" aria-label="Deshacer" title="Deshacer (Ctrl+Z)" className={iconButton} disabled={!p.canUndo} onClick={p.onUndo}>
        <Undo2 size={18} />
      </button>
      <button type="button" aria-label="Rehacer" title="Rehacer (Ctrl+Y)" className={iconButton} disabled={!p.canRedo} onClick={p.onRedo}>
        <Redo2 size={18} />
      </button>
      <button type="button" aria-label="Eliminar anotación" title="Eliminar anotación (Supr)" className={iconButton} disabled={!p.hasSelection} onClick={p.onDelete}>
        <Trash2 size={18} />
      </button>
      {p.hasCrop && (
        <button type="button" className={`${iconButton} flex items-center gap-1 text-sm`} onClick={p.onClearCrop}>
          <X size={16} /> Quitar recorte
        </button>
      )}
    </div>
  );
}
