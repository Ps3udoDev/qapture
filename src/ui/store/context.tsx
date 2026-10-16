import { createContext, useContext, type ReactNode } from "react";
import { useStore } from "zustand";
import type { StoreApi } from "zustand/vanilla";
import type { WorkspaceState } from "./workspace";

const WorkspaceContext = createContext<StoreApi<WorkspaceState> | null>(null);

export function WorkspaceProvider({ store, children }: { store: StoreApi<WorkspaceState>; children: ReactNode }) {
  return <WorkspaceContext.Provider value={store}>{children}</WorkspaceContext.Provider>;
}

/** Los selectores deben devolver valores estables (campos o acciones), nunca objetos nuevos. */
export function useWorkspace<T>(selector: (state: WorkspaceState) => T): T {
  const store = useContext(WorkspaceContext);
  if (!store) throw new Error("useWorkspace requiere <WorkspaceProvider>.");
  return useStore(store, selector);
}
