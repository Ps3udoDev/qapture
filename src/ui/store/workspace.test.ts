import { describe, expect, it, vi } from "vitest";
import { emptyMeta, fieldToInput } from "../../domain/project";
import { createMemoryRepositories } from "../../infrastructure/memory/repositories";
import { AppError } from "../../ports/errors";
import { memoryPreferences } from "./preferences";
import { createWorkspaceStore } from "./workspace";

const meta = (name: string) => ({ ...emptyMeta(), name, client: "ACME" });

async function setup(lastProject: string | null = null) {
  const repos = createMemoryRepositories();
  const prefs = memoryPreferences(lastProject);
  const store = createWorkspaceStore(repos, prefs);
  return { repos, prefs, store, s: () => store.getState() };
}

describe("workspace store", () => {
  it("arranca vacío y listo", async () => {
    const { s } = await setup();
    await s().init();
    expect(s().ready).toBe(true);
    expect(s().project).toBeNull();
    expect(s().templates.map((t) => t.id)).toContain("tpl-funcional");
  });

  it("abre el último proyecto recordado y, si ya no existe, el primero", async () => {
    const a = await setup();
    const p1 = await a.repos.projects.create({ meta: meta("Uno"), templateId: "tpl-funcional" });
    const p2 = await a.repos.projects.create({ meta: meta("Dos"), templateId: "tpl-funcional" });
    a.prefs.setLastProject(p1.id);
    const store = createWorkspaceStore(a.repos, a.prefs);
    await store.getState().init();
    expect(store.getState().project?.id).toBe(p1.id);

    a.prefs.setLastProject("borrado");
    const fallback = createWorkspaceStore(a.repos, a.prefs);
    await fallback.getState().init();
    expect(fallback.getState().project?.id).toBe(p2.id);
    expect(a.prefs.getLastProject()).toBe(p2.id);
  });

  it("valida antes de crear proyecto y selecciona el creado", async () => {
    const { repos, s } = await setup();
    await s().init();
    const spy = vi.spyOn(repos.projects, "create");
    expect(await s().createProject({ meta: meta(" "), templateId: "tpl-funcional" })).toEqual({
      ok: false,
      errors: { name: "«Nombre del proyecto» es obligatorio." },
    });
    expect(spy).not.toHaveBeenCalled();
    expect(await s().createProject({ meta: meta("Recibos"), templateId: "tpl-funcional" })).toEqual({ ok: true });
    expect(s().project?.name).toBe("Recibos");
    expect(s().projects).toHaveLength(1);
  });

  it("crea, duplica y elimina casos moviendo la selección", async () => {
    const { s } = await setup();
    await s().init();
    await s().createProject({ meta: meta("Recibos"), templateId: "tpl-funcional" });
    const env = s().project!.fields.find((f) => f.key === "entorno")!.id;

    const invalid = await s().saveCase({ title: "Caso", status: "Aprobado", values: {} });
    expect(invalid).toEqual({ ok: false, errors: { [env]: "«Entorno» es obligatorio." } });

    expect(await s().saveCase({ title: "Caso", status: "Aprobado", values: { [env]: "QA" } })).toEqual({ ok: true });
    const first = s().cases[0];
    expect(first.code).toBe("EJ-001");
    expect(s().selectedCaseId).toBe(first.id);
    expect(s().projects[0].caseCount).toBe(1);

    await s().duplicateCase(first.id);
    expect(s().cases.map((c) => c.code)).toEqual(["EJ-001", "EJ-002"]);
    expect(s().selectedCaseId).toBe(s().cases[1].id);

    await s().deleteCase(s().cases[1].id);
    expect(s().cases.map((c) => c.code)).toEqual(["EJ-001"]);
    expect(s().selectedCaseId).toBe(first.id);
  });

  it("recupera un formulario con columnas desactualizadas", async () => {
    const { repos, s } = await setup();
    await s().init();
    await s().createProject({ meta: meta("Recibos"), templateId: "tpl-funcional" });
    const project = s().project!;
    const env = project.fields.find((f) => f.key === "entorno")!.id;
    const menu = project.fields.find((f) => f.key === "menu")!.id;
    await repos.projects.saveFields(project.id, project.fields.filter((f) => f.id !== menu).map(fieldToInput));

    const input = { title: "Caso", status: "Aprobado" as const, values: { [env]: "QA", [menu]: "Cobros" } };
    const stale = await s().saveCase(input);
    expect(stale).toEqual({ ok: false, errors: { _form: "La columna enviada no pertenece al proyecto. Recarga el proyecto." } });
    expect(s().project!.fields.some((f) => f.id === menu)).toBe(false);
    expect(await s().saveCase(input)).toEqual({ ok: true });
  });

  it("al eliminar el proyecto abre otro y lo recuerda", async () => {
    const { prefs, s } = await setup();
    await s().init();
    await s().createProject({ meta: meta("Uno"), templateId: "tpl-funcional" });
    const first = s().project!.id;
    await s().createProject({ meta: meta("Dos"), templateId: "tpl-funcional" });
    await s().deleteProject();
    expect(s().project?.id).toBe(first);
    expect(prefs.getLastProject()).toBe(first);
  });

  it("muestra un toast cuando falla algo que no es validación", async () => {
    const { repos, s } = await setup();
    vi.spyOn(repos.projects, "list").mockRejectedValue(new AppError("db", "No se pudo completar la operación."));
    await s().init();
    expect(s().ready).toBe(true);
    expect(s().toasts).toEqual([expect.objectContaining({ kind: "error", message: "No se pudo completar la operación." })]);
  });
});
