import { useWorkspace } from "../store/context";

export function Workspace() {
  const project = useWorkspace((s) => s.project);
  return <main className="flex-1 p-6">{project?.name}</main>;
}
