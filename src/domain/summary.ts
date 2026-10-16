import { STATUSES, type Status } from "./status";

export function countByStatus(cases: { status: Status }[]): Record<Status, number> {
  const counts = Object.fromEntries(STATUSES.map((s) => [s, 0])) as Record<Status, number>;
  for (const c of cases) counts[c.status] += 1;
  return counts;
}

/** Aprobado / (Aprobado + Fallido + Bloqueado); null si no hay ejecuciones con veredicto. */
export function successRate(cases: { status: Status }[]): number | null {
  const c = countByStatus(cases);
  const finished = c.Aprobado + c.Fallido + c.Bloqueado;
  return finished === 0 ? null : c.Aprobado / finished;
}

export const formatRate = (rate: number | null) => (rate === null ? "—" : `${Math.round(rate * 100)} %`);
