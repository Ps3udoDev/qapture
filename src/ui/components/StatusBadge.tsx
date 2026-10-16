import type { Status } from "../../domain/status";

const TONES: Record<Status, string> = {
  "No ejecutado": "bg-t4s-cream text-t4s-gray",
  "En ejecución": "bg-t4s-blue/10 text-t4s-blue",
  "Por documentar": "bg-amber-50 text-amber-800",
  Aprobado: "bg-t4s-lime/25 text-t4s-charcoal",
  Fallido: "bg-red-50 text-red-800",
  Bloqueado: "bg-yellow-100 text-yellow-900",
};

export function StatusBadge({ status }: { status: Status }) {
  return <span className={`inline-block rounded-full px-2 py-0.5 text-xs font-medium ${TONES[status]}`}>{status}</span>;
}
