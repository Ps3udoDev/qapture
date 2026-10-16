import { fireEvent, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { renderApp, seedProject } from "../test-utils";

describe("AppShell", () => {
  it("invita a crear la primera matriz", async () => {
    await renderApp();
    expect(screen.getByRole("heading", { name: "Crea tu primera matriz" })).toBeInTheDocument();
    expect(screen.getByRole("img", { name: "Teams4Soft" })).toBeInTheDocument();
  });

  it("crea un proyecto validando nombre y fechas", async () => {
    const { user } = await renderApp();
    await user.click(within(screen.getByRole("main")).getByRole("button", { name: /Nueva matriz/ }));
    const dialog = screen.getByRole("dialog", { name: "Nueva matriz" });

    await user.click(within(dialog).getByRole("button", { name: "Crear matriz" }));
    expect(within(dialog).getByText("«Nombre del proyecto» es obligatorio.")).toBeInTheDocument();

    await user.type(within(dialog).getByLabelText(/Nombre del proyecto/), "Recibos GMX");
    fireEvent.change(within(dialog).getByLabelText("Fecha de inicio"), { target: { value: "2026-11-01" } });
    fireEvent.change(within(dialog).getByLabelText("Fecha de fin"), { target: { value: "2026-10-01" } });
    await user.click(within(dialog).getByRole("button", { name: "Crear matriz" }));
    expect(within(dialog).getByText("La fecha de fin no puede ser anterior a la de inicio.")).toBeInTheDocument();

    fireEvent.change(within(dialog).getByLabelText("Fecha de fin"), { target: { value: "2026-11-30" } });
    expect(within(dialog).getByRole("radio", { name: /Funcional/ })).toBeChecked();
    await user.click(within(dialog).getByRole("button", { name: "Crear matriz" }));

    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(screen.getByRole("combobox", { name: "Proyecto" })).toHaveDisplayValue("Recibos GMX");
    expect(await screen.findByText("Matriz «Recibos GMX» creada.")).toBeInTheDocument();
  });

  it("cambia de proyecto con el selector", async () => {
    const { user } = await renderApp({
      setup: async (repos) => {
        await seedProject(repos, "Uno");
        await seedProject(repos, "Dos");
      },
    });
    const select = screen.getByRole("combobox", { name: "Proyecto" });
    expect(select).toHaveDisplayValue("ACME · Dos");
    await user.selectOptions(select, screen.getByRole("option", { name: "ACME · Uno" }));
    expect(select).toHaveDisplayValue("ACME · Uno");
  });
});
