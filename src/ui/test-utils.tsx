import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { emptyMeta } from "../domain/project";
import { createMemoryRepositories, type MemoryRepositories } from "../infrastructure/memory/repositories";
import type { Repositories } from "../ports/repositories";
import { AppShell } from "./screens/AppShell";
import { WorkspaceProvider } from "./store/context";
import { memoryPreferences } from "./store/preferences";
import { createWorkspaceStore } from "./store/workspace";

export async function seedProject(repos: Repositories, name = "Recibos") {
  return repos.projects.create({ meta: { ...emptyMeta(), name, client: "ACME" }, templateId: "tpl-funcional" });
}

export const PNG_BYTES = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
export const png = (name = "captura.png") => new File([new Uint8Array(PNG_BYTES)], name, { type: "image/png" });

export async function seedCases(repos: Repositories) {
  const p = await seedProject(repos);
  const entorno = p.fields.find((f) => f.key === "entorno")!.id;
  const a = await repos.cases.create(p.id, { title: "Recibos", status: "No ejecutado", values: { [entorno]: "QA" } });
  const b = await repos.cases.create(p.id, { title: "Pólizas", status: "No ejecutado", values: { [entorno]: "QA" } });
  return { p, a, b };
}

export async function renderApp(options: { setup?: (repos: MemoryRepositories) => Promise<void>; lastProject?: string | null; tester?: string } = {}) {
  const repos = createMemoryRepositories();
  await options.setup?.(repos);
  if (options.tester) repos.settings.snapshot.settings.profile.tester = options.tester;
  const store = createWorkspaceStore(repos, memoryPreferences(options.lastProject ?? null));
  const user = userEvent.setup();
  render(
    <WorkspaceProvider store={store}>
      <AppShell />
    </WorkspaceProvider>,
  );
  await screen.findByTestId("app-ready");
  return { repos, store, user };
}
