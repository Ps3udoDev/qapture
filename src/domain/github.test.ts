import { describe, expect, it } from "vitest";
import type { FieldDef } from "./fields";
import { DEFAULT_LABELS, INSTALL_URL, REVOKE_URL, isRoutingStale, parseRepo, validateProjectGithub, type ProjectGithubInput } from "./github";

const field = (id: string, type: FieldDef["type"], archived: boolean): FieldDef => ({
  id,
  key: id,
  label: "Capa",
  type,
  options: ["Front", "Back"],
  required: false,
  clearOnDuplicate: false,
  width: 12,
  position: 0,
  archived,
});
const fields = (): FieldDef[] => [field("capa", "list", false), field("texto", "text", false), field("vieja", "list", true)];
const linked = (fullName: string, isDefault: boolean) => ({ fullName, isDefault, defaultLabels: ["bug"] });
const input = (): ProjectGithubInput => ({
  repos: [linked("TEAMS4SOFT/GMX-frontend", true), linked("TEAMS4SOFT/GMX-backend", false)],
  routingFieldId: "capa",
  routingMap: { Front: "teams4soft/gmx-frontend", Back: "TEAMS4SOFT/GMX-backend" },
  evidenceRepo: "TEAMS4SOFT/qa-evidencias",
});
const first = (i: ProjectGithubInput, f = fields()) => Object.values(validateProjectGithub(i, f))[0];

describe("constantes", () => {
  it("coinciden con las Global Constraints", () => {
    expect(INSTALL_URL).toBe("https://github.com/apps/qapture-qa/installations/new");
    expect(REVOKE_URL).toBe("https://github.com/settings/apps/authorizations");
    expect(DEFAULT_LABELS).toEqual(["bug", "qapture"]);
  });
});

describe("parseRepo", () => {
  it("acepta dueño/repositorio", () => {
    expect(parseRepo("TEAMS4SOFT/GMX-frontend")).toEqual({ owner: "TEAMS4SOFT", repo: "GMX-frontend" });
    expect(parseRepo("Ps3udoDev/booknow-mcp-service")).not.toBeNull();
  });
  it("rechaza formatos inválidos", () => {
    for (const bad of ["a/b/c", "/x", "x/", "-a/b", "a/..", "a/.", "", "abc", `${"a".repeat(40)}/x`]) expect(parseRepo(bad), bad).toBeNull();
  });
});

describe("validateProjectGithub", () => {
  it("una configuración válida pasa", () => {
    expect(validateProjectGithub(input(), fields())).toEqual({});
    expect(validateProjectGithub({ repos: [], routingFieldId: null, routingMap: {}, evidenceRepo: null }, fields())).toEqual({});
  });
  it("rechaza un repo inválido", () => {
    const i = input();
    i.repos[1].fullName = "sin-barra";
    expect(validateProjectGithub(i, fields())).toEqual({ repos: "«sin-barra» no es un repositorio válido (usa dueño/repositorio)." });
  });
  it("rechaza un repo repetido sin distinguir mayúsculas", () => {
    const i = input();
    i.repos[1].fullName = "teams4soft/GMX-FRONTEND";
    expect(first(i)).toBe("El repositorio «teams4soft/GMX-FRONTEND» está repetido.");
  });
  it("exige exactamente un repo predeterminado", () => {
    const none = input();
    none.repos[0].isDefault = false;
    expect(first(none)).toBe("Marca un repositorio como predeterminado.");
    const two = input();
    two.repos[1].isDefault = true;
    expect(first(two)).toBe("Marca un repositorio como predeterminado.");
  });
  it("limita las etiquetas", () => {
    const message = "Las etiquetas deben tener entre 1 y 50 caracteres, sin comas (máximo 10).";
    for (const labels of [[""], ["a".repeat(51)], ["a,b"], Array.from({ length: 11 }, (_, n) => `l${n}`)]) {
      const i = input();
      i.repos[0].defaultLabels = labels;
      expect(validateProjectGithub(i, fields())).toEqual({ labels: message });
    }
    const ok = input();
    ok.repos[0].defaultLabels = Array.from({ length: 10 }, (_, n) => `l${n}`);
    ok.repos[0].defaultLabels[0] = "ñ".repeat(50);
    expect(validateProjectGithub(ok, fields())).toEqual({});
  });
  it("la columna de ruteo debe existir y estar activa", () => {
    for (const id of ["no-existe", "vieja"]) {
      const i = input();
      i.routingFieldId = id;
      expect(validateProjectGithub(i, fields())).toEqual({ routing: "La columna de ruteo no existe en este proyecto." });
    }
  });
  it("la columna de ruteo debe ser Lista", () => {
    const i = input();
    i.routingFieldId = "texto";
    expect(validateProjectGithub(i, fields())).toEqual({ routing: "La columna de ruteo debe ser de tipo Lista." });
  });
  it("las claves del mapa deben ser opciones", () => {
    const i = input();
    i.routingMap.Móvil = "TEAMS4SOFT/GMX-frontend";
    expect(validateProjectGithub(i, fields())).toEqual({ routing: "«Móvil» no es una opción de «Capa»." });
  });
  it("los valores del mapa deben ser repos vinculados", () => {
    const i = input();
    i.routingMap.Back = "TEAMS4SOFT/otro";
    expect(validateProjectGithub(i, fields())).toEqual({ routing: "«TEAMS4SOFT/otro» no está vinculado al proyecto." });
  });
  it("el repo de evidencias debe ser válido", () => {
    const i = input();
    i.evidenceRepo = "a/..";
    expect(validateProjectGithub(i, fields())).toEqual({ evidenceRepo: "«a/..» no es un repositorio válido (usa dueño/repositorio)." });
  });
  it("sin columna de ruteo el mapa se descarta", () => {
    const i = input();
    i.routingFieldId = null;
    expect(validateProjectGithub(i, fields())).toEqual({});
  });
  it("el primer mensaje sigue el orden de Rust", () => {
    const i = input();
    i.repos[0].defaultLabels = [""];
    i.routingFieldId = "texto";
    i.evidenceRepo = "x";
    expect(Object.keys(validateProjectGithub(i, fields()))).toEqual(["labels", "routing", "evidenceRepo"]);
  });
});

describe("isRoutingStale", () => {
  it("sigue la columna de ruteo", () => {
    const config = { ...input(), routingStale: false };
    expect(isRoutingStale(config, fields())).toBe(false);
    const archived = fields();
    archived[0].archived = true;
    expect(isRoutingStale(config, archived)).toBe(true);
    const renamed = fields();
    renamed[0].options = ["Frontend", "Back"];
    expect(isRoutingStale(config, renamed)).toBe(true);
    const retyped = fields();
    retyped[0].type = "text";
    expect(isRoutingStale(config, retyped)).toBe(true);
    expect(isRoutingStale(config, fields().slice(1))).toBe(true);
    expect(isRoutingStale({ ...config, routingFieldId: null }, [])).toBe(false);
  });
});
