import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { AppError } from "../../ports/errors";
import type { OverlayApi } from "../../ports/capture";
import { OverlayApp } from "./OverlayApp";

const HINT = "Arrastra para seleccionar · Enter: pantalla completa · Esc: cancelar";

function fakeApi(overrides: Partial<OverlayApi> = {}) {
  return {
    loadFrame: vi.fn(async () => new Uint8Array([0x89, 0x50, 0x4e, 0x47])),
    ready: vi.fn(async () => {}),
    finish: vi.fn(async () => {}),
    cancel: vi.fn(async () => {}),
    ...overrides,
  } satisfies OverlayApi;
}

async function renderOverlay(api = fakeApi()) {
  render(<OverlayApp api={api} />);
  const frame = await screen.findByTestId("frame");
  return { api, frame, surface: screen.getByTestId("overlay-surface") };
}

const drag = (surface: HTMLElement, from: [number, number], to: [number, number]) => {
  fireEvent.pointerDown(surface, { button: 0, clientX: from[0], clientY: from[1], pointerId: 1 });
  fireEvent.pointerMove(surface, { clientX: to[0], clientY: to[1], pointerId: 1 });
  fireEvent.pointerUp(surface, { clientX: to[0], clientY: to[1], pointerId: 1 });
};

describe("overlay de captura", () => {
  beforeEach(() => {
    Object.assign(URL, { createObjectURL: vi.fn(() => "blob:frame"), revokeObjectURL: vi.fn() });
  });

  it("muestra la imagen congelada con la pista y avisa cuando se pintó", async () => {
    const { api, frame } = await renderOverlay();
    expect(frame).toHaveAttribute("src", "blob:frame");
    expect(screen.getByRole("status")).toHaveTextContent(HINT);
    fireEvent.load(frame);
    expect(api.ready).toHaveBeenCalledTimes(1);
  });

  it("arrastrar muestra la medida y envía la selección con el tamaño de la ventana", async () => {
    const { api, surface } = await renderOverlay();
    fireEvent.pointerDown(surface, { button: 0, clientX: 100, clientY: 50, pointerId: 1 });
    fireEvent.pointerMove(surface, { clientX: 300, clientY: 150, pointerId: 1 });
    expect(screen.getByTestId("selection")).toHaveTextContent("200 × 100");
    fireEvent.pointerUp(surface, { clientX: 300, clientY: 150, pointerId: 1 });
    expect(api.finish).toHaveBeenCalledWith({
      x: 100,
      y: 50,
      width: 200,
      height: 100,
      viewportWidth: window.innerWidth,
      viewportHeight: window.innerHeight,
    });
  });

  it("un clic sin arrastre no captura", async () => {
    const { api, surface } = await renderOverlay();
    drag(surface, [100, 100], [102, 101]);
    expect(api.finish).not.toHaveBeenCalled();
    expect(screen.queryByTestId("selection")).not.toBeInTheDocument();
  });

  it("Enter captura el monitor completo y Esc cancela, una sola vez", async () => {
    const { api } = await renderOverlay();
    fireEvent.keyDown(window, { key: "Enter" });
    fireEvent.keyDown(window, { key: "Enter" });
    expect(api.finish).toHaveBeenCalledTimes(1);
    expect(api.finish).toHaveBeenCalledWith(null);

    const other = fakeApi();
    render(<OverlayApp api={other} />);
    fireEvent.keyDown(window, { key: "Escape" });
    expect(other.cancel).toHaveBeenCalledTimes(1);
  });

  it("si Rust rechaza la selección muestra el motivo y permite reintentar", async () => {
    const finish = vi
      .fn<OverlayApi["finish"]>()
      .mockRejectedValueOnce(new AppError("validation", "La selección es demasiado pequeña."))
      .mockResolvedValue(undefined);
    const { surface } = await renderOverlay(fakeApi({ finish }));
    drag(surface, [10, 10], [20, 20]);
    expect(await screen.findByRole("status")).toHaveTextContent("La selección es demasiado pequeña.");
    expect(screen.queryByTestId("selection")).not.toBeInTheDocument();
    await act(async () => {
      fireEvent.keyDown(window, { key: "Enter" });
    });
    expect(finish).toHaveBeenCalledTimes(2);
  });

  it("si no puede leer su imagen cancela la captura", async () => {
    const api = fakeApi({ loadFrame: vi.fn(async () => Promise.reject(new Error("sin sesión"))) });
    render(<OverlayApp api={api} />);
    await waitFor(() => expect(api.cancel).toHaveBeenCalledTimes(1));
  });
});
