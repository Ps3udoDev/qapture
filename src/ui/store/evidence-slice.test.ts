import { describe, expect, it } from "vitest";
import { emptyAnnotations } from "../../domain/annotations";
import { createMemoryRepositories } from "../../infrastructure/memory/repositories";
import { seedProject } from "../test-utils";
import { memoryPreferences } from "./preferences";
import { createWorkspaceStore } from "./workspace";

const PNG = new Uint8Array([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);

async function setup() {
  const repos = createMemoryRepositories();
  const p = await seedProject(repos);
  const entorno = p.fields.find((f) => f.key === "entorno")!.id;
  const c1 = await repos.cases.create(p.id, { title: "Recibos", status: "No ejecutado", values: { [entorno]: "QA" } });
  const c2 = await repos.cases.create(p.id, { title: "Pólizas", status: "No ejecutado", values: { [entorno]: "QA" } });
  const store = createWorkspaceStore(repos, memoryPreferences(p.id));
  await store.getState().init();
  const save = (caseId = c1.id, description = "Pantalla") =>
    store.getState().saveEditor({ caseId, description, annotations: emptyAnnotations(), render: PNG });
  return { repos, store, c1, c2, save };
}

describe("slice de evidencias", () => {
  it("abre el editor con el caso activo, guarda y actualiza el contador", async () => {
    const { store, c1, save } = await setup();
    await store.getState().intakeImages([PNG]);
    expect(store.getState().editor).toMatchObject({ kind: "new", caseId: c1.id });
    expect(await save()).toEqual({ ok: true });
    const s = store.getState();
    expect(s.editor).toBeNull();
    expect(s.evidence.map((e) => e.description)).toEqual(["Pantalla"]);
    expect(s.cases.find((c) => c.id === c1.id)?.evidenceCount).toBe(1);
    expect(s.toasts.at(-1)?.message).toBe("Evidencia 1 agregada a EJ-001.");
  });

  it("devuelve el error de descripción sin cerrar el editor", async () => {
    const { store, save } = await setup();
    await store.getState().intakeImages([PNG]);
    expect(await save(undefined, " ")).toEqual({ ok: false, errors: { description: "«Descripción» es obligatorio." } });
    expect(store.getState().editor).not.toBeNull();
  });

  it("encola varias imágenes y las abre en secuencia; cancelar descarta el staging", async () => {
    const { repos, store, save } = await setup();
    await store.getState().intakeImages([PNG, PNG, PNG]);
    expect(store.getState().editorQueue).toHaveLength(2);
    await save();
    expect(store.getState().editorQueue).toHaveLength(1);
    const current = store.getState().editor;
    await store.getState().cancelEditor();
    if (current?.kind === "new") await expect(repos.evidence.readImage(current.imagePath)).rejects.toBeTruthy();
    expect(store.getState().editorQueue).toHaveLength(0);
    expect(store.getState().editor?.kind).toBe("new");
  });

  it("sin caso seleccionado avisa y no abre el editor", async () => {
    const { store } = await setup();
    store.getState().selectCase(null);
    await store.getState().intakeImages([PNG]);
    expect(store.getState().editor).toBeNull();
    expect(store.getState().toasts.at(-1)?.message).toBe("Crea o selecciona un caso antes de agregar evidencias.");
  });

  it("re-anota, mueve, reordena y elimina", async () => {
    const { store, c1, c2, save } = await setup();
    for (const d of ["uno", "dos"]) {
      await store.getState().intakeImages([PNG]);
      await save(c1.id, d);
    }
    const [first, second] = store.getState().evidence;
    store.getState().editEvidence(first.id);
    expect(store.getState().editor).toMatchObject({ kind: "edit" });
    await save(c1.id, "uno editada");
    expect(store.getState().evidence[0].description).toBe("uno editada");

    await store.getState().reorderEvidence([second.id, first.id]);
    expect(store.getState().evidence.map((e) => e.description)).toEqual(["dos", "uno editada"]);

    await store.getState().moveEvidence(second.id, c2.id);
    expect(store.getState().evidence.map((e) => e.description)).toEqual(["uno editada"]);
    expect(store.getState().cases.find((c) => c.id === c2.id)?.evidenceCount).toBe(1);

    await store.getState().deleteEvidence(first.id);
    expect(store.getState().evidence).toEqual([]);
  });
});
