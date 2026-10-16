import { act, fireEvent, screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { emptyAnnotations, serializeAnnotations } from "../../domain/annotations";
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

async function seedWithEvidence(repos: Repositories, descriptions = ["Primera", "Segunda"]) {
  const seeded = await seedCases(repos);
  for (const description of descriptions) {
    const staged = await repos.evidence.stage(new Uint8Array(PNG_BYTES));
    await repos.evidence.add(
      { caseId: seeded.a.id, stagedPath: staged.path, description, annotations: serializeAnnotations(emptyAnnotations()) },
      new Uint8Array(PNG_BYTES),
    );
  }
  return seeded;
}

const gallery = () => screen.getByRole("region", { name: "Evidencias" });
const cards = () => within(screen.getByRole("list", { name: "Lista de evidencias" })).getAllByRole("listitem");

describe("galería de evidencias", () => {
  it("muestra miniaturas, el contador en la lista y el visor", async () => {
    const { user } = await renderApp({ setup: async (repos) => void (await seedWithEvidence(repos)) });
    expect(await within(gallery()).findByRole("heading", { name: "Evidencias (2)" })).toBeInTheDocument();
    const list = screen.getByRole("list", { name: "Lista de casos" });
    expect(within(list).getByLabelText("2 evidencias")).toBeInTheDocument();
    await user.click(within(gallery()).getByRole("button", { name: "Ver evidencia 1" }));
    const viewer = screen.getByRole("dialog", { name: "Evidencia 1 de 2" });
    await user.keyboard("{ArrowRight}");
    expect(within(viewer).getByText("Segunda")).toBeInTheDocument();
    expect(screen.getByRole("dialog", { name: "Evidencia 2 de 2" })).toBeInTheDocument();
  });

  it("reordena, edita la descripción y re-anota sin dejar la miniatura en caché", async () => {
    const { user } = await renderApp({ setup: async (repos) => void (await seedWithEvidence(repos)) });
    await within(gallery()).findByRole("heading", { name: "Evidencias (2)" });
    await user.click(within(gallery()).getByRole("button", { name: "Mover evidencia 2 antes" }));
    await waitFor(() => expect(within(cards()[0]).getByText("Segunda")).toBeInTheDocument());

    await user.click(within(gallery()).getByRole("button", { name: "Editar descripción de evidencia 1" }));
    const dialog = screen.getByRole("dialog", { name: "Editar descripción" });
    await user.clear(within(dialog).getByLabelText("Descripción"));
    await user.type(within(dialog).getByLabelText("Descripción"), "Segunda editada");
    await user.click(within(dialog).getByRole("button", { name: "Guardar" }));
    expect(await within(gallery()).findByText("Segunda editada")).toBeInTheDocument();

    const before = within(gallery()).getByRole("img", { name: "Evidencia 1" }).getAttribute("src");
    await user.click(within(gallery()).getByRole("button", { name: "Anotar evidencia 1" }));
    const editorDialog = await screen.findByRole("dialog", { name: "Editor de evidencia" });
    expect(within(editorDialog).getByRole("heading", { name: "Anotar evidencia 1" })).toBeInTheDocument();
    expect(within(editorDialog).getByLabelText("Descripción")).toHaveValue("Segunda editada");
    await user.click(within(editorDialog).getByRole("button", { name: "Guardar evidencia" }));
    expect(await screen.findByText("Evidencia 1 actualizada.")).toBeInTheDocument();
    expect(within(gallery()).getByRole("img", { name: "Evidencia 1" }).getAttribute("src")).not.toBe(before);
  });

  it("reasigna a otro caso y elimina con confirmación", async () => {
    const { user } = await renderApp({ setup: async (repos) => void (await seedWithEvidence(repos, ["Única"])) });
    await within(gallery()).findByRole("heading", { name: "Evidencias (1)" });
    await user.selectOptions(within(gallery()).getByLabelText("Mover evidencia 1 a otro caso"), "EJ-002 · Pólizas");
    expect(await screen.findByText("Evidencia movida a EJ-002.")).toBeInTheDocument();
    expect(within(gallery()).getByRole("heading", { name: "Evidencias (0)" })).toBeInTheDocument();
    const list = screen.getByRole("list", { name: "Lista de casos" });
    expect(within(list).getByLabelText("1 evidencia")).toBeInTheDocument();

    await user.click(within(list).getByText("Pólizas"));
    await within(gallery()).findByRole("heading", { name: "Evidencias (1)" });
    await user.click(within(gallery()).getByRole("button", { name: "Eliminar evidencia 1" }));
    await user.click(within(screen.getByRole("dialog", { name: "Eliminar evidencia 1" })).getByRole("button", { name: "Eliminar evidencia" }));
    expect(await screen.findByText("Evidencia 1 eliminada.")).toBeInTheDocument();
    expect(within(list).queryByLabelText("1 evidencia")).not.toBeInTheDocument();
  });

  it("elegir archivo abre el editor", async () => {
    const { user } = await renderApp({ setup: async (repos) => void (await seedCases(repos)) });
    await user.upload(screen.getByLabelText("Elegir imágenes de evidencia"), png());
    expect(await screen.findByRole("dialog", { name: "Editor de evidencia" })).toBeInTheDocument();
  });

  it("cierra el visor si la evidencia mostrada desaparece de la lista", async () => {
    const { user, store } = await renderApp({ setup: async (repos) => void (await seedWithEvidence(repos)) });
    await within(gallery()).findByRole("heading", { name: "Evidencias (2)" });
    await user.click(within(gallery()).getByRole("button", { name: "Ver evidencia 2" }));
    expect(screen.getByRole("dialog", { name: "Evidencia 2 de 2" })).toBeInTheDocument();
    const target = store.getState().cases.find((c) => c.code === "EJ-002")!;
    await act(async () => {
      await store.getState().moveEvidence(store.getState().evidence[1].id, target.id);
    });
    await waitFor(() => expect(within(gallery()).getByRole("heading", { name: "Evidencias (1)" })).toBeInTheDocument());
    expect(screen.queryByRole("dialog", { name: "Evidencia 2 de 2" })).not.toBeInTheDocument();
  });

  it("avisa si no se pueden leer las imágenes soltadas", async () => {
    await renderApp({ setup: async (repos) => void (await seedCases(repos)) });
    const file = png();
    Object.defineProperty(file, "arrayBuffer", { value: () => Promise.reject(new Error("x")) });
    fireEvent.drop(screen.getByRole("region", { name: "Detalle del caso" }), {
      dataTransfer: { files: [file], items: [], types: ["Files"] },
    });
    expect(await screen.findByText("No se pudieron leer las imágenes.")).toBeInTheDocument();
    expect(screen.queryByRole("dialog", { name: "Editor de evidencia" })).not.toBeInTheDocument();
  });
});
