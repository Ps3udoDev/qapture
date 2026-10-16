import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { Repositories } from "../../ports/repositories";
import { renderApp, seedProject } from "../test-utils";

export const PNG_BYTES = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
export const png = (name = "captura.png") => new File([new Uint8Array(PNG_BYTES)], name, { type: "image/png" });

export async function seedCases(repos: Repositories) {
  const p = await seedProject(repos);
  const entorno = p.fields.find((f) => f.key === "entorno")!.id;
  const a = await repos.cases.create(p.id, { title: "Recibos", status: "No ejecutado", values: { [entorno]: "QA" } });
  const b = await repos.cases.create(p.id, { title: "Pólizas", status: "No ejecutado", values: { [entorno]: "QA" } });
  return { p, a, b };
}

const paste = (files: File[], types = ["Files"]) => fireEvent.paste(window, { clipboardData: { files, items: [], types } });
const editor = () => screen.findByRole("dialog", { name: "Editor de evidencia" });

describe("registro de evidencias", () => {
  it("Ctrl+V abre el editor con el caso activo, exige descripción y guarda", async () => {
    const { user, store } = await renderApp({ setup: async (repos) => void (await seedCases(repos)) });
    paste([png()]);
    const dialog = await editor();
    expect(within(dialog).getByLabelText("Caso")).toHaveDisplayValue("EJ-001 · Recibos");
    await user.click(within(dialog).getByRole("button", { name: "Guardar evidencia" }));
    expect(within(dialog).getByText("«Descripción» es obligatorio.")).toBeInTheDocument();
    await user.type(within(dialog).getByLabelText("Descripción"), "Recibo aplicado");
    await user.click(within(dialog).getByRole("button", { name: "Guardar evidencia" }));
    await waitFor(() => expect(screen.queryByRole("dialog", { name: "Editor de evidencia" })).not.toBeInTheDocument());
    expect(screen.getByText("Evidencia 1 agregada a EJ-001.")).toBeInTheDocument();
    expect(store.getState().evidence).toHaveLength(1);
  });

  it("no intercepta el pegado con un diálogo abierto ni sin imagen", async () => {
    const { user } = await renderApp({ setup: async (repos) => void (await seedCases(repos)) });
    paste([], ["text/plain"]);
    await user.click(screen.getByRole("button", { name: /Nuevo caso/ }));
    paste([png()]);
    expect(screen.queryByRole("dialog", { name: "Editor de evidencia" })).not.toBeInTheDocument();
    expect(screen.getByRole("dialog", { name: "Nuevo caso" })).toBeInTheDocument();
  });

  it("arrastrar 3 imágenes y un PDF encola las imágenes y avisa del omitido", async () => {
    await renderApp({ setup: async (repos) => void (await seedCases(repos)) });
    const pdf = new File([new Uint8Array([0x25, 0x50, 0x44, 0x46])], "informe.pdf", { type: "application/pdf" });
    fireEvent.drop(screen.getByRole("region", { name: "Detalle del caso" }), {
      dataTransfer: { files: [png("a.png"), png("b.png"), pdf, png("c.png")], items: [], types: ["Files"] },
    });
    const dialog = await editor();
    expect(within(dialog).getByText("2 en cola")).toBeInTheDocument();
    expect(screen.getByText("Se omitió 1 archivo: solo se aceptan imágenes PNG, JPEG o WebP.")).toBeInTheDocument();
  });
});
