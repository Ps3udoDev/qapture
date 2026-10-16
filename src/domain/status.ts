export const STATUSES = ["No ejecutado", "En ejecución", "Por documentar", "Aprobado", "Fallido", "Bloqueado"] as const;
export type Status = (typeof STATUSES)[number];
export const DEFAULT_STATUS: Status = "No ejecutado";

export function isStatus(value: string): value is Status {
  return (STATUSES as readonly string[]).includes(value);
}
