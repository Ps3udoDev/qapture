import type { CaseRecord } from "./cases";
import { activeFields, type FieldDef } from "./fields";

const MAX_TITLE = 256;

export interface IssueEvidence {
  description: string;
}

/** Valor tal cual se guardó; los booleanos como `Sí`/`No` (no es la vista de la UI, que formatea fechas). */
function issueValue(field: FieldDef, value: string): string {
  return field.type === "boolean" ? (value === "true" ? "Sí" : "No") : value;
}

const marker = (code: string) => `<!-- qapture:case=${code};v=1 -->`;

function evidenceSection(evidences: IssueEvidence[]): string | null {
  if (evidences.length === 0) return null;
  const items = evidences.map((e, i) => {
    const n = i + 1;
    const desc = e.description.trim();
    return `${desc ? `**${n}.** ${desc}` : `**${n}.**`}\n{{img:${n}}}`;
  });
  return `### Evidencias\n\n${items.join("\n\n")}`;
}

export function buildIssue(input: {
  caseRecord: CaseRecord;
  fields: FieldDef[];
  project: { name: string; client: string };
  evidences: IssueEvidence[];
  tester: string;
  role: string;
  appVersion: string;
  today: string;
}): { title: string; body: string } {
  const { caseRecord: c, project } = input;
  const title = [`[${c.code}] ${c.title}`.trimEnd()].map((t) => [...t].slice(0, MAX_TITLE).join(""))[0];

  const client = project.client.trim();
  const role = input.role.trim();
  const header1 =
    `**Proyecto:** ${project.name}` + (client ? ` · **Cliente:** ${client}` : "") + ` · **Estado:** ${c.status}`;
  const header2 = `**Reportado por:** ${input.tester}` + (role ? ` (${role})` : "") + ` vía Qapture ${input.appVersion} · ${input.today}`;

  const withValue = activeFields(input.fields)
    .map((f) => ({ f, v: c.values[f.id] ?? "" }))
    .filter(({ v }) => v.trim() !== "");
  const lines = withValue.filter(({ f }) => f.type !== "longtext").map(({ f, v }) => `**${f.label}:** ${issueValue(f, v)}`);
  const longs = withValue.filter(({ f }) => f.type === "longtext").map(({ f, v }) => `### ${f.label}\n${v}`);

  const parts = [header1, header2];
  if (lines.length > 0) parts.push(lines.join("\n"));
  parts.push(...longs);
  const evidence = evidenceSection(input.evidences);
  if (evidence) parts.push(evidence);
  parts.push(marker(c.code));
  return { title, body: parts.join("\n\n") };
}

export function buildRetestComment(input: {
  caseRecord: CaseRecord;
  evidences: IssueEvidence[];
  tester: string;
  today: string;
}): string {
  const c = input.caseRecord;
  const parts = [`Re-test el ${input.today} por ${input.tester}: estado ${c.status}.`, "<!-- notas -->"];
  const evidence = evidenceSection(input.evidences);
  if (evidence) parts.push(evidence);
  parts.push(marker(c.code));
  return parts.join("\n\n");
}
