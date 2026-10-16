import { useState } from "react";
import type { Evidence } from "../../domain/evidence";
import { Dialog, DialogFooter } from "../components/Dialog";
import { FormField } from "../components/FormField";
import { btn, inputCls } from "../components/styles";
import { useWorkspace } from "../store/context";

export function DescriptionDialog({ evidence, onClose }: { evidence: Evidence; onClose: () => void }) {
  const update = useWorkspace((s) => s.updateEvidenceDescription);
  const [value, setValue] = useState(evidence.description);
  const [error, setError] = useState<string | undefined>();
  const [saving, setSaving] = useState(false);
  return (
    <Dialog title="Editar descripción" onClose={onClose}>
      <form
        className="grid gap-4"
        onSubmit={async (e) => {
          e.preventDefault();
          setSaving(true);
          const result = await update(evidence.id, value);
          setSaving(false);
          if (result.ok) onClose();
          else setError(result.errors.description ?? result.errors._form);
        }}
      >
        <FormField label="Descripción" htmlFor="evidence-description-edit" error={error}>
          <textarea id="evidence-description-edit" rows={6} className={inputCls} value={value} onChange={(e) => setValue(e.target.value)} />
        </FormField>
        <DialogFooter>
          <button type="button" className={btn.secondary} onClick={onClose}>
            Cancelar
          </button>
          <button type="submit" className={btn.primary} disabled={saving}>
            Guardar
          </button>
        </DialogFooter>
      </form>
    </Dialog>
  );
}
