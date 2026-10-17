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

export async function renderApp(options: { setup?: (repos: MemoryRepositories) => Promise<void>; lastProject?: string | null; tester?: string } = {}) {
  const repos = createMemoryRepositories();
  await options.setup?.(repos);
  const store = createWorkspaceStore(repos, memoryPreferences(options.lastProject ?? null, options.tester ?? ""));
  const user = userEvent.setup();
  render(
    <WorkspaceProvider store={store}>
      <AppShell />
    </WorkspaceProvider>,
  );
  await screen.findByTestId("app-ready");
  return { repos, store, user };
}
