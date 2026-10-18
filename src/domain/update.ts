/** F7: actualizaciones. Espejo de `system/updater.rs` y de `UpdateInfoDto`. */

export interface UpdateInfo {
  version: string;
  currentVersion: string;
  /** Cuerpo del release de GitHub (markdown); se muestra con `plainNotes`, nunca como HTML. */
  notes: string;
  date: string | null;
  /** En la versión portable no se instala: se descarga el ZIP nuevo. */
  portable: boolean;
}

export interface UpdateProgress {
  downloaded: number;
  /** `null` si el servidor no informa el tamaño. */
  total: number | null;
}

export const PORTABLE_MESSAGE = "Estás usando la versión portable: descarga el ZIP nuevo y reemplaza Qapture.exe. Tus datos en .\\data se conservan.";
export const SAVE_FIRST_MESSAGE = "Guarda o descarta la captura abierta antes de actualizar.";

export const availableMessage = (info: Pick<UpdateInfo, "version" | "currentVersion">) =>
  `Qapture ${info.version} está disponible (tienes ${info.currentVersion}).`;

export const upToDateMessage = (version: string) => `Qapture está al día (versión ${version}).`;

/** Porcentaje entero 0-100, o `null` si no se conoce el total. */
export function progressPercent(downloaded: number, total: number | null): number | null {
  if (total === null || total <= 0) return null;
  return Math.min(100, Math.floor((downloaded / total) * 100));
}

export function progressPercentMessage(percent: number | null): string {
  return percent === null ? "Descargando actualización…" : `Descargando actualización… ${percent} %`;
}

export const progressMessage = (downloaded: number, total: number | null): string => progressPercentMessage(progressPercent(downloaded, total));

/**
 * Notas del release (markdown de GitHub) como texto plano: sin marcas de título `#`, sin `**` y con
 * `[texto](url)` reducido a `texto`. Conserva los saltos de línea; el resultado se muestra como texto.
 */
export function plainNotes(markdown: string): string {
  return markdown
    .replace(/\r\n/g, "\n")
    .split("\n")
    .map((line) =>
      line
        .replace(/^\s{0,3}#{1,6}\s+/, "")
        .replace(/\*\*/g, "")
        .replace(/\[([^\]]*)\]\([^)]*\)/g, "$1"),
    )
    .join("\n")
    .trim();
}

export function parseVersion(version: string): [number, number, number] | null {
  const parts = version.trim().replace(/^v/, "").split(".");
  if (parts.length !== 3 || parts.some((p) => !/^\d+$/.test(p))) return null;
  return [Number(parts[0]), Number(parts[1]), Number(parts[2])];
}

/** Igual que `should_prompt` en Rust: avisa sin versión pospuesta o si `found` es más nueva (X.Y.Z numérico). */
export function shouldPrompt(found: string, dismissed: string | null): boolean {
  if (dismissed === null) return true;
  const a = parseVersion(found);
  const b = parseVersion(dismissed);
  if (!a || !b) return found !== dismissed;
  for (let i = 0; i < 3; i++) if (a[i] !== b[i]) return a[i] > b[i];
  return false;
}
