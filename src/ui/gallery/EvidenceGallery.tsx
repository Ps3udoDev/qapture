import { Upload } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import type { CaseRecord } from "../../domain/cases";
import { ACCEPTED_IMAGE_TYPES, evidenceLabel, moveItem, type Evidence } from "../../domain/evidence";
import { ConfirmDialog } from "../components/ConfirmDialog";
import { btn } from "../components/styles";
import { useWorkspace } from "../store/context";
import { DescriptionDialog } from "./DescriptionDialog";
import { EvidenceCard } from "./EvidenceCard";
import { EvidenceViewer } from "./EvidenceViewer";
import { useEvidenceIntake } from "./useEvidenceIntake";

export function EvidenceGallery({ caseRecord }: { caseRecord: CaseRecord }) {
  const evidence = useWorkspace((s) => s.evidence);
  const cases = useWorkspace((s) => s.cases);
  const imageUrl = useWorkspace((s) => s.imageUrl);
  const editEvidence = useWorkspace((s) => s.editEvidence);
  const moveEvidence = useWorkspace((s) => s.moveEvidence);
  const reorderEvidence = useWorkspace((s) => s.reorderEvidence);
  const deleteEvidence = useWorkspace((s) => s.deleteEvidence);
  const intake = useEvidenceIntake();
  const picker = useRef<HTMLInputElement>(null);
  const [viewing, setViewing] = useState<number | null>(null);
  const [renaming, setRenaming] = useState<Evidence | null>(null);
  const [deleting, setDeleting] = useState<Evidence | null>(null);
  useEffect(() => {
    if (viewing !== null && viewing >= evidence.length) setViewing(null);
  }, [viewing, evidence.length]);
  useEffect(() => setViewing(null), [caseRecord.id]);
  const ids = evidence.map((e) => e.id);
  const others = cases.filter((c) => c.id !== caseRecord.id);
  const urlFor = (e: Evidence) => imageUrl(e.renderedPath, e.updatedAt);
  const annotate = (e: Evidence) => {
    setViewing(null);
    editEvidence(e.id);
  };

  return (
    <section aria-label="Evidencias" className="mt-6">
      <div className="flex items-center justify-between">
        <h3 className="font-semibold">Evidencias ({evidence.length})</h3>
        <button type="button" className={btn.secondary} onClick={() => picker.current?.click()}>
          <Upload size={16} /> Elegir imagen
        </button>
        <input
          ref={picker}
          type="file"
          hidden
          multiple
          accept={ACCEPTED_IMAGE_TYPES.join(",")}
          aria-label="Elegir imágenes de evidencia"
          onChange={async (e) => {
            const files = Array.from(e.target.files ?? []);
            e.target.value = "";
            await intake(files);
          }}
        />
      </div>
      {evidence.length === 0 ? (
        <p className="mt-3 rounded-xl border border-dashed border-t4s-line p-6 text-center text-sm text-t4s-gray">
          Pega una captura con <kbd className="rounded bg-t4s-cream px-1">Ctrl</kbd>+<kbd className="rounded bg-t4s-cream px-1">V</kbd>, arrástrala aquí o elige un archivo.
        </p>
      ) : (
        <ol aria-label="Lista de evidencias" className="mt-3 grid grid-cols-2 gap-4 xl:grid-cols-3">
          {evidence.map((e, index) => (
            <EvidenceCard
              key={e.id}
              evidence={e}
              url={urlFor(e)}
              isFirst={index === 0}
              isLast={index === evidence.length - 1}
              otherCases={others}
              onView={() => setViewing(index)}
              onAnnotate={() => annotate(e)}
              onRename={() => setRenaming(e)}
              onShift={(delta) => void reorderEvidence(moveItem(ids, index, index + delta))}
              onMove={(target) => void moveEvidence(e.id, target)}
              onDelete={() => setDeleting(e)}
              onDropEvidence={(dragged) => {
                const from = ids.indexOf(dragged);
                if (from >= 0) void reorderEvidence(moveItem(ids, from, index));
              }}
            />
          ))}
        </ol>
      )}
      {viewing !== null && viewing < evidence.length && (
        <EvidenceViewer items={evidence} index={viewing} urlFor={urlFor} onIndex={setViewing} onAnnotate={annotate} onClose={() => setViewing(null)} />
      )}
      {renaming && <DescriptionDialog evidence={renaming} onClose={() => setRenaming(null)} />}
      {deleting && (
        <ConfirmDialog
          title={`Eliminar ${evidenceLabel(deleting).toLowerCase()}`}
          message="La captura original, su versión anotada y su descripción se eliminarán de forma permanente."
          confirmLabel="Eliminar evidencia"
          onCancel={() => setDeleting(null)}
          onConfirm={async () => {
            const target = deleting;
            setDeleting(null);
            await deleteEvidence(target.id);
          }}
        />
      )}
    </section>
  );
}
