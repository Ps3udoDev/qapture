import { useEffect, useState } from "react";
import { Toasts } from "../components/Toasts";
import { useWorkspace } from "../store/context";
import { EmptyState } from "./EmptyState";
import { NewProjectDialog } from "./NewProjectDialog";
import { ProjectSettingsDialog } from "./ProjectSettingsDialog";
import { TopBar } from "./TopBar";
import { Workspace } from "./Workspace";

export function AppShell() {
  const init = useWorkspace((s) => s.init);
  const ready = useWorkspace((s) => s.ready);
  const project = useWorkspace((s) => s.project);
  const [dialog, setDialog] = useState<"new-project" | "settings" | null>(null);

  const listenCaptures = useWorkspace((s) => s.listenCaptures);
  const pushToast = useWorkspace((s) => s.pushToast);

  useEffect(() => {
    void init();
  }, [init]);

  useEffect(() => {
    // Siempre montado: la captura puede llegar sin proyecto abierto (F6 la dispara con el atajo global).
    // En StrictMode el efecto corre dos veces; si la suscripción llega tarde, se cancela al llegar.
    let active = true;
    let unlisten: (() => void) | undefined;
    void listenCaptures()
      .then((off) => {
        if (active) unlisten = off;
        else off();
      })
      .catch(() => {
        if (active) pushToast("error", "No se pudo escuchar las capturas.");
      });
    return () => {
      active = false;
      unlisten?.();
    };
  }, [listenCaptures, pushToast]);

  if (!ready) return <div className="grid h-full place-items-center text-t4s-gray">Cargando Qapture…</div>;

  return (
    <div data-testid="app-ready" className="flex h-full flex-col">
      <TopBar onNewProject={() => setDialog("new-project")} onSettings={() => setDialog("settings")} />
      {project ? <Workspace /> : <EmptyState onCreate={() => setDialog("new-project")} />}
      {dialog === "new-project" && <NewProjectDialog onClose={() => setDialog(null)} />}
      {dialog === "settings" && project && <ProjectSettingsDialog onClose={() => setDialog(null)} />}
      <Toasts />
    </div>
  );
}
