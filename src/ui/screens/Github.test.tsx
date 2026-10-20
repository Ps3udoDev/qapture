import { act, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { GhAccount } from "../../domain/github";
import { renderApp } from "../test-utils";

const account = (over: Partial<GhAccount> = {}): GhAccount => ({ login: "ana", name: "Ana Pérez", avatarDataUrl: null, connectedAt: "2026-10-19T10:00:00Z", needsReconnect: false, ...over });

async function openGithub(options: Parameters<typeof renderApp>[0] = {}) {
  const ctx = await renderApp(options);
  await ctx.user.click(screen.getByRole("button", { name: "Ajustes" }));
  await screen.findByRole("dialog", { name: "Ajustes" });
  await ctx.user.click(screen.getByRole("tab", { name: "GitHub" }));
  return ctx;
}

afterEach(() => vi.restoreAllMocks());

describe("pestaña GitHub de Ajustes", () => {
  it("«Conectar con GitHub» muestra el código y lo copia", async () => {
    const { user, repos } = await openGithub();
    expect(screen.getByText("Conecta tu cuenta para reportar casos como issues e importar issues como casos.")).toBeInTheDocument();
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", { value: { writeText }, configurable: true });
    await user.click(screen.getByRole("button", { name: "Conectar con GitHub" }));
    expect(await screen.findByText("WDJB-MJHT")).toBeInTheDocument();
    expect(screen.getByText("Ingresa este código en GitHub:")).toBeInTheDocument();
    expect(screen.getByText("El código vence en 15:00.")).toBeInTheDocument();
    expect(repos.github.starts).toBe(1);
    await user.click(screen.getByRole("button", { name: "Copiar código" }));
    expect(writeText).toHaveBeenCalledWith("WDJB-MJHT");
    await user.click(screen.getByRole("button", { name: "Abrir GitHub" }));
    expect(repos.settings.calls).toContain("openLink:githubDevice");
    await user.click(within(screen.getByRole("tabpanel")).getByRole("button", { name: "Cancelar" }));
    expect(screen.queryByText("WDJB-MJHT")).not.toBeInTheDocument();
    expect(repos.github.cancels).toBe(1);
  });

  it("al llegar gh:connected muestra @login", async () => {
    const { user, repos } = await openGithub();
    await user.click(screen.getByRole("button", { name: "Conectar con GitHub" }));
    await screen.findByText("WDJB-MJHT");
    act(() => repos.github.emitConnected(account()));
    expect(await screen.findByText("@ana")).toBeInTheDocument();
    expect(screen.getByText("Ana Pérez")).toBeInTheDocument();
    expect(screen.getByText("Conectado como @ana.")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Instalar o configurar Qapture en una cuenta u organización" }));
    await user.click(screen.getByRole("button", { name: "Revocar acceso en GitHub" }));
    expect(repos.settings.calls).toEqual(expect.arrayContaining(["openLink:githubInstall", "openLink:githubRevoke"]));
  });

  it("cuenta con needsReconnect muestra «Reconectar»", async () => {
    const { user, repos } = await openGithub({
      setup: async (r) => {
        r.github.accountValue = account({ needsReconnect: true });
      },
    });
    expect(await screen.findByText("La sesión de GitHub venció o fue revocada.")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Reconectar" }));
    expect(await screen.findByText("WDJB-MJHT")).toBeInTheDocument();
    expect(repos.github.starts).toBe(1);
  });

  it("«Desconectar» pide confirmación", async () => {
    const { user, repos } = await openGithub({
      setup: async (r) => {
        r.github.accountValue = account();
      },
    });
    await user.click(await screen.findByRole("button", { name: "Desconectar" }));
    const confirm = await screen.findByRole("dialog", { name: "Desconectar GitHub" });
    expect(within(confirm).getByText("Se borrará el acceso guardado en este equipo. Tus proyectos conservan los repos vinculados.")).toBeInTheDocument();
    await user.click(within(confirm).getByRole("button", { name: "Cancelar" }));
    expect(repos.github.accountValue).not.toBeNull();
    await user.click(screen.getByRole("button", { name: "Desconectar" }));
    await user.click(within(await screen.findByRole("dialog", { name: "Desconectar GitHub" })).getByRole("button", { name: "Desconectar" }));
    await waitFor(() => expect(repos.github.accountValue).toBeNull());
    expect(await screen.findByRole("button", { name: "Conectar con GitHub" })).toBeInTheDocument();
  });

  it("al agotarse la cuenta regresiva muestra de nuevo «Conectar con GitHub»", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    try {
      const { user, repos } = await openGithub();
      repos.github.deviceCode = { ...repos.github.deviceCode, expiresIn: 2 };
      await user.click(screen.getByRole("button", { name: "Conectar con GitHub" }));
      await screen.findByText("WDJB-MJHT");
      await act(async () => {
        await vi.advanceTimersByTimeAsync(3000);
      });
      expect(await screen.findByRole("button", { name: "Conectar con GitHub" })).toBeInTheDocument();
    } finally {
      vi.useRealTimers();
    }
  });
});
