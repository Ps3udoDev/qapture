import type { CaseRecord } from "./cases";
import type { FieldDef } from "./fields";
import type { ProjectGithub } from "./github";

/** Igual que `eq_ignore_ascii_case` de Rust: solo pliega A-Z. */
const asciiLower = (s: string) => s.replace(/[A-Z]/g, (c) => c.toLowerCase());

export const shortRepo = (fullName: string) => fullName.slice(fullName.indexOf("/") + 1);

/** Repo sugerido para el caso: el mapeado por la columna de ruteo o, si no, el predeterminado. */
export function suggestRepo(config: ProjectGithub, fields: FieldDef[], caseRecord: CaseRecord): string | null {
  if (config.repos.length === 0) return null;
  const fallback = (config.repos.find((r) => r.isDefault) ?? config.repos[0]).fullName;
  if (config.routingFieldId === null) return fallback;
  const field = fields.find((f) => f.id === config.routingFieldId);
  if (!field || field.archived || field.type !== "list") return fallback;
  const value = caseRecord.values[field.id] ?? "";
  const target = value === "" || !Object.hasOwn(config.routingMap, value) ? undefined : config.routingMap[value].trim();
  if (target === undefined) return fallback;
  const linked = config.repos.find((r) => asciiLower(r.fullName.trim()) === asciiLower(target));
  return linked ? linked.fullName : fallback;
}
