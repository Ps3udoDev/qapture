import { screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { renderApp, seedProject } from "../test-utils";

async function openVersions(tester = "Ana Pérez") {
  const ctx = await renderApp({ tester, setup: async (repos) => void (await seedProject(repos)) });
  await ctx.user.click(screen.getByRole("button", { name: /Configurar/ }));
  const dialog = screen.getByRole("dialog", { name: "Configurar proyecto" });
  await ctx.user.click(within(dialog).getByRole("tab", { name: "Versiones" }));
  return { ...ctx, dialog };
}

describe("control de versiones del documento", () => {
  it("registra, edita y elimina versiones", async () => {
    const { user, dialog } = await openVersions();
    expect(within(dialog).getByText(/Sin versiones registradas/)).toBeInTheDocument();
    expect(within(dialog).getByLabelText(/^Versión/)).toHaveValue("1.0");
    expect(within(dialog).getByLabelText(/Elaborado por/)).toHaveValue("Ana Pérez");
    await user.type(within(dialog).getByLabelText("Cambios"), "Versión inicial");
    await user.click(within(dialog).getByRole("button", { name: "Registrar versión" }));

    const history = within(dialog).getByRole("region", { name: "Historial de versiones" });
    expect(await within(history).findByText("Versión inicial")).toBeInTheDocument();
    await waitFor(() => expect(within(dialog).getByLabelText(/^Versión/)).toHaveValue("1.1"));

    await user.click(within(history).getByRole("button", { name: "Editar versión 1.0" }));
    const changes = within(dialog).getByLabelText("Cambios");
    await user.clear(changes);
    await user.type(changes, "Se agregan casos de Pólizas");
    await user.click(within(dialog).getByRole("button", { name: "Guardar versión" }));
    expect(await within(history).findByText("Se agregan casos de Pólizas")).toBeInTheDocument();

    await user.click(within(history).getByRole("button", { name: "Eliminar versión 1.0" }));
    expect(await within(dialog).findByText(/Sin versiones registradas/)).toBeInTheDocument();
  });

  it("muestra los errores de validación y de versión repetida", async () => {
    const { user, dialog } = await openVersions("");
    await user.click(within(dialog).getByRole("button", { name: "Registrar versión" }));
    expect(within(dialog).getByText("«Elaborado por» es obligatorio.")).toBeInTheDocument();
    await user.type(within(dialog).getByLabelText(/Elaborado por/), "Ana");
    await user.click(within(dialog).getByRole("button", { name: "Registrar versión" }));
    await waitFor(() => expect(within(dialog).getByLabelText(/^Versión/)).toHaveValue("1.1"));
    const version = within(dialog).getByLabelText(/^Versión/);
    await user.clear(version);
    await user.type(version, "1.0");
    await user.click(within(dialog).getByRole("button", { name: "Registrar versión" }));
    expect(await within(dialog).findByText("La versión «1.0» ya existe en este proyecto.")).toBeInTheDocument();
  });
});
