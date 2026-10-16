import { describe, expect, it, vi } from "vitest";
import { emptyDocVersion } from "../../domain/doc-versions";
import { defaultExportOptions } from "../../domain/export";
import { emptyMeta } from "../../domain/project";
import { createMemoryRepositories } from "./repositories";

async function setup() {
  const repos = createMemoryRepositories();
  const p = await repos.projects.create({ meta: { ...emptyMeta(), name: "Recibos", client: "ACME" }, templateId: "tpl-funcional" });
  return { repos, p };
}

const input = (version: string, date = "2026-10-16") => ({ ...emptyDocVersion(date, "Ana Pérez"), version });

describe("versiones del documento en memoria", () => {
  it("lista por fecha, valida y no repite versiones", async () => {
    const { repos, p } = await setup();
    await repos.docVersions.create(p.id, input(" 1.1 ", "2026-10-20"));
    const first = await repos.docVersions.create(p.id, input("1.0", "2026-10-01"));
    expect((await repos.docVersions.list(p.id)).map((v) => v.version)).toEqual(["1.0", "1.1"]);
    await expect(repos.docVersions.create(p.id, input("V1.1"))).resolves.toBeTruthy();
    await expect(repos.docVersions.create(p.id, input("1.0"))).rejects.toMatchObject({ message: "La versión «1.0» ya existe en este proyecto." });
    await expect(repos.docVersions.create(p.id, input(""))).rejects.toMatchObject({ message: "«Versión» es obligatorio." });
    const updated = await repos.docVersions.update(first.id, { ...input("1.0"), changes: "Casos de Pólizas" });
    expect(updated.changes).toBe("Casos de Pólizas");
    await repos.docVersions.remove(first.id);
    await expect(repos.docVersions.remove(first.id)).rejects.toMatchObject({ code: "not_found" });
  });

  it("se borran con el proyecto", async () => {
    const { repos, p } = await setup();
    await repos.docVersions.create(p.id, input("1.0"));
    await repos.projects.remove(p.id);
    await expect(repos.docVersions.list(p.id)).rejects.toMatchObject({ code: "not_found" });
  });
});

describe("exportador en memoria", () => {
  it("exporta, emite avance y solo abre lo exportado", async () => {
    const { repos, p } = await setup();
    const progress = vi.fn();
    const off = await repos.exporter.onProgress(progress);
    const result = await repos.exporter.exportProject(p.id, { ...defaultExportOptions("Ana"), docVersionId: "ver-1" });
    expect(result).toMatchObject({ fileName: "T4S_ACME_Recibos_Evidencias_20261016_v1.0.xlsx", cases: 0, evidence: 0 });
    expect(progress).toHaveBeenCalledWith({ stage: "saving", current: 0, total: 0 });
    off();
    await repos.exporter.openFile(result!.path);
    expect(repos.exporter.opened).toEqual([result!.path]);
    await expect(repos.exporter.revealFile("C:\\otro.xlsx")).rejects.toMatchObject({ code: "validation" });
  });

  it("simula cancelar «Guardar como» y el archivo abierto en Excel", async () => {
    const { repos, p } = await setup();
    repos.exporter.mode = "cancel";
    await expect(repos.exporter.exportTemplate(p.id, defaultExportOptions("Ana"))).resolves.toBeNull();
    repos.exporter.mode = "locked";
    await expect(repos.exporter.exportProject(p.id, defaultExportOptions("Ana"))).rejects.toMatchObject({
      code: "file_locked",
      message: "Cierra el archivo en Excel y vuelve a intentar.",
    });
    expect(repos.exporter.calls.map((c) => c.kind)).toEqual(["template", "project"]);
  });
});
