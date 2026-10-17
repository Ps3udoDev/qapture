import { useEffect, useState } from "react";
import { Toasts } from "../components/Toasts";
import { useWorkspace } from "../store/context";
import { BackupsDialog } from "./BackupsDialog";
import { EmptyState } from "./EmptyState";
import { ExportDialog } from "./ExportDialog";
import { ImportIssuesDialog } from "./ImportIssuesDialog";
import { NewProjectDialog } from "./NewProjectDialog";
import { ProjectSettingsDialog } from "./ProjectSettingsDialog";
import { RecoveryScreen } from "./RecoveryScreen";
import { TopBar } from "./TopBar";
import { Workspace } from "./Workspace";

export function AppShell() {
  const init = useWorkspace((s) => s.init);
  const ready = useWorkspace((s) => s.ready);
  const project = useWorkspace((s) => s.project);
  const recovery = useWorkspace((s) => s.recovery);
  const [dialog, setDialog] = useState<"new-project" | "settings" | "export" | "backups" | null>(null);

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

  if (recovery) {
    return (
      <div data-testid="app-ready" className="flex h-full flex-col overflow-y-auto bg-t4s-cream">
        <RecoveryScreen />
        <ImportIssuesDialog />
        <Toasts />
      </div>
    );
  }

  return (
    <div data-testid="app-ready" className="flex h-full flex-col">
      <TopBar onNewProject={() => setDialog("new-project")} onSettings={() => setDialog("settings")} onExport={() => setDialog("export")} onBackups={() => setDialog("backups")} />
      {project ? <Workspace /> : <EmptyState onCreate={() => setDialog("new-project")} />}
      {dialog === "new-project" && <NewProjectDialog onClose={() => setDialog(null)} />}
      {dialog === "settings" && project && <ProjectSettingsDialog onClose={() => setDialog(null)} />}
      {dialog === "export" && project && <ExportDialog onClose={() => setDialog(null)} />}
      {dialog === "backups" && <BackupsDialog onClose={() => setDialog(null)} />}
      <ImportIssuesDialog />
      <Toasts />
    </div>
  );
}
