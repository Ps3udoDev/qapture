import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { emptyAnnotations } from "../../domain/annotations";
import type { FormResult } from "../store/workspace";
import { AnnotationEditor, type AnnotationEditorProps } from "./AnnotationEditor";
import { MOCK_RENDER } from "./__mocks__/AnnotatorCanvas";

const cases = [
  { id: "c1", code: "EJ-001", title: "Recibos" },
  { id: "c2", code: "EJ-002", title: "Pólizas" },
];

function setup(over: Partial<AnnotationEditorProps> = {}) {
  const onSave = vi.fn(async (): Promise<FormResult> => ({ ok: true }));
  const onCancel = vi.fn();
  render(
    <AnnotationEditor
      title="Nueva evidencia"
      imagePath="img.png"
      loadImage={async () => new Uint8Array()}
      cases={cases}
      initialCaseId="c2"
      caseLocked={false}
      initialDescription=""
      initialAnnotations={emptyAnnotations()}
      pending={0}
      onSave={onSave}
      onCancel={onCancel}
      {...over}
    />,
  );
  const dialog = screen.getByRole("dialog", { name: "Editor de evidencia" });
  return { user: userEvent.setup(), onSave, onCancel, dialog, canvas: within(dialog).getByTestId("annotator-canvas") };
}

describe("editor de evidencia", () => {
  it("preselecciona el caso y exige la descripción", async () => {
    const { user, onSave, dialog } = setup();
    expect(within(dialog).getByLabelText("Caso")).toHaveValue("c2");
    await user.click(within(dialog).getByRole("button", { name: "Guardar evidencia" }));
    expect(within(dialog).getByText("«Descripción» es obligatorio.")).toBeInTheDocument();
    expect(onSave).not.toHaveBeenCalled();
  });

  it("guarda caso, descripción, anotaciones y render", async () => {
    const { user, onSave, dialog } = setup();
    await user.click(within(dialog).getByRole("button", { name: "dibujar marcador" }));
    await user.selectOptions(within(dialog).getByLabelText("Caso"), "c1");
    await user.type(within(dialog).getByLabelText("Descripción"), "① Recibo aplicado");
    await user.click(within(dialog).getByRole("button", { name: "Guardar evidencia" }));
    expect(onSave).toHaveBeenCalledWith({
      caseId: "c1",
      description: "① Recibo aplicado",
      annotations: expect.objectContaining({ shapes: [expect.objectContaining({ kind: "marker" })] }),
      render: MOCK_RENDER,
    });
  });

  it("muestra el error del guardado", async () => {
    const { user, dialog } = setup({ onSave: async () => ({ ok: false, errors: { _form: "El caso no existe." } }) });
    await user.type(within(dialog).getByLabelText("Descripción"), "x");
    await user.click(within(dialog).getByRole("button", { name: "Guardar evidencia" }));
    expect(await within(dialog).findByText("El caso no existe.")).toBeInTheDocument();
  });

  it("deshace y rehace con botones y teclado", async () => {
    const { user, dialog, canvas } = setup();
    await user.click(within(dialog).getByRole("button", { name: "dibujar marcador" }));
    expect(canvas).toHaveAttribute("data-shapes", "1");
    await user.keyboard("{Control>}z{/Control}");
    expect(canvas).toHaveAttribute("data-shapes", "0");
    await user.keyboard("{Control>}y{/Control}");
    expect(canvas).toHaveAttribute("data-shapes", "1");
    await user.click(within(dialog).getByRole("button", { name: "Deshacer" }));
    expect(canvas).toHaveAttribute("data-shapes", "0");
  });

  it("edita el texto seleccionado y lo borra con Supr", async () => {
    const { user, dialog, canvas } = setup();
    await user.click(within(dialog).getByRole("button", { name: "dibujar texto" }));
    const input = within(dialog).getByRole("textbox", { name: "Texto" });
    await user.clear(input);
    await user.type(input, "Monto");
    expect(canvas).toHaveAttribute("data-texts", "Monto");
    await user.click(within(dialog).getByRole("button", { name: "Seleccionar" }));
    await user.keyboard("{Delete}");
    expect(canvas).toHaveAttribute("data-shapes", "0");
  });

  it("cambia de herramienta con su tecla, bloquea el caso al re-anotar y cancela con Esc", async () => {
    const { user, dialog, onCancel } = setup({ caseLocked: true, pending: 2 });
    await user.keyboard("r");
    expect(within(dialog).getByRole("button", { name: "Recuadro" })).toHaveAttribute("aria-pressed", "true");
    expect(within(dialog).getByLabelText("Caso")).toBeDisabled();
    expect(within(dialog).getByText("2 en cola")).toBeInTheDocument();
    await user.keyboard("{Escape}");
    expect(onCancel).toHaveBeenCalled();
  });
});
