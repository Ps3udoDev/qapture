import type {
  Activity,
  AgentClientConfig,
  AgentDataChanged,
} from "../../domain/agents";
import type { AgentSettings } from "../../ports/settings";
export interface MemoryAgents extends AgentSettings {
  config: AgentClientConfig;
  rows: Activity[];
  regenerated: number;
  emit(event: AgentDataChanged): void;
}
export function createMemoryAgents(): MemoryAgents {
  const handlers = new Set<(event: AgentDataChanged) => void>();
  const m: MemoryAgents = {
    config: {
      bridgePath: "C:\\Qapture\\qapture-mcp.exe",
      listening: true,
      claudeCode: 'claude mcp add qapture -- "C:\\Qapture\\qapture-mcp.exe"',
      codex: "[mcp_servers.qapture]\ncommand = 'C:\\Qapture\\qapture-mcp.exe'",
      json: JSON.stringify(
        {
          mcpServers: { qapture: { command: "C:\\Qapture\\qapture-mcp.exe" } },
        },
        null,
        2,
      ),
    },
    rows: [],
    regenerated: 0,
    async clientConfig() {
      return structuredClone(m.config);
    },
    async activity(limit, client) {
      return structuredClone(
        m.rows.filter((r) => !client || r.client === client).slice(0, limit),
      );
    },
    async regenerateKey() {
      m.regenerated++;
    },
    async onDataChanged(handler) {
      handlers.add(handler);
      return () => {
        handlers.delete(handler);
      };
    },
    emit(event) {
      for (const handler of handlers) handler(event);
    },
  };
  return m;
}
