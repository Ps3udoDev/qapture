import { describe, expect, it } from "vitest";
import { FIXTURE_NOT_ALLOWED } from "../../domain/fixtures";
import { createMemoryRepositories } from "../../infrastructure/memory/repositories";
import { memoryPreferences } from "./preferences";
import { createWorkspaceStore } from "./workspace";

async function setup() {
  const repos = createMemoryRepositories();
  const store = createWorkspaceStore(repos, memoryPreferences(null, ""));
  await store.getState().init();
  return { repos, store };
}

describe("fixture slice", () => {
  it("agrega archivos válidos y los lista en orden", async () => {
    const { store } = await setup();
    const file1 = new File(["col1,col2\n1,2"], "datos.csv", { type: "text/csv" });
    const file2 = new File(["%PDF-1.4"], "reporte.pdf", { type: "application/pdf" });

    await store.getState().addFixtures("case-1", [file1, file2]);

    const fixtures = store.getState().fixtures;
    expect(fixtures).toHaveLength(2);
    expect(fixtures[0].fileName).toBe("datos.csv");
    expect(fixtures[0].position).toBe(0);
    expect(fixtures[1].fileName).toBe("reporte.pdf");
    expect(fixtures[1].position).toBe(1);
    expect(store.getState().fixturesCaseId).toBe("case-1");
  });

  it("rechaza archivos inválidos con toast de error sin detener los demás", async () => {
    const { store } = await setup();
    const badFile = new File(["binario"], "malware.exe", { type: "application/x-msdownload" });
    const goodFile = new File(["xml"], "config.xml", { type: "application/xml" });

    await store.getState().addFixtures("case-1", [badFile, goodFile]);

    const fixtures = store.getState().fixtures;
    expect(fixtures).toHaveLength(1);
    expect(fixtures[0].fileName).toBe("config.xml");

    const toasts = store.getState().toasts;
    expect(toasts.some((t) => t.kind === "error" && t.message === FIXTURE_NOT_ALLOWED)).toBe(true);
  });

  it("elimina un fixture de la lista", async () => {
    const { store } = await setup();
    const file1 = new File(["a"], "a.csv", { type: "text/csv" });
    const file2 = new File(["b"], "b.csv", { type: "text/csv" });
    await store.getState().addFixtures("case-1", [file1, file2]);

    const firstId = store.getState().fixtures[0].id;
    await store.getState().removeFixture(firstId);

    const remaining = store.getState().fixtures;
    expect(remaining).toHaveLength(1);
    expect(remaining[0].fileName).toBe("b.csv");
    expect(remaining[0].position).toBe(0);
  });

  it("copiar ruta registra el id y muestra toast de éxito", async () => {
    const { repos, store } = await setup();
    const file = new File(["datos"], "datos.csv", { type: "text/csv" });
    await store.getState().addFixtures("case-1", [file]);

    const id = store.getState().fixtures[0].id;
    await store.getState().copyFixturePath(id);

    expect(repos.fixtures.copied).toContain(id);
    const toasts = store.getState().toasts;
    expect(toasts.some((t) => t.kind === "success" && t.message === "Ruta copiada.")).toBe(true);
  });

  it("mostrar en carpeta registra el id", async () => {
    const { repos, store } = await setup();
    const file = new File(["datos"], "datos.csv", { type: "text/csv" });
    await store.getState().addFixtures("case-1", [file]);

    const id = store.getState().fixtures[0].id;
    await store.getState().revealFixture(id);

    expect(repos.fixtures.revealed).toContain(id);
  });

  it("descarta la respuesta de loadFixtures si en el intermedio cambió el caso", async () => {
    const { repos, store } = await setup();
    const file1 = new File(["1"], "caso1.csv", { type: "text/csv" });
    const file2 = new File(["2"], "caso2.csv", { type: "text/csv" });
    await repos.fixtures.add("case-1", file1.name, new Uint8Array([1]));
    await repos.fixtures.add("case-2", file2.name, new Uint8Array([2]));

    // Interceptamos list para ralentizar case-1
    const originalList = repos.fixtures.list.bind(repos.fixtures);
    let resolveCase1: (() => void) | null = null;
    repos.fixtures.list = async (caseId: string) => {
      if (caseId === "case-1") {
        await new Promise<void>((resolve) => {
          resolveCase1 = resolve;
        });
      }
      return originalList(caseId);
    };

    const promise1 = store.getState().loadFixtures("case-1");
    // Inmediatamente cambiamos a case-2
    await store.getState().loadFixtures("case-2");

    expect(store.getState().fixturesCaseId).toBe("case-2");
    expect(store.getState().fixtures.map((f) => f.fileName)).toEqual(["caso2.csv"]);

    // Ahora resolvemos la carga de case-1
    if (resolveCase1) (resolveCase1 as () => void)();
    await promise1;

    // No debe haberse sobrescrito la lista con los fixtures de case-1
    expect(store.getState().fixturesCaseId).toBe("case-2");
    expect(store.getState().fixtures.map((f) => f.fileName)).toEqual(["caso2.csv"]);
  });

  it("al cambiar de caso no muestra la lista del anterior aunque la carga falle", async () => {
    const { store, repos } = await setup();
    await repos.fixtures.add("case-1", "caso1.csv", new Uint8Array([1]));
    await store.getState().loadFixtures("case-1");
    expect(store.getState().fixtures).toHaveLength(1);

    repos.fixtures.list = () => Promise.reject(new Error("fallo"));
    await store.getState().loadFixtures("case-2");

    expect(store.getState().fixturesCaseId).toBe("case-2");
    expect(store.getState().fixtures).toEqual([]);
  });
});
