import { Copy, Pencil, Trash2 } from "lucide-react";
import { useState } from "react";
import { activeFields } from "../../domain/fields";
import { ConfirmDialog } from "../components/ConfirmDialog";
import { displayValue } from "../components/FieldControl";
import { StatusBadge } from "../components/StatusBadge";
import { btn } from "../components/styles";
import { useWorkspace } from "../store/context";

export function CaseDetail({ onEdit }: { onEdit: (caseId: string) => void }) {
  const project = useWorkspace((s) => s.project);
  const cases = useWorkspace((s) => s.cases);
  const selectedId = useWorkspace((s) => s.selectedCaseId);
  const duplicateCase = useWorkspace((s) => s.duplicateCase);
  const deleteCase = useWorkspace((s) => s.deleteCase);
  const [confirming, setConfirming] = useState(false);
  const current = cases.find((c) => c.id === selectedId);

  if (!project) return null;
  if (!current) {
    return (
      <section aria-label="Detalle del caso" className="grid flex-1 place-items-center text-t4s-gray">
        Selecciona un caso para ver su detalle.
      </section>
    );
  }

  return (
    <section aria-label="Detalle del caso" className="flex-1 overflow-y-auto p-6">
      <div className="mx-auto max-w-4xl">
        <div className="flex items-start justify-between gap-4">
          <div>
            <p className="font-mono text-sm text-t4s-blue">{current.code}</p>
            <h2 className="text-2xl font-semibold">{current.title}</h2>
            <div className="mt-2">
              <StatusBadge status={current.status} />
            </div>
          </div>
          <div className="flex gap-2">
            <button type="button" className={btn.secondary} onClick={() => onEdit(current.id)}>
              <Pencil size={16} /> Editar
            </button>
            <button type="button" className={btn.secondary} onClick={() => void duplicateCase(current.id)}>
              <Copy size={16} /> Duplicar
            </button>
            <button type="button" aria-label="Eliminar caso" className={btn.ghost} onClick={() => setConfirming(true)}>
              <Trash2 size={16} />
            </button>
          </div>
        </div>
        <dl className="mt-6 grid grid-cols-2 gap-x-6 gap-y-4 rounded-xl border border-t4s-line bg-white p-5">
          {activeFields(project.fields).map((f) => (
            <div key={f.id} className={f.type === "longtext" ? "col-span-2" : ""}>
              <dt className="text-xs font-medium uppercase tracking-wide text-t4s-gray">{f.label}</dt>
              <dd className="mt-1 whitespace-pre-wrap text-sm">{displayValue(f, current.values[f.id] ?? "")}</dd>
            </div>
          ))}
        </dl>
        <div className="mt-6 rounded-xl border border-dashed border-t4s-line p-6 text-center text-sm text-t4s-gray">
          Las evidencias (capturas y anotaciones) llegan en la fase F2.
        </div>
      </div>
      {confirming && (
        <ConfirmDialog
          title={`Eliminar ${current.code}`}
          message="El caso se eliminará de forma permanente. Su código no se reutilizará."
          confirmLabel="Eliminar caso"
          onCancel={() => setConfirming(false)}
          onConfirm={async () => {
            setConfirming(false);
            await deleteCase(current.id);
          }}
        />
      )}
    </section>
  );
}
