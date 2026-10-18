import { act, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { UpdateInfo } from "../../domain/update";
import { renderApp } from "../test-utils";

const info = (over: Partial<UpdateInfo> = {}): UpdateInfo => ({
  version: "1.0.1",
  currentVersion: "1.0.0",
  notes: "Correcciones.\n<b>no es html</b>",
  date: null,
  portable: false,
  ...over,
});

describe("actualizaciones", () => {
  it("Instalar y reiniciar llama al updater; Después deja el indicador en Ajustes", async () => {
    const { user, repos } = await renderApp();
    expect(screen.getByRole("button", { name: "Ajustes" })).toBeInTheDocument();
    act(() => repos.updater.emitAvailable(info()));
    const dialog = await screen.findByRole("dialog", { name: "Nueva versión disponible" });
    expect(within(dialog).getByText("Qapture 1.0.1 está disponible (tienes 1.0.0).")).toBeInTheDocument();
    // Las notas son texto plano: el HTML no se interpreta.
    expect(within(dialog).getByText(/<b>no es html<\/b>/)).toBeInTheDocument();
    expect(dialog.querySelector("b")).toBeNull();

    await user.click(within(dialog).getByRole("button", { name: "Después" }));
    expect(screen.queryByRole("dialog", { name: "Nueva versión disponible" })).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Ajustes (actualización disponible)" })).toBeInTheDocument();

    act(() => repos.updater.emitAvailable(info({ version: "1.0.2" })));
    const again = await screen.findByRole("dialog", { name: "Nueva versión disponible" });
    expect(screen.getByRole("button", { name: "Ajustes" })).toBeInTheDocument();
    await user.click(within(again).getByRole("button", { name: "Instalar y reiniciar" }));
    expect(repos.updater.installs).toBe(1);
  });

  it("muestra el progreso y deshabilita los botones mientras descarga", async () => {
    const { user, repos } = await renderApp();
    repos.updater.hold = true;
    act(() => repos.updater.emitAvailable(info()));
    const dialog = await screen.findByRole("dialog", { name: "Nueva versión disponible" });
    await user.click(within(dialog).getByRole("button", { name: "Instalar y reiniciar" }));
    act(() => repos.updater.emitProgress({ downloaded: 50, total: 200 }));
    expect(within(dialog).getByText("Descargando actualización… 25 %")).toBeInTheDocument();
    expect(within(dialog).getByRole("button", { name: "Instalar y reiniciar" })).toBeDisabled();
    expect(within(dialog).getByRole("button", { name: "Después" })).toBeDisabled();
    await act(async () => repos.updater.release());
  });

  it("en la versión portable ofrece Descargar en vez de instalar", async () => {
    const { user, repos } = await renderApp();
    act(() => repos.updater.emitAvailable(info({ portable: true })));
    const dialog = await screen.findByRole("dialog", { name: "Nueva versión disponible" });
    expect(within(dialog).getByText("Estás usando la versión portable: descarga el ZIP nuevo y reemplaza Qapture.exe. Tus datos en .\\data se conservan.")).toBeInTheDocument();
    expect(within(dialog).queryByRole("button", { name: "Instalar y reiniciar" })).not.toBeInTheDocument();
    await user.click(within(dialog).getByRole("button", { name: "Descargar" }));
    expect(repos.settings.calls).toContain("openLink:releases");
    expect(repos.updater.installs).toBe(0);
  });

  it("Acerca de busca actualizaciones y muestra el resultado", async () => {
    const { user, repos } = await renderApp();
    await user.click(screen.getByRole("button", { name: "Ajustes" }));
    const settings = await screen.findByRole("dialog", { name: "Ajustes" });
    await user.click(within(settings).getByRole("tab", { name: "Acerca de" }));

    repos.updater.next = null;
    await user.click(within(settings).getByRole("button", { name: "Buscar actualizaciones" }));
    expect(await screen.findByText(`Qapture está al día (versión ${repos.settings.snapshot.appVersion}).`)).toBeInTheDocument();

    repos.updater.next = info();
    await user.click(within(settings).getByRole("button", { name: "Buscar actualizaciones" }));
    expect(await within(settings).findByText("Versión 1.0.1 disponible")).toBeInTheDocument();
    expect(within(settings).getByRole("button", { name: "Instalar y reiniciar" })).toBeInTheDocument();
  });
});
