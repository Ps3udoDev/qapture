import { describe, expect, it, vi } from "vitest";
import { buildProfile, DEFAULT_STATUS_MAP, suggestTarget } from "../../domain/guion";
import { createMemoryRepositories } from "../../infrastructure/memory/repositories";
import { memoryPreferences } from "./preferences";
import { createWorkspaceStore } from "./workspace";

async function setup() {
  const repos = createMemoryRepositories();
  const store = createWorkspaceStore(repos, memoryPreferences(null, ""));
  await store.getState().init();
  return { repos, store };
}
function profileFor(store: Awaited<ReturnType<typeof setup>>["store"]) {
  const sheet = store.getState().guionWizard!.inspection.sheets[0];
  const row = sheet.suggestedHeaderRow!;
  return buildProfile(sheet, row, sheet.rows[row - 1].map((h, i) => suggestTarget(h, sheet.rows.slice(row).map((r) => r[i] ?? ""))), DEFAULT_STATUS_MAP.import, sheet.cover);
}

describe("importar guion", () => {
  it("abre, previsualiza e importa y selecciona el proyecto", async () => {
    const { repos, store } = await setup();
    await store.getState().openGuionImport();
    const inspection = store.getState().guionWizard!.inspection;
    const profile = profileFor(store);
    await store.getState().previewGuion(profile);
    expect(store.getState().guionWizard?.preview?.cases).toHaveLength(3);
    expect(await store.getState().importGuion({ token: inspection.token, profile, projectName: "Guion importado", saveProfileAs: "Perfil de ejemplo" })).toEqual({ ok: true });
    expect(store.getState().project?.name).toBe("Guion importado");
    expect(store.getState().cases).toHaveLength(3);
    expect(store.getState().guionWizard).toBeNull();
    expect(await repos.guion.listProfiles()).toHaveLength(1);
    expect(store.getState().dataBusy).toBe(false);
  });
  it("cancelar el archivo no abre el asistente", async () => {
    const { repos, store } = await setup();
    repos.guion.mode = "cancel";
    await store.getState().openGuionImport();
    expect(store.getState().guionWizard).toBeNull();
  });
  it("un error de validación mantiene abierto el asistente", async () => {
    const { store } = await setup();
    await store.getState().openGuionImport();
    const input = { token: store.getState().guionWizard!.inspection.token, profile: profileFor(store), projectName: "", saveProfileAs: null };
    expect(await store.getState().importGuion(input)).toMatchObject({ ok: false, errors: { _form: "«Nombre del proyecto» es obligatorio." } });
    expect(store.getState().guionWizard).not.toBeNull();
    expect(store.getState().dataBusy).toBe(false);
  });
  it("excluye operaciones simultáneas y una respuesta tardía no reabre el asistente", async () => {
    const { repos, store } = await setup();
    await Promise.all([store.getState().openGuionImport(), store.getState().importPackage()]);
    expect(repos.data.calls).toEqual([]);
    const profile = profileFor(store);
    let finish!: (value: Awaited<ReturnType<typeof repos.guion.preview>>) => void;
    vi.spyOn(repos.guion, "preview").mockImplementation(() => new Promise((resolve) => { finish = resolve; }));
    const pending = store.getState().previewGuion(profile);
    expect(await store.getState().importGuion({ token: "x", profile, projectName: "Otro", saveProfileAs: null })).toMatchObject({ ok: false });
    store.getState().closeGuionImport();
    finish({ cases: [], fields: [], warnings: [], errors: [] });
    await pending;
    expect(store.getState().guionWizard).toBeNull();
    expect(store.getState().dataBusy).toBe(false);
  });
  it("un segundo import no reutiliza el token mientras el primero está pendiente", async () => {
    const { repos, store } = await setup();
    await store.getState().openGuionImport();
    const input = { token: store.getState().guionWizard!.inspection.token, profile: profileFor(store), projectName: "Guion", saveProfileAs: null };
    let finish!: () => void;
    const original = repos.guion.import.bind(repos.guion);
    const importer = vi.spyOn(repos.guion, "import").mockImplementation(async (request) => {
      await new Promise<void>((resolve) => { finish = resolve; });
      return original(request);
    });
    const first = store.getState().importGuion(input);
    expect(await store.getState().importGuion(input)).toMatchObject({ ok: false });
    expect(importer).toHaveBeenCalledTimes(1);
    finish();
    expect(await first).toEqual({ ok: true });
  });
});
