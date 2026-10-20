import { describe, expect, it } from "vitest";
import type { GhAccount } from "../../domain/github";
import { emptyMeta } from "../../domain/project";
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

  describe("reporte", () => {
    const input = (caseId: string, evidenceIds: string[] = []) => ({ caseId, repo: "o/r", title: "[EJ-1] t", body: "b", labels: [], assignees: [], milestone: null, evidenceIds });
    const reportSetup = async () => {
      const { repos, store } = setup();
      const p = await repos.projects.create({ meta: { ...emptyMeta(), name: "Recibos" }, templateId: "tpl-funcional" });
      const c = await repos.cases.create(p.id, { title: "Caso", status: "No ejecutado", values: { [p.fields.find((f) => f.key === "entorno")!.id]: "QA" } });
      await store.getState().selectProject(p.id);
      return { repos, store, projectId: p.id, caseId: c.id };
    };

    it("report_success_adds_the_link_and_toasts", async () => {
      const { store, projectId, caseId } = await reportSetup();
      const out = await store.getState().reportIssue(input(caseId));
      expect(out.ok).toBe(true);
      expect(store.getState().github.links[projectId]).toHaveLength(1);
      expect(store.getState().toasts.at(-1)).toMatchObject({ kind: "success", message: "Issue #1 creado en o/r." });
    });

    it("assisted_report_keeps_pending_items", async () => {
      const { repos, store, caseId } = await reportSetup();
      repos.github.forcedUpload = { mode: "assisted", reason: "No hay repo de evidencias configurado." };
      await store.getState().reportIssue(input(caseId, ["e1", "e2"]));
      expect(store.getState().github.pending[`${caseId}|o/r`]).toEqual({
        issueUrl: "https://github.com/o/r/issues/1",
        items: [
          { evidenceId: "e1", n: 1, done: false },
          { evidenceId: "e2", n: 2, done: false },
        ],
        reason: "No hay repo de evidencias configurado.",
      });
    });

    it("conflict_reloads_links_and_returns_form_error", async () => {
      const { repos, store, projectId, caseId } = await reportSetup();
      repos.github.links.push({ caseId, owner: "o", repo: "r", fullName: "o/r", issueNumber: 45, htmlUrl: "https://github.com/o/r/issues/45", direction: "exported", issueState: "open" });
      const out = await store.getState().reportIssue(input(caseId));
      expect(out).toEqual({ ok: false, errors: { _form: "El caso ya tiene un issue en o/r (#45)." } });
      expect(store.getState().github.links[projectId]).toHaveLength(1);
    });

    it("copy_pending_toasts_with_n", async () => {
      const { repos, store, caseId } = await reportSetup();
      repos.github.forcedUpload = { mode: "assisted" };
      await store.getState().reportIssue(input(caseId, ["e1", "e2"]));
      await store.getState().copyPendingEvidence(`${caseId}|o/r`, "e2");
      expect(repos.github.copies).toBe(1);
      expect(store.getState().toasts.at(-1)).toMatchObject({ kind: "success", message: "Captura 2 copiada. Pégala con Ctrl+V en GitHub." });
    });

    it("marking_all_done_clears_the_pending_entry", async () => {
      const { repos, store, caseId } = await reportSetup();
      const key = `${caseId}|o/r`;
      repos.github.forcedUpload = { mode: "assisted" };
      await store.getState().reportIssue(input(caseId, ["e1", "e2"]));
      store.getState().markPendingDone(key, "e1", true);
      expect(store.getState().github.pending[key].items.map((i) => i.done)).toEqual([true, false]);
      store.getState().markPendingDone(key, "e2", true);
      expect(store.getState().github.pending[key]).toBeUndefined();
    });
  });
});
