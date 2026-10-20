import { CircleCheck, CircleDot } from "lucide-react";
import { useMemo } from "react";
import { shortRepo } from "../../domain/repo-routing";
import { useWorkspace } from "../store/context";

const chip = "inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-xs font-medium";

/** Issues vinculados al caso; si quedan capturas sin pegar (modo asistido), un chip abre el panel. */
export function IssueChips({ caseId, onOpenPending }: { caseId: string; onOpenPending: (pendingKey: string) => void }) {
  const projectId = useWorkspace((s) => s.project?.id);
  const projectLinks = useWorkspace((s) => (projectId ? s.github.links[projectId] : undefined));
  const pending = useWorkspace((s) => s.github.pending);
  const openIssue = useWorkspace((s) => s.openIssue);
  const links = useMemo(() => (projectLinks ?? []).filter((l) => l.caseId === caseId), [projectLinks, caseId]);
  if (links.length === 0) return null;

  return (
    <ul aria-label="Issues en GitHub" className="mt-2 flex flex-wrap gap-2">
      {links.map((l) => {
        const name = `${shortRepo(l.fullName)}#${l.issueNumber}`;
        const key = `${caseId}|${l.fullName}`;
        const left = pending[key]?.items.filter((i) => !i.done).length ?? 0;
        const closed = l.issueState === "closed";
        return (
          <li key={key} className="flex gap-1">
            <button
              type="button"
              aria-label={name}
              title={`${l.fullName}#${l.issueNumber}`}
              onClick={() => void openIssue(caseId, l.fullName)}
              className={`${chip} ${closed ? "border-purple-200 bg-purple-50 text-purple-800" : "border-green-200 bg-green-50 text-green-800"}`}
            >
              {closed ? <CircleCheck size={12} /> : <CircleDot size={12} />} {name}
            </button>
            {left > 0 && (
              <button type="button" onClick={() => onOpenPending(key)} className={`${chip} border-amber-200 bg-amber-50 text-amber-800`}>
                {left} capturas sin pegar
              </button>
            )}
          </li>
        );
      })}
    </ul>
  );
}
