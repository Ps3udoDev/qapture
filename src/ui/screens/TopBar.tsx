import { FileSpreadsheet, Plus, Settings, SlidersHorizontal } from "lucide-react";
import mark from "../../assets/brand/qapture-mark.svg";
import { btn, inputCls } from "../components/styles";
import { useWorkspace } from "../store/context";
import { hasPostponedUpdate } from "../store/update-slice";
import { DataMenu } from "./DataMenu";

export function TopBar({ onNewProject, onSettings, onAppSettings, onExport, onBackups }: { onNewProject: () => void; onSettings?: () => void; onAppSettings?: () => void; onExport?: () => void; onBackups?: () => void }) {
  const projects = useWorkspace((s) => s.projects);
  const project = useWorkspace((s) => s.project);
  const selectProject = useWorkspace((s) => s.selectProject);
  const updatePending = useWorkspace(hasPostponedUpdate);

  return (
    <header className="flex items-center gap-4 border-b border-t4s-line bg-white px-5 py-3">
      <div className="flex items-center gap-2">
        <img src={mark} alt="" className="h-8 w-8 rounded-lg" />
        <div className="leading-tight">
          <p className="font-semibold">Qapture</p>
          <p className="text-xs text-t4s-gray">Teams4Soft · Evidencias QA</p>
        </div>
      </div>
      {projects.length > 0 && (
        <label className="ml-6 flex items-center gap-2 text-sm">
          <span className="text-t4s-gray">Proyecto</span>
          <select aria-label="Proyecto" className={`${inputCls} w-72`} value={project?.id ?? ""} onChange={(e) => void selectProject(e.target.value)}>
            {projects.map((p) => (
              <option key={p.id} value={p.id}>
                {p.client ? `${p.client} · ${p.name}` : p.name}
              </option>
            ))}
          </select>
        </label>
      )}
      <div className="ml-auto flex gap-2">
        {onBackups && <DataMenu onBackups={onBackups} />}
        {onExport && project && (
          <button type="button" className={btn.secondary} onClick={onExport}>
            <FileSpreadsheet size={16} /> Exportar
          </button>
        )}
        {onSettings && project && (
          <button type="button" className={btn.secondary} onClick={onSettings}>
            <Settings size={16} /> Configurar
          </button>
        )}
        {onAppSettings && (
          <button type="button" className={`${btn.secondary} relative`} aria-label={updatePending ? "Ajustes (actualización disponible)" : "Ajustes"} title={updatePending ? "Ajustes (actualización disponible)" : "Ajustes"} onClick={onAppSettings}>
            <SlidersHorizontal size={16} />
            {updatePending && <span aria-hidden="true" data-testid="update-dot" className="absolute -right-1 -top-1 h-2.5 w-2.5 rounded-full bg-t4s-blue ring-2 ring-white" />}
          </button>
        )}
        <button type="button" className={btn.primary} onClick={onNewProject}>
          <Plus size={16} /> Nueva matriz
        </button>
      </div>
    </header>
  );
}
