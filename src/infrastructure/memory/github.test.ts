import { describe, expect, it, vi } from "vitest";
import { AppError } from "../../ports/errors";
import { createMemoryRepositories } from "./repositories";

const account = { login: "ps3udo", name: null, avatarDataUrl: null, connectedAt: "2026-10-19T00:00:00Z", needsReconnect: false };

describe("memory github", () => {
  it("emite la conexión, la guarda y permite desuscribirse", async () => {
    const { github } = createMemoryRepositories();
    const handler = vi.fn();
    const off = await github.onConnected(handler);
    github.emitConnected(account);
    expect(handler).toHaveBeenCalledWith(account);
    expect(await github.account()).toEqual(account);
    off();
    github.emitConnected(account);
    expect(handler).toHaveBeenCalledTimes(1);
    await github.disconnect();
    expect(await github.account()).toBeNull();
  });

  it("nextError falla una sola vez", async () => {
    const { github } = createMemoryRepositories();
    github.nextError = new AppError("gh_auth", "x");
    await expect(github.listRepos()).rejects.toMatchObject({ code: "gh_auth" });
    expect(await github.listRepos()).toEqual([]);
  });

  it("valida y guarda la configuración de un proyecto, marcando el ruteo viejo", async () => {
    const repos = createMemoryRepositories();
    const project = await repos.projects.create({
      meta: { name: "P", client: "", testType: "", releaseVersion: "", startDate: "", endDate: "" },
      templateId: "tpl-funcional",
    });
    const bad = { repos: [{ fullName: "x", isDefault: true, defaultLabels: [] }], routingFieldId: null, routingMap: {}, evidenceRepo: null };
    await expect(repos.github.setProjectGithub(project.id, bad)).rejects.toMatchObject({ code: "validation" });
    const ok = { repos: [{ fullName: "a/b", isDefault: true, defaultLabels: ["bug"] }], routingFieldId: "no-existe", routingMap: {}, evidenceRepo: null };
    await expect(repos.github.setProjectGithub(project.id, ok)).rejects.toMatchObject({ message: "La columna de ruteo no existe en este proyecto." });
    const saved = await repos.github.setProjectGithub(project.id, { ...ok, routingFieldId: null });
    expect(saved.routingStale).toBe(false);
    expect((await repos.github.getProjectGithub(project.id)).repos[0].fullName).toBe("a/b");
  });

  it("simula vínculos, conflicto por (caso, repo), modo forzado, copias y aperturas", async () => {
    const { github } = createMemoryRepositories();
    const input = { caseId: "c1", repo: "o/r", title: "t", body: "b", labels: [], assignees: [], milestone: null, evidenceIds: ["e1"] };
    const first = await github.createIssue(input);
    expect(first).toMatchObject({ uploadMode: "repo", assistedReason: null, pendingEvidence: [], commentUrl: null });
    await expect(github.createIssue(input)).rejects.toMatchObject({ code: "gh_conflict", message: "El caso ya tiene un issue en o/r (#1)." });
    github.forcedUpload = { mode: "assisted", reason: "Tu cuenta no puede escribir en o/r." };
    const comment = await github.commentIssue(input);
    expect(comment).toMatchObject({ uploadMode: "assisted", assistedReason: "Tu cuenta no puede escribir en o/r.", pendingEvidence: ["e1"] });
    expect(comment.commentUrl).toContain("/issues/1#");
    await github.copyEvidence("e1");
    await github.openIssue("c1", "o/r");
    expect(github.copies).toBe(1);
    expect(github.opened).toEqual([{ caseId: "c1", repo: "o/r" }]);
  });
});
