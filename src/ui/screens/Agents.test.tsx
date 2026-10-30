import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { createMemoryRepositories } from "../../infrastructure/memory/repositories";
import { WorkspaceProvider } from "../store/context";
import { createWorkspaceStore } from "../store/workspace";
import { memoryPreferences } from "../store/preferences";
import { emptyMeta } from "../../domain/project";
import { AppSettingsDialog } from "./AppSettingsDialog";
afterEach(cleanup);
async function setup() {
  const closed = vi.fn();
  const repos = createMemoryRepositories();
  const store = createWorkspaceStore(repos, memoryPreferences());
  await store.getState().init();
  render(
    <WorkspaceProvider store={store}>
      <AppSettingsDialog initialTab="Agentes" onClose={closed} />
    </WorkspaceProvider>,
  );
  await screen.findByText("Sin actividad.");
  return { repos, store, closed };
}
it("con el interruptor apagado los grupos están deshabilitados y encender no los marca", async () => {
  await setup();
  for (const name of [
    "Lectura",
    "Escritura",
    "Captura de pantalla",
    "Exportar",
  ])
    expect(screen.getByRole("checkbox", { name })).toBeDisabled();
  fireEvent.click(
    screen.getByRole("checkbox", {
      name: "Permitir que agentes de IA usen Qapture (MCP)",
    }),
  );
  for (const name of [
    "Lectura",
    "Escritura",
    "Captura de pantalla",
    "Exportar",
  ]) {
    expect(screen.getByRole("checkbox", { name })).toBeEnabled();
    expect(screen.getByRole("checkbox", { name })).not.toBeChecked();
  }
});
it("recomendada marca solo Lectura y Escritura; los permisos se guardan", async () => {
  const { repos } = await setup();
  fireEvent.click(
    screen.getByRole("checkbox", {
      name: "Permitir que agentes de IA usen Qapture (MCP)",
    }),
  );
  fireEvent.click(screen.getByText("Activar configuración recomendada"));
  expect(screen.getByRole("checkbox", { name: "Lectura" })).toBeChecked();
  expect(screen.getByRole("checkbox", { name: "Escritura" })).toBeChecked();
  expect(
    screen.getByRole("checkbox", { name: "Captura de pantalla" }),
  ).not.toBeChecked();
  expect(screen.getByRole("checkbox", { name: "Exportar" })).not.toBeChecked();
  fireEvent.click(screen.getByRole("button", { name: "Guardar" }));
  await waitFor(() =>
    expect(repos.settings.snapshot.settings.agents.read).toBe(true),
  );
});
it("cada grupo muestra herramientas y confirmaciones solo para captura y exportación", async () => {
  await setup();
  for (const tool of [
    "get_evidence_image",
    "update_case",
    "capture_screen",
    "export_excel",
  ])
    expect(screen.getByText(new RegExp(tool))).toBeInTheDocument();
  expect(
    screen.getAllByRole("checkbox", { name: /Pedir confirmación/ }),
  ).toHaveLength(2);
});
it("Copiar en Codex copia el bloque toml", async () => {
  const copy = vi.fn().mockResolvedValue(undefined);
  Object.defineProperty(navigator, "clipboard", {
    configurable: true,
    value: { writeText: copy },
  });
  const { repos } = await setup();
  fireEvent.click(screen.getByRole("tab", { name: "Codex" }));
  await waitFor(() =>
    expect(screen.getByRole("button", { name: "Copiar" })).toBeEnabled(),
  );
  fireEvent.click(screen.getByRole("button", { name: "Copiar" }));
  expect(copy).toHaveBeenCalledWith(repos.agents.config.codex);
});
it("regenerar clave pide confirmación y avisa", async () => {
  const { repos, store } = await setup();
  fireEvent.click(screen.getByRole("button", { name: "Regenerar clave" }));
  expect(repos.agents.regenerated).toBe(0);
  const buttons = screen.getAllByRole("button", { name: "Regenerar clave" });
  fireEvent.click(buttons[buttons.length - 1]);
  await waitFor(() => expect(repos.agents.regenerated).toBe(1));
  expect(
    store
      .getState()
      .toasts.some(
        (t) => t.message === "Clave nueva: vuelve a conectar tus agentes.",
      ),
  ).toBe(true);
});
it("avisa cuando el canal no escucha y filtra actividad por cliente", async () => {
  const { repos, store } = await setup();
  repos.agents.config.listening = false;
  repos.agents.rows = [
    {
      id: "1",
      at: "2026-10-29T12:00:00Z",
      client: "codex",
      tool: "create_case",
      projectId: null,
      caseId: "EJ-001",
      ok: true,
      detail: "Caso",
    },
    {
      id: "2",
      at: "2026-10-29T12:00:00Z",
      client: "claude",
      tool: "get_project",
      projectId: null,
      caseId: null,
      ok: false,
      detail: "Proyecto",
    },
  ];
  await store.getState().loadAgents();
  await screen.findByText(
    "Qapture no pudo abrir el canal para agentes. Reinicia la app.",
  );
  expect(await screen.findByText("EJ-001")).toBeInTheDocument();
  fireEvent.change(
    screen.getByRole("textbox", { name: "Filtrar por cliente" }),
    { target: { value: "codex" } },
  );
  await waitFor(() =>
    expect(screen.queryByText("claude")).not.toBeInTheDocument(),
  );
  expect(screen.getByText("create_case")).toBeInTheDocument();
});

it("Ir al caso navega y cierra Ajustes, pero rechaza casos eliminados", async () => {
  const { repos, store, closed } = await setup();
  const project = await repos.projects.create({
    meta: { ...emptyMeta(), name: "Agentes", client: "ACME" },
    templateId: "tpl-funcional",
  });
  const env = project.fields.find((field) => field.key === "entorno")!;
  const item = await repos.cases.create(project.id, {
    title: "Caso",
    status: "No ejecutado",
    values: { [env.id]: "QA" },
  });
  repos.agents.rows = [
    {
      id: "nav",
      at: "2026-10-29T12:00:00Z",
      client: "codex",
      tool: "create_case",
      projectId: project.id,
      caseId: item.id,
      ok: true,
      detail: "create_case",
    },
  ];
  await store.getState().loadAgents();
  fireEvent.click(await screen.findByRole("button", { name: "Ir al caso" }));
  await waitFor(() => expect(closed).toHaveBeenCalledOnce());
  expect(store.getState().selectedCaseId).toBe(item.id);
  await repos.cases.remove(item.id);
  fireEvent.click(screen.getByRole("button", { name: "Ir al caso" }));
  await waitFor(() =>
    expect(
      store
        .getState()
        .toasts.some((toast) => toast.message === "El caso ya no existe."),
    ).toBe(true),
  );
  expect(closed).toHaveBeenCalledOnce();
  expect(store.getState().selectedCaseId).toBeNull();
});
