import { describe, expect, it } from "vitest";
import type { CaseRecord } from "./cases";
import type { FieldDef } from "./fields";
import { buildIssue, buildRetestComment } from "./issue-template";

const field = (id: string, label: string, type: FieldDef["type"], position: number, extra: Partial<FieldDef> = {}): FieldDef => ({
  id, key: id, label, type, options: type === "list" ? ["Front", "Back"] : [], required: false,
  clearOnDuplicate: false, width: 100, position, archived: false, ...extra,
});

const fields: FieldDef[] = [
  field("f1", "Módulo", "text", 0),
  field("f2", "Componente", "list", 1),
  field("f3", "Bloqueante", "boolean", 2),
  field("f4", "Pasos", "longtext", 3),
  field("f5", "Viejo", "text", 4, { archived: true }),
  field("f6", "Vacío", "text", 5),
];

const caseRecord = (over: Partial<CaseRecord> = {}): CaseRecord => ({
  id: "c1", projectId: "p1", code: "EJ-014", title: "Login falla", status: "Fallido",
  values: { f1: "Acceso", f2: "Front", f3: "true", f4: "1. Abrir\n2. Entrar", f5: "x" },
  evidenceCount: 2, createdAt: "", updatedAt: "", ...over,
});

const base = {
  fields, project: { name: "Portal", client: "ACME" }, evidences: [{ description: "Pantalla de error" }, { description: "" }],
  tester: "ana", role: "QA", appVersion: "1.1.0", today: "2026-10-20",
};

describe("buildIssue", () => {
  it("builds the exact body", () => {
    const { title, body } = buildIssue({ ...base, caseRecord: caseRecord() });
    expect(title).toBe("[EJ-014] Login falla");
    expect(body.split("\n")).toEqual([
      "**Proyecto:** Portal · **Cliente:** ACME · **Estado:** Fallido",
      "",
      "**Reportado por:** ana (QA) vía Qapture 1.1.0 · 2026-10-20",
      "",
      "**Módulo:** Acceso",
      "**Componente:** Front",
      "**Bloqueante:** Sí",
      "",
      "### Pasos",
      "1. Abrir",
      "2. Entrar",
      "",
      "### Evidencias",
      "",
      "**1.** Pantalla de error",
      "{{img:1}}",
      "",
      "**2.**",
      "{{img:2}}",
      "",
      "<!-- qapture:case=EJ-014;v=1 -->",
    ]);
  });

  it("omits client, role and evidences when absent", () => {
    const { body } = buildIssue({
      ...base, project: { name: "Portal", client: "" }, role: "", evidences: [],
      caseRecord: caseRecord({ values: { f3: "false" } }),
    });
    expect(body.split("\n")).toEqual([
      "**Proyecto:** Portal · **Estado:** Fallido",
      "",
      "**Reportado por:** ana vía Qapture 1.1.0 · 2026-10-20",
      "",
      "**Bloqueante:** No",
      "",
      "<!-- qapture:case=EJ-014;v=1 -->",
    ]);
    expect(body).not.toContain("Evidencias");
  });

  it("omits the field block when no field has a value", () => {
    const { body } = buildIssue({ ...base, evidences: [], caseRecord: caseRecord({ values: {} }) });
    expect(body.split("\n\n")).toHaveLength(3);
  });

  it("trims the title to 256 characters", () => {
    const { title } = buildIssue({ ...base, caseRecord: caseRecord({ title: "x".repeat(400) }) });
    expect([...title]).toHaveLength(256);
    expect(title.startsWith("[EJ-014] xxx")).toBe(true);
  });
});

describe("buildRetestComment", () => {
  it("builds the exact comment", () => {
    const text = buildRetestComment({
      caseRecord: caseRecord(), evidences: [{ description: "Otra vez" }], tester: "ana", today: "2026-10-21",
    });
    expect(text.split("\n")).toEqual([
      "Re-test el 2026-10-21 por ana: estado Fallido.",
      "",
      "<!-- notas -->",
      "",
      "### Evidencias",
      "",
      "**1.** Otra vez",
      "{{img:1}}",
      "",
      "<!-- qapture:case=EJ-014;v=1 -->",
    ]);
  });

  it("skips evidences when none", () => {
    const text = buildRetestComment({ caseRecord: caseRecord(), evidences: [], tester: "ana", today: "2026-10-21" });
    expect(text).toBe("Re-test el 2026-10-21 por ana: estado Fallido.\n\n<!-- notas -->\n\n<!-- qapture:case=EJ-014;v=1 -->");
  });
});
