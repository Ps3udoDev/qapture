import { screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { emptyDocVersion } from "../../domain/doc-versions";
import type { MemoryRepositories } from "../../infrastructure/memory/repositories";
import type { Repositories } from "../../ports/repositories";
import { renderApp } from "../test-utils";
import { seedCases } from "./Evidence.test";

const FILE = "T4S_ACME_Recibos_Evidencias_20261016_v1.0.xlsx";

async function openExport(options: { tester?: string; setup?: (repos: Repositories) => Promise<void> } = {}) {
  const ctx = await renderApp({ tester: options.tester ?? "Ana Pérez", setup: options.setup ?? (async (repos) => void (await seedCases(repos))) });
  await ctx.user.click(screen.getByRole("button", { name: /Exportar/ }));
  const repos = ctx.repos as MemoryRepositories;
  return { ...ctx, repos, dialog: screen.getByRole("dialog", { name: "Exportar Excel" }) };
}

describe("exportar a Excel", () => {
  it("registra la primera versión, exporta y ofrece abrir el archivo y la carpeta", async () => {
    const { user, repos, dialog, store } = await openExport();
    expect(within(dialog).getByLabelText(/Tester/)).toHaveValue("Ana Pérez");
    expect(within(dialog).getByLabelText(/Versión del documento/)).toHaveDisplayValue("Nueva versión…");
    expect(within(dialog).getByLabelText(/Número de versión/)).toHaveValue("1.0");
    expect(within(dialog).getByLabelText("Cambios")).toHaveValue("Versión inicial");
    await user.click(within(dialog).getByRole("button", { name: "Exportar Excel" }));

    await waitFor(() => expect(screen.queryByRole("dialog", { name: "Exportar Excel" })).not.toBeInTheDocument());
    const [call] = repos.exporter.calls;
    const [version] = store.getState().docVersions;
    expect(version).toMatchObject({ version: "1.0", author: "Ana Pérez", changes: "Versión inicial" });
    expect(call).toMatchObject({ kind: "project", options: { tester: "Ana Pérez", docVersionId: version.id, includeEvidence: true, includeSummary: true, imageMaxWidth: 940 } });
    expect(screen.getByText(`Excel guardado: ${FILE}`)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Abrir archivo" }));
    await waitFor(() => expect(repos.exporter.opened).toEqual([`C:\\Users\\ana\\Documents\\${FILE}`]));
    expect(screen.queryByText(`Excel guardado: ${FILE}`)).not.toBeInTheDocument();
  });

  it("usa la última versión registrada y deja quitar hojas", async () => {
    const { user, repos, dialog } = await openExport({
      setup: async (repos) => {
        const { p } = await seedCases(repos);
        await repos.docVersions.create(p.id, { ...emptyDocVersion("2026-10-01", "Ana"), version: "1.0" });
        await repos.docVersions.create(p.id, { ...emptyDocVersion("2026-10-15", "Ana"), version: "1.1" });
      },
    });
    expect(within(dialog).getByLabelText(/Versión del documento/)).toHaveDisplayValue("1.1 · 15/10/2026");
    await user.click(within(dialog).getByLabelText("Incluir hoja Resumen"));
    await user.click(within(dialog).getByRole("button", { name: "Exportar Excel" }));
    await waitFor(() => expect(repos.exporter.calls).toHaveLength(1));
    expect(repos.exporter.calls[0].options).toMatchObject({ includeEvidence: true, includeSummary: false });
  });

  it("con el archivo abierto en Excel muestra el mensaje y no cierra el diálogo", async () => {
    const { user, repos, dialog } = await openExport();
    repos.exporter.mode = "locked";
    await user.click(within(dialog).getByRole("button", { name: "Exportar Excel" }));
    expect(await within(dialog).findByRole("alert")).toHaveTextContent("Cierra el archivo en Excel y vuelve a intentar.");
    expect(screen.getByRole("dialog", { name: "Exportar Excel" })).toBeInTheDocument();
  });

  it("si se cancela «Guardar como» queda abierto y reutiliza la versión ya registrada", async () => {
    const { user, repos, dialog, store } = await openExport();
    repos.exporter.mode = "cancel";
    await user.click(within(dialog).getByRole("button", { name: "Exportar Excel" }));
    await waitFor(() => expect(within(dialog).getByLabelText(/Versión del documento/)).toHaveDisplayValue(/^1\.0 · /));
    repos.exporter.mode = "save";
    await user.click(within(dialog).getByRole("button", { name: "Exportar Excel" }));
    await waitFor(() => expect(screen.queryByRole("dialog", { name: "Exportar Excel" })).not.toBeInTheDocument());
    expect(store.getState().docVersions).toHaveLength(1);
    expect(repos.exporter.calls[1].options.docVersionId).toBe(store.getState().docVersions[0].id);
  });

  it("la plantilla vacía no pide versión ni hojas", async () => {
    const { user, repos, dialog } = await openExport();
    await user.click(within(dialog).getByRole("radio", { name: /Plantilla vacía/ }));
    expect(within(dialog).queryByLabelText(/Versión del documento/)).not.toBeInTheDocument();
    expect(within(dialog).queryByLabelText("Incluir hoja Evidencias")).not.toBeInTheDocument();
    await user.click(within(dialog).getByRole("button", { name: "Exportar plantilla" }));
    await waitFor(() => expect(repos.exporter.calls).toHaveLength(1));
    expect(repos.exporter.calls[0]).toMatchObject({ kind: "template", options: { docVersionId: null } });
  });

  it("exige el tester y lo recuerda para la próxima vez", async () => {
    const { user, repos, dialog } = await openExport({ tester: "" });
    await user.click(within(dialog).getByRole("button", { name: "Exportar Excel" }));
    expect(within(dialog).getByText("«Tester» es obligatorio.")).toBeInTheDocument();
    expect(repos.exporter.calls).toHaveLength(0);
    await user.type(within(dialog).getByLabelText(/Tester/), "  Luis Gómez ");
    await user.click(within(dialog).getByRole("button", { name: "Exportar Excel" }));
    await waitFor(() => expect(screen.queryByRole("dialog", { name: "Exportar Excel" })).not.toBeInTheDocument());
    await user.click(screen.getByRole("button", { name: /Exportar/ }));
    expect(within(screen.getByRole("dialog", { name: "Exportar Excel" })).getByLabelText(/Tester/)).toHaveValue("Luis Gómez");
  });

  it("avisa si alguna imagen no se pudo incluir", async () => {
    const { user, repos, dialog } = await openExport();
    repos.exporter.missingImages = 2;
    await user.click(within(dialog).getByRole("button", { name: "Exportar Excel" }));
    expect(await screen.findByText("2 imágenes no se pudieron incluir: en la hoja Evidencias aparece un aviso en su lugar.")).toBeInTheDocument();
  });
});
