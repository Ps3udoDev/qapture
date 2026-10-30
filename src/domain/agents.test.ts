import { expect, it } from "vitest";
import { RECOMMENDED, summarizeAgentChanges } from "./agents";
it("summarizes one change verbatim and many as a count", () => {
  expect(summarizeAgentChanges(["Creó EJ-001"])).toBe("Creó EJ-001");
  expect(summarizeAgentChanges(["a", "b", "c"])).toBe(
    "Un agente hizo 3 cambios",
  );
});
it("recommended enables only read and write", () => {
  expect(RECOMMENDED).toEqual({ read: true, write: true });
});
