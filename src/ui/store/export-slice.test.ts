import { describe, expect, it } from "vitest";
import { emptyDocVersion } from "../../domain/doc-versions";
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

});
