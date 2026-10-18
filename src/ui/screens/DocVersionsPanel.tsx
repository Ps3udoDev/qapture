import { Pencil, Trash2 } from "lucide-react";
import { useEffect, useState, type FormEvent } from "react";
import type { FormErrors } from "../../domain/cases";
import { emptyDocVersion, formatIsoDate, suggestNextVersion, todayIso, type DocVersion, type DocVersionInput } from "../../domain/doc-versions";
import { FormField } from "../components/FormField";
import { btn, iconButton, inputCls } from "../components/styles";
import { useWorkspace } from "../store/context";

const toInput = (v: DocVersion): DocVersionInput => ({
  version: v.version,
  date: v.date,
  author: v.author,
  reviewer: v.reviewer,
  approver: v.approver,
  changes: v.changes,
});

/** Control de versiones del documento (UC-05): alimenta la tabla de la carátula. */
export function DocVersionsPanel() {
  const docVersions = useWorkspace((s) => s.docVersions);
  const saveDocVersion = useWorkspace((s) => s.saveDocVersion);
  const deleteDocVersion = useWorkspace((s) => s.deleteDocVersion);
  const testerName = useWorkspace((s) => s.testerName);
  const blank = (): DocVersionInput => ({ ...emptyDocVersion(todayIso(), testerName()), version: suggestNextVersion(docVersions) });
  const [editing, setEditing] = useState<string | null>(null);
  const [form, setForm] = useState<DocVersionInput>(blank);
  const [errors, setErrors] = useState<FormErrors>({});
  const [resets, setResets] = useState(0);
  useEffect(() => {
    // Al cargar o cambiar el historial, el formulario nuevo sugiere la siguiente versión.
    if (!editing) setForm((f) => (f.version ? f : { ...f, version: suggestNextVersion(docVersions) }));
  }, [docVersions, editing, resets]);
  const set = (key: keyof DocVersionInput) => (value: string) => setForm((f) => ({ ...f, [key]: value }));

  async function submit(e: FormEvent) {
    e.preventDefault();
    const result = await saveDocVersion(form, editing ?? undefined);
    if (!result.ok) {
      setErrors(result.errors);
      return;
    }
    setEditing(null);
    setErrors({});
    // Versión vacía: el efecto de arriba sugiere la siguiente con el historial ya recargado.
    setForm(emptyDocVersion(todayIso(), form.author));
    setResets((n) => n + 1);
  }

  function edit(v: DocVersion) {
    setEditing(v.id);
    setForm(toInput(v));
    setErrors({});
  }

  const field = (key: keyof DocVersionInput, label: string, required = false) => (
    <FormField label={label} required={required} htmlFor={`dv-${key}`} error={errors[key]}>
      <input id={`dv-${key}`} className={inputCls} value={form[key]} onChange={(e) => set(key)(e.target.value)} />
    </FormField>
  );

  return (
    <div className="space-y-6">
      <section aria-label="Historial de versiones">
        {docVersions.length === 0 ? (
          <p className="text-sm text-t4s-gray">Sin versiones registradas. La carátula del Excel muestra este historial.</p>
        ) : (
          <table className="w-full text-left text-sm">
            <thead className="text-xs text-t4s-gray">
              <tr>
                <th className="py-1">Versión</th>
                <th>Fecha</th>
                <th>Elaborado</th>
                <th>Revisado</th>
                <th>Aprobado</th>
                <th>Cambios</th>
                <th className="sr-only">Acciones</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-t4s-line">
              {docVersions.map((v) => (
                <tr key={v.id}>
                  <td className="py-2 font-medium">{v.version}</td>
                  <td>{formatIsoDate(v.date)}</td>
                  <td>{v.author}</td>
                  <td>{v.reviewer || "—"}</td>
                  <td>{v.approver || "—"}</td>
                  <td className="max-w-56 truncate" title={v.changes}>
                    {v.changes || "—"}
                  </td>
                  <td className="whitespace-nowrap text-right">
                    <button type="button" aria-label={`Editar versión ${v.version}`} className={iconButton} onClick={() => edit(v)}>
                      <Pencil size={16} />
                    </button>
                    <button type="button" aria-label={`Eliminar versión ${v.version}`} className={iconButton} onClick={() => void deleteDocVersion(v.id)}>
                      <Trash2 size={16} />
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>
      <form onSubmit={submit} noValidate className="grid grid-cols-3 gap-4 border-t border-t4s-line pt-4">
        <h3 className="col-span-3 text-sm font-medium">{editing ? `Editar versión ${form.version}` : "Nueva versión"}</h3>
        {field("version", "Versión", true)}
        <FormField label="Fecha" required htmlFor="dv-date" error={errors.date}>
          <input id="dv-date" type="date" className={inputCls} value={form.date} onChange={(e) => set("date")(e.target.value)} />
        </FormField>
        {field("author", "Elaborado por", true)}
        {field("reviewer", "Revisado por")}
        {field("approver", "Aprobado por")}
        <div />
        <FormField label="Cambios" htmlFor="dv-changes" error={errors.changes} className="col-span-3">
          <textarea id="dv-changes" rows={2} className={inputCls} value={form.changes} onChange={(e) => set("changes")(e.target.value)} />
        </FormField>
        {errors._form && (
          <p role="alert" className="col-span-3 text-sm text-red-700">
            {errors._form}
          </p>
        )}
        <div className="col-span-3 flex justify-end gap-2">
          {editing && (
            <button
              type="button"
              className={btn.ghost}
              onClick={() => {
                setEditing(null);
                setForm(blank());
                setErrors({});
              }}
            >
              Cancelar edición
            </button>
          )}
          <button type="submit" className={btn.primary}>
            {editing ? "Guardar versión" : "Registrar versión"}
          </button>
        </div>
      </form>
    </div>
  );
}
