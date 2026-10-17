import { act, fireEvent, screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { PNG_BYTES, renderApp, seedCases, seedProject } from "../test-utils";

async function openSettings(options: Parameters<typeof renderApp>[0] = {}) {
  const ctx = await renderApp(options);
  await ctx.user.click(screen.getByRole("button", { name: "Ajustes" }));
  const dialog = await screen.findByRole("dialog", { name: "Ajustes" });
  return { ...ctx, dialog };
}

const gotoTab = (user: Awaited<ReturnType<typeof renderApp>>["user"], name: string) => user.click(screen.getByRole("tab", { name }));
const press = (init: KeyboardEventInit) => fireEvent.keyDown(window, init);

describe("ajustes de la aplicación", () => {
  it("graba una combinación y la guarda", async () => {
    const { user, repos, dialog } = await openSettings();
    await gotoTab(user, "Atajos");
    await user.click(within(dialog).getByRole("button", { name: "Grabar «Mostrar/ocultar»" }));
    expect(within(dialog).getByText("Presiona la combinación…")).toBeInTheDocument();
    press({ ctrlKey: true, shiftKey: true, code: "F12", key: "F12" });
    expect(within(dialog).getByRole("button", { name: "Grabar «Mostrar/ocultar»" })).toHaveTextContent("Ctrl+Shift+F12");
    await user.click(within(dialog).getByRole("button", { name: "Guardar" }));
    expect(await screen.findByText("Ajustes guardados.")).toBeInTheDocument();
    expect(repos.settings.snapshot.settings.shortcuts.toggle).toBe("Ctrl+Shift+F12");
    expect(screen.queryByRole("dialog", { name: "Ajustes" })).not.toBeInTheDocument();
  });

  it("ignora los modificadores solos, Esc cancela y Retroceso restaura el valor por defecto", async () => {
    const { user, dialog } = await openSettings();
    await gotoTab(user, "Atajos");
    const rec = () => within(dialog).getByRole("button", { name: "Grabar «Capturar pantalla»" });
    await user.click(rec());
    press({ ctrlKey: true, code: "ControlLeft", key: "Control" });
    expect(rec()).toHaveTextContent("Presiona la combinación…");
    press({ key: "Escape", code: "Escape" });
    expect(screen.getByRole("dialog", { name: "Ajustes" })).toBeInTheDocument();
    expect(rec()).toHaveTextContent("Ctrl+Alt+S");
    await user.click(rec());
    press({ ctrlKey: true, altKey: true, code: "KeyP", key: "p" });
    expect(rec()).toHaveTextContent("Ctrl+Alt+P");
    await user.click(rec());
    press({ key: "Backspace", code: "Backspace" });
    expect(rec()).toHaveTextContent("Ctrl+Alt+S");
  });

  it("muestra el conflicto en su fila y no cierra", async () => {
    const { user, repos, dialog } = await openSettings();
    await gotoTab(user, "Atajos");
    await user.click(within(dialog).getByRole("button", { name: "Grabar «Capturar pantalla»" }));
    press({ ctrlKey: true, altKey: true, code: "KeyV", key: "v" });
    await user.click(within(dialog).getByRole("button", { name: "Guardar" }));
    const row = within(dialog).getByRole("group", { name: "Pegar evidencia" }); // el error queda en la fila de la segunda acción
    expect(await within(row).findByText("«Capturar pantalla» y «Pegar evidencia» usan la misma combinación (Ctrl+Alt+V).")).toBeInTheDocument();
    expect(repos.settings.saves).toBe(0);
    expect(screen.getByRole("dialog", { name: "Ajustes" })).toBeInTheDocument();
  });

  it("marca los atajos que no se registraron al arrancar", async () => {
    const { user, dialog } = await openSettings({
      setup: async (repos) => {
        repos.settings.failuresAtStart = [{ action: "paste", accelerator: "Ctrl+Alt+V" }];
      },
    });
    await gotoTab(user, "Atajos");
    expect(within(within(dialog).getByRole("group", { name: "Pegar evidencia" })).getByText("No registrado")).toBeInTheDocument();
    expect(within(within(dialog).getByRole("group", { name: "Capturar pantalla" })).queryByText("No registrado")).not.toBeInTheDocument();
  });

  it("elige la carpeta de exportación y la usa el diálogo de exportar", async () => {
    const { user, repos, dialog } = await openSettings({ tester: "Ana Pérez", setup: async (repos) => void (await seedCases(repos)) });
    repos.settings.pickedFolder = "C:\\Exports";
    await gotoTab(user, "Exportación");
    await user.click(within(dialog).getByRole("button", { name: "Elegir…" }));
    expect(within(dialog).getByLabelText("Carpeta por defecto")).toHaveValue("C:\\Exports");
    const width = within(dialog).getByLabelText("Ancho máximo de imagen (px)");
    await user.clear(width);
    await user.type(width, "1200");
    await user.click(within(dialog).getByRole("button", { name: "Guardar" }));
    await waitFor(() => expect(screen.queryByRole("dialog", { name: "Ajustes" })).not.toBeInTheDocument());
    expect(repos.settings.snapshot.settings.export.defaultFolder).toBe("C:\\Exports");

    await user.click(screen.getByRole("button", { name: /Exportar/ }));
    const exportDialog = await screen.findByRole("dialog", { name: "Exportar Excel" });
    await user.click(within(exportDialog).getByRole("button", { name: "Exportar Excel" }));
    await waitFor(() => expect(repos.exporter.calls).toHaveLength(1));
    expect(repos.exporter.calls[0].options.imageMaxWidth).toBe(1200);
  });

  it("Datos abre respaldos encima y Acerca de muestra la versión y los enlaces", async () => {
    const { user, repos, dialog } = await openSettings();
    await gotoTab(user, "Datos");
    expect(within(dialog).getByText(/com\.teams4soft\.qapture/)).toBeInTheDocument();
    await user.click(within(dialog).getByRole("button", { name: "Abrir carpeta de datos" }));
    await user.click(within(dialog).getByRole("button", { name: "Respaldos…" }));
    const backups = await screen.findByRole("dialog", { name: "Respaldos" });
    await user.click(within(backups).getByRole("button", { name: "Cerrar" }));
    expect(screen.getByRole("dialog", { name: "Ajustes" })).toBeInTheDocument();
    await gotoTab(user, "Acerca de");
    expect(within(dialog).getByText("Qapture 0.5.0")).toBeInTheDocument();
    await user.click(within(dialog).getByRole("button", { name: "Releases" }));
    await user.click(within(dialog).getByRole("button", { name: "Documentación" }));
    expect(repos.settings.calls).toEqual(["openDataFolder", "openLink:releases", "openLink:docs"]);
  });

  it("el nuevo caso precarga el Responsable con el tester del perfil", async () => {
    const { user } = await renderApp({ tester: "Ana Pérez", setup: async (repos) => void (await seedProject(repos)) });
    await user.click(await screen.findByRole("button", { name: /Nuevo caso/ }));
    const dialog = await screen.findByRole("dialog", { name: "Nuevo caso" });
    expect(within(dialog).getByLabelText("Responsable")).toHaveValue("Ana Pérez");
  });

  it("Esc oculta la ventana solo si la opción está activa y no hay diálogos", async () => {
    const { repos, user, store } = await renderApp({ setup: async (repos) => void (await seedProject(repos)) });
    press({ key: "Escape", code: "Escape" });
    expect(repos.settings.hidden).toBe(0);

    await act(async () => {
      repos.settings.snapshot.settings.system.escHides = true;
      await store.getState().loadSettings();
    });
    press({ key: "Escape", code: "Escape" });
    await waitFor(() => expect(repos.settings.hidden).toBe(1));

    await user.click(screen.getByRole("button", { name: "Ajustes" }));
    await screen.findByRole("dialog", { name: "Ajustes" });
    press({ key: "Escape", code: "Escape" });
    await waitFor(() => expect(screen.queryByRole("dialog", { name: "Ajustes" })).not.toBeInTheDocument());
    expect(repos.settings.hidden).toBe(1);
  });

  it("Nuevo caso con una evidencia en el editor no la descarta", async () => {
    const { repos, store } = await renderApp({ setup: async (repos) => void (await seedCases(repos)) });
    const caseId = store.getState().cases[0].id;
    const staged = await repos.evidence.stage(new Uint8Array(PNG_BYTES));
    act(() => store.getState().openEvidenceEditor({ imagePath: staged.path, caseId }));
    const editor = await screen.findByRole("dialog", { name: "Editor de evidencia" });

    act(() => repos.settings.emitNewCase());
    expect(store.getState().caseFormRequest).toEqual({ caseId: null });
    expect(screen.getByRole("dialog", { name: "Editor de evidencia" })).toBe(editor);
    expect(screen.queryByRole("dialog", { name: "Nuevo caso" })).not.toBeInTheDocument();
    expect(repos.evidence.stagedCount()).toBe(1);

    fireEvent.click(within(editor).getByRole("button", { name: "Cancelar" }));
    expect(await screen.findByRole("dialog", { name: "Nuevo caso" })).toBeInTheDocument();
    expect(repos.evidence.stagedCount()).toBe(0);
  });

it("editar un caso desde el formulario por caseFormRequest lo actualiza sin crear otro", async () => {
    const { user, store } = await renderApp({ setup: async (repos) => void (await seedCases(repos)) });
    await user.click(screen.getByRole("button", { name: /Editar/ }));
    const dialog = await screen.findByRole("dialog", { name: "Editar EJ-001" });
    const title = within(dialog).getByLabelText(/Caso de prueba/);
    await user.clear(title);
    await user.type(title, "Recibos v2");
    await user.click(within(dialog).getByRole("button", { name: "Guardar caso" }));
    await waitFor(() => expect(screen.queryByRole("dialog", { name: "Editar EJ-001" })).not.toBeInTheDocument());
    const cases = store.getState().cases;
    expect(cases).toHaveLength(2);
    expect(cases.find((c) => c.code === "EJ-001")?.title).toBe("Recibos v2");
  });

  it("Nuevo caso con un formulario de edición abierto conserva la edición en curso", async () => {
    const { user, repos, store } = await renderApp({ setup: async (repos) => void (await seedCases(repos)) });
    await user.click(screen.getByRole("button", { name: /Editar/ }));
    const dialog = await screen.findByRole("dialog", { name: "Editar EJ-001" });
    const title = within(dialog).getByLabelText(/Caso de prueba/);
    await user.type(title, " editado");

    act(() => repos.settings.emitNewCase());
    expect(screen.getByRole("dialog", { name: "Editar EJ-001" })).toBeInTheDocument();
    expect(screen.queryByRole("dialog", { name: "Nuevo caso" })).not.toBeInTheDocument();
    expect(within(screen.getByRole("dialog", { name: "Editar EJ-001" })).getByLabelText(/Caso de prueba/)).toHaveValue("Recibos editado");
    await user.click(screen.getByRole("button", { name: "Guardar caso" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
    expect(store.getState().cases).toHaveLength(2);
  });
});
