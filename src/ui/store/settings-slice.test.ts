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
    expect(s().toasts.map((t) => t.message)).not.toContain("Ajustes guardados.");
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

  it("restaurar un respaldo recarga los ajustes de la base restaurada", async () => {
    const { repos, s } = setup();
    await s().init();
    // Rust restaura la base: los ajustes que devuelve `get_settings` ya son los del respaldo.
    repos.settings.snapshot.settings.profile.tester = "Marta";
    repos.settings.snapshot.settings.system.escHides = true;
    expect(await s().restoreBackup("qapture-diario-20261015.db")).toBe(true);
    expect(s().settings?.settings.profile.tester).toBe("Marta");
    expect(s().settings?.settings.system.escHides).toBe(true);
    expect(s().testerName()).toBe("Marta");
  });

  it("salir de recuperación recarga los ajustes y entonces migra el tester", async () => {
    const { repos, s } = setup("Luis");
    repos.data.status = { recovery: true, detail: "dañada" };
    await s().init();
    expect(repos.settings.saves).toBe(0);
    repos.settings.snapshot.settings.export.imageMaxWidth = 800;
    await s().recover(null);
    expect(s().recovery).toBeNull();
    expect(s().settings?.settings.export.imageMaxWidth).toBe(800);
    expect(s().settings?.settings.profile.tester).toBe("Luis");
    expect(repos.settings.saves).toBe(1);
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

  it("un atajo que sigue ocupado no impide guardar el resto y se sigue avisando", async () => {
    const { repos, s } = setup();
    repos.settings.failuresAtStart = [{ action: "paste", accelerator: "Ctrl+Alt+V" }];
    repos.settings.refuse.add("Ctrl+Alt+V");
    await s().init();
    const next = structuredClone(DEFAULT_SETTINGS);
    next.profile.tester = "Ana";
    expect(await s().saveSettings(next)).toEqual({ ok: true });
    expect(s().settings?.settings.profile.tester).toBe("Ana");
    expect(s().settings?.shortcutFailures).toEqual([{ action: "paste", accelerator: "Ctrl+Alt+V" }]);
    expect(s().toasts.slice(-2).map((t) => t.message)).toEqual([
      "Ajustes guardados.",
      "No se pudo registrar Ctrl+Alt+V («Pegar evidencia»): otra aplicación ya la usa. Cámbialo en Ajustes › Atajos.",
    ]);
  });

  it("guardar reintenta el atajo que falló al arrancar y deja de avisar si ya quedó libre", async () => {
    const { repos, s } = setup();
    repos.settings.failuresAtStart = [{ action: "paste", accelerator: "Ctrl+Alt+V" }];
    await s().init();
    expect(await s().saveSettings(structuredClone(DEFAULT_SETTINGS))).toEqual({ ok: true });
    expect(s().settings?.shortcutFailures).toEqual([]);
    expect(s().toasts.at(-1)?.message).toBe("Ajustes guardados.");
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
