import { describe, expect, it } from "vitest";
import type { CaseRecord } from "./cases";
import type { FieldDef } from "./fields";
import type { ProjectGithub } from "./github";
import { shortRepo, suggestRepo } from "./repo-routing";

const comp: FieldDef = {
  id: "f1", key: "componente", label: "Componente", type: "list", options: ["Front", "Back", "Otro"],
  required: false, clearOnDuplicate: false, width: 100, position: 0, archived: false,
};
const record = (v: string): CaseRecord => ({
  id: "c", projectId: "p", code: "A-1", title: "t", status: "Fallido", values: v ? { f1: v } : {},
  evidenceCount: 0, createdAt: "", updatedAt: "",
});
const config: ProjectGithub = {
  repos: [
    { fullName: "Org/Web", isDefault: false, defaultLabels: [] },
    { fullName: "org/api", isDefault: true, defaultLabels: [] },
  ],
  routingFieldId: "f1", routingMap: { Front: "org/web", Back: "org/api" }, evidenceRepo: null, routingStale: false,
};

describe("suggestRepo", () => {
  it("routes by the mapped option, returning the linked casing", () => {
    expect(suggestRepo(config, [comp], record("Front"))).toBe("Org/Web");
  });
  it("falls back to the default for unmapped or empty values", () => {
    expect(suggestRepo(config, [comp], record("Otro"))).toBe("org/api");
    expect(suggestRepo(config, [comp], record(""))).toBe("org/api");
  });
  it("falls back to the default when the column is archived or not a list", () => {
    expect(suggestRepo(config, [{ ...comp, archived: true }], record("Front"))).toBe("org/api");
    expect(suggestRepo(config, [{ ...comp, type: "text" }], record("Front"))).toBe("org/api");
    expect(suggestRepo(config, [], record("Front"))).toBe("org/api");
  });
  it("falls back to the default when there is no routing field or the target is unlinked", () => {
    expect(suggestRepo({ ...config, routingFieldId: null }, [comp], record("Front"))).toBe("org/api");
    expect(suggestRepo({ ...config, routingMap: { Front: "x/y" } }, [comp], record("Front"))).toBe("org/api");
  });
  it("returns null without repos", () => {
    expect(suggestRepo({ ...config, repos: [] }, [comp], record("Front"))).toBeNull();
  });
});

describe("shortRepo", () => {
  it("returns the part after the slash", () => {
    expect(shortRepo("org/api")).toBe("api");
    expect(shortRepo("solo")).toBe("solo");
  });
});
