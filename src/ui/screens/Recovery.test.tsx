import { screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { MemoryRepositories } from "../../infrastructure/memory/repositories";
import { renderApp } from "../test-utils";

const damaged = async (r: MemoryRepositories) => {
  r.data.status = { recovery: true, detail: "base dañada: database disk image is malformed" };
};

describe("pantalla de recuperación", () => {
  it("reemplaza la app y ofrece los respaldos", async () => {
    await renderApp({ setup: damaged });
    expect(screen.getByRole("heading", { name: "Qapture no pudo abrir tus datos" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Nueva matriz/ })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Datos/ })).not.toBeInTheDocument();
    expect(await screen.findAllByRole("button", { name: "Restaurar" })).toHaveLength(2);
    expect(screen.getByText("base dañada: database disk image is malformed")).toBeInTheDocument();
  });

  it("restaura un respaldo y vuelve a la app", async () => {
    const { user, repos } = await renderApp({ setup: damaged });
    const [newest] = await screen.findAllByRole("button", { name: "Restaurar" });
    await user.click(newest);
    await user.click(within(screen.getByRole("dialog", { name: "Restaurar respaldo" })).getByRole("button", { name: "Restaurar" }));
    expect(await screen.findByText("Base recuperada desde el respaldo.")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Datos/ })).toBeInTheDocument(); // la barra superior volvió
    expect(repos.data.calls).toEqual(["recover:qapture-diario-20261016.db"]);
  });

  it("empezar vacía e importar un paquete abre la importación", async () => {
    const { user, repos } = await renderApp({ setup: damaged });
    await user.click(screen.getByRole("button", { name: "Empezar vacía e importar un paquete" }));
    await user.click(within(screen.getByRole("dialog", { name: "Empezar con una base vacía" })).getByRole("button", { name: "Empezar vacía" }));
    expect(await screen.findByText("Importado: 1 proyecto, 3 casos y 4 capturas.")).toBeInTheDocument();
    expect(repos.data.calls).toEqual(["recover:vacía", "importPackage"]);
    expect(screen.getByRole("combobox", { name: "Proyecto" })).toHaveDisplayValue("Recibos (importado)");
  });
});
