import { describe, expect, it } from "vitest";
import { DEFAULT_SETTINGS } from "../../domain/settings";
import { createMemoryRepositories } from "../../infrastructure/memory/repositories";
import { seedProject } from "../test-utils";
import { memoryPreferences } from "./preferences";
import { createWorkspaceStore } from "./workspace";

function setup(tester = "") {
  const repos = createMemoryRepositories();
  const prefs = memoryPreferences(null, tester);
  const store = createWorkspaceStore(repos, prefs);
  return { repos, store, s: () => store.getState() };
}

describe("slice de ajustes", () => {
  it("init carga los ajustes y avisa los atajos que no se registraron", async () => {
    const { repos, s } = setup();
    repos.settings.failuresAtStart = [{ action: "paste", accelerator: "Ctrl+Alt+V" }];
    await s().init();
    expect(s().settings?.settings).toEqual(DEFAULT_SETTINGS);
    expect(s().toasts.map((t) => t.message)).toEqual(["No se pudo registrar Ctrl+Alt+V («Pegar evidencia»): otra aplicación ya la usa. Cámbialo en Ajustes › Atajos."]);
    expect(s().toasts[0].kind).toBe("error");
  });

  it("migra una sola vez el tester guardado en el equipo", async () => {
    const { repos, store, s } = setup("Luis");
    await s().init();
    expect(s().settings?.settings.profile.tester).toBe("Luis");
    expect(s().testerName()).toBe("Luis");
    expect(repos.settings.saves).toBe(1);
    await store.getState().init();
    expect(repos.settings.saves).toBe(1);
  });

  it("no migra en modo recuperación, pero sí carga los ajustes", async () => {
    const { repos, s } = setup("Luis");
    repos.data.status = { recovery: true, detail: "dañada" };
    await s().init();
    expect(s().settings).not.toBeNull();
    expect(repos.settings.saves).toBe(0);
  });

  it("guarda con toast y valida antes de llamar a Rust", async () => {
    const { repos, s } = setup();
    await s().init();
    const next = structuredClone(DEFAULT_SETTINGS);
    next.profile.email = "ana@";
    expect(await s().saveSettings(next)).toEqual({ ok: false, errors: { "profile.email": "«Correo» no es válido." } });
    expect(repos.settings.saves).toBe(0);
    next.profile.email = "ana@acme.com";
    expect(await s().saveSettings(next)).toEqual({ ok: true });
    expect(s().toasts.at(-1)?.message).toBe("Ajustes guardados.");
    expect(s().settings?.settings.profile.email).toBe("ana@acme.com");
  });

  it("un atajo rechazado deja el error en el formulario y no cambia los ajustes", async () => {
    const { repos, s } = setup();
    await s().init();
    repos.settings.refuse.add("Ctrl+Alt+P");
    const next = structuredClone(DEFAULT_SETTINGS);
    next.shortcuts.capture = "Ctrl+Alt+P";
    expect(await s().saveSettings(next)).toEqual({
      ok: false,
      errors: { _form: "No se pudo registrar Ctrl+Alt+P para «Capturar pantalla»: otra aplicación ya la usa. Se conservan los atajos anteriores." },
    });
    expect(s().settings?.settings.shortcuts.capture).toBe("Ctrl+Alt+S");
  });

  it("Nuevo caso desde el atajo abre el formulario o avisa sin proyecto", async () => {
    const { repos, s } = setup();
    await s().init();
    const off = await s().listenNewCase();
    repos.settings.emitNewCase();
    expect(s().toasts.at(-1)?.message).toBe("Crea o abre una matriz para registrar casos.");
    expect(s().caseFormRequest).toBeNull();
    await seedProject(repos);
    await s().init();
    repos.settings.emitNewCase();
    expect(s().caseFormRequest).toEqual({ caseId: null });
    s().closeCaseForm();
    expect(s().caseFormRequest).toBeNull();
    off();
    repos.settings.emitNewCase();
    expect(s().caseFormRequest).toBeNull();
  });
});
