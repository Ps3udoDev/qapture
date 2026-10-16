import { describe, expect, it, vi } from "vitest";
import { createMemoryRepositories } from "./repositories";

describe("captura en memoria", () => {
  it("en modo image deja la captura en staging y emite el evento", async () => {
    const repos = createMemoryRepositories();
    const handler = vi.fn();
    const off = await repos.capture.onCaptured(handler);
    await repos.capture.start();
    expect(repos.capture.starts).toBe(1);
    expect(repos.evidence.stagedCount()).toBe(1);
    const [path] = handler.mock.calls[0];
    await expect(repos.evidence.readImage(path)).resolves.toBeInstanceOf(Uint8Array);
    off();
    expect(repos.capture.listeners()).toBe(0);
  });

  it("cancel no crea nada y error falla con el código capture", async () => {
    const repos = createMemoryRepositories();
    const handler = vi.fn();
    await repos.capture.onCaptured(handler);
    repos.capture.mode = "cancel";
    await repos.capture.start();
    repos.capture.mode = "error";
    await expect(repos.capture.start()).rejects.toMatchObject({
      code: "capture",
      message: "No se pudo capturar la pantalla. Inténtalo nuevamente.",
    });
    expect(handler).not.toHaveBeenCalled();
    expect(repos.evidence.stagedCount()).toBe(0);
  });
});
