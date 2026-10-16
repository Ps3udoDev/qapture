import { useEffect, useRef, useState } from "react";
import { DEFAULT_COLOR, findShape, removeShape, setCrop, updateShape, type Annotations } from "../../domain/annotations";
import type { FormErrors } from "../../domain/cases";
import { validateDescription } from "../../domain/evidence";
import { FormField } from "../components/FormField";
import { btn, inputCls } from "../components/styles";
import type { FormResult } from "../store/workspace";
import { AnnotatorCanvas } from "./AnnotatorCanvas";
import { canRedo, canUndo, commit, createHistory, redo, undo } from "./history";
import type { SizeLevel, Tool } from "./shape-factory";
import { TOOL_BY_KEY, Toolbar } from "./Toolbar";
import type { AnnotatorHandle } from "./types";

export interface EditorCase {
  id: string;
  code: string;
  title: string;
}

export interface EditorSaveInput {
  caseId: string;
  description: string;
  annotations: Annotations;
  render: Uint8Array;
}

export interface AnnotationEditorProps {
  title: string;
  imagePath: string;
  loadImage(path: string): Promise<Uint8Array>;
  cases: EditorCase[];
  initialCaseId: string | null;
  caseLocked: boolean;
  initialDescription: string;
  initialAnnotations: Annotations;
  pending: number;
  onSave(input: EditorSaveInput): Promise<FormResult>;
  onCancel(): void;
}

const isTyping = (target: EventTarget | null) =>
  target instanceof HTMLElement && (target.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName));

export function AnnotationEditor(props: AnnotationEditorProps) {
  const { onCancel } = props;
  const canvas = useRef<AnnotatorHandle>(null);
  const [history, setHistory] = useState(() => createHistory(props.initialAnnotations));
  const [tool, setTool] = useState<Tool>("arrow");
  const [color, setColor] = useState<string>(DEFAULT_COLOR);
  const [sizeLevel, setSizeLevel] = useState<SizeLevel>("m");
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [caseId, setCaseId] = useState(props.initialCaseId ?? "");
  const [description, setDescription] = useState(props.initialDescription);
  const [errors, setErrors] = useState<FormErrors>({});
  const [saving, setSaving] = useState(false);
  const annotations = history.present;
  const selected = selectedId ? findShape(annotations, selectedId) : null;
  const apply = (next: Annotations) => setHistory((h) => commit(h, next));

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const typing = isTyping(e.target);
      const key = e.key.toLowerCase();
      if ((e.ctrlKey || e.metaKey) && !typing) {
        if (key === "z" && !e.shiftKey) {
          e.preventDefault();
          setHistory(undo);
          setSelectedId(null);
        } else if (key === "y" || (key === "z" && e.shiftKey)) {
          e.preventDefault();
          setHistory(redo);
        }
        return;
      }
      if (typing || e.altKey || e.ctrlKey || e.metaKey) return;
      if (e.key === "Escape") onCancel();
      else if ((e.key === "Delete" || e.key === "Backspace") && selectedId) {
        setHistory((h) => commit(h, removeShape(h.present, selectedId)));
        setSelectedId(null);
      } else if (TOOL_BY_KEY[key]) setTool(TOOL_BY_KEY[key]);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onCancel, selectedId]);

  const changeColor = (value: string) => {
    setColor(value);
    if (selected && "color" in selected) apply(updateShape(annotations, selected.id, { ...selected, color: value }));
  };

  const save = async () => {
    const next: FormErrors = {};
    if (!caseId) next.caseId = "Elige el caso de la evidencia.";
    const message = validateDescription(description);
    if (message) next.description = message;
    setErrors(next);
    if (Object.keys(next).length || !canvas.current) return;
    setSaving(true);
    try {
      const render = await canvas.current.exportPng();
      const result = await props.onSave({ caseId, description, annotations, render });
      if (!result.ok) setErrors(result.errors);
    } catch {
      setErrors({ _form: "No se pudo generar la imagen anotada." });
    } finally {
      setSaving(false);
    }
  };

  return (
    <div role="dialog" aria-modal="true" aria-label="Editor de evidencia" className="fixed inset-0 z-40 flex flex-col bg-t4s-cream">
      <header className="flex items-center justify-between border-b border-t4s-line bg-white px-5 py-3">
        <h2 className="text-lg font-semibold">{props.title}</h2>
        {props.pending > 0 && <span className="rounded-full bg-t4s-lime/30 px-3 py-1 text-xs font-medium">{props.pending} en cola</span>}
      </header>
      <Toolbar
        tool={tool}
        onTool={setTool}
        color={color}
        onColor={changeColor}
        sizeLevel={sizeLevel}
        onSizeLevel={setSizeLevel}
        canUndo={canUndo(history)}
        canRedo={canRedo(history)}
        onUndo={() => {
          setHistory(undo);
          setSelectedId(null);
        }}
        onRedo={() => setHistory(redo)}
        hasSelection={!!selected}
        onDelete={() => {
          if (!selectedId) return;
          apply(removeShape(annotations, selectedId));
          setSelectedId(null);
        }}
        hasCrop={!!annotations.crop}
        onClearCrop={() => apply(setCrop(annotations, null))}
      />
      <div className="flex min-h-0 flex-1">
        <div className="min-w-0 flex-1">
          <AnnotatorCanvas
            ref={canvas}
            imagePath={props.imagePath}
            loadImage={props.loadImage}
            annotations={annotations}
            tool={tool}
            color={color}
            sizeLevel={sizeLevel}
            selectedId={selectedId}
            onSelect={setSelectedId}
            onCommit={apply}
            onToolDone={() => setTool("select")}
          />
        </div>
        <aside className="flex w-80 shrink-0 flex-col gap-4 overflow-y-auto border-l border-t4s-line bg-white p-5">
          <FormField label="Caso" htmlFor="evidence-case" error={errors.caseId}>
            <select id="evidence-case" className={inputCls} value={caseId} disabled={props.caseLocked} onChange={(e) => setCaseId(e.target.value)}>
              <option value="">Elige un caso…</option>
              {props.cases.map((c) => (
                <option key={c.id} value={c.id}>
                  {c.code} · {c.title}
                </option>
              ))}
            </select>
          </FormField>
          <FormField label="Descripción" htmlFor="evidence-description" error={errors.description}>
            <textarea id="evidence-description" rows={6} className={inputCls} value={description} onChange={(e) => setDescription(e.target.value)} />
          </FormField>
          <p className="text-xs text-t4s-gray">Explica aquí qué señala cada número ①②③.</p>
          {selected?.kind === "text" && (
            <FormField label="Texto" htmlFor="annotation-text">
              <input id="annotation-text" className={inputCls} value={selected.text} onChange={(e) => apply(updateShape(annotations, selected.id, { ...selected, text: e.target.value }))} />
            </FormField>
          )}
          {errors._form && (
            <p role="alert" className="text-sm text-red-700">
              {errors._form}
            </p>
          )}
          <div className="mt-auto flex justify-end gap-2 border-t border-t4s-line pt-4">
            <button type="button" className={btn.secondary} onClick={onCancel}>
              Cancelar
            </button>
            <button type="button" className={btn.primary} disabled={saving} onClick={() => void save()}>
              {saving ? "Guardando…" : "Guardar evidencia"}
            </button>
          </div>
        </aside>
      </div>
    </div>
  );
}
