import { useMemo, useState, type FormEvent } from "react";
import { caseToInput, emptyCaseInput, type CaseInput, type FormErrors } from "../../domain/cases";
import { activeFields } from "../../domain/fields";
import { STATUSES, type Status } from "../../domain/status";
import { Dialog, DialogFooter } from "../components/Dialog";
import { FieldControl } from "../components/FieldControl";
import { FormField } from "../components/FormField";
import { btn, inputCls } from "../components/styles";
import { useWorkspace } from "../store/context";

export function CaseFormDialog({ caseId, onClose }: { caseId?: string; onClose: () => void }) {
  const project = useWorkspace((s) => s.project);
  const cases = useWorkspace((s) => s.cases);
  const saveCase = useWorkspace((s) => s.saveCase);
  const existing = caseId ? cases.find((c) => c.id === caseId) : undefined;
  const projectFields = project?.fields ?? [];
  const fields = useMemo(() => activeFields(projectFields), [projectFields]);
  const [input, setInput] = useState<CaseInput>(() => (existing ? caseToInput(projectFields, existing) : emptyCaseInput(projectFields)));
  const [errors, setErrors] = useState<FormErrors>({});
  const [busy, setBusy] = useState(false);

  const setValue = (fieldId: string, value: string) => setInput((i) => ({ ...i, values: { ...i.values, [fieldId]: value } }));

  async function submit(e: FormEvent) {
    e.preventDefault();
    setBusy(true);
    const result = await saveCase(input, caseId);
    setBusy(false);
    if (result.ok) onClose();
    else setErrors(result.errors);
  }

  return (
    <Dialog wide title={existing ? `Editar ${existing.code}` : "Nuevo caso"} kicker="Registro de ejecución" onClose={onClose}>
      <form onSubmit={submit} noValidate className="grid grid-cols-2 gap-4">
        <p className="col-span-2 text-sm text-t4s-gray">Usa una ejecución diferente para cada entorno o repetición de la prueba.</p>
        <FormField label="Caso de prueba" required htmlFor="case-title" error={errors.title} className="col-span-2">
          <input id="case-title" className={inputCls} value={input.title} onChange={(e) => setInput((i) => ({ ...i, title: e.target.value }))} />
        </FormField>
        <FormField label="Estado" htmlFor="case-status" error={errors.status}>
          <select id="case-status" className={inputCls} value={input.status} onChange={(e) => setInput((i) => ({ ...i, status: e.target.value as Status }))}>
            {STATUSES.map((s) => (
              <option key={s}>{s}</option>
            ))}
          </select>
        </FormField>
        {fields.map((f) => (
          <FormField key={f.id} label={f.label} required={f.required} htmlFor={`f-${f.id}`} error={errors[f.id]} className={f.type === "longtext" ? "col-span-2" : ""}>
            <FieldControl id={`f-${f.id}`} field={f} value={input.values[f.id] ?? ""} onChange={(v) => setValue(f.id, v)} />
          </FormField>
        ))}
        {errors._form && (
          <p role="alert" className="col-span-2 text-sm text-red-700">
            {errors._form}
          </p>
        )}
        <DialogFooter>
          <button type="button" className={btn.secondary} onClick={onClose}>
            Cancelar
          </button>
          <button type="submit" className={btn.primary} disabled={busy}>
            Guardar caso
          </button>
        </DialogFooter>
      </form>
    </Dialog>
  );
}
