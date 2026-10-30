import { afterEach, expect, it, vi } from "vitest";
import { emptyMeta } from "../../domain/project";
import { createMemoryRepositories } from "../../infrastructure/memory/repositories";
import { createWorkspaceStore } from "./workspace";
import { memoryPreferences } from "./preferences";
afterEach(() => vi.useRealTimers());
async function setup() {
  const repos = createMemoryRepositories();
  const project = await repos.projects.create({
    meta: { ...emptyMeta(), name: "Agentes", client: "ACME" },
    templateId: "tpl-funcional",
  });
  const store = createWorkspaceStore(repos, memoryPreferences());
  await store.getState().init();
  const off = await store.getState().listenAgents();
  return { repos, project, store, off };
}
const flush = async () => {
  for (let i = 0; i < 30; i++) await Promise.resolve();
};
it("data-changed for the open project reloads cases", async () => {
  const { repos, project, off } = await setup();
  const spy = vi.spyOn(repos.cases, "list");
  repos.agents.emit({ projectId: project.id, caseId: null, summary: "Cambio" });
  await flush();
  expect(spy).toHaveBeenCalledWith(project.id);
  off();
});
it("data-changed for another project only reloads the project list", async () => {
  const { repos, store, off } = await setup();
  const project = store.getState().project;
  const cases = vi.spyOn(repos.cases, "list");
  const projects = vi.spyOn(repos.projects, "list");
  repos.agents.emit({ projectId: "other", caseId: null, summary: "Cambio" });
  await flush();
  expect(projects).toHaveBeenCalledOnce();
  expect(cases).not.toHaveBeenCalled();
  expect(store.getState().project).toBe(project);
  off();
});
it("data-changed for the selected case reloads its evidence", async () => {
  const { repos, project, store, off } = await setup();
  const field = project.fields.find((f) => f.key === "entorno")!;
  const c = await repos.cases.create(project.id, {
    title: "Caso",
    status: "No ejecutado",
    values: { [field.id]: "QA" },
  });
  store.getState().selectCase(c.id);
  await flush();
  const spy = vi.spyOn(repos.evidence, "list");
  repos.agents.emit({
    projectId: project.id,
    caseId: c.id,
    summary: "Captura",
  });
  await flush();
  expect(spy).toHaveBeenCalledWith(c.id);
  off();
});
it("groups toasts in a 5 s window and unsubscribes", async () => {
  vi.useFakeTimers();
  const { repos, store, off } = await setup();
  for (let i = 0; i < 3; i++)
    repos.agents.emit({ projectId: null, caseId: null, summary: "Cambio" });
  await flush();
  expect(store.getState().toasts).toHaveLength(0);
  await vi.advanceTimersByTimeAsync(5000);
  expect(store.getState().toasts.map((t) => t.message)).toEqual([
    "Un agente hizo 3 cambios",
  ]);
  off();
  repos.agents.emit({ projectId: null, caseId: null, summary: "No" });
  await vi.advanceTimersByTimeAsync(5000);
  expect(store.getState().toasts).toHaveLength(1);
});

it("a slow activity filter cannot replace the newest filter", async () => {
  const { repos, store, off } = await setup();
  let finishOld!: (
    rows: Awaited<ReturnType<typeof repos.agents.activity>>,
  ) => void;
  const oldRows = new Promise<
    Awaited<ReturnType<typeof repos.agents.activity>>
  >((resolve) => {
    finishOld = resolve;
  });
  vi.spyOn(repos.agents, "activity").mockImplementation((_limit, client) =>
    client === "old"
      ? oldRows
      : Promise.resolve([
          {
            id: "new",
            at: "2026-10-29T12:00:00Z",
            client: "new",
            tool: "ping",
            projectId: null,
            caseId: null,
            ok: true,
            detail: "ping",
          },
        ]),
  );
  const old = store.getState().loadAgents("old");
  await store.getState().loadAgents("new");
  finishOld([]);
  await old;
  expect(store.getState().agentActivity.map((row) => row.client)).toEqual([
    "new",
  ]);
  off();
});

it("rotating the key preserves the selected activity filter", async () => {
  const { repos, store, off } = await setup();
  repos.agents.rows = ["codex", "claude"].map((client) => ({
    id: client,
    at: "2026-10-29T12:00:00Z",
    client,
    tool: "get_project",
    projectId: null,
    caseId: null,
    ok: true,
    detail: "get_project",
  }));
  await store.getState().loadAgents("codex");
  await store.getState().regenerateAgentKey();
  expect(repos.agents.regenerated).toBe(1);
  expect(store.getState().agentActivity.map((row) => row.client)).toEqual([
    "codex",
  ]);
  off();
});
