import { useEffect, useState } from "react";
import { Toasts } from "../components/Toasts";
import { useWorkspace } from "../store/context";
import { EmptyState } from "./EmptyState";
import { NewProjectDialog } from "./NewProjectDialog";
import { TopBar } from "./TopBar";
import { Workspace } from "./Workspace";

export function AppShell() {
  const init = useWorkspace((s) => s.init);
  const ready = useWorkspace((s) => s.ready);
  const project = useWorkspace((s) => s.project);
  const [dialog, setDialog] = useState<"new-project" | null>(null);

  useEffect(() => {
    void init();
  }, [init]);

  if (!ready) return <div className="grid h-full place-items-center text-t4s-gray">Cargando Qapture…</div>;

  return (
    <div data-testid="app-ready" className="flex h-full flex-col">
      <TopBar onNewProject={() => setDialog("new-project")} />
      {project ? <Workspace /> : <EmptyState onCreate={() => setDialog("new-project")} />}
      {dialog === "new-project" && <NewProjectDialog onClose={() => setDialog(null)} />}
      <Toasts />
    </div>
  );
}
