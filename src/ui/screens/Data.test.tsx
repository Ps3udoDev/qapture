import { screen, waitFor, within } from "@testing-library/react";
import type { UserEvent } from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { renderApp, seedProject } from "../test-utils";

async function openMenu(user: UserEvent) {
  await user.click(screen.getByRole("button", { name: /Datos/ }));
  return screen.getByRole("menu", { name: "Datos" });
}

async function choose(user: UserEvent, item: string) {
  await user.click(within(await openMenu(user)).getByRole("menuitem", { name: item }));
}

describe("menú Datos", () => {
  it("importa la Bitácora v1, abre el proyecto importado y cierra el menú", async () => {
    const { user } = await renderApp();
    await choose(user, "Importar Bitácora v1…");
    expect(await screen.findByText("Importado «Aplicación de recibos»: 1 caso y 2 capturas.")).toBeInTheDocument();
    expect(screen.getByRole("combobox", { name: "Proyecto" })).toHaveDisplayValue("Aplicación de recibos");
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
  });

  it("muestra los problemas de un archivo rechazado", async () => {
    const { user, store } = await renderApp({ setup: async (r) => void (r.data.mode = "invalid") });
    await choose(user, "Importar Bitácora v1…");
    const dialog = await screen.findByRole("dialog", { name: "No se pudo importar" });
    expect(within(dialog).getByText("No se importó nada. Revisa estos problemas:")).toBeInTheDocument();
    expect(within(dialog).getAllByRole("listitem")).toHaveLength(2);
    await user.click(within(dialog).getByRole("button", { name: "Entendido" }));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(store.getState().projects).toEqual([]);
  });

  it("exportar paquete solo se habilita con un proyecto abierto", async () => {
    const { user } = await renderApp();
    expect(within(await openMenu(user)).getByRole("menuitem", { name: "Exportar paquete .qapture…" })).toBeDisabled();
  });

  it("exporta el paquete y ofrece abrir la carpeta", async () => {
    const { user, repos } = await renderApp({ setup: async (r) => void (await seedProject(r)) });
    await choose(user, "Exportar paquete .qapture…");
    expect(await screen.findByText("Paquete guardado: T4S_ACME_Recibos_20261016.qapture")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Abrir carpeta" }));
    await waitFor(() => expect(repos.exporter.revealed).toHaveLength(1));
  });

  it("el menú Datos se maneja con el teclado", async () => {
    const { user } = await renderApp();
    const menu = await openMenu(user);
    const item = (name: string) => within(menu).getByRole("menuitem", { name });
    expect(item("Importar Bitácora v1…")).toHaveFocus();
    await user.keyboard("{ArrowDown}");
    expect(item("Importar paquete .qapture…")).toHaveFocus();
    await user.keyboard("{ArrowUp}{ArrowUp}");
    expect(item("Respaldos…")).toHaveFocus(); // circular, salta el ítem deshabilitado
    await user.keyboard("{Home}");
    expect(item("Importar Bitácora v1…")).toHaveFocus();
    await user.keyboard("{End}");
    expect(item("Respaldos…")).toHaveFocus();
    await user.keyboard("{Escape}");
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Datos/ })).toHaveFocus();
  });

  it("Escape y un clic fuera cierran el menú", async () => {
    const { user } = await renderApp();
    await openMenu(user);
    await user.keyboard("{Escape}");
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    await openMenu(user);
    await user.click(document.body);
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
  });
});

describe("respaldos", () => {
  it("lista, crea y restaura con confirmación", async () => {
    const { user, repos } = await renderApp({ setup: async (r) => void (await seedProject(r)) });
    await choose(user, "Respaldos…");
    const dialog = await screen.findByRole("dialog", { name: "Respaldos" });
    await waitFor(() => expect(within(dialog).getAllByRole("row")).toHaveLength(3)); // encabezado + 2
    expect(within(dialog).getAllByText("Diario")).toHaveLength(2);

    await user.click(within(dialog).getByRole("button", { name: /Respaldar ahora/ }));
    expect(await screen.findByText("Respaldo creado: qapture-manual-20261016-100001.db")).toBeInTheDocument();
    await waitFor(() => expect(within(dialog).getAllByRole("row")).toHaveLength(4));

    const row = within(dialog).getByRole("row", { name: /15\/10\/2026 09:00/ });
    await user.click(within(row).getByRole("button", { name: "Restaurar" }));
    const confirm = screen.getByRole("dialog", { name: "Restaurar respaldo" });
    expect(within(confirm).getByText(/volverá al estado del 15\/10\/2026 09:00/)).toBeInTheDocument();
    await user.click(within(confirm).getByRole("button", { name: "Restaurar" }));

    expect(await screen.findByText("Se restauró el respaldo del 15/10/2026 09:00.")).toBeInTheDocument();
    expect(repos.data.calls).toContain("restore:qapture-diario-20261015.db");
    expect(screen.queryByRole("dialog", { name: "Respaldos" })).not.toBeInTheDocument();
  });

  it("cancelar la confirmación no restaura", async () => {
    const { user, repos } = await renderApp();
    await choose(user, "Respaldos…");
    const dialog = await screen.findByRole("dialog", { name: "Respaldos" });
    const [first] = await within(dialog).findAllByRole("button", { name: "Restaurar" });
    await user.click(first);
    await user.click(within(screen.getByRole("dialog", { name: "Restaurar respaldo" })).getByRole("button", { name: "Cancelar" }));
    expect(repos.data.calls).toEqual([]);
    expect(screen.getByRole("dialog", { name: "Respaldos" })).toBeInTheDocument();
  });
});
