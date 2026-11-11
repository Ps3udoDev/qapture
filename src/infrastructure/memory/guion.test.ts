import { describe, expect, it, vi } from "vitest";
import { buildProfile, DEFAULT_STATUS_MAP, suggestTarget, type GuionImportInput } from "../../domain/guion";
import { createMemoryGuion } from "./guion";

async function setup() {
  const importProject = vi.fn(async (name: string) => ({ id: "project-1", name }));
  const guion = createMemoryGuion({ importProject });
  const inspection = (await guion.inspect())!;
  const sheet = inspection.sheets[0];
  const profile = buildProfile(sheet, 15, sheet.rows[14].map((h, i) => suggestTarget(h, sheet.rows.slice(15).map((r) => r[i] ?? ""))), DEFAULT_STATUS_MAP.import, sheet.cover);
  const input: GuionImportInput = { token: inspection.token, profile, projectName: "Guion", saveProfileAs: null };
  return { guion, input, importProject };
}
describe("guion en memoria", () => {
  it("bloquea códigos vacíos, repetidos y sin asignar antes de crear el proyecto", async () => {
    const { guion, input, importProject } = await setup();
    guion.inspection.sheets[0].rows[16][2] = "CP01";
    guion.inspection.sheets[0].rows[17][2] = "";
    expect((await guion.preview(input.token, input.profile)).errors).toEqual(["Hay filas sin código: 18.", "Hay códigos repetidos: CP01."]);
    await expect(guion.import(input)).rejects.toThrow("Hay códigos repetidos: CP01.");
    expect(importProject).not.toHaveBeenCalled();
    input.profile.columns = input.profile.columns.filter((c) => c.target.kind !== "code");
    expect((await guion.preview(input.token, input.profile)).errors).toEqual(["Falta asignar la columna del código del caso."]);
  });
  it("guarda, sugiere y borra perfiles con nombre único Unicode", async () => {
    const { guion, input, importProject } = await setup();
    await guion.import({ ...input, saveProfileAs: "GUION ÁREA" });
    const inspection = (await guion.inspect())!;
    const saved = (await guion.listProfiles())[0];
    expect(inspection.suggestedProfileId).toBe(saved.id);
    await expect(guion.import({ ...input, token: inspection.token, saveProfileAs: "guion área" })).rejects.toThrow("Ya existe un perfil llamado «guion área».");
    expect(importProject).toHaveBeenCalledTimes(1);
    await guion.deleteProfile(saved.id);
    expect(await guion.listProfiles()).toEqual([]);
    await expect(guion.deleteProfile(saved.id)).rejects.toThrow("El perfil no existe.");
  });
  it("invalida el token al importar", async () => {
    const { guion, input } = await setup();
    await guion.import(input);
    await expect(guion.preview(input.token, input.profile)).rejects.toThrow("Vuelve a elegir el archivo del guion.");
  });
});
