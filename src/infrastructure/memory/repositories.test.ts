import { describe, expect, it } from "vitest";
import { emptyMeta, fieldToInput } from "../../domain/project";
import { createMemoryRepositories } from "./repositories";

async function setup() {
  const repos = createMemoryRepositories();
  const project = await repos.projects.create({ meta: { ...emptyMeta(), name: "Recibos" }, templateId: "tpl-funcional" });
  const id = (key: string) => project.fields.find((f) => f.key === key)!.id;
  return { repos, project, id };
}

describe("memoria", () => {
  it("asigna códigos y no los reutiliza", async () => {
    const { repos, project, id } = await setup();
    const input = { title: "Caso", status: "Aprobado" as const, values: { [id("entorno")]: "QA" } };
    await repos.cases.create(project.id, input);
    const b = await repos.cases.create(project.id, input);
    await repos.cases.remove(b.id);
    expect((await repos.cases.create(project.id, input)).code).toBe("EJ-003");
  });

  it("rechaza casos inválidos y columnas ajenas", async () => {
    const { repos, project, id } = await setup();
    await expect(repos.cases.create(project.id, { title: "Caso", status: "Aprobado", values: {} })).rejects.toMatchObject({
      code: "validation",
      message: "«Entorno» es obligatorio.",
    });
    await expect(
      repos.cases.create(project.id, { title: "Caso", status: "Aprobado", values: { [id("entorno")]: "QA", otro: "x" } }),
    ).rejects.toMatchObject({ message: "La columna enviada no pertenece al proyecto. Recarga el proyecto." });
  });

  it("duplica limpiando las columnas marcadas", async () => {
    const { repos, project, id } = await setup();
    const src = await repos.cases.create(project.id, {
      title: "Caso",
      status: "Aprobado",
      values: { [id("entorno")]: "QA", [id("resultado_obtenido")]: "OK" },
    });
    const copy = await repos.cases.duplicate(src.id);
    expect(copy).toMatchObject({ code: "EJ-002", status: "No ejecutado", values: { [id("entorno")]: "QA" } });
  });

  it("archiva columnas con datos y elimina las vacías", async () => {
    const { repos, project, id } = await setup();
    await repos.cases.create(project.id, { title: "Caso", status: "Aprobado", values: { [id("entorno")]: "QA", [id("modulo")]: "Cobranzas" } });
    const kept = project.fields.filter((f) => f.key !== "modulo" && f.key !== "menu").map(fieldToInput);
    const fields = await repos.projects.saveFields(project.id, kept);
    expect(fields.find((f) => f.key === "menu")).toBeUndefined();
    expect(fields.find((f) => f.key === "modulo")?.archived).toBe(true);
  });

  it("al eliminar un proyecto elimina sus casos", async () => {
    const { repos, project, id } = await setup();
    await repos.cases.create(project.id, { title: "Caso", status: "Aprobado", values: { [id("entorno")]: "QA" } });
    await repos.projects.remove(project.id);
    expect(await repos.projects.list()).toEqual([]);
    await expect(repos.cases.list(project.id)).rejects.toMatchObject({ code: "not_found" });
  });
});
