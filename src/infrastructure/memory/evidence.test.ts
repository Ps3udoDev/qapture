import { describe, expect, it } from "vitest";
import { emptyAnnotations, serializeAnnotations } from "../../domain/annotations";
import { emptyMeta } from "../../domain/project";
import { createMemoryRepositories } from "./repositories";

const PNG = new Uint8Array([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
const ANN = serializeAnnotations(emptyAnnotations());

async function setup() {
  const repos = createMemoryRepositories();
  const p = await repos.projects.create({ meta: { ...emptyMeta(), name: "Recibos" }, templateId: "tpl-funcional" });
  const entorno = p.fields.find((f) => f.key === "entorno")!.id;
  const newCase = (title: string) => repos.cases.create(p.id, { title, status: "No ejecutado", values: { [entorno]: "QA" } });
  const add = async (caseId: string, description: string) => {
    const staged = await repos.evidence.stage(PNG);
    return repos.evidence.add({ caseId, stagedPath: staged.path, description, annotations: ANN }, PNG);
  };
  return { repos, p, newCase, add };
}

describe("evidencias en memoria", () => {
  it("valida como Rust y cuenta por caso", async () => {
    const { repos, p, newCase, add } = await setup();
    const c = await newCase("Recibos");
    await expect(repos.evidence.stage(new Uint8Array([1, 2, 3]))).rejects.toMatchObject({ message: "El archivo no es una imagen PNG, JPEG o WebP." });
    const staged = await repos.evidence.stage(PNG);
    await expect(repos.evidence.add({ caseId: c.id, stagedPath: staged.path, description: " ", annotations: ANN }, PNG)).rejects.toMatchObject({
      message: "«Descripción» es obligatorio.",
    });
    const e = await add(c.id, "Pantalla");
    expect(e.position).toBe(0);
    expect((await repos.cases.list(p.id))[0].evidenceCount).toBe(1);
    await expect(repos.evidence.readImage(e.renderedPath)).resolves.toEqual(PNG);
  });

  it("mueve dentro del proyecto, reordena y borra en cascada", async () => {
    const { repos, p, newCase, add } = await setup();
    const a = await newCase("A");
    const b = await newCase("B");
    const e1 = await add(a.id, "uno");
    const e2 = await add(a.id, "dos");
    await repos.evidence.move(e1.id, b.id);
    expect((await repos.evidence.list(a.id)).map((e) => [e.description, e.position])).toEqual([["dos", 0]]);
    const other = await repos.projects.create({ meta: { ...emptyMeta(), name: "Otra" }, templateId: "tpl-funcional" });
    const foreign = await repos.cases.create(other.id, { title: "X", status: "No ejecutado", values: { [other.fields.find((f) => f.key === "entorno")!.id]: "QA" } });
    await expect(repos.evidence.move(e2.id, foreign.id)).rejects.toMatchObject({ message: "Solo puedes mover evidencias entre casos de la misma matriz." });
    await expect(repos.evidence.reorder(a.id, [])).rejects.toMatchObject({ message: "La lista de evidencias cambió. Recarga el caso." });
    await repos.cases.remove(b.id);
    await expect(repos.evidence.readImage(e1.renderedPath)).rejects.toMatchObject({ code: "not_found" });
    await repos.projects.remove(p.id);
    await expect(repos.evidence.readImage(e2.renderedPath)).rejects.toMatchObject({ code: "not_found" });
  });
});
