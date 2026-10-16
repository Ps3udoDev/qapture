import type { StoreApi } from "zustand/vanilla";
import { AppShell } from "./ui/screens/AppShell";
import { WorkspaceProvider } from "./ui/store/context";
import type { WorkspaceState } from "./ui/store/workspace";

export default function App({ store }: { store: StoreApi<WorkspaceState> }) {
  return (
    <WorkspaceProvider store={store}>
      <AppShell />
    </WorkspaceProvider>
  );
}
