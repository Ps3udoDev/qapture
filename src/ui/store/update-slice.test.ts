import { describe, expect, it } from "vitest";
import { progressMessage, type UpdateInfo } from "../../domain/update";
import { createMemoryRepositories } from "../../infrastructure/memory/repositories";
import { AppError } from "../../ports/errors";
import { memoryPreferences } from "./preferences";
import { isUpdatePromptOpen } from "./update-slice";
import { createWorkspaceStore } from "./workspace";

const info = (version: string, portable = false): UpdateInfo => ({ version, currentVersion: "1.0.0", notes: "Notas", date: null, portable });

async function setup() {
  const repos = createMemoryRepositories();
  const store = createWorkspaceStore(repos, memoryPreferences(null, ""));
  await store.getState().init();
  const unlisten = await store.getState().listenUpdates();
  return { repos, store, unlisten, s: () => store.getState(), open: () => isUpdatePromptOpen(store.getState()) };
}

describe("slice del updater", () => {
  it("la revisión automática abre el diálogo una vez por versión", async () => {
    const { repos, s, open } = await setup();
    repos.updater.emitAvailable(info("1.0.1"));
    expect(open()).toBe(true);
    s().dismissUpdate();
    expect(open()).toBe(false);
    expect(s().update.dismissed).toBe("1.0.1");
    expect(s().update.available?.version).toBe("1.0.1");
    repos.updater.emitAvailable(info("1.0.1"));
    expect(open()).toBe(false);
    repos.updater.emitAvailable(info("1.0.2"));
    expect(open()).toBe(true);
  });

  it("buscar a mano sin novedades avisa que está al día y sin red muestra el error", async () => {
    const { repos, s } = await setup();
    repos.updater.next = null;
    await s().checkForUpdates(true);
    const version = repos.settings.snapshot.appVersion;
    expect(s().toasts.map((t) => t.message)).toEqual([`Qapture está al día (versión ${version}).`]);
    expect(s().update.status).toBe("idle");
    const offline = "No se pudo buscar actualizaciones. Revisa tu conexión a internet.";
    repos.updater.next = new AppError("internal", offline);
    await s().checkForUpdates(true);
    expect(s().toasts.at(-1)).toMatchObject({ kind: "error", message: offline });
  });

  it("la revisión automática falla en silencio y no avisa si está al día", async () => {
    const { repos, s } = await setup();
    repos.updater.next = null;
    await s().checkForUpdates(false);
    repos.updater.next = new AppError("internal", "sin red");
    await s().checkForUpdates(false);
    expect(s().toasts).toEqual([]);
  });

  it("encontrar una versión nueva a mano la deja disponible", async () => {
    const { repos, s, open } = await setup();
    repos.updater.next = info("1.0.1");
    await s().checkForUpdates(true);
    expect(s().update.available?.version).toBe("1.0.1");
    expect(open()).toBe(true);
    expect(s().toasts).toEqual([]);
  });

  it("no instala con una captura sin guardar en el editor", async () => {
    const { repos, store, s } = await setup();
    repos.updater.emitAvailable(info("1.0.1"));
    store.setState({ editorQueue: [{ imagePath: "C:/tmp/a.png" }] });
    await s().installUpdate();
    expect(s().toasts.map((t) => t.message)).toEqual(["Guarda o descarta la captura abierta antes de actualizar."]);
    expect(repos.updater.installs).toBe(0);
    expect(s().update.status).toBe("idle");
  });

  it("instala y muestra el progreso de descarga", async () => {
    const { repos, s } = await setup();
    repos.updater.emitAvailable(info("1.0.1"));
    repos.updater.emitProgress({ downloaded: 50, total: 200 });
    expect(s().update.status).toBe("downloading");
    expect(s().update.progress).toBe(25);
    expect(progressMessage(50, 200)).toBe("Descargando actualización… 25 %");
    repos.updater.emitProgress({ downloaded: 80, total: null });
    expect(s().update.progress).toBeNull();
    await s().installUpdate();
    expect(repos.updater.installs).toBe(1);
  });

  it("un error al instalar vuelve a reposo y lo avisa", async () => {
    const { repos, s } = await setup();
    repos.updater.emitAvailable(info("1.0.1"));
    repos.updater.installError = new AppError("internal", "No se pudo instalar la actualización: disco lleno.");
    await s().installUpdate();
    expect(s().update.status).toBe("idle");
    expect(s().update.progress).toBeNull();
    expect(s().toasts.at(-1)?.message).toBe("No se pudo instalar la actualización: disco lleno.");
  });

  it("dejar de escuchar corta los eventos", async () => {
    const { repos, unlisten, s } = await setup();
    unlisten();
    repos.updater.emitAvailable(info("1.0.1"));
    expect(s().update.available).toBeNull();
  });
});
