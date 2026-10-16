import { useEffect, useState } from "react";
import { isAcceptedImage } from "../../domain/evidence";
import { filesFrom, hasFiles } from "../gallery/files";
import { useEvidenceIntake } from "../gallery/useEvidenceIntake";
import { CaseDetail } from "./CaseDetail";
import { CaseFormDialog } from "./CaseFormDialog";
import { CaseListPane } from "./CaseListPane";
import { EvidenceEditorHost } from "./EvidenceEditorHost";

export function Workspace() {
  const [editing, setEditing] = useState<{ caseId?: string } | null>(null);
  const intake = useEvidenceIntake();

  useEffect(() => {
    // Soltar un archivo fuera de una zona de drop haría que WebView2 navegue al archivo y la app desaparecería.
    const keepInApp = (e: DragEvent) => {
      if (hasFiles(e.dataTransfer)) e.preventDefault();
    };
    window.addEventListener("dragover", keepInApp);
    window.addEventListener("drop", keepInApp);
    return () => {
      window.removeEventListener("dragover", keepInApp);
      window.removeEventListener("drop", keepInApp);
    };
  }, []);

  useEffect(() => {
    // Ctrl+V local (como en la v1): solo si hay una imagen y no hay ningún diálogo abierto.
    const onPaste = (e: ClipboardEvent) => {
      if (document.querySelector('[aria-modal="true"]')) return;
      // Excel o Word copian texto e imagen a la vez: en un campo editable gana el texto.
      const target = e.target;
      const editable = target instanceof HTMLElement && (target.isContentEditable || ["INPUT", "TEXTAREA"].includes(target.tagName));
      if (editable && Array.from(e.clipboardData?.types ?? []).includes("text/plain")) return;
      const images = filesFrom(e.clipboardData).filter(isAcceptedImage);
      if (!images.length) return;
      e.preventDefault();
      void intake(images);
    };
    window.addEventListener("paste", onPaste);
    return () => window.removeEventListener("paste", onPaste);
  }, [intake]);

  return (
    <main className="flex min-h-0 flex-1">
      <CaseListPane onNewCase={() => setEditing({})} />
      <CaseDetail onEdit={(caseId) => setEditing({ caseId })} />
      {editing && <CaseFormDialog caseId={editing.caseId} onClose={() => setEditing(null)} />}
      <EvidenceEditorHost />
    </main>
  );
}
