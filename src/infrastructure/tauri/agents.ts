import { listen } from "@tauri-apps/api/event";
import { z } from "zod";
import type { AgentSettings } from "../../ports/settings";
import { call } from "./client";
import {
  agentClientConfigSchema,
  activitySchema,
  agentDataChangedSchema,
} from "./dto";
export function tauriAgents(): AgentSettings {
  return {
    clientConfig: () => call("mcp_client_config", {}, agentClientConfigSchema),
    activity: (limit, client) =>
      call("mcp_activity", { limit, client }, z.array(activitySchema)),
    regenerateKey: async () => {
      await call("mcp_regenerate_key", {}, z.null());
    },
    onDataChanged: (handler) =>
      listen("agent:data-changed", (event) => {
        handler(agentDataChangedSchema.parse(event.payload));
      }),
  };
}
