import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => invoke(...args) }));

const { tauriOverlayApi } = await import("./capture");

describe("adaptador Tauri del overlay", () => {
  beforeEach(() => { invoke.mockReset(); });

  it("pide su imagen congelada como bytes", async () => {
    invoke.mockResolvedValue(new Uint8Array([1, 2, 3]).buffer);
    await expect(tauriOverlayApi().loadFrame()).resolves.toEqual(new Uint8Array([1, 2, 3]));
    expect(invoke).toHaveBeenCalledWith("capture_frame", {});
  });

  it("envía la selección o null para el monitor completo", async () => {
    invoke.mockResolvedValue(null);
    const api = tauriOverlayApi();
    const selection = { x: 1, y: 2, width: 3, height: 4, viewportWidth: 5, viewportHeight: 6 };
    await api.finish(selection);
    await api.finish(null);
    await api.ready();
    await api.cancel();
    expect(invoke.mock.calls).toEqual([
      ["finish_capture", { selection }],
      ["finish_capture", { selection: null }],
      ["overlay_ready", {}],
      ["cancel_capture", {}],
    ]);
  });

  it("traduce el error de Rust para mostrarlo en el overlay", async () => {
    invoke.mockRejectedValue({ code: "validation", message: "La selección es demasiado pequeña.", detail: "x" });
    await expect(tauriOverlayApi().finish(null)).rejects.toMatchObject({ message: "La selección es demasiado pequeña." });
  });
});
