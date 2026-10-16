import { Images, Plus, Search } from "lucide-react";
import { useMemo } from "react";
import { filterCases } from "../../domain/cases";
import { STATUSES, type Status } from "../../domain/status";
import { StatusBadge } from "../components/StatusBadge";
import { btn, inputCls } from "../components/styles";
import { useWorkspace } from "../store/context";

export function CaseListPane({ onNewCase }: { onNewCase: () => void }) {
  const project = useWorkspace((s) => s.project);
  const cases = useWorkspace((s) => s.cases);
  const search = useWorkspace((s) => s.search);
  const status = useWorkspace((s) => s.statusFilter);
  const selectedId = useWorkspace((s) => s.selectedCaseId);
  const setSearch = useWorkspace((s) => s.setSearch);
  const setStatusFilter = useWorkspace((s) => s.setStatusFilter);
  const selectCase = useWorkspace((s) => s.selectCase);
  const visible = useMemo(
    () => filterCases(cases, project?.fields ?? [], { search, status }),
    [cases, project, search, status],
  );
  const envField = project?.fields.find((f) => f.key === "entorno" && !f.archived);

  return (
    <aside className="flex w-80 shrink-0 flex-col border-r border-t4s-line bg-white">
      <div className="space-y-3 p-4">
        <div className="flex items-center justify-between">
          <h1 className="font-semibold">Casos de prueba</h1>
          <span data-testid="case-count" className="rounded-full bg-t4s-cream px-2 text-xs">
            {visible.length === cases.length ? cases.length : `${visible.length} / ${cases.length}`}
          </span>
        </div>
        <button type="button" className={`${btn.primary} w-full`} onClick={onNewCase}>
          <Plus size={16} /> Nuevo caso
        </button>
        <label className="relative block">
          <Search size={14} className="absolute left-3 top-1/2 -translate-y-1/2 text-t4s-gray" />
          <input
            type="search"
            aria-label="Buscar casos"
            placeholder="Buscar código, caso o dato…"
            className={`${inputCls} pl-8`}
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
        </label>
        <select aria-label="Filtrar por estado" className={inputCls} value={status} onChange={(e) => setStatusFilter(e.target.value as Status | "")}>
          <option value="">Todos los estados</option>
          {STATUSES.map((s) => (
            <option key={s}>{s}</option>
          ))}
        </select>
      </div>
      <ul aria-label="Lista de casos" className="flex-1 overflow-y-auto px-2 pb-4">
        {visible.map((c) => (
          <li key={c.id}>
            <button
              type="button"
              onClick={() => selectCase(c.id)}
              aria-current={c.id === selectedId}
              className={`w-full rounded-lg px-3 py-2 text-left text-sm ${c.id === selectedId ? "bg-t4s-blue/10" : "hover:bg-t4s-cream"}`}
            >
              <span className="flex items-center justify-between">
                <span className="font-mono text-xs text-t4s-blue">{c.code}</span>
                <StatusBadge status={c.status} />
              </span>
              <span className="mt-0.5 block truncate font-medium">{c.title}</span>
              <span className="flex items-center justify-between gap-2">
                <span className="truncate text-xs text-t4s-gray">{envField ? c.values[envField.id] ?? "" : ""}</span>
                {c.evidenceCount > 0 && (
                  <span aria-label={`${c.evidenceCount} ${c.evidenceCount === 1 ? "evidencia" : "evidencias"}`} className="inline-flex shrink-0 items-center gap-1 rounded-full bg-t4s-cream px-2 text-xs text-t4s-gray">
                    <Images size={12} /> {c.evidenceCount}
                  </span>
                )}
              </span>
            </button>
          </li>
        ))}
        {visible.length === 0 && (
          <li className="px-3 py-6 text-center text-sm text-t4s-gray">
            {cases.length ? "Ningún caso coincide con el filtro." : "Aún no hay casos. Crea el primero."}
          </li>
        )}
      </ul>
    </aside>
  );
}
