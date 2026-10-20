import { useEffect, useState } from "react";
import { Toasts } from "../components/Toasts";
import { useStoreApi, useWorkspace } from "../store/context";
import { isUpdatePromptOpen } from "../store/update-slice";
import { BackupsDialog } from "./BackupsDialog";
import { EmptyState } from "./EmptyState";
import { ExportDialog } from "./ExportDialog";
import { AppSettingsDialog } from "./AppSettingsDialog";
import { ImportIssuesDialog } from "./ImportIssuesDialog";
import { NewProjectDialog } from "./NewProjectDialog";
import { ProjectSettingsDialog } from "./ProjectSettingsDialog";
import { RecoveryScreen } from "./RecoveryScreen";
import { TopBar } from "./TopBar";
import { UpdateDialog } from "./UpdateDialog";
import { Workspace } from "./Workspace";

export function AppShell() {
  const init = useWorkspace((s) => s.init);
  const ready = useWorkspace((s) => s.ready);
  const project = useWorkspace((s) => s.project);
  const recovery = useWorkspace((s) => s.recovery);
  const [dialog, setDialog] = useState<"new-project" | "settings" | "app-settings" | "github-settings" | "export" | "backups" | null>(null);

  const listenCaptures = useWorkspace((s) => s.listenCaptures);
  const listenNewCase = useWorkspace((s) => s.listenNewCase);
  const listenUpdates = useWorkspace((s) => s.listenUpdates);
  const listenGithub = useWorkspace((s) => s.listenGithub);
  const updatePrompt = useWorkspace(isUpdatePromptOpen);
  const pushToast = useWorkspace((s) => s.pushToast);
  const hideWindow = useWorkspace((s) => s.hideWindow);
  const escHides = useWorkspace((s) => s.settings?.settings.system.escHides ?? false);
  const store = useStoreApi();

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

  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | undefined;
    void listenNewCase()
      .then((off) => {
        if (active) unlisten = off;
        else off();
      })
      .catch(() => {
        if (active) pushToast("error", "No se pudo escuchar el atajo de nuevo caso.");
      });
    return () => {
      active = false;
      unlisten?.();
    };
  }, [listenNewCase, pushToast]);

  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | undefined;
    void listenUpdates()
      .then((off) => {
        if (active) unlisten = off;
        else off();
      })
      .catch(() => {
        if (active) pushToast("error", "No se pudo escuchar las actualizaciones.");
      });
    return () => {
      active = false;
      unlisten?.();
    };
  }, [listenUpdates, pushToast]);

  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | undefined;
    void listenGithub()
      .then((off) => {
        if (active) unlisten = off;
        else off();
      })
      .catch(() => {
        if (active) pushToast("error", "No se pudo escuchar los eventos de GitHub.");
      });
    return () => {
      active = false;
      unlisten?.();
    };
  }, [listenGithub, pushToast]);

  useEffect(() => {
    if (!escHides) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      if (document.querySelector('[role="dialog"], [role="menu"]') || store.getState().editor) return;
      const target = e.target;
      if (target instanceof HTMLElement && ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName)) return;
      void hideWindow();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [escHides, hideWindow, store]);

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
      <TopBar onNewProject={() => setDialog("new-project")} onSettings={() => setDialog("settings")} onAppSettings={() => setDialog("app-settings")} onExport={() => setDialog("export")} onBackups={() => setDialog("backups")} />
      {project ? <Workspace /> : <EmptyState onCreate={() => setDialog("new-project")} />}
      {dialog === "new-project" && <NewProjectDialog onClose={() => setDialog(null)} />}
      {dialog === "settings" && project && <ProjectSettingsDialog onClose={() => setDialog(null)} onOpenAccountSettings={() => setDialog("github-settings")} />}
      {dialog === "app-settings" && <AppSettingsDialog onClose={() => setDialog(null)} />}
      {dialog === "github-settings" && <AppSettingsDialog initialTab="GitHub" onClose={() => setDialog(null)} />}
      {dialog === "export" && project && <ExportDialog onClose={() => setDialog(null)} />}
      {dialog === "backups" && <BackupsDialog onClose={() => setDialog(null)} />}
      {updatePrompt && <UpdateDialog />}
      <ImportIssuesDialog />
      <Toasts />
    </div>
  );
}
