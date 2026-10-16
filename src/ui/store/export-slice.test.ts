import { describe, expect, it, vi } from "vitest";
import { emptyDocVersion } from "../../domain/doc-versions";
import { defaultExportOptions, type ExportProgress } from "../../domain/export";
import { AppError } from "../../ports/errors";
import { createMemoryRepositories } from "../../infrastructure/memory/repositories";
import { seedProject } from "../test-utils";
import { memoryPreferences } from "./preferences";
import { createWorkspaceStore } from "./workspace";

async function setup() {
  const repos = createMemoryRepositories();
  const p = await seedProject(repos);
  const prefs = memoryPreferences(p.id, "Ana Pérez");
  const store = createWorkspaceStore(repos, prefs);
  await store.getState().init();
  return { repos, store, prefs, p };
}

const newVersion = { ...emptyDocVersion("2026-10-16", "Ana Pérez"), version: "1.0", changes: "Versión inicial" };
const options = defaultExportOptions("Ana Pérez");

describe("slice de exportación", () => {
  it("carga las versiones del proyecto abierto y las guarda con toast", async () => {
    const { store } = await setup();
    expect(store.getState().docVersions).toEqual([]);
    expect(await store.getState().saveDocVersion(newVersion)).toEqual({ ok: true });
    expect(store.getState().docVersions.map((v) => v.version)).toEqual(["1.0"]);
    expect(store.getState().toasts.at(-1)?.message).toBe("Versión 1.0 registrada.");
    expect(await store.getState().saveDocVersion({ ...newVersion, author: " " })).toEqual({
      ok: false,
      errors: { author: "«Elaborado por» es obligatorio." },
    });
  });

  it("publica el avance mientras exporta y lo limpia al terminar", async () => {
    const { store } = await setup();
    const seen: (ExportProgress | null)[] = [];
    store.subscribe((s, prev) => {
      if (s.exporting !== prev.exporting) seen.push(s.exporting);
    });
    expect(await store.getState().exportExcel({ kind: "project", options, newVersion })).toEqual({ ok: true });
    expect(seen).toContainEqual({ stage: "saving", current: 0, total: 0 });
    expect(seen.at(-1)).toBeNull();
    expect(store.getState().toasts.at(-1)?.actions?.map((a) => a.label)).toEqual(["Abrir archivo", "Abrir carpeta"]);
  });

  it("rechaza un segundo export mientras corre el primero", async () => {
    const { store, repos } = await setup();
    const first = store.getState().exportExcel({ kind: "template", options });
    expect(await store.getState().exportExcel({ kind: "template", options })).toEqual({
      ok: false,
      errors: { _form: "Ya hay una exportación en curso." },
    });
    expect(await first).toEqual({ ok: true });
    expect(repos.exporter.calls).toHaveLength(1);
  });

  it("si se cancela «Guardar como» la versión nueva queda registrada una sola vez", async () => {
    const { store, repos, prefs } = await setup();
    repos.exporter.mode = "cancel";
    expect(await store.getState().exportExcel({ kind: "project", options: { ...options, tester: " Luis " }, newVersion })).toEqual({ ok: false, errors: {} });
    expect(store.getState().docVersions).toHaveLength(1);
    expect(prefs.getTester()).toBe("Luis");
    const [saved] = store.getState().docVersions;
    repos.exporter.mode = "save";
    expect(await store.getState().exportExcel({ kind: "project", options: { ...options, docVersionId: saved.id } })).toEqual({ ok: true });
    expect(store.getState().docVersions).toHaveLength(1);
  });

  it("el archivo abierto en Excel vuelve como error del formulario", async () => {
    const { store, repos } = await setup();
    repos.exporter.mode = "locked";
    expect(await store.getState().exportExcel({ kind: "project", options, newVersion })).toEqual({
      ok: false,
      errors: { _form: "Cierra el archivo en Excel y vuelve a intentar." },
    });
    expect(store.getState().exporting).toBeNull();
  });

  it("si la exportación falla tras crear la versión, recarga las versiones", async () => {
    const { store, repos } = await setup();
    vi.spyOn(repos.exporter, "exportProject").mockRejectedValueOnce(new AppError("export", "No se pudo generar el Excel. Inténtalo nuevamente."));
    expect(await store.getState().exportExcel({ kind: "project", options, newVersion })).toEqual({ ok: false, errors: {} });
    expect(store.getState().toasts.at(-1)?.message).toBe("No se pudo generar el Excel. Inténtalo nuevamente.");
    expect(store.getState().docVersions).toHaveLength(1);
  });
});
