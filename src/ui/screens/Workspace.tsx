import { useEffect } from "react";
import { isAcceptedImage } from "../../domain/evidence";
import { useWorkspace } from "../store/context";
import { filesFrom, hasFiles } from "../gallery/files";
import { useEvidenceIntake } from "../gallery/useEvidenceIntake";
import { CaseDetail, type GithubNav } from "./CaseDetail";
import { CaseFormDialog } from "./CaseFormDialog";
import { CaseListPane } from "./CaseListPane";
import { EvidenceEditorHost } from "./EvidenceEditorHost";

export function Workspace({ onOpenAccountSettings, onOpenProjectGithub }: GithubNav) {
  const projectId = useWorkspace((s) => s.project?.id);
  const loadProjectLinks = useWorkspace((s) => s.loadProjectLinks);
  const request = useWorkspace((s) => s.caseFormRequest);
  const editorOpen = useWorkspace((s) => s.editor !== null);
  const openCaseForm = useWorkspace((s) => s.openCaseForm);
  const closeCaseForm = useWorkspace((s) => s.closeCaseForm);
  const intake = useEvidenceIntake();

  // Vínculos caso-issue del proyecto: alimentan el ícono de la lista y los chips del caso.
  useEffect(() => {
    if (projectId) void loadProjectLinks(projectId);
  }, [projectId, loadProjectLinks]);

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
      <CaseListPane onNewCase={() => openCaseForm()} />
      <CaseDetail onEdit={(caseId) => openCaseForm(caseId)} onOpenAccountSettings={onOpenAccountSettings} onOpenProjectGithub={onOpenProjectGithub} />
      {request && !editorOpen && <CaseFormDialog key={request.caseId ?? "new"} caseId={request.caseId ?? undefined} onClose={closeCaseForm} />}
      <EvidenceEditorHost />
    </main>
  );
}
