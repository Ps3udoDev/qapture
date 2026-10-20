import { RefreshCw } from "lucide-react";
import type { Evidence } from "../../domain/evidence";
import type { RepoMeta } from "../../domain/github";
import { btn, inputCls } from "../components/styles";

const MAX_PICKS = 10;
const TEN_MB = 10 * 1024 * 1024;

/** Casillas de etiquetas o asignados; incluye las elegidas que el repo no informa (p. ej. etiquetas por defecto). */
export function PickList({ legend, options, value, onChange }: { legend: string; options: string[]; value: string[]; onChange: (next: string[]) => void }) {
  const all = [...options, ...value.filter((v) => !options.includes(v))];
  return (
    <fieldset>
      <legend className="mb-1 text-sm font-medium">{legend}</legend>
      {all.length === 0 ? (
        <p className="text-sm text-t4s-gray">—</p>
      ) : (
        <div className="flex flex-wrap gap-x-4 gap-y-1">
          {all.map((option) => {
            const checked = value.includes(option);
            return (
              <label key={option} className="inline-flex items-center gap-1.5 text-sm">
                <input
                  type="checkbox"
                  checked={checked}
                  disabled={!checked && value.length >= MAX_PICKS}
                  onChange={(e) => onChange(e.target.checked ? [...value, option] : value.filter((v) => v !== option))}
                />
                {option}
              </label>
            );
          })}
        </div>
      )}
    </fieldset>
  );
}

export function IssueMetaFields({ meta, labels, assignees, milestone, onLabels, onAssignees, onMilestone }: {
  meta: RepoMeta | null;
  labels: string[];
  assignees: string[];
  milestone: number | null;
  onLabels: (next: string[]) => void;
  onAssignees: (next: string[]) => void;
  onMilestone: (next: number | null) => void;
}) {
  return (
    <div className="grid grid-cols-2 gap-4">
      <PickList legend="Etiquetas" options={meta?.labels ?? []} value={labels} onChange={onLabels} />
      <PickList legend="Asignados" options={meta?.assignees ?? []} value={assignees} onChange={onAssignees} />
      <label className="col-span-2 block text-sm font-medium">
        Milestone
        <select className={`${inputCls} mt-1 font-normal`} value={milestone ?? ""} onChange={(e) => onMilestone(e.target.value === "" ? null : Number(e.target.value))}>
          <option value="">Sin milestone</option>
          {(meta?.milestones ?? []).map((m) => (
            <option key={m.number} value={m.number}>
              {m.title}
            </option>
          ))}
        </select>
      </label>
    </div>
  );
}

/** Aviso inline cuando la selección ya no coincide con la última descripción armada. */
const STALE_NOTICE = "Cambiaste las capturas: regenera la descripción para que coincidan.";

/**
 * Capturas elegibles con miniatura. Cada elegida lleva el número `n` de su `{{img:n}}` (posición
 * dentro de las elegidas), el mismo que usa el aviso de 10 MB.
 */
export function EvidencePicker({ evidence, selected, sizes, stale, urlFor, onToggle, onRegenerate }: {
  evidence: Evidence[];
  selected: Set<string>;
  sizes: Record<string, number>;
  /** La selección cambió desde la última vez que se armó la descripción. */
  stale: boolean;
  urlFor: (e: Evidence) => string;
  onToggle: (id: string, on: boolean) => void;
  onRegenerate: () => void;
}) {
  const chosen = evidence.filter((e) => selected.has(e.id));
  const numberOf = new Map(chosen.map((e, i) => [e.id, i + 1]));
  const heavy = chosen.map((e, i) => ((sizes[e.id] ?? 0) > TEN_MB ? i + 1 : null)).filter((n): n is number => n !== null);
  return (
    <fieldset>
      <div className="mb-1 flex items-center justify-between gap-2">
        <legend className="text-sm font-medium">Capturas</legend>
        <span className="flex items-center gap-2">
          {stale && <span className="text-xs text-amber-700">{STALE_NOTICE}</span>}
          <button type="button" className={btn.ghost} onClick={onRegenerate}>
            <RefreshCw size={14} /> Regenerar descripción
          </button>
        </span>
      </div>
      {evidence.length === 0 ? (
        <p className="text-sm text-t4s-gray">El caso no tiene capturas.</p>
      ) : (
        <ul className="grid grid-cols-3 gap-2">
          {evidence.map((e) => {
            const n = numberOf.get(e.id);
            const label = n ? (e.description ? `Captura ${n} · ${e.description}` : `Captura ${n}`) : `${e.description || "Sin descripción"} · no se incluye`;
            return (
              <li key={e.id}>
                <label className="flex cursor-pointer flex-col gap-1 rounded-md border border-t4s-line p-1.5 text-xs">
                  <img src={urlFor(e)} alt="" className="h-20 w-full rounded object-cover" />
                  <span className="flex items-center gap-1.5">
                    <input type="checkbox" aria-label={label} checked={n !== undefined} onChange={(ev) => onToggle(e.id, ev.target.checked)} />
                    {n && <span className="shrink-0 font-medium">Captura {n}</span>}
                    {e.description && <span className="truncate text-t4s-gray">{e.description}</span>}
                  </span>
                </label>
              </li>
            );
          })}
        </ul>
      )}
      {heavy.map((n) => (
        <p key={n} role="alert" className="mt-2 text-sm text-amber-700">
          La captura {n} pesa más de 10 MB; GitHub podría rechazarla.
        </p>
      ))}
    </fieldset>
  );
}
