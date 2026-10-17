import { describe, expect, it } from "vitest";
import { emptyMeta } from "../../domain/project";
import { createMemoryRepositories, type MemoryRepositories } from "../../infrastructure/memory/repositories";
import { memoryPreferences } from "./preferences";
import { createWorkspaceStore } from "./workspace";

async function setup(configure?: (repos: MemoryRepositories) => Promise<void> | void) {
  const repos = createMemoryRepositories();
  await configure?.(repos);
  const store = createWorkspaceStore(repos, memoryPreferences(null, ""));
  await store.getState().init();
  return { repos, store };
}

const lastToast = (store: Awaited<ReturnType<typeof setup>>["store"]) => {
  const { toasts } = store.getState();
  return toasts[toasts.length - 1];
};

const seed = (repos: MemoryRepositories) =>
  repos.projects.create({ meta: { ...emptyMeta(), name: "Recibos", client: "ACME" }, templateId: "tpl-funcional" });

describe("importar", () => {
  it("v1: abre el proyecto importado y muestra el resumen", async () => {
    const { repos, store } = await setup();
    await store.getState().importLegacy();
    const s = store.getState();
    expect(s.project?.name).toBe("Aplicación de recibos");
    expect(s.projects.map((p) => p.name)).toEqual(["Aplicación de recibos"]);
    expect(lastToast(store)).toMatchObject({ kind: "success", message: "Importado «Aplicación de recibos»: 1 caso y 2 capturas." });
    expect(repos.data.calls).toEqual(["importLegacy"]);
    expect(s.dataBusy).toBe(false);
  });

  it("cerrar el diálogo de archivo no cambia nada", async () => {
    const { repos, store } = await setup((r) => void (r.data.mode = "cancel"));
    await store.getState().importPackage();
    expect(store.getState().projects).toEqual([]);
    expect(store.getState().toasts).toEqual([]);
    expect(repos.data.calls).toEqual(["importPackage"]);
  });

  it("un archivo rechazado deja la lista de problemas y no importa nada", async () => {
    const { store } = await setup((r) => void (r.data.mode = "invalid"));
    await store.getState().importLegacy();
    expect(store.getState().importIssues).toEqual({
      title: "No se importó nada. Revisa estos problemas:",
      items: ["Proyecto «Aplicación de recibos», EJ-001: Completa caso y entorno.", "Proyecto «Aplicación de recibos», EJ-002: Fecha inválida."],
    });
    expect(store.getState().projects).toEqual([]);
    expect(store.getState().toasts).toEqual([]);
    store.getState().dismissImportIssues();
    expect(store.getState().importIssues).toBeNull();
  });

  it("no corre dos operaciones de datos a la vez", async () => {
    const { repos, store } = await setup();
    const s = store.getState();
    await Promise.all([s.importLegacy(), s.importPackage()]);
    expect(repos.data.calls).toEqual(["importLegacy"]);
  });
});

describe("exportar paquete", () => {
  it("sin proyecto abierto no hace nada", async () => {
    const { repos, store } = await setup();
    await store.getState().exportPackage();
    expect(repos.data.calls).toEqual([]);
  });

  it("avisa con «Abrir carpeta», que muestra el paquete", async () => {
    const { repos, store } = await setup(async (r) => void (await seed(r)));
    await store.getState().exportPackage();
    const toast = lastToast(store);
    expect(toast.message).toBe("Paquete guardado: T4S_ACME_Recibos_20261016.qapture");
    toast.actions?.[0].run();
    await Promise.resolve();
    expect(repos.exporter.revealed).toEqual(["C:\\Users\\ana\\Documents\\T4S_ACME_Recibos_20261016.qapture"]);
  });
});

describe("respaldos", () => {
  it("lista, crea y restaura", async () => {
    const { repos, store } = await setup(async (r) => void (await seed(r)));
    await store.getState().loadBackups();
    expect(store.getState().backups).toHaveLength(2);
    await store.getState().createBackup();
    expect(store.getState().backups[0].fileName).toBe("qapture-manual-20261016-100001.db");
    expect(lastToast(store).message).toBe("Respaldo creado: qapture-manual-20261016-100001.db");
    expect(await store.getState().restoreBackup("qapture-diario-20261015.db")).toBe(true);
    expect(lastToast(store).message).toBe("Se restauró el respaldo del 15/10/2026 09:00.");
    expect(store.getState().backups[0].kind).toBe("before-restore");
    expect(store.getState().project?.name).toBe("Recibos");
    expect(repos.data.calls).toEqual(["createBackup", "restore:qapture-diario-20261015.db"]);
  });

  it("un respaldo que no existe avisa y devuelve false", async () => {
    const { store } = await setup();
    expect(await store.getState().restoreBackup("qapture-manual-20991231-000000.db")).toBe(false);
    expect(lastToast(store)).toMatchObject({ kind: "error", message: "El respaldo no existe." });
  });
});

describe("modo recuperación", () => {
  const damaged = (r: MemoryRepositories) => void (r.data.status = { recovery: true, detail: "base dañada" });

  it("al iniciar no carga proyectos y trae los respaldos", async () => {
    const { store } = await setup(damaged);
    const s = store.getState();
    expect(s.recovery).toEqual({ recovery: true, detail: "base dañada" });
    expect(s.ready).toBe(true);
    expect(s.backups).toHaveLength(2);
  });

  it("recuperar con un respaldo vuelve al uso normal", async () => {
    const { repos, store } = await setup(damaged);
    await store.getState().recover("qapture-diario-20261016.db");
    expect(store.getState().recovery).toBeNull();
    expect(lastToast(store).message).toBe("Base recuperada desde el respaldo.");
    expect(repos.data.calls).toEqual(["recover:qapture-diario-20261016.db"]);
  });

  it("empezar vacía e importar un paquete abre la importación a continuación", async () => {
    const { repos, store } = await setup(damaged);
    await store.getState().recover(null, true);
    expect(repos.data.calls).toEqual(["recover:vacía", "importPackage"]);
    expect(store.getState().project?.name).toBe("Recibos (importado)");
    expect(lastToast(store).message).toBe("Importado «Recibos (importado)»: 3 casos y 4 capturas.");
  });
});
