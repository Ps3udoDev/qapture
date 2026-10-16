import { screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { renderApp, seedProject } from "../test-utils";

async function openSettings(options: Parameters<typeof renderApp>[0] = {}) {
  const ctx = await renderApp({ setup: options.setup ?? (async (repos) => void (await seedProject(repos))) });
  await ctx.user.click(screen.getByRole("button", { name: /Configurar/ }));
  return { ...ctx, dialog: screen.getByRole("dialog", { name: "Configurar proyecto" }) };
}

describe("configuración del proyecto", () => {
  it("edita los datos del proyecto", async () => {
    const { user, dialog } = await openSettings();
    const name = within(dialog).getByLabelText(/Nombre del proyecto/);
    await user.clear(name);
    await user.type(name, "Recibos v2");
    await user.click(within(dialog).getByRole("button", { name: "Guardar datos" }));
    expect(screen.getByRole("combobox", { name: "Proyecto" })).toHaveDisplayValue("ACME · Recibos v2");
  });

  it("agrega una columna de lista y aparece en el formulario de casos", async () => {
    const { user, dialog } = await openSettings();
    await user.click(within(dialog).getByRole("tab", { name: "Columnas" }));
    await user.click(within(dialog).getByRole("button", { name: /Agregar columna/ }));
    await user.type(within(dialog).getByLabelText("Nombre de la columna 12"), "Severidad");
    await user.selectOptions(within(dialog).getByLabelText("Tipo de Severidad"), "list");
    await user.type(within(dialog).getByLabelText("Opciones de Severidad"), "Alta{Enter}Media{Enter}Baja");
    await user.click(within(dialog).getByRole("button", { name: "Guardar columnas" }));
    await user.click(within(dialog).getByRole("button", { name: "Cerrar" }));

    await user.click(screen.getByRole("button", { name: /Nuevo caso/ }));
    const form = screen.getByRole("dialog", { name: "Nuevo caso" });
    const severidad = within(form).getByLabelText("Severidad");
    expect(within(severidad).getAllByRole("option").map((o) => o.textContent)).toEqual(["—", "Alta", "Media", "Baja"]);
  });

  it("muestra errores de columnas y no guarda", async () => {
    const { user, dialog } = await openSettings();
    await user.click(within(dialog).getByRole("tab", { name: "Columnas" }));
    await user.click(within(dialog).getByRole("button", { name: /Agregar columna/ }));
    await user.type(within(dialog).getByLabelText("Nombre de la columna 12"), "entorno");
    await user.click(within(dialog).getByRole("button", { name: "Guardar columnas" }));
    expect(within(dialog).getByText("La columna «entorno» está repetida.")).toBeInTheDocument();
  });

  it("archiva una columna con datos y la restaura", async () => {
    const { user, dialog } = await openSettings({
      setup: async (repos) => {
        const p = await seedProject(repos);
        const id = (key: string) => p.fields.find((f) => f.key === key)!.id;
        await repos.cases.create(p.id, { title: "Caso", status: "Aprobado", values: { [id("entorno")]: "QA", [id("modulo")]: "Cobranzas" } });
      },
    });
    await user.click(within(dialog).getByRole("tab", { name: "Columnas" }));
    await user.click(within(dialog).getByRole("button", { name: "Quitar Módulo" }));
    await user.click(within(dialog).getByRole("button", { name: "Quitar Menú" }));
    await user.click(within(dialog).getByRole("button", { name: "Guardar columnas" }));

    const archived = within(dialog).getByRole("region", { name: "Columnas archivadas" });
    expect(within(archived).getByText("Módulo")).toBeInTheDocument();
    expect(within(archived).queryByText("Menú")).not.toBeInTheDocument();

    await user.click(within(archived).getByRole("button", { name: "Restaurar Módulo" }));
    await user.click(within(dialog).getByRole("button", { name: "Guardar columnas" }));
    await user.click(within(dialog).getByRole("button", { name: "Cerrar" }));
    expect(within(screen.getByRole("region", { name: "Detalle del caso" })).getByText("Cobranzas")).toBeInTheDocument();
  });

  it("guarda las columnas como plantilla y elimina plantillas propias", async () => {
    const { user, dialog } = await openSettings();
    await user.click(within(dialog).getByRole("tab", { name: "Plantillas" }));
    await user.type(within(dialog).getByLabelText(/Nombre de la plantilla/), "ACME Cobranzas");
    await user.click(within(dialog).getByRole("button", { name: "Guardar como plantilla" }));
    expect(within(dialog).getByText("ACME Cobranzas")).toBeInTheDocument();
    expect(within(dialog).queryByRole("button", { name: "Eliminar plantilla Funcional" })).not.toBeInTheDocument();
    await user.click(within(dialog).getByRole("button", { name: "Eliminar plantilla ACME Cobranzas" }));
    expect(within(dialog).queryByText("ACME Cobranzas")).not.toBeInTheDocument();
  });

  it("elimina el proyecto solo tras escribir su nombre", async () => {
    const { user, dialog } = await openSettings();
    await user.click(within(dialog).getByRole("button", { name: "Eliminar proyecto" }));
    const confirm = screen.getByRole("dialog", { name: "Eliminar Recibos" });
    const action = within(confirm).getByRole("button", { name: "Eliminar proyecto" });
    expect(action).toBeDisabled();
    await user.type(within(confirm).getByLabelText(/Escribe «Recibos»/), "Recibos");
    await user.click(action);
    expect(screen.getByRole("heading", { name: "Crea tu primera matriz" })).toBeInTheDocument();
  });
});
