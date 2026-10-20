import { act, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { GhAccount, GhRepo, ProjectGithub } from "../../domain/github";
import type { MemoryRepositories } from "../../infrastructure/memory/repositories";
import { AppError } from "../../ports/errors";
import { renderApp, seedProject } from "../test-utils";

const account = (over: Partial<GhAccount> = {}): GhAccount => ({ login: "ana", name: "Ana Pérez", avatarDataUrl: null, connectedAt: "2026-10-19T10:00:00Z", needsReconnect: false, ...over });

async function openGithub(options: Parameters<typeof renderApp>[0] = {}) {
  const ctx = await renderApp(options);
  await ctx.user.click(screen.getByRole("button", { name: "Ajustes" }));
  await screen.findByRole("dialog", { name: "Ajustes" });
  await ctx.user.click(screen.getByRole("tab", { name: "GitHub" }));
  return ctx;
}

afterEach(() => vi.restoreAllMocks());

describe("pestaña GitHub de Ajustes", () => {
  it("«Conectar con GitHub» muestra el código y lo copia", async () => {
    const { user, repos } = await openGithub();
    expect(screen.getByText("Conecta tu cuenta para reportar casos como issues e importar issues como casos.")).toBeInTheDocument();
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", { value: { writeText }, configurable: true });
    await user.click(screen.getByRole("button", { name: "Conectar con GitHub" }));
    expect(await screen.findByText("WDJB-MJHT")).toBeInTheDocument();
    expect(screen.getByText("Ingresa este código en GitHub:")).toBeInTheDocument();
    expect(screen.getByText("El código vence en 15:00.")).toBeInTheDocument();
    expect(repos.github.starts).toBe(1);
    await user.click(screen.getByRole("button", { name: "Copiar código" }));
    expect(writeText).toHaveBeenCalledWith("WDJB-MJHT");
    await user.click(screen.getByRole("button", { name: "Abrir GitHub" }));
    expect(repos.settings.calls).toContain("openLink:githubDevice");
    await user.click(within(screen.getByRole("tabpanel")).getByRole("button", { name: "Cancelar" }));
    expect(screen.queryByText("WDJB-MJHT")).not.toBeInTheDocument();
    expect(repos.github.cancels).toBe(1);
  });

  it("al llegar gh:connected muestra @login", async () => {
    const { user, repos } = await openGithub();
    await user.click(screen.getByRole("button", { name: "Conectar con GitHub" }));
    await screen.findByText("WDJB-MJHT");
    act(() => repos.github.emitConnected(account()));
    expect(await screen.findByText("@ana")).toBeInTheDocument();
    expect(screen.getByText("Ana Pérez")).toBeInTheDocument();
    expect(screen.getByText("Conectado como @ana.")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Instalar o configurar Qapture en una cuenta u organización" }));
    await user.click(screen.getByRole("button", { name: "Revocar acceso en GitHub" }));
    expect(repos.settings.calls).toEqual(expect.arrayContaining(["openLink:githubInstall", "openLink:githubRevoke"]));
  });

  it("cuenta con needsReconnect muestra «Reconectar»", async () => {
    const { user, repos } = await openGithub({
      setup: async (r) => {
        r.github.accountValue = account({ needsReconnect: true });
      },
    });
    expect(await screen.findByText("La sesión de GitHub venció o fue revocada.")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Reconectar" }));
    expect(await screen.findByText("WDJB-MJHT")).toBeInTheDocument();
    expect(repos.github.starts).toBe(1);
  });

  it("«Desconectar» pide confirmación", async () => {
    const { user, repos } = await openGithub({
      setup: async (r) => {
        r.github.accountValue = account();
      },
    });
    await user.click(await screen.findByRole("button", { name: "Desconectar" }));
    const confirm = await screen.findByRole("dialog", { name: "Desconectar GitHub" });
    expect(within(confirm).getByText("Se borrará el acceso guardado en este equipo. Tus proyectos conservan los repos vinculados.")).toBeInTheDocument();
    await user.click(within(confirm).getByRole("button", { name: "Cancelar" }));
    expect(repos.github.accountValue).not.toBeNull();
    await user.click(screen.getByRole("button", { name: "Desconectar" }));
    await user.click(within(await screen.findByRole("dialog", { name: "Desconectar GitHub" })).getByRole("button", { name: "Desconectar" }));
    await waitFor(() => expect(repos.github.accountValue).toBeNull());
    expect(await screen.findByRole("button", { name: "Conectar con GitHub" })).toBeInTheDocument();
  });

  it("al agotarse la cuenta regresiva avisa que el código venció y muestra de nuevo «Conectar con GitHub»", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    try {
      const { user, repos } = await openGithub();
      repos.github.deviceCode = { ...repos.github.deviceCode, expiresIn: 2 };
      await user.click(screen.getByRole("button", { name: "Conectar con GitHub" }));
      await screen.findByText("WDJB-MJHT");
      await act(async () => {
        await vi.advanceTimersByTimeAsync(3000);
      });
      expect(await screen.findByRole("button", { name: "Conectar con GitHub" })).toBeInTheDocument();
      expect(screen.getByText("El código venció. Vuelve a pulsar «Conectar con GitHub».")).toBeInTheDocument();
      expect(repos.github.cancels).toBe(1);
    } finally {
      vi.useRealTimers();
    }
  });
});

const ghRepo = (fullName: string, over: Partial<GhRepo> = {}): GhRepo => {
  const [owner, name] = fullName.split("/");
  return { owner, name, fullName, private: false, canWriteIssues: true, canWriteContents: true, ...over };
};

/** Proyecto con una columna de tipo Lista, que la plantilla no trae. */
async function seedWithList(r: MemoryRepositories) {
  const p = await seedProject(r);
  const inputs = p.fields.map((f) => ({ ...f, id: f.id as string | null }));
  const saved = await r.projects.saveFields(p.id, [
    ...inputs,
    { id: null, label: "Severidad", type: "list", options: ["Caja", "Reportes"], required: false, clearOnDuplicate: false, width: 20, archived: false },
  ]);
  return { id: p.id, fields: saved };
}

async function openProjectGithub(
  repoList: GhRepo[] = [ghRepo("acme/web"), ghRepo("acme/api", { private: true })],
  withAccount = true,
  extra?: (r: MemoryRepositories) => void,
) {
  const ctx = await renderApp({
    setup: async (r) => {
      await seedWithList(r);
      r.github.repos = repoList;
      if (withAccount) r.github.accountValue = account();
      extra?.(r);
    },
  });
  await ctx.user.click(screen.getByRole("button", { name: /Configurar/ }));
  const dialog = screen.getByRole("dialog", { name: "Configurar proyecto" });
  await ctx.user.click(within(dialog).getByRole("tab", { name: "GitHub" }));
  return { ...ctx, dialog };
}

describe("pestaña GitHub de Configurar proyecto", () => {
  it("vincular dos repos y marcar el predeterminado guarda la config", async () => {
    const { user, repos, store, dialog } = await openProjectGithub();
    const add = await within(dialog).findByLabelText("Agregar repositorio");
    await waitFor(() => expect(within(add).getByRole("option", { name: /acme\/web/ })).toBeInTheDocument());
    await user.selectOptions(add, "acme/web");
    await user.selectOptions(within(dialog).getByLabelText("Agregar repositorio"), "acme/api");
    expect(within(dialog).getByRole("radio", { name: "Predeterminado acme/web" })).toBeChecked();
    await user.click(within(dialog).getByRole("radio", { name: "Predeterminado acme/api" }));
    const labels = within(dialog).getByLabelText("Etiquetas de acme/api");
    expect(labels).toHaveValue("bug, qapture");
    await user.clear(labels);
    await user.type(labels, "bug, ui");
    await user.click(within(dialog).getByRole("button", { name: "Guardar" }));
    expect(await screen.findByText("Configuración de GitHub guardada.")).toBeInTheDocument();
    const projectId = store.getState().project!.id;
    const saved = await repos.github.getProjectGithub(projectId);
    expect(saved.repos).toEqual([
      { fullName: "acme/web", isDefault: false, defaultLabels: ["bug", "qapture"] },
      { fullName: "acme/api", isDefault: true, defaultLabels: ["bug", "ui"] },
    ]);
  });

  it("sin predeterminado muestra «Marca un repositorio como predeterminado.» y no guarda", async () => {
    const { user, repos, store, dialog } = await openProjectGithub();
    const add = await within(dialog).findByLabelText("Agregar repositorio");
    await waitFor(() => expect(within(add).getByRole("option", { name: /acme\/web/ })).toBeInTheDocument());
    await user.selectOptions(add, "acme/web");
    await user.selectOptions(within(dialog).getByLabelText("Agregar repositorio"), "acme/api");
    await user.click(within(dialog).getByRole("button", { name: "Quitar acme/web" }));
    await user.click(within(dialog).getByRole("button", { name: "Guardar" }));
    expect(await within(dialog).findByText("Marca un repositorio como predeterminado.")).toBeInTheDocument();
    expect((await repos.github.getProjectGithub(store.getState().project!.id)).repos).toEqual([]);
  });

  it("el ruteo solo ofrece columnas de tipo Lista", async () => {
    const { dialog, store } = await openProjectGithub();
    const select = within(dialog).getByLabelText("Columna de ruteo");
    const fields = store.getState().project!.fields.filter((f) => !f.archived);
    const lists = fields.filter((f) => f.type === "list").map((f) => f.label);
    expect(lists.length).toBeGreaterThan(0);
    expect(fields.some((f) => f.type !== "list")).toBe(true);
    const names = within(select).getAllByRole("option").map((o) => o.textContent);
    expect(names).toEqual(["Sin ruteo", ...lists]);
  });

  it("routingStale muestra el aviso", async () => {
    const ctx = await renderApp({
      setup: async (r) => {
        const p = await seedWithList(r);
        r.github.repos = [ghRepo("acme/web")];
        r.github.accountValue = account();
        const field = p.fields.find((f) => f.type === "list")!;
        await r.github.setProjectGithub(p.id, {
          repos: [{ fullName: "acme/web", isDefault: true, defaultLabels: ["bug"] }],
          routingFieldId: field.id,
          routingMap: { [field.options[0]]: "acme/web" },
          evidenceRepo: null,
        });
        // Se renombra la opción enrutada: la config guardada queda obsoleta.
        await r.projects.saveFields(
          p.id,
          p.fields.map((f) => (f.id === field.id ? { ...f, options: f.options.map((o, i) => (i === 0 ? `${o} (renombrada)` : o)) } : f)),
        );
      },
    });
    await ctx.user.click(screen.getByRole("button", { name: /Configurar/ }));
    const dialog = screen.getByRole("dialog", { name: "Configurar proyecto" });
    await ctx.user.click(within(dialog).getByRole("tab", { name: "GitHub" }));
    expect(await within(dialog).findByText("La columna o alguna opción del ruteo cambió; revísalo.")).toBeInTheDocument();
  });

  it("sin cuenta muestra el enlace a Ajustes", async () => {
    const { user, dialog } = await openProjectGithub([], false);
    expect(within(dialog).getByText("Conecta tu cuenta de GitHub en Ajustes › GitHub para vincular repositorios.")).toBeInTheDocument();
    await user.click(within(dialog).getByRole("button", { name: "Abrir Ajustes" }));
    const settings = await screen.findByRole("dialog", { name: "Ajustes" });
    expect(within(settings).getByRole("tab", { name: "GitHub", selected: true })).toBeInTheDocument();
    expect(screen.queryByRole("dialog", { name: "Configurar proyecto" })).not.toBeInTheDocument();
  });

  it("¿No ves tu repo? copia el enlace de instalación y abre la guía para el admin", async () => {
    const { user, dialog, repos } = await openProjectGithub();
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", { value: { writeText }, configurable: true });
    await waitFor(() => expect(within(dialog).getByRole("button", { name: "¿No ves tu repo?" })).toBeEnabled());
    await user.click(within(dialog).getByRole("button", { name: "¿No ves tu repo?" }));
    expect(writeText).toHaveBeenCalledWith("https://github.com/apps/qapture-qa/installations/new");
    expect(await screen.findByText("Pide a un owner de la organización que instale Qapture en ese repositorio. Enlace copiado.")).toBeInTheDocument();
    await waitFor(() => expect(repos.settings.calls).toContain("openLink:githubAdminGuide"));
  });

  it("no se puede editar ni guardar hasta que la config carga", async () => {
    let resolve: (config: ProjectGithub) => void = () => {};
    const { user, dialog, repos, store } = await openProjectGithub(undefined, true, (r) => {
      const real = r.github.getProjectGithub.bind(r.github);
      vi.spyOn(r.github, "getProjectGithub").mockImplementation(
        (id) =>
          new Promise<ProjectGithub>((ok) => {
            resolve = (config) => ok(config);
            void real(id);
          }),
      );
    });
    const save = within(dialog).getByRole("button", { name: "Guardar" });
    expect(save).toBeDisabled();
    expect(within(dialog).getByLabelText("Agregar repositorio")).toBeDisabled();
    const projectId = store.getState().project!.id;
    await act(async () => {
      resolve({ repos: [{ fullName: "acme/web", isDefault: true, defaultLabels: ["bug"] }], routingFieldId: null, routingMap: {}, evidenceRepo: null, routingStale: false });
    });
    await waitFor(() => expect(within(dialog).getByRole("button", { name: "Guardar" })).toBeEnabled());
    expect(within(dialog).getByText("acme/web", { selector: "span" })).toBeInTheDocument();
    await user.click(within(dialog).getByRole("button", { name: "Guardar" }));
    expect(await screen.findByText("Configuración de GitHub guardada.")).toBeInTheDocument();
    vi.restoreAllMocks();
    expect((await repos.github.getProjectGithub(projectId)).repos).toHaveLength(1);
  });

  it("si la config no carga, avisa y no deja guardar", async () => {
    const { dialog, repos, store } = await openProjectGithub(undefined, true, (r) => {
      vi.spyOn(r.github, "getProjectGithub").mockRejectedValue(new AppError("io", "No se pudo leer la configuración."));
    });
    expect(await within(dialog).findByText("No se pudo cargar la configuración de GitHub del proyecto. Cierra y vuelve a abrir esta pestaña.")).toBeInTheDocument();
    expect(within(dialog).getByRole("button", { name: "Guardar" })).toBeDisabled();
    expect(within(dialog).getByLabelText("Agregar repositorio")).toBeDisabled();
    vi.restoreAllMocks();
    expect((await repos.github.getProjectGithub(store.getState().project!.id)).repos).toEqual([]);
  });
});
