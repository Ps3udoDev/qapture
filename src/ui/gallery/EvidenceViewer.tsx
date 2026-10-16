import { ChevronLeft, ChevronRight, PenLine } from "lucide-react";
import { useEffect } from "react";
import type { Evidence } from "../../domain/evidence";
import { Dialog } from "../components/Dialog";
import { btn } from "../components/styles";

export function EvidenceViewer({ items, index, urlFor, onIndex, onAnnotate, onClose }: {
  items: Evidence[];
  index: number;
  urlFor(e: Evidence): string;
  onIndex(index: number): void;
  onAnnotate(e: Evidence): void;
  onClose(): void;
}) {
  const current = items[index];
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "ArrowLeft" && index > 0) onIndex(index - 1);
      if (e.key === "ArrowRight" && index < items.length - 1) onIndex(index + 1);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [index, items.length, onIndex]);
  if (!current) return null;

  return (
    <Dialog title={`Evidencia ${index + 1} de ${items.length}`} wide onClose={onClose}>
      <img src={urlFor(current)} alt={`Evidencia ${index + 1}`} className="max-h-[65vh] w-full rounded-lg bg-t4s-cream object-contain" />
      <p className="mt-3 whitespace-pre-wrap text-sm">{current.description}</p>
      <div className="mt-4 flex justify-between">
        <div className="flex gap-2">
          <button type="button" className={btn.secondary} disabled={index === 0} onClick={() => onIndex(index - 1)}>
            <ChevronLeft size={16} /> Anterior
          </button>
          <button type="button" className={btn.secondary} disabled={index === items.length - 1} onClick={() => onIndex(index + 1)}>
            Siguiente <ChevronRight size={16} />
          </button>
        </div>
        <button type="button" className={btn.primary} onClick={() => onAnnotate(current)}>
          <PenLine size={16} /> Anotar
        </button>
      </div>
    </Dialog>
  );
}
