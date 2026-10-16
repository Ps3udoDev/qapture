import { useState, type FormEvent } from "react";
import type { FormErrors } from "../../domain/cases";
import { emptyMeta, type ProjectMeta } from "../../domain/project";
import { Dialog, DialogFooter } from "../components/Dialog";
import { btn } from "../components/styles";
import { useWorkspace } from "../store/context";
import { ProjectMetaFields } from "./ProjectMetaFields";

export function NewProjectDialog({ onClose }: { onClose: () => void }) {
  const templates = useWorkspace((s) => s.templates);
  const createProject = useWorkspace((s) => s.createProject);
  const [meta, setMeta] = useState<ProjectMeta>(emptyMeta);
  const [templateId, setTemplateId] = useState(() => templates.find((t) => t.id === "tpl-funcional")?.id ?? templates[0]?.id ?? "");
  const [errors, setErrors] = useState<FormErrors>({});
  const [busy, setBusy] = useState(false);

  async function submit(e: FormEvent) {
    e.preventDefault();
    setBusy(true);
    const result = await createProject({ meta, templateId });
    setBusy(false);
    if (result.ok) onClose();
    else setErrors(result.errors);
  }

  return (
    <Dialog title="Nueva matriz" kicker="Proyecto de pruebas" onClose={onClose}>
      <form onSubmit={submit} noValidate className="grid grid-cols-2 gap-4">
        <ProjectMetaFields idPrefix="np" meta={meta} errors={errors} onChange={setMeta} />
        <fieldset className="col-span-2">
          <legend className="mb-2 text-sm font-medium">Plantilla de columnas</legend>
          <div className="grid grid-cols-2 gap-2">
            {templates.map((t) => (
              <label
                key={t.id}
                className={`cursor-pointer rounded-lg border p-3 text-sm ${templateId === t.id ? "border-t4s-blue bg-t4s-blue/5" : "border-t4s-line"}`}
              >
                <input type="radio" name="template" value={t.id} checked={templateId === t.id} onChange={() => setTemplateId(t.id)} className="sr-only" />
                <span className="font-medium">{t.name}</span>
                <span className="block text-xs text-t4s-gray">
                  {t.fields.length} columnas{t.builtin ? " · incluida" : ""}
                </span>
              </label>
            ))}
          </div>
        </fieldset>
        {errors._form && (
          <p role="alert" className="col-span-2 text-sm text-red-700">
            {errors._form}
          </p>
        )}
        <DialogFooter>
          <button type="button" className={btn.secondary} onClick={onClose}>
            Cancelar
          </button>
          <button type="submit" className={btn.primary} disabled={busy || !templateId}>
            Crear matriz
          </button>
        </DialogFooter>
      </form>
    </Dialog>
  );
}
