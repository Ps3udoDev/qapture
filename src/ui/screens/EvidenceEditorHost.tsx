import { emptyAnnotations, parseAnnotations } from "../../domain/annotations";
import { AnnotationEditor } from "../annotator/AnnotationEditor";
import { useWorkspace } from "../store/context";

export function EvidenceEditorHost() {
  const editor = useWorkspace((s) => s.editor);
  const queue = useWorkspace((s) => s.editorQueue);
  const cases = useWorkspace((s) => s.cases);
  const saveEditor = useWorkspace((s) => s.saveEditor);
  const cancelEditor = useWorkspace((s) => s.cancelEditor);
  const readImage = useWorkspace((s) => s.readImage);
  if (!editor) return null;

  const evidence = editor.kind === "edit" ? editor.evidence : null;
  return (
    <AnnotationEditor
      key={evidence ? `${evidence.id}:${evidence.updatedAt}` : editor.kind === "new" ? editor.imagePath : ""}
      title={evidence ? `Anotar evidencia ${evidence.position + 1}` : "Nueva evidencia"}
      imagePath={evidence ? evidence.originalPath : editor.kind === "new" ? editor.imagePath : ""}
      loadImage={readImage}
      cases={cases.map((c) => ({ id: c.id, code: c.code, title: c.title }))}
      initialCaseId={evidence ? evidence.caseId : editor.kind === "new" ? editor.caseId : null}
      caseLocked={!!evidence}
      initialDescription={evidence?.description ?? ""}
      initialAnnotations={(evidence && parseAnnotations(evidence.annotations)) || emptyAnnotations()}
      pending={queue.length}
      onSave={saveEditor}
      onCancel={() => void cancelEditor()}
    />
  );
}
