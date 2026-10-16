import { expect, it } from "vitest";
import type { Status } from "./status";
import { countByStatus, formatRate, successRate } from "./summary";

const cases = (statuses: Status[]) => statuses.map((status) => ({ status }));

it("cuenta por estado", () => {
  const counts = countByStatus(cases(["Aprobado", "Aprobado", "Fallido"]));
  expect(counts.Aprobado).toBe(2);
  expect(counts.Fallido).toBe(1);
  expect(counts["No ejecutado"]).toBe(0);
});

it("éxito = Aprobado / (Aprobado + Fallido + Bloqueado)", () => {
  const list = cases(["Aprobado", "Aprobado", "Fallido", "Bloqueado", "No ejecutado", "Por documentar"]);
  expect(successRate(list)).toBe(0.5);
  expect(successRate(cases(["No ejecutado"]))).toBeNull();
  expect(formatRate(0.5)).toBe("50 %");
  expect(formatRate(null)).toBe("—");
});
