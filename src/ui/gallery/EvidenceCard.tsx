import { ArrowLeft, ArrowRight, Pencil, PenLine, Trash2 } from "lucide-react";
import type { CaseRecord } from "../../domain/cases";
import { evidenceLabel, type Evidence } from "../../domain/evidence";
import { iconButton, inputCls } from "../components/styles";

export const DRAG_TYPE = "application/x-qapture-evidence";

export interface EvidenceCardProps {
  evidence: Evidence;
  url: string;
  isFirst: boolean;
  isLast: boolean;
  otherCases: CaseRecord[];
  onView(): void;
  onAnnotate(): void;
  onRename(): void;
  onShift(delta: -1 | 1): void;
  onMove(targetCaseId: string): void;
  onDelete(): void;
  onDropEvidence(draggedId: string): void;
}

export function EvidenceCard(p: EvidenceCardProps) {
  const label = evidenceLabel(p.evidence);
  return (
    <li
      draggable
      onDragStart={(e) => {
        e.dataTransfer.setData(DRAG_TYPE, p.evidence.id);
        e.dataTransfer.effectAllowed = "move";
      }}
      onDragOver={(e) => {
        if (Array.from(e.dataTransfer.types).includes(DRAG_TYPE)) e.preventDefault();
      }}
      onDrop={(e) => {
        const dragged = e.dataTransfer.getData(DRAG_TYPE);
        if (!dragged) return;
        e.preventDefault();
        e.stopPropagation();
        if (dragged !== p.evidence.id) p.onDropEvidence(dragged);
      }}
      className="overflow-hidden rounded-xl border border-t4s-line bg-white"
    >
      <button type="button" aria-label={`Ver ${label.toLowerCase()}`} onClick={p.onView} className="block aspect-video w-full bg-t4s-cream">
        <img src={p.url} alt={label} className="h-full w-full object-contain" draggable={false} />
      </button>
      <div className="space-y-2 p-3">
        <p className="text-xs font-semibold text-t4s-blue">{label}</p>
        <p className="line-clamp-3 whitespace-pre-wrap text-sm">{p.evidence.description}</p>
        <div className="flex flex-wrap items-center gap-1">
          <button type="button" aria-label={`Anotar ${label.toLowerCase()}`} title="Anotar" className={iconButton} onClick={p.onAnnotate}>
            <PenLine size={16} />
          </button>
          <button type="button" aria-label={`Editar descripción de ${label.toLowerCase()}`} title="Editar descripción" className={iconButton} onClick={p.onRename}>
            <Pencil size={16} />
          </button>
          <button type="button" aria-label={`Mover ${label.toLowerCase()} antes`} title="Mover antes" className={iconButton} disabled={p.isFirst} onClick={() => p.onShift(-1)}>
            <ArrowLeft size={16} />
          </button>
          <button type="button" aria-label={`Mover ${label.toLowerCase()} después`} title="Mover después" className={iconButton} disabled={p.isLast} onClick={() => p.onShift(1)}>
            <ArrowRight size={16} />
          </button>
          <button type="button" aria-label={`Eliminar ${label.toLowerCase()}`} title="Eliminar" className={iconButton} onClick={p.onDelete}>
            <Trash2 size={16} />
          </button>
        </div>
        {p.otherCases.length > 0 && (
          <select aria-label={`Mover ${label.toLowerCase()} a otro caso`} className={`${inputCls} py-1 text-xs`} value="" onChange={(e) => e.target.value && p.onMove(e.target.value)}>
            <option value="">Mover a otro caso…</option>
            {p.otherCases.map((c) => (
              <option key={c.id} value={c.id}>
                {c.code} · {c.title}
              </option>
            ))}
          </select>
        )}
      </div>
    </li>
  );
}
