import { screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { renderApp, seedProject } from "../test-utils";

async function openWithProject() {
  return renderApp({ setup: async (repos) => void (await seedProject(repos)) });
}

async function createCase(user: Awaited<ReturnType<typeof renderApp>>["user"], title: string, entorno: string, extra: Record<string, string> = {}) {
  await user.click(screen.getByRole("button", { name: /Nuevo caso/ }));
  const dialog = screen.getByRole("dialog", { name: "Nuevo caso" });
  await user.type(within(dialog).getByLabelText(/Caso de prueba/), title);
  await user.type(within(dialog).getByLabelText(/^Entorno/), entorno);
  for (const [label, value] of Object.entries(extra)) await user.type(within(dialog).getByLabelText(label), value);
  await user.click(within(dialog).getByRole("button", { name: "Guardar caso" }));
}

describe("casos", () => {
  it("exige los obligatorios y crea EJ-001", async () => {
    const { user } = await openWithProject();
    await user.click(screen.getByRole("button", { name: /Nuevo caso/ }));
    const dialog = screen.getByRole("dialog", { name: "Nuevo caso" });
    await user.click(within(dialog).getByRole("button", { name: "Guardar caso" }));
    expect(within(dialog).getByText("«Entorno» es obligatorio.")).toBeInTheDocument();
    expect(within(dialog).getByText("«Caso de prueba» es obligatorio.")).toBeInTheDocument();
    await user.click(within(dialog).getByRole("button", { name: "Cancelar" }));

    await createCase(user, "Aplicación de recibos", "Máquina Virtual", { "Datos de entrada": "Recibo 1{Enter}Sucursal 2" });
    const list = screen.getByRole("list", { name: "Lista de casos" });
    expect(within(list).getByText("EJ-001")).toBeInTheDocument();
    const detail = screen.getByRole("region", { name: "Detalle del caso" });
    expect(within(detail).getByRole("heading", { name: "Aplicación de recibos" })).toBeInTheDocument();
    expect(within(detail).getByText("Recibo 1\nSucursal 2", { normalizer: (s) => s })).toBeInTheDocument();
  });

  it("duplica limpiando resultados y edita", async () => {
    const { user } = await openWithProject();
    await createCase(user, "Recibos", "QA", { "Resultado obtenido": "OK", Observaciones: "nada" });
    await user.click(screen.getByRole("button", { name: /Duplicar/ }));

    const detail = screen.getByRole("region", { name: "Detalle del caso" });
    expect(within(detail).getByText("EJ-002")).toBeInTheDocument();
    expect(within(detail).getByText("No ejecutado")).toBeInTheDocument();
    expect(within(detail).queryByText("OK")).not.toBeInTheDocument();
    expect(within(detail).getByText("QA")).toBeInTheDocument();

    await user.click(within(detail).getByRole("button", { name: /Editar/ }));
    const dialog = screen.getByRole("dialog", { name: "Editar EJ-002" });
    await user.selectOptions(within(dialog).getByLabelText("Estado"), "Fallido");
    await user.click(within(dialog).getByRole("button", { name: "Guardar caso" }));
    expect(within(screen.getByRole("region", { name: "Detalle del caso" })).getByText("Fallido")).toBeInTheDocument();
  });

  it("busca sin acentos y filtra por estado", async () => {
    const { user } = await openWithProject();
    await createCase(user, "Aplicación de recibos", "QA");
    await createCase(user, "Emisión de pólizas", "Producción");
    const list = screen.getByRole("list", { name: "Lista de casos" });

    await user.type(screen.getByRole("searchbox", { name: "Buscar casos" }), "RECÍB");
    expect(within(list).getAllByRole("listitem")).toHaveLength(1);
    expect(screen.getByTestId("case-count")).toHaveTextContent("1 / 2");

    await user.clear(screen.getByRole("searchbox", { name: "Buscar casos" }));
    await user.selectOptions(screen.getByRole("combobox", { name: "Filtrar por estado" }), "Aprobado");
    expect(within(list).getByText("Ningún caso coincide con el filtro.")).toBeInTheDocument();
  });

  it("elimina con confirmación y no reutiliza el código", async () => {
    const { user } = await openWithProject();
    await createCase(user, "Uno", "QA");
    await createCase(user, "Dos", "QA");
    await user.click(screen.getByRole("button", { name: "Eliminar caso" }));
    await user.click(within(screen.getByRole("dialog", { name: "Eliminar EJ-002" })).getByRole("button", { name: "Eliminar caso" }));
    await createCase(user, "Tres", "QA");
    expect(within(screen.getByRole("list", { name: "Lista de casos" })).getByText("EJ-003")).toBeInTheDocument();
  });
});
