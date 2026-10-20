import { describe, expect, it } from "vitest";
import type { GhAccount } from "../../domain/github";
import { createMemoryRepositories } from "../../infrastructure/memory/repositories";
import { AppError } from "../../ports/errors";
import { memoryPreferences } from "./preferences";
import { createWorkspaceStore } from "./workspace";

const account = (over: Partial<GhAccount> = {}): GhAccount => ({ login: "ana", name: "Ana", avatarDataUrl: null, connectedAt: "2026-10-19T10:00:00Z", needsReconnect: false, ...over });
const setup = () => {
  const repos = createMemoryRepositories();
  const store = createWorkspaceStore(repos, memoryPreferences(null));
  return { repos, store };
};

describe("slice de GitHub", () => {
  it("connect_shows_the_code_until_connected_event", async () => {
    const { repos, store } = setup();
    await store.getState().listenGithub();
    await store.getState().connectGithub();
    expect(store.getState().github.flow?.code.userCode).toBe("WDJB-MJHT");
    expect(repos.github.starts).toBe(1);
    repos.github.emitConnected(account());
    expect(store.getState().github.flow).toBeNull();
    expect(store.getState().github.account?.login).toBe("ana");
    expect(store.getState().toasts.at(-1)).toMatchObject({ kind: "success", message: "Conectado como @ana." });
  });

  it("connect_failed_clears_the_flow_and_toasts", async () => {
    const { repos, store } = setup();
    await store.getState().listenGithub();
    await store.getState().connectGithub();
    repos.github.emitConnectFailed("El código venció. Vuelve a pulsar «Conectar con GitHub».");
    expect(store.getState().github.flow).toBeNull();
    expect(store.getState().toasts.at(-1)).toMatchObject({ kind: "error", message: "El código venció. Vuelve a pulsar «Conectar con GitHub»." });
  });

  it("gh_auth_error_reloads_account", async () => {
    const { repos, store } = setup();
    repos.github.accountValue = account({ needsReconnect: true });
    repos.github.nextError = new AppError("gh_auth", "Tu cuenta de GitHub no está conectada o la sesión venció.");
    await store.getState().loadGithubRepos();
    expect(store.getState().toasts.at(-1)?.kind).toBe("error");
    expect(store.getState().github.account?.needsReconnect).toBe(true);
    expect(store.getState().github.repos).toBeNull();
  });

  it("disconnect_clears_account_and_repos", async () => {
    const { repos, store } = setup();
    repos.github.accountValue = account();
    repos.github.repos = [{ owner: "o", name: "r", fullName: "o/r", private: false, canWriteIssues: true, canWriteContents: true }];
    await store.getState().loadGithubAccount();
    await store.getState().loadGithubRepos();
    expect(store.getState().github.repos).toHaveLength(1);
    await store.getState().disconnectGithub();
    expect(store.getState().github.account).toBeNull();
    expect(store.getState().github.repos).toBeNull();
    expect(repos.github.accountValue).toBeNull();
  });

  it("cancel_clears_the_flow", async () => {
    const { repos, store } = setup();
    await store.getState().connectGithub();
    await store.getState().cancelGithubConnect();
    expect(store.getState().github.flow).toBeNull();
    expect(repos.github.cancels).toBe(1);
  });
});
