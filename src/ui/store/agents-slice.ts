import type { StoreApi } from "zustand/vanilla";
import type { Activity, AgentClientConfig } from "../../domain/agents";
import { summarizeAgentChanges } from "../../domain/agents";
import type { Repositories } from "../../ports/repositories";
import type { WorkspaceState } from "./workspace";
import type { SliceHelpers } from "./evidence-slice";
export interface AgentsSlice {
  agentConfig: AgentClientConfig | null;
  agentActivity: Activity[];
  loadAgents(client?: string): Promise<void>;
  regenerateAgentKey(): Promise<void>;
  listenAgents(): Promise<() => void>;
}
export function createAgentsSlice(
  repos: Repositories,
  set: StoreApi<WorkspaceState>["setState"],
  get: StoreApi<WorkspaceState>["getState"],
  h: SliceHelpers,
): AgentsSlice {
  let loadSequence = 0;
  return {
    agentConfig: null,
    agentActivity: [],
    async loadAgents(client) {
      const sequence = ++loadSequence;
      try {
        const [agentConfig, agentActivity] = await Promise.all([
          repos.agents.clientConfig(),
          repos.agents.activity(200, client),
        ]);
        if (sequence === loadSequence) set({ agentConfig, agentActivity });
      } catch (error) {
        h.report(error);
      }
    },
    async regenerateAgentKey() {
      try {
        await repos.agents.regenerateKey();
        set({ agentConfig: await repos.agents.clientConfig() });
        get().pushToast(
          "success",
          "Clave nueva: vuelve a conectar tus agentes.",
        );
      } catch (error) {
        h.report(error);
      }
    },
    async listenAgents() {
      let summaries: string[] = [];
      let timer: ReturnType<typeof setTimeout> | undefined;
      let disposed = false;
      // Serializa recargas para que una respuesta anterior no pise datos más nuevos.
      let pending = Promise.resolve();
      const off = await repos.agents.onDataChanged((event) => {
        if (disposed) return;
        summaries.push(event.summary);
        timer ??= setTimeout(() => {
          get().pushToast("success", summarizeAgentChanges(summaries));
          summaries = [];
          timer = undefined;
        }, 5000);
        pending = pending
          .then(async () => {
            const projects = await repos.projects.list();
            if (disposed) return;
            set({ projects });
            const projectId = get().project?.id;
            if (event.projectId && projectId === event.projectId) {
              const [project, cases] = await Promise.all([
                repos.projects.get(projectId),
                repos.cases.list(projectId),
              ]);
              if (!disposed && get().project?.id === projectId)
                set({ project, cases });
            }
            if (
              !disposed &&
              event.caseId &&
              event.caseId === get().selectedCaseId
            )
              await get().loadEvidence();
          })
          .catch(h.report);
      });
      return () => {
        disposed = true;
        off();
        if (timer) clearTimeout(timer);
        summaries = [];
      };
    },
  };
}
