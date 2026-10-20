import { listen } from "@tauri-apps/api/event";
import { z } from "zod";
import type { IssueTracker } from "../../ports/issue-tracker";
import { call } from "./client";
import { deviceCodeSchema, ghAccountSchema, ghConnectFailedSchema, ghRepoSchema, projectGithubSchema } from "./dto";

export const GH_CONNECTED_EVENT = "gh:connected";
export const GH_CONNECT_FAILED_EVENT = "gh:connect-failed";

export function tauriGithub(): IssueTracker {
  return {
    account: () => call("gh_account", {}, ghAccountSchema.nullable()),
    connectStart: () => call("gh_connect_start", {}, deviceCodeSchema),
    connectCancel: async () => {
      await call("gh_connect_cancel", {}, z.null());
    },
    disconnect: async () => {
      await call("gh_disconnect", {}, z.null());
    },
    listRepos: () => call("gh_list_repos", {}, z.array(ghRepoSchema)),
    getProjectGithub: (projectId) => call("gh_project_github_get", { projectId }, projectGithubSchema),
    setProjectGithub: (projectId, config) => call("gh_project_github_set", { projectId, config }, projectGithubSchema),
    onConnected: (handler) =>
      listen<unknown>(GH_CONNECTED_EVENT, (event) => {
        const parsed = ghAccountSchema.safeParse(event.payload);
        if (parsed.success) handler(parsed.data);
        else console.warn(`Evento ${GH_CONNECTED_EVENT} con formato inesperado.`, parsed.error.message);
      }),
    onConnectFailed: (handler) =>
      listen<unknown>(GH_CONNECT_FAILED_EVENT, (event) => {
        const parsed = ghConnectFailedSchema.safeParse(event.payload);
        if (parsed.success) handler(parsed.data.message);
        else console.warn(`Evento ${GH_CONNECT_FAILED_EVENT} con formato inesperado.`, parsed.error.message);
      }),
  };
}
