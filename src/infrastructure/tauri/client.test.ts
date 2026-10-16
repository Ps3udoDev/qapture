import { beforeEach, describe, expect, it, vi } from "vitest";
import { z } from "zod";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => invoke(...args) }));

const { call, callBytes } = await import("./client");

describe("call", () => {
  beforeEach(() => {
    invoke.mockReset();
  });

  it("devuelve datos validados", async () => {
    invoke.mockResolvedValue({ ok: 1 });
    await expect(call("cmd", { a: 1 }, z.strictObject({ ok: z.number() }))).resolves.toEqual({ ok: 1 });
    expect(invoke).toHaveBeenCalledWith("cmd", { a: 1 });
  });

  it("traduce el error serializado de Rust", async () => {
    invoke.mockRejectedValue({ code: "validation", message: "«Entorno» es obligatorio.", detail: "x" });
    await expect(call("cmd", {}, z.null())).rejects.toMatchObject({ code: "validation", message: "«Entorno» es obligatorio." });
  });

  it("marca como internal un rechazo desconocido", async () => {
    invoke.mockRejectedValue("boom");
    await expect(call("cmd", {}, z.null())).rejects.toMatchObject({ code: "internal" });
  });

  it("marca como contract una respuesta con forma inesperada", async () => {
    invoke.mockResolvedValue({ ok: "no" });
    await expect(call("cmd", {}, z.strictObject({ ok: z.number() }))).rejects.toMatchObject({ code: "contract" });
  });

  it("pasa cuerpos binarios tal cual", async () => {
    invoke.mockResolvedValue({ ok: 1 });
    const body = new Uint8Array([1, 2]);
    await call("cmd", body, z.strictObject({ ok: z.number() }));
    expect(invoke).toHaveBeenCalledWith("cmd", body);
  });

  it("callBytes devuelve Uint8Array y rechaza respuestas no binarias", async () => {
    invoke.mockResolvedValue(new Uint8Array([7, 7]).buffer);
    await expect(callBytes("img", { path: "x" })).resolves.toEqual(new Uint8Array([7, 7]));
    invoke.mockResolvedValue({ no: "binario" });
    await expect(callBytes("img", {})).rejects.toMatchObject({ code: "contract" });
  });
});
