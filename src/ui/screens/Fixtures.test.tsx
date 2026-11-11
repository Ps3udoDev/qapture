import { act, fireEvent, screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { renderApp, seedCases } from "../test-utils";

describe("sección «Archivos de prueba»", () => {
  it("agrega con «Agregar…» y muestra nombre y tamaño", async () => {
    const { store } = await renderApp({
      setup: async (repos) => {
        await seedCases(repos);
      },
    });

    expect(screen.getByRole("heading", { name: "Archivos de prueba" })).toBeInTheDocument();
    expect(screen.getByText("Sin archivos de prueba. Arrastra aquí o usa «Agregar…».")).toBeInTheDocument();

    const input = screen.getByLabelText("Agregar archivos de prueba");
    const file = new File([new Uint8Array(2048)], "datos.xlsx", {
      type: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
    });
    fireEvent.change(input, { target: { files: [file] } });

    expect(await screen.findByText("datos.xlsx")).toBeInTheDocument();
    expect(screen.getByText("2 KB")).toBeInTheDocument();
    expect(screen.queryByText("Sin archivos de prueba. Arrastra aquí o usa «Agregar…».")).not.toBeInTheDocument();
    expect(store.getState().fixtures).toHaveLength(1);
  });

  it("dropping_on_the_section_adds_a_fixture_not_an_evidence", async () => {
    const { store } = await renderApp({
      setup: async (repos) => {
        await seedCases(repos);
      },
    });

    const section = screen.getByRole("region", { name: "Archivos de prueba" });
    const excelFile = new File([new Uint8Array(1024)], "planilla.xlsx");

    fireEvent.drop(section, {
      dataTransfer: { files: [excelFile], items: [], types: ["Files"] },
    });

    expect(await within(section).findByText("planilla.xlsx")).toBeInTheDocument();
    expect(store.getState().fixtures).toHaveLength(1);
    expect(store.getState().evidence).toHaveLength(0);

    const hasEvidenceToast = store
      .getState()
      .toasts.some((t) => t.message.toLowerCase().includes("captura") || t.message.toLowerCase().includes("omit"));
    expect(hasEvidenceToast).toBe(false);
  });

  it("un tipo no aceptado muestra el mensaje fijo", async () => {
    const { store } = await renderApp({
      setup: async (repos) => {
        await seedCases(repos);
      },
    });

    const input = screen.getByLabelText("Agregar archivos de prueba");
    const exeFile = new File([new Uint8Array(100)], "malware.exe");

    fireEvent.change(input, { target: { files: [exeFile] } });

    expect(await screen.findByText("Ese tipo de archivo no se acepta como archivo de prueba.")).toBeInTheDocument();
    expect(store.getState().fixtures).toHaveLength(0);
  });

  it("«Copiar ruta» y «Mostrar en carpeta» llaman al puerto", async () => {
    let caseId = "";
    const { repos, user } = await renderApp({
      setup: async (r) => {
        const seeded = await seedCases(r);
        caseId = seeded.a.id;
        await r.fixtures.add(caseId, "guia.pdf", new Uint8Array([1, 2, 3]));
      },
    });

    expect(await screen.findByText("guia.pdf")).toBeInTheDocument();

    const copyBtn = screen.getByRole("button", { name: "Copiar ruta" });
    await user.click(copyBtn);
    expect(repos.fixtures.copied).toHaveLength(1);
    expect(await screen.findByText("Ruta copiada.")).toBeInTheDocument();

    const revealBtn = screen.getByRole("button", { name: "Mostrar en carpeta" });
    await user.click(revealBtn);
    expect(repos.fixtures.revealed).toHaveLength(1);
  });

  it("«Quitar» pide confirmación y lo saca de la lista", async () => {
    let caseId = "";
    const { user, store } = await renderApp({
      setup: async (r) => {
        const seeded = await seedCases(r);
        caseId = seeded.a.id;
        await r.fixtures.add(caseId, "borrame.txt", new Uint8Array([10, 20]));
      },
    });

    expect(await screen.findByText("borrame.txt")).toBeInTheDocument();

    const removeBtn = screen.getByRole("button", { name: "Quitar" });
    await user.click(removeBtn);

    const dialog = screen.getByRole("dialog", { name: "Quitar archivo de prueba" });
    expect(within(dialog).getByText("Se quitará «borrame.txt» de este caso.")).toBeInTheDocument();

    await user.click(within(dialog).getByRole("button", { name: "Cancelar" }));
    expect(screen.queryByRole("dialog", { name: "Quitar archivo de prueba" })).not.toBeInTheDocument();
    expect(screen.getByText("borrame.txt")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Quitar" }));
    const dialogConfirm = screen.getByRole("dialog", { name: "Quitar archivo de prueba" });
    await user.click(within(dialogConfirm).getByRole("button", { name: "Quitar" }));

    await waitFor(() => {
      expect(screen.queryByText("borrame.txt")).not.toBeInTheDocument();
    });
    expect(screen.getByText("Sin archivos de prueba. Arrastra aquí o usa «Agregar…».")).toBeInTheDocument();
    expect(store.getState().fixtures).toHaveLength(0);
  });

  it("muestra la nota si no está vacía y cambia de lista al cambiar de caso", async () => {
    let caseA = "";
    let caseB = "";
    const { store } = await renderApp({
      setup: async (r) => {
        const seeded = await seedCases(r);
        caseA = seeded.a.id;
        caseB = seeded.b.id;
        await r.fixtures.add(caseA, "doc.pdf", new Uint8Array([1, 2]));
        await r.fixtures.add(caseA, "vacio.txt", new Uint8Array([3]));
        await r.fixtures.add(caseB, "otro.xlsx", new Uint8Array([4, 5]));
      },
    });

    expect(await screen.findByText("doc.pdf")).toBeInTheDocument();

    act(() => {
      store.setState((s) => ({
        fixtures: s.fixtures.map((f) => (f.fileName === "doc.pdf" ? { ...f, note: "Archivo de entrada" } : f)),
      }));
    });

    expect(screen.getByText("Archivo de entrada")).toBeInTheDocument();
    expect(screen.getByText("vacio.txt")).toBeInTheDocument();
    expect(screen.queryByText("otro.xlsx")).not.toBeInTheDocument();

    // Cambiar de caso a caseB
    await screen.findByRole("heading", { name: "Recibos" });
    const list = screen.getByRole("list", { name: "Lista de casos" });
    const itemB = within(list).getByText("Pólizas");
    fireEvent.click(itemB);

    expect(await screen.findByText("otro.xlsx")).toBeInTheDocument();
    expect(screen.queryByText("doc.pdf")).not.toBeInTheDocument();
    expect(store.getState().fixturesCaseId).toBe(caseB);
  });
});
