import { describe, expect, it } from "vitest";
import { bump, changelogSection, checkVersions, insertSection, notesFor, readVersions } from "./release-lib.mjs";

const files = (v, lockExtra = "") => ({
  pkg: `{\n  "name": "qapture",\n  "version": "${v}"\n}\n`,
  cargoToml: `[package]\nname = "qapture"\nversion = "${v}"\n\n[dependencies]\nserde = { version = "1" }\n`,
  tauriConf: `{\n  "productName": "Qapture",\n  "version": "${v}"\n}\n`,
  cargoLock: `[[package]]\nname = "other"\nversion = "${v}"\n\n[[package]]\nname = "qapture"\nversion = "${v}"\n${lockExtra}`,
});

describe("release-lib", () => {
  it("lee la versión de los cuatro archivos", () => {
    expect(readVersions(files("0.6.0"))).toEqual({
      pkg: "0.6.0",
      cargoToml: "0.6.0",
      tauriConf: "0.6.0",
      cargoLock: "0.6.0",
    });
  });

  it("detecta versiones desalineadas y nombra el archivo", () => {
    const v = { pkg: "1.0.0", cargoToml: "0.6.0", tauriConf: "1.0.0", cargoLock: "1.0.0" };
    expect(checkVersions(v)).toEqual(["src-tauri/Cargo.toml tiene 0.6.0 y package.json 1.0.0."]);
  });

  it("devuelve vacío si todo coincide", () => {
    const v = readVersions(files("0.6.0"));
    expect(checkVersions(v)).toEqual([]);
    expect(checkVersions(v, "v0.6.0")).toEqual([]);
  });

  it("exige que el tag coincida", () => {
    const v = { pkg: "1.0.0", cargoToml: "1.0.0", tauriConf: "1.0.0", cargoLock: "1.0.0" };
    expect(checkVersions(v, "v1.0.1")).toEqual(["El tag v1.0.1 no coincide con la versión 1.0.0."]);
  });

  it("rechaza versiones que no son X.Y.Z", () => {
    const v = { pkg: "1.0.0-rc.1", cargoToml: "1.0.0-rc.1", tauriConf: "1.0.0-rc.1", cargoLock: "1.0.0-rc.1" };
    expect(checkVersions(v)).toEqual(["La versión debe ser X.Y.Z (el MSI no admite prereleases): 1.0.0-rc.1."]);
  });

  it("el bump solo toca la versión de qapture en Cargo.lock", () => {
    const out = bump(files("0.6.0"), "1.0.0");
    expect(out.cargoLock).toBe(
      `[[package]]\nname = "other"\nversion = "0.6.0"\n\n[[package]]\nname = "qapture"\nversion = "1.0.0"\n`,
    );
    expect(readVersions(out)).toEqual({ pkg: "1.0.0", cargoToml: "1.0.0", tauriConf: "1.0.0", cargoLock: "1.0.0" });
  });

  it("el bump no toca versiones de dependencias ni el formato", () => {
    const out = bump(files("0.6.0"), "1.0.0");
    expect(out.cargoToml).toBe(
      `[package]\nname = "qapture"\nversion = "1.0.0"\n\n[dependencies]\nserde = { version = "1" }\n`,
    );
    expect(out.pkg).toBe(`{\n  "name": "qapture",\n  "version": "1.0.0"\n}\n`);
  });

  it("el bump rechaza una versión inválida", () => {
    expect(() => bump(files("0.6.0"), "1.0")).toThrow(/X\.Y\.Z/);
  });

  it("agrupa feat y fix y omite el resto", () => {
    const subjects = ["feat(f7): updater", "fix(f7): x", "chore: y", "Merge pull request #8"];
    expect(changelogSection("1.0.0", "2026-10-20", subjects)).toBe(
      "## 1.0.0 — 2026-10-20\n\n### Nuevo\n- updater\n\n### Correcciones\n- x\n",
    );
  });

  it("avisa cuando no hay cambios destacados", () => {
    expect(changelogSection("1.0.1", "2026-10-21", ["chore: y"])).toBe(
      "## 1.0.1 — 2026-10-21\n\n- Sin cambios destacados.\n",
    );
  });

  it("inserta la sección nueva debajo del título", () => {
    const log = "# Cambios\n\n## 0.6.0\n\nviejo\n";
    expect(insertSection(log, "## 1.0.0\n\n- a\n")).toBe("# Cambios\n\n## 1.0.0\n\n- a\n\n## 0.6.0\n\nviejo\n");
  });

  it("extrae las notas de una versión del CHANGELOG", () => {
    const log = "# Cambios\n\n## 1.0.0 — 2026-10-20\n\n### Nuevo\n- a\n\n## 0.6.0\n\nviejo\n";
    expect(notesFor(log, "1.0.0")).toBe("### Nuevo\n- a\n");
    expect(notesFor(log, "0.6.0")).toBe("viejo\n");
    expect(notesFor(log, "2.0.0")).toBe("");
  });
});
