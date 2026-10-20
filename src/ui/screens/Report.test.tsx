import { screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { emptyAnnotations, serializeAnnotations } from "../../domain/annotations";
import type { CaseIssueLink, GhAccount, LinkedRepo } from "../../domain/github";
import type { MemoryRepositories } from "../../infrastructure/memory/repositories";
import { PNG_BYTES, renderApp, seedProject } from "../test-utils";

const account: GhAccount = { login: "ana", name: "Ana Pérez", avatarDataUrl: null, connectedAt: "2026-10-19T10:00:00Z", needsReconnect: false };
const TESTING = "Ps3udoDev/testing";
const BACKEND = "Ps3udoDev/backend";

interface Seeded {
  caseId: string;
  otherId: string;
  evidenceIds: string[];
}

/** Proyecto con columna Lista «Componente», dos casos (el primero con 2 capturas) y la config GitHub pedida. */
async function seed(
  r: MemoryRepositories,
  options: { account?: boolean; repos?: LinkedRepo[]; routing?: Record<string, string>; links?: (s: Seeded) => CaseIssueLink[] } = {},
): Promise<Seeded> {
  const p = await seedProject(r);
  const saved = await r.projects.saveFields(p.id, [
    ...p.fields.map((f) => ({ ...f, id: f.id as string | null })),
    { id: null, label: "Componente", type: "list", options: ["Front", "Back"], required: false, clearOnDuplicate: false, width: 20, archived: false },
  ]);
  const entorno = saved.find((f) => f.key === "entorno")!.id;
  const componente = saved.find((f) => f.label === "Componente")!.id;
  const a = await r.cases.create(p.id, { title: "Recibos", status: "Fallido", values: { [entorno]: "QA", [componente]: "Back" } });
  const b = await r.cases.create(p.id, { title: "Pólizas", status: "No ejecutado", values: { [entorno]: "QA" } });
  const evidenceIds: string[] = [];
  for (const description of ["Primera", "Segunda"]) {
    const staged = await r.evidence.stage(new Uint8Array(PNG_BYTES));
    const e = await r.evidence.add({ caseId: a.id, stagedPath: staged.path, description, annotations: serializeAnnotations(emptyAnnotations()) }, new Uint8Array(PNG_BYTES));
    evidenceIds.push(e.id);
  }
  if (options.account !== false) r.github.accountValue = account;
  const repos = options.repos ?? [{ fullName: TESTING, isDefault: true, defaultLabels: ["bug", "qapture"] }];
  if (repos.length > 0) {
    await r.github.setProjectGithub(p.id, {
      repos,
      routingFieldId: options.routing ? componente : null,
      routingMap: options.routing ?? {},
      evidenceRepo: null,
    });
  }
  r.github.metas[TESTING] = { labels: ["bug", "qapture", "ui"], milestones: [{ number: 3, title: "Sprint 3" }], assignees: ["ana", "luis"] };
  const seeded = { caseId: a.id, otherId: b.id, evidenceIds };
  r.github.links = options.links?.(seeded) ?? [];
  return seeded;
}

const link = (caseId: string, fullName = TESTING, issueNumber = 1): CaseIssueLink => {
  const [owner, repo] = fullName.split("/");
  return { caseId, owner, repo, fullName, issueNumber, htmlUrl: `https://github.com/${fullName}/issues/${issueNumber}`, direction: "exported", issueState: "open" };
};

async function openReport(options: Parameters<typeof seed>[1] = {}) {
  let seeded: Seeded | undefined;
  const ctx = await renderApp({
    setup: async (r) => {
      seeded = await seed(r, options);
    },
  });
  const list = screen.getByRole("list", { name: "Lista de casos" });
  await ctx.user.click(within(list).getByText("Recibos"));
  await ctx.user.click(await screen.findByRole("button", { name: "Reportar en GitHub" }));
  return { ...ctx, seeded: seeded! };
}

afterEach(() => vi.restoreAllMocks());

describe("reportar en GitHub", () => {
  it("sin cuenta muestra «Conecta tu cuenta…»", async () => {
    const { user } = await openReport({ account: false });
    const dialog = await screen.findByRole("dialog", { name: "Reportar en GitHub" });
    expect(await within(dialog).findByText("Conecta tu cuenta de GitHub en Ajustes › GitHub.")).toBeInTheDocument();
    await user.click(within(dialog).getByRole("button", { name: "Abrir Ajustes" }));
    await screen.findByRole("dialog", { name: "Ajustes" });
    expect(screen.getByRole("tab", { name: "GitHub" })).toHaveAttribute("aria-selected", "true");
    expect(screen.queryByRole("dialog", { name: "Reportar en GitHub" })).not.toBeInTheDocument();
  });

  it("sin repos muestra «Este proyecto no tiene repositorios vinculados.»", async () => {
    const { user } = await openReport({ repos: [] });
    const dialog = await screen.findByRole("dialog", { name: "Reportar en GitHub" });
    expect(await within(dialog).findByText("Este proyecto no tiene repositorios vinculados.")).toBeInTheDocument();
    await user.click(within(dialog).getByRole("button", { name: "Vincular repositorio" }));
    await screen.findByRole("dialog", { name: "Configurar proyecto" });
    expect(screen.getByRole("tab", { name: "GitHub" })).toHaveAttribute("aria-selected", "true");
  });

  it("precarga el repo sugerido por la columna de ruteo", async () => {
    await openReport({
      repos: [
        { fullName: TESTING, isDefault: true, defaultLabels: [] },
        { fullName: BACKEND, isDefault: false, defaultLabels: [] },
      ],
      routing: { Back: BACKEND },
    });
    const dialog = await screen.findByRole("dialog", { name: "Reportar en GitHub" });
    await waitFor(() => expect(within(dialog).getByLabelText("Repositorio")).toHaveValue(BACKEND));
    expect(within(dialog).getByText(`Se creará en ${BACKEND} como @ana.`)).toBeInTheDocument();
  });

  it("crea el issue y muestra el chip testing#1", async () => {
    const { user, repos, seeded } = await openReport();
    const create = vi.spyOn(repos.github, "createIssue");
    const dialog = await screen.findByRole("dialog", { name: "Reportar en GitHub" });
    await waitFor(() => expect(within(dialog).getByLabelText("Repositorio")).toHaveValue(TESTING));
    expect((within(dialog).getByLabelText("Título") as HTMLInputElement).value).toMatch(/^\[[A-Z]+-\d+\] Recibos$/);
    expect(await within(dialog).findByRole("checkbox", { name: "ui" })).not.toBeChecked();
    expect(within(dialog).getByRole("checkbox", { name: "bug" })).toBeChecked();
    expect(within(dialog).getByRole("option", { name: "Sin milestone" })).toBeInTheDocument();
    await user.click(within(dialog).getByRole("tab", { name: "Vista previa" }));
    expect(within(dialog).getByTestId("issue-preview").textContent).toContain("{{img:2}}");
    await user.click(within(dialog).getByRole("button", { name: "Crear issue" }));
    await waitFor(() => expect(screen.queryByRole("dialog", { name: "Reportar en GitHub" })).not.toBeInTheDocument());
    expect(create).toHaveBeenCalledTimes(1);
    expect(create.mock.calls[0][0]).toMatchObject({ caseId: seeded.caseId, repo: TESTING, labels: ["bug", "qapture"], evidenceIds: seeded.evidenceIds, milestone: null });
    expect(screen.getByText(`Issue #1 creado en ${TESTING}.`)).toBeInTheDocument();
    await user.click(await screen.findByRole("button", { name: "testing#1" }));
    expect(repos.github.opened).toEqual([{ caseId: seeded.caseId, repo: TESTING }]);
  });

  it("al cambiar de repo no arrastra el milestone ni los asignados", async () => {
    const { user, repos } = await openReport({
      repos: [
        { fullName: TESTING, isDefault: true, defaultLabels: [] },
        { fullName: BACKEND, isDefault: false, defaultLabels: [] },
      ],
    });
    repos.github.metas[BACKEND] = { labels: [], milestones: [], assignees: ["luis"] };
    const create = vi.spyOn(repos.github, "createIssue");
    const dialog = await screen.findByRole("dialog", { name: "Reportar en GitHub" });
    await waitFor(() => expect(within(dialog).getByLabelText("Repositorio")).toHaveValue(TESTING));
    await user.selectOptions(within(dialog).getByLabelText("Milestone"), await within(dialog).findByRole("option", { name: "Sprint 3" }));
    await user.click(within(dialog).getByRole("checkbox", { name: "ana" }));
    await user.selectOptions(within(dialog).getByLabelText("Repositorio"), BACKEND);
    expect(await within(dialog).findByRole("checkbox", { name: "luis" })).not.toBeChecked();
    expect(within(dialog).queryByRole("checkbox", { name: "ana" })).not.toBeInTheDocument();
    expect(within(dialog).getByLabelText("Milestone")).toHaveValue("");
    await user.click(within(dialog).getByRole("button", { name: "Crear issue" }));
    await waitFor(() => expect(create).toHaveBeenCalledTimes(1));
    expect(create.mock.calls[0][0]).toMatchObject({ repo: BACKEND, milestone: null, assignees: [] });
  });

  it("con vínculo existente cambia a «Comentar en Ps3udoDev/testing#1»", async () => {
    const { user, repos, seeded } = await openReport({ links: (s) => [link(s.caseId)] });
    const comment = vi.spyOn(repos.github, "commentIssue");
    const dialog = await screen.findByRole("dialog", { name: `Comentar en ${TESTING}#1` });
    expect(await within(dialog).findByText(`Se comentará en ${TESTING}#1 como @ana.`)).toBeInTheDocument();
    expect((within(dialog).getByLabelText("Descripción") as HTMLTextAreaElement).value).toMatch(/^Re-test el \d{4}-\d{2}-\d{2} por ana: estado Fallido\./);
    await user.click(within(dialog).getByRole("button", { name: "Comentar" }));
    await waitFor(() => expect(comment).toHaveBeenCalledTimes(1));
    expect(comment.mock.calls[0][0]).toMatchObject({ caseId: seeded.caseId, repo: TESTING });
    expect(await screen.findByText(`Comentario agregado en ${TESTING}#1.`)).toBeInTheDocument();
  });

  it("modo asistido abre el panel y «Copiar» copia la captura 1", async () => {
    const { user, repos, seeded } = await openReport();
    repos.github.forcedUpload = { mode: "assisted", reason: `Tu cuenta no puede escribir en ${TESTING}.` };
    const dialog = await screen.findByRole("dialog", { name: "Reportar en GitHub" });
    await waitFor(() => expect(within(dialog).getByLabelText("Repositorio")).toHaveValue(TESTING));
    await user.click(within(dialog).getByRole("button", { name: "Crear issue" }));
    const panel = await screen.findByRole("dialog", { name: "Termina de adjuntar las capturas" });
    expect(within(panel).getByText(`Tu cuenta no puede escribir en ${TESTING}.`)).toBeInTheDocument();
    expect(within(panel).getByText("Copia cada captura y pégala con Ctrl+V en un comentario del issue (o editando la descripción).")).toBeInTheDocument();
    expect(repos.github.opened).toEqual([{ caseId: seeded.caseId, repo: TESTING }]);
    await user.click(within(panel).getAllByRole("button", { name: "Copiar" })[0]);
    expect(repos.github.copied).toEqual([seeded.evidenceIds[0]]);
    expect(await screen.findByText("Captura 1 copiada. Pégala con Ctrl+V en GitHub.")).toBeInTheDocument();
    await user.click(within(panel).getAllByRole("checkbox", { name: "Pegada" })[0]);
    const footer = within(panel).getAllByRole("button", { name: "Cerrar" });
    await user.click(footer[footer.length - 1]);
    expect(screen.queryByRole("dialog", { name: "Termina de adjuntar las capturas" })).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "1 capturas sin pegar" }));
    expect(await screen.findByRole("dialog", { name: "Termina de adjuntar las capturas" })).toBeInTheDocument();
  });

  it("doble clic en «Crear issue» crea un solo issue", async () => {
    const { user, repos } = await openReport();
    const create = vi.spyOn(repos.github, "createIssue");
    const dialog = await screen.findByRole("dialog", { name: "Reportar en GitHub" });
    await waitFor(() => expect(within(dialog).getByLabelText("Repositorio")).toHaveValue(TESTING));
    await user.dblClick(within(dialog).getByRole("button", { name: "Crear issue" }));
    await waitFor(() => expect(screen.queryByRole("dialog", { name: "Reportar en GitHub" })).not.toBeInTheDocument());
    expect(create).toHaveBeenCalledTimes(1);
    expect(repos.github.links).toHaveLength(1);
  });

  it("avisa si una captura pesa más de 10 MB", async () => {
    const { user, repos, seeded } = await openReport();
    const dialog = await screen.findByRole("dialog", { name: "Reportar en GitHub" });
    await waitFor(() => expect(within(dialog).getByLabelText("Repositorio")).toHaveValue(TESTING));
    expect(within(dialog).queryByText(/pesa más de 10 MB/)).not.toBeInTheDocument();
    // El tamaño se lee al abrir; se reabre con la segunda captura grande.
    const big = new Uint8Array(10 * 1024 * 1024 + 1);
    vi.spyOn(repos.evidence, "readImage").mockImplementation(async (path) => (path.includes(seeded.evidenceIds[1]) ? big : new Uint8Array(PNG_BYTES)));
    await user.click(within(dialog).getByRole("button", { name: "Cancelar" }));
    await user.click(screen.getByRole("button", { name: "Reportar en GitHub" }));
    const again = await screen.findByRole("dialog", { name: "Reportar en GitHub" });
    expect(await within(again).findByText("La captura 2 pesa más de 10 MB; GitHub podría rechazarla.")).toBeInTheDocument();
    expect(within(again).getByRole("checkbox", { name: "Captura 2 · Segunda" })).toBeChecked();
    // Sin la primera, la grande pasa a ser la 1 en el aviso y en su miniatura ({{img:1}}).
    await user.click(within(again).getByRole("checkbox", { name: "Captura 1 · Primera" }));
    expect(await within(again).findByText("La captura 1 pesa más de 10 MB; GitHub podría rechazarla.")).toBeInTheDocument();
    expect(within(again).getByRole("checkbox", { name: "Captura 1 · Segunda" })).toBeChecked();
    expect(within(again).getByRole("checkbox", { name: "Primera · no se incluye" })).not.toBeChecked();
  });

  const STALE = "Cambiaste las capturas: regenera la descripción para que coincidan.";

  it("si la descripción no se editó, cambiar las capturas la rearma sola", async () => {
    const { user } = await openReport();
    const dialog = await screen.findByRole("dialog", { name: "Reportar en GitHub" });
    await waitFor(() => expect(within(dialog).getByLabelText("Repositorio")).toHaveValue(TESTING));
    const body = () => (within(dialog).getByLabelText("Descripción") as HTMLTextAreaElement).value;
    await waitFor(() => expect(body()).toContain("{{img:2}}"));
    await user.click(within(dialog).getByRole("checkbox", { name: "Captura 1 · Primera" }));
    expect(body()).not.toContain("{{img:2}}");
    expect(body()).toContain("**1.** Segunda\n{{img:1}}");
    expect(within(dialog).queryByText(STALE)).not.toBeInTheDocument();
  });

  it("si la descripción se editó, avisa que las capturas cambiaron hasta regenerarla", async () => {
    const { user, repos } = await openReport();
    const create = vi.spyOn(repos.github, "createIssue");
    const dialog = await screen.findByRole("dialog", { name: "Reportar en GitHub" });
    await waitFor(() => expect(within(dialog).getByLabelText("Repositorio")).toHaveValue(TESTING));
    const body = () => (within(dialog).getByLabelText("Descripción") as HTMLTextAreaElement).value;
    await waitFor(() => expect(body()).toContain("{{img:2}}"));
    await user.type(within(dialog).getByLabelText("Descripción"), " Nota del tester");
    await user.click(within(dialog).getByRole("checkbox", { name: "Captura 1 · Primera" }));
    expect(within(dialog).getByText(STALE)).toBeInTheDocument();
    expect(body()).toContain("Nota del tester");
    expect(body()).toContain("{{img:2}}");
    // Volver a la selección original quita el aviso.
    await user.click(within(dialog).getByRole("checkbox", { name: "Primera · no se incluye" }));
    expect(within(dialog).queryByText(STALE)).not.toBeInTheDocument();
    await user.click(within(dialog).getByRole("checkbox", { name: "Captura 1 · Primera" }));
    expect(within(dialog).getByText(STALE)).toBeInTheDocument();
    await user.click(within(dialog).getByRole("button", { name: "Regenerar descripción" }));
    expect(within(dialog).queryByText(STALE)).not.toBeInTheDocument();
    expect(body()).not.toContain("{{img:2}}");
    expect(body()).not.toContain("Nota del tester");
    await user.click(within(dialog).getByRole("button", { name: "Crear issue" }));
    await waitFor(() => expect(create).toHaveBeenCalledTimes(1));
    expect(create.mock.calls[0][0].body).toContain("**1.** Segunda\n{{img:1}}");
  });

  it("la lista marca los casos con issues", async () => {
    await renderApp({
      setup: async (r) => {
        await seed(r, { links: (s) => [link(s.caseId)] });
      },
    });
    const list = screen.getByRole("list", { name: "Lista de casos" });
    expect(await within(list).findAllByLabelText("Tiene issues en GitHub")).toHaveLength(1);
    const item = within(list).getByText("Recibos").closest("li")!;
    expect(within(item).getByLabelText("Tiene issues en GitHub")).toBeInTheDocument();
  });
});
