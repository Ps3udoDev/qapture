import { useState } from "react";
import { CaseDetail } from "./CaseDetail";
import { CaseFormDialog } from "./CaseFormDialog";
import { CaseListPane } from "./CaseListPane";

export function Workspace() {
  const [editing, setEditing] = useState<{ caseId?: string } | null>(null);
  return (
    <main className="flex min-h-0 flex-1">
      <CaseListPane onNewCase={() => setEditing({})} />
      <CaseDetail onEdit={(caseId) => setEditing({ caseId })} />
      {editing && <CaseFormDialog caseId={editing.caseId} onClose={() => setEditing(null)} />}
    </main>
  );
}
