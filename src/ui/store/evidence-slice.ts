import type { StoreApi } from "zustand/vanilla";
import * as useCases from "../../application/use-cases";
import { serializeAnnotations } from "../../domain/annotations";
import type { Evidence } from "../../domain/evidence";
import type { Repositories } from "../../ports/repositories";
import type { EditorSaveInput } from "../annotator/AnnotationEditor";
import type { FormResult, WorkspaceState } from "./workspace";

export type EditorSession = { kind: "new"; imagePath: string; caseId: string | null } | { kind: "edit"; evidence: Evidence };

export interface EditorRequest {
  imagePath: string;
  caseId?: string | null;
}

export interface EvidenceSlice {
  evidence: Evidence[];
  editor: EditorSession | null;
  editorQueue: EditorRequest[];
  loadEvidence(): Promise<void>;
  intakeImages(images: Uint8Array[]): Promise<void>;
  openEvidenceEditor(request: EditorRequest): void;
  editEvidence(evidenceId: string): void;
  cancelEditor(): Promise<void>;
  saveEditor(input: EditorSaveInput): Promise<FormResult>;
  readImage(path: string): Promise<Uint8Array>;
  imageUrl(path: string, version: string): string;
  updateEvidenceDescription(evidenceId: string, description: string): Promise<FormResult>;
  moveEvidence(evidenceId: string, targetCaseId: string): Promise<void>;
  reorderEvidence(orderedIds: string[]): Promise<void>;
  deleteEvidence(evidenceId: string): Promise<void>;
}

type Set = StoreApi<WorkspaceState>["setState"];
type Get = StoreApi<WorkspaceState>["getState"];

export interface SliceHelpers {
  report(error: unknown): void;
  refreshCases(): Promise<void>;
  form<T>(run: () => Promise<useCases.Outcome<T>>, onOk: (value: T) => Promise<void> | void): Promise<FormResult>;
}

export function createEvidenceSlice(repos: Repositories, set: Set, get: Get, h: SliceHelpers): EvidenceSlice {
  const session = (request: EditorRequest): EditorSession => ({
    kind: "new",
    imagePath: request.imagePath,
    caseId: request.caseId ?? get().selectedCaseId,
  });
  const openNext = () => {
    const [head, ...rest] = get().editorQueue;
    set({ editor: head ? session(head) : null, editorQueue: rest });
  };
  const afterChange = async () => {
    await h.refreshCases();
    await get().loadEvidence();
  };

  return {
    evidence: [],
    editor: null,
    editorQueue: [],

    async loadEvidence() {
      const caseId = get().selectedCaseId;
      if (!caseId) {
        set({ evidence: [] });
        return;
      }
      try {
        const evidence = await repos.evidence.list(caseId);
        if (get().selectedCaseId === caseId) set({ evidence });
      } catch (error) {
        h.report(error);
      }
    },

    async intakeImages(images) {
      const caseId = get().selectedCaseId;
      if (!caseId) {
        get().pushToast("error", "Crea o selecciona un caso antes de agregar evidencias.");
        return;
      }
      for (const image of images) {
        try {
          const staged = await repos.evidence.stage(image);
          get().openEvidenceEditor({ imagePath: staged.path, caseId });
        } catch (error) {
          h.report(error);
        }
      }
    },

    openEvidenceEditor(request) {
      if (get().editor) set((s) => ({ editorQueue: [...s.editorQueue, request] }));
      else set({ editor: session(request) });
    },

    editEvidence(evidenceId) {
      const evidence = get().evidence.find((e) => e.id === evidenceId);
      if (evidence && !get().editor) set({ editor: { kind: "edit", evidence } });
    },

    async cancelEditor() {
      const editor = get().editor;
      openNext();
      if (editor?.kind === "new") {
        try {
          await repos.evidence.discardStaged(editor.imagePath);
        } catch {
          // Si falla, el próximo arranque vacía evidence/_staging.
        }
      }
    },

    async saveEditor(input) {
      const editor = get().editor;
      if (!editor) return { ok: false, errors: { _form: "El editor no está abierto." } };
      const annotations = serializeAnnotations(input.annotations);
      if (editor.kind === "new") {
        return h.form(
          () => useCases.attachEvidence(repos, { caseId: input.caseId, stagedPath: editor.imagePath, description: input.description, annotations }, input.render),
          async (saved) => {
            const code = get().cases.find((c) => c.id === saved.caseId)?.code ?? "el caso";
            openNext();
            await afterChange();
            get().pushToast("success", `Evidencia ${saved.position + 1} agregada a ${code}.`);
          },
        );
      }
      return h.form(
        () => useCases.reannotateEvidence(repos, { evidenceId: editor.evidence.id, description: input.description, annotations }, input.render),
        async (saved) => {
          openNext();
          await afterChange();
          get().pushToast("success", `Evidencia ${saved.position + 1} actualizada.`);
        },
      );
    },

    readImage: (path) => repos.evidence.readImage(path),
    imageUrl: (path, version) => repos.evidence.imageUrl(path, version),

    updateEvidenceDescription: (evidenceId, description) =>
      h.form(
        () => useCases.renameEvidence(repos, evidenceId, description),
        async () => {
          await get().loadEvidence();
          get().pushToast("success", "Descripción guardada.");
        },
      ),

    async moveEvidence(evidenceId, targetCaseId) {
      try {
        await repos.evidence.move(evidenceId, targetCaseId);
        await afterChange();
        const code = get().cases.find((c) => c.id === targetCaseId)?.code ?? "otro caso";
        get().pushToast("success", `Evidencia movida a ${code}.`);
      } catch (error) {
        h.report(error);
      }
    },

    async reorderEvidence(orderedIds) {
      const caseId = get().selectedCaseId;
      if (!caseId) return;
      try {
        set({ evidence: await repos.evidence.reorder(caseId, orderedIds) });
      } catch (error) {
        h.report(error);
        await get().loadEvidence();
      }
    },

    async deleteEvidence(evidenceId) {
      const removed = get().evidence.find((e) => e.id === evidenceId);
      try {
        await repos.evidence.remove(evidenceId);
        await afterChange();
        if (removed) get().pushToast("success", `Evidencia ${removed.position + 1} eliminada.`);
      } catch (error) {
        h.report(error);
      }
    },
  };
}
