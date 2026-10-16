import { useState, type FormEvent } from "react";
import type { FormErrors } from "../../domain/cases";
import { projectToMeta, type ProjectMeta } from "../../domain/project";
import { ConfirmDialog } from "../components/ConfirmDialog";
import { Dialog } from "../components/Dialog";
import { btn } from "../components/styles";
import { useWorkspace } from "../store/context";
import { ColumnEditor } from "./ColumnEditor";
import { ProjectMetaFields } from "./ProjectMetaFields";
import { TemplatesPanel } from "./TemplatesPanel";

const TABS = ["Datos", "Columnas", "Plantillas"] as const;
type Tab = (typeof TABS)[number];

export function ProjectSettingsDialog({ onClose }: { onClose: () => void }) {
  const project = useWorkspace((s) => s.project);
  const updateMeta = useWorkspace((s) => s.updateProjectMeta);
  const deleteProject = useWorkspace((s) => s.deleteProject);
  const [tab, setTab] = useState<Tab>("Datos");
  const [meta, setMeta] = useState<ProjectMeta>(() => projectToMeta(project!));
  const [errors, setErrors] = useState<FormErrors>({});
  const [confirming, setConfirming] = useState(false);

  if (!project) return null;

  async function submit(e: FormEvent) {
    e.preventDefault();
    const result = await updateMeta(meta);
    setErrors(result.ok ? {} : result.errors);
  }

  return (
    <Dialog wide title="Configurar proyecto" kicker={project.name} onClose={onClose}>
      <div role="tablist" className="mb-4 flex gap-1 border-b border-t4s-line">
        {TABS.map((t) => (
          <button
            key={t}
            type="button"
            role="tab"
            aria-selected={tab === t}
            onClick={() => setTab(t)}
            className={`-mb-px border-b-2 px-3 py-2 text-sm ${tab === t ? "border-t4s-blue font-medium text-t4s-blue" : "border-transparent text-t4s-gray"}`}
          >
            {t}
          </button>
        ))}
      </div>
      {tab === "Datos" && (
        <form onSubmit={submit} noValidate className="grid grid-cols-2 gap-4">
          <ProjectMetaFields idPrefix="ps" meta={meta} errors={errors} onChange={setMeta} />
          {errors._form && (
            <p role="alert" className="col-span-2 text-sm text-red-700">
              {errors._form}
            </p>
          )}
          <div className="col-span-2 flex justify-between border-t border-t4s-line pt-4">
            <button type="button" className={btn.ghost} onClick={() => setConfirming(true)}>
              Eliminar proyecto
            </button>
            <button type="submit" className={btn.primary}>
              Guardar datos
            </button>
          </div>
        </form>
      )}
      {tab === "Columnas" && <ColumnEditor />}
      {tab === "Plantillas" && <TemplatesPanel />}
      {confirming && (
        <ConfirmDialog
          title={`Eliminar ${project.name}`}
          message="Se eliminarán el proyecto y todos sus casos. Esta acción no se puede deshacer."
          confirmLabel="Eliminar proyecto"
          requireText={project.name}
          onCancel={() => setConfirming(false)}
          onConfirm={async () => {
            setConfirming(false);
            onClose();
            await deleteProject();
          }}
        />
      )}
    </Dialog>
  );
}
