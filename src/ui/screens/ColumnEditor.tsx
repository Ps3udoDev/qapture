import { ArrowDown, ArrowUp, Plus, RotateCcw, Trash2 } from "lucide-react";
import { useEffect, useState } from "react";
import type { FormErrors } from "../../domain/cases";
import { FIELD_TYPES, FIELD_TYPE_LABELS, type FieldType } from "../../domain/fields";
import { fieldToInput, newFieldInput, type FieldInput } from "../../domain/project";
import { btn, iconButton, inputCls } from "../components/styles";
import { useWorkspace } from "../store/context";

export function ColumnEditor() {
  const fields = useWorkspace((s) => s.project?.fields);
  const saveFields = useWorkspace((s) => s.saveFields);
  const [rows, setRows] = useState<FieldInput[]>(() => (fields ?? []).map(fieldToInput));
  const [errors, setErrors] = useState<FormErrors>({});
  const [busy, setBusy] = useState(false);

  // Cuando el store recarga el proyecto tras guardar, las filas se sincronizan con las columnas
  // reales: ids nuevos, columnas archivadas por tener datos y columnas vacías eliminadas.
  useEffect(() => setRows((fields ?? []).map(fieldToInput)), [fields]);

  const active = rows.filter((r) => !r.archived);
  const archived = rows.filter((r) => r.archived);
  const update = (row: FieldInput, patch: Partial<FieldInput>) => setRows((all) => all.map((r) => (r === row ? { ...r, ...patch } : r)));
  const remove = (row: FieldInput) => setRows((all) => all.filter((r) => r !== row));
  const move = (row: FieldInput, delta: number) =>
    setRows((all) => {
      const list = all.filter((r) => !r.archived);
      const from = list.indexOf(row);
      const to = from + delta;
      if (to < 0 || to >= list.length) return all;
      [list[from], list[to]] = [list[to], list[from]];
      return [...list, ...all.filter((r) => r.archived)];
    });
  const restore = (row: FieldInput) => setRows((all) => [...all.filter((r) => !r.archived), { ...row, archived: false }, ...all.filter((r) => r.archived && r !== row)]);

  async function save() {
    setBusy(true);
    const result = await saveFields([...active, ...archived]);
    setBusy(false);
    setErrors(result.ok ? {} : result.errors);
  }

  const messages = Object.values(errors);

  return (
    <div className="space-y-4">
      {messages.length > 0 && (
        <ul role="alert" className="rounded-lg bg-red-50 p-3 text-sm text-red-800">
          {messages.map((m) => (
            <li key={m}>{m}</li>
          ))}
        </ul>
      )}
      <p className="text-sm text-t4s-gray">
        "Caso de prueba", "Estado" e "ID" son fijos. Si quitas una columna que ya tiene datos, se archiva y puedes restaurarla.
      </p>
      <ol className="space-y-2">
        {active.map((row, index) => {
          const name = row.label || `columna ${index + 1}`;
          return (
            <li key={row.id ?? `new-${index}`} className="grid grid-cols-[1fr_9rem_5rem_auto] items-start gap-2 rounded-lg border border-t4s-line p-3">
              <input aria-label={`Nombre de la columna ${index + 1}`} className={inputCls} value={row.label} onChange={(e) => update(row, { label: e.target.value })} />
              <select aria-label={`Tipo de ${name}`} className={inputCls} value={row.type} onChange={(e) => update(row, { type: e.target.value as FieldType })}>
                {FIELD_TYPES.map((t) => (
                  <option key={t} value={t}>
                    {FIELD_TYPE_LABELS[t]}
                  </option>
                ))}
              </select>
              <input
                aria-label={`Ancho de ${name}`}
                type="number"
                min={8}
                max={100}
                className={inputCls}
                value={row.width}
                onChange={(e) => update(row, { width: Number(e.target.value) })}
              />
              <div className="flex">
                <button type="button" aria-label={`Subir ${name}`} className={iconButton} disabled={index === 0} onClick={() => move(row, -1)}>
                  <ArrowUp size={16} />
                </button>
                <button type="button" aria-label={`Bajar ${name}`} className={iconButton} disabled={index === active.length - 1} onClick={() => move(row, 1)}>
                  <ArrowDown size={16} />
                </button>
                <button type="button" aria-label={`Quitar ${name}`} className={iconButton} onClick={() => remove(row)}>
                  <Trash2 size={16} />
                </button>
              </div>
              <div className="col-span-4 flex flex-wrap items-center gap-4 text-sm">
                <label className="flex items-center gap-1.5">
                  <input type="checkbox" checked={row.required} onChange={(e) => update(row, { required: e.target.checked })} /> Obligatorio
                </label>
                <label className="flex items-center gap-1.5">
                  <input type="checkbox" checked={row.clearOnDuplicate} onChange={(e) => update(row, { clearOnDuplicate: e.target.checked })} /> Limpiar al duplicar
                </label>
              </div>
              {row.type === "list" && (
                <textarea
                  aria-label={`Opciones de ${name}`}
                  placeholder="Una opción por línea"
                  rows={3}
                  className={`${inputCls} col-span-4`}
                  value={row.options.join("\n")}
                  onChange={(e) => update(row, { options: e.target.value.split("\n") })}
                />
              )}
            </li>
          );
        })}
      </ol>
      <button type="button" className={btn.secondary} onClick={() => setRows((all) => [...all.filter((r) => !r.archived), newFieldInput(), ...all.filter((r) => r.archived)])}>
        <Plus size={16} /> Agregar columna
      </button>
      {archived.length > 0 && (
        <section aria-label="Columnas archivadas" className="rounded-lg bg-t4s-cream p-3">
          <h3 className="mb-2 text-sm font-medium">Columnas archivadas</h3>
          <ul className="space-y-1">
            {archived.map((row) => (
              <li key={row.id ?? row.label} className="flex items-center justify-between text-sm">
                <span>{row.label}</span>
                <button type="button" aria-label={`Restaurar ${row.label}`} className={btn.ghost} onClick={() => restore(row)}>
                  <RotateCcw size={14} /> Restaurar
                </button>
              </li>
            ))}
          </ul>
        </section>
      )}
      <div className="flex justify-end border-t border-t4s-line pt-4">
        <button type="button" className={btn.primary} disabled={busy} onClick={() => void save()}>
          Guardar columnas
        </button>
      </div>
    </div>
  );
}
