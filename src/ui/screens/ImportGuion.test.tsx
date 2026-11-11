import { screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { buildProfile, DEFAULT_STATUS_MAP, suggestTarget } from "../../domain/guion";
import type { MemoryRepositories } from "../../infrastructure/memory/repositories";
import { renderApp } from "../test-utils";

function profile(repos: MemoryRepositories) {
  const sheet = repos.guion.inspection.sheets[0];
  return buildProfile(sheet, 15, sheet.rows[14].map((h, i) => suggestTarget(h, sheet.rows.slice(15).map((r) => r[i] ?? ""))), DEFAULT_STATUS_MAP.import, sheet.cover);
}
async function open(app: Awaited<ReturnType<typeof renderApp>>) {
  await app.user.click(screen.getByRole("button", { name: /Datos/ }));
  await app.user.click(screen.getByRole("menuitem", { name: "Importar guion de pruebas…" }));
}
async function columns(app: Awaited<ReturnType<typeof renderApp>>) {
  await open(app);
  await app.user.click(await screen.findByRole("button", { name: "Siguiente" }));
}
describe("importar guion", () => {
  it("el menú Datos abre el asistente y cancelar el archivo no abre nada", async () => {
    const app = await renderApp();
    app.repos.guion.mode = "cancel";
    await open(app);
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(app.store.getState().guionWizard).toBeNull();
    app.repos.guion.mode = "ok";
    await open(app);
    expect(await screen.findByRole("dialog", { name: "Importar guion de pruebas" })).toBeInTheDocument();
    expect(screen.getByText("Archivo y hoja")).toBeInTheDocument();
  });
  it("recorre los tres pasos e importa", async () => {
    const app = await renderApp();
    await open(app);
    await app.user.selectOptions(await screen.findByLabelText("Hoja"), "Guión de Pruebas");
    await app.user.click(screen.getByRole("button", { name: "Siguiente" }));
    expect(screen.getByLabelText("Fila de encabezados")).toHaveValue(15);
    expect(screen.getByLabelText("Destino de ID")).toHaveValue("code");
    await app.user.click(screen.getByRole("button", { name: "Vista previa" }));
    expect(await screen.findByText("3 casos")).toBeInTheDocument();
    await app.user.clear(screen.getByLabelText("Nombre del proyecto"));
    expect(screen.getByRole("button", { name: "Importar" })).toBeDisabled();
    await app.user.type(screen.getByLabelText("Nombre del proyecto"), "Guion ejecutable");
    await app.user.click(screen.getByRole("button", { name: "Importar" }));
    await waitFor(() => expect(app.store.getState().project?.name).toBe("Guion ejecutable"));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(app.store.getState().cases).toHaveLength(3);
  });
  it("con códigos repetidos Importar queda deshabilitado y se ve el error", async () => {
    const app = await renderApp({ setup: async (r) => { r.guion.inspection.sheets[0].rows[16][2] = "CP01"; } });
    await columns(app);
    await app.user.click(screen.getByRole("button", { name: "Vista previa" }));
    expect(await screen.findByText("Hay códigos repetidos: CP01.")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Importar" })).toBeDisabled();
  });
  it("sin columna de código la vista previa lo dice", async () => {
    const app = await renderApp();
    await columns(app);
    await app.user.selectOptions(screen.getByLabelText("Destino de ID"), "none");
    await app.user.click(screen.getByRole("button", { name: "Vista previa" }));
    expect(await screen.findByText("Falta asignar la columna del código del caso.")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Importar" })).toBeDisabled();
  });
  it("un perfil sugerido salta al paso 3 y se puede borrar", async () => {
    const app = await renderApp({ setup: async (r) => {
      r.guion.profiles.push({ id: "profile-saved", name: "Perfil sintético", definition: profile(r), createdAt: "", updatedAt: "" });
    } });
    await open(app);
    expect(await screen.findByLabelText("Perfil")).toHaveValue("profile-saved");
    await app.user.click(screen.getByRole("button", { name: "Siguiente" }));
    expect(await screen.findByText("3 casos")).toBeInTheDocument();
    expect(screen.queryByLabelText("Fila de encabezados")).not.toBeInTheDocument();
    await app.user.click(screen.getByRole("button", { name: "Atrás" }));
    await app.user.click(screen.getByRole("button", { name: "Borrar perfil" }));
    const confirm = screen.getByRole("dialog", { name: "Borrar perfil" });
    await app.user.click(within(confirm).getByRole("button", { name: "Borrar" }));
    await waitFor(() => expect(app.repos.guion.profiles).toHaveLength(0));
    expect(screen.getByLabelText("Perfil")).toHaveValue("");
  });
  it("muestra el error de validación y conserva el asistente", async () => {
    const app = await renderApp();
    await columns(app);
    await app.user.click(screen.getByRole("button", { name: "Vista previa" }));
    await screen.findByText("3 casos");
    app.repos.guion.profiles.push({ id: "duplicate", name: "Repetido", definition: profile(app.repos), createdAt: "", updatedAt: "" });
    await app.user.click(screen.getByLabelText("Guardar como perfil"));
    await app.user.type(screen.getByLabelText("Nombre del perfil"), "Repetido");
    await app.user.click(screen.getByRole("button", { name: "Importar" }));
    expect(await screen.findByText("Ya existe un perfil llamado «Repetido».")).toBeInTheDocument();
    expect(screen.getByRole("dialog", { name: "Importar guion de pruebas" })).toBeInTheDocument();
    expect(app.store.getState().projects).toHaveLength(0);
  });
  it("permite elegir encabezados a mano, tipos, roles, estados y carátula", async () => {
    const app = await renderApp({ setup: async (r) => {
      r.guion.inspection.sheets[0].suggestedHeaderRow = null;
      r.guion.inspection.sheets[0].rows[15][10] = "Pendiente";
    } });
    const preview = vi.spyOn(app.repos.guion, "preview");
    await columns(app);
    expect(screen.getByLabelText("Fila de encabezados")).toHaveValue(null);
    expect(screen.getByText("No se encontró la fila de encabezados. Elígela a mano.")).toBeInTheDocument();
    await app.user.type(screen.getByLabelText("Fila de encabezados"), "15");
    expect(screen.getByLabelText("Tipo de Fecha de Prueba")).toHaveValue("date");
    expect(screen.getByLabelText("Rol de Fecha de Prueba")).toHaveValue("executionDate");
    await app.user.selectOptions(screen.getByLabelText("Tipo de Criticidad"), "list");
    await app.user.selectOptions(screen.getByLabelText("Rol de Condiciones"), "conditions");
    await app.user.selectOptions(screen.getByLabelText("Estado para Pendiente"), "Fallido");
    await app.user.click(screen.getByLabelText("Usar estas celdas"));
    await app.user.click(screen.getByRole("button", { name: "Vista previa" }));
    await screen.findByText("3 casos");
    const definition = preview.mock.calls[0][1];
    expect(definition.headerRow).toBe(15);
    expect(definition.cover).toEqual({ executionDate: null, responsible: null, observations: null });
    expect(definition.statusMap.import.pendiente).toBe("Fallido");
    expect(definition.columns.find((c) => c.column === "H")?.target).toMatchObject({ type: "list", options: ["Crítico", "Importante"] });
    expect(definition.columns.find((c) => c.column === "M")?.target).toMatchObject({ role: "conditions" });
  });
  it("elegir un perfil cambia a su hoja y salta a la vista previa", async () => {
    const app = await renderApp({ setup: async (r) => {
      const second = structuredClone(r.guion.inspection.sheets[0]);
      second.name = "Segunda hoja";
      r.guion.inspection.sheets.push(second);
      const definition = profile(r);
      definition.sheet = second.name;
      definition.columns[0].header = "Encabezado de otro guion"; // No sugerido automáticamente.
      r.guion.profiles.push({ id: "second", name: "Perfil segunda hoja", definition, createdAt: "", updatedAt: "" });
    } });
    const preview = vi.spyOn(app.repos.guion, "preview");
    await open(app);
    await app.user.selectOptions(await screen.findByLabelText("Perfil"), "second");
    expect(await screen.findByText("3 casos")).toBeInTheDocument();
    expect(preview.mock.calls[0][1].sheet).toBe("Segunda hoja");
    await app.user.click(screen.getByRole("button", { name: "Atrás" }));
    expect(screen.getByLabelText("Hoja")).toHaveValue("Segunda hoja");
    await app.user.click(screen.getByRole("button", { name: "Borrar perfil" }));
    await app.user.keyboard("{Escape}");
    expect(screen.queryByRole("dialog", { name: "Borrar perfil" })).not.toBeInTheDocument();
    expect(screen.getByRole("dialog", { name: "Importar guion de pruebas" })).toBeInTheDocument();
    expect(app.repos.guion.profiles).toHaveLength(1);
  });
  it("bloquea navegación y cierre mientras se solicita la vista previa", async () => {
    const app = await renderApp();
    const original = app.repos.guion.preview.bind(app.repos.guion);
    let release!: () => void;
    const pending = new Promise<void>((resolve) => { release = resolve; });
    vi.spyOn(app.repos.guion, "preview").mockImplementation(async (...args) => { await pending; return original(...args); });
    await columns(app);
    await app.user.click(screen.getByRole("button", { name: "Vista previa" }));
    expect(screen.getByRole("button", { name: "Vista previa" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Atrás" })).toBeDisabled();
    await app.user.keyboard("{Escape}");
    expect(screen.getByRole("dialog", { name: "Importar guion de pruebas" })).toBeInTheDocument();
    release();
    expect(await screen.findByText("3 casos")).toBeInTheDocument();
  });
});
