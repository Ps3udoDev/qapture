import { Bug, Copy, Pencil, Trash2 } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { activeFields } from "../../domain/fields";
import { EvidenceGallery } from "../gallery/EvidenceGallery";
import { filesFrom, hasFiles } from "../gallery/files";
import { useEvidenceIntake } from "../gallery/useEvidenceIntake";
import { ConfirmDialog } from "../components/ConfirmDialog";
import { displayValue } from "../components/FieldControl";
import { StatusBadge } from "../components/StatusBadge";
import { btn } from "../components/styles";
import { useWorkspace } from "../store/context";
import { AssistedPastePanel } from "./AssistedPastePanel";
import { IssueChips } from "./IssueChips";
import { ReportIssueDialog } from "./ReportIssueDialog";

export interface GithubNav {
  onOpenAccountSettings: () => void;
  onOpenProjectGithub: () => void;
}

export function CaseDetail({ onEdit, onOpenAccountSettings, onOpenProjectGithub }: { onEdit: (caseId: string) => void } & GithubNav) {
  const project = useWorkspace((s) => s.project);
  const cases = useWorkspace((s) => s.cases);
  const pushToast = useWorkspace((s) => s.pushToast);
  const selectedId = useWorkspace((s) => s.selectedCaseId);
  const duplicateCase = useWorkspace((s) => s.duplicateCase);
  const deleteCase = useWorkspace((s) => s.deleteCase);
  const [confirming, setConfirming] = useState(false);
  const [busy, setBusy] = useState(false);
  const intake = useEvidenceIntake();
  const [dropping, setDropping] = useState(false);
  const [reporting, setReporting] = useState(false);
  const [pendingKey, setPendingKey] = useState<string | null>(null);
  const current = cases.find((c) => c.id === selectedId);
  useEffect(() => {
    setReporting(false);
    setPendingKey(null);
  }, [selectedId]);
  const closePanel = useCallback(() => setPendingKey(null), []);

  if (!project) return null;
  if (!current) {
    return (
      <section
        aria-label="Detalle del caso"
        className="grid flex-1 place-items-center text-t4s-gray"
        onDragOver={(e) => {
          if (hasFiles(e.dataTransfer)) e.preventDefault();
        }}
        onDrop={(e) => {
          if (!hasFiles(e.dataTransfer)) return;
          e.preventDefault();
          pushToast("error", "Crea o selecciona un caso antes de agregar evidencias.");
        }}
      >
        Selecciona un caso para ver su detalle.
      </section>
    );
  }

  return (
    <section
      aria-label="Detalle del caso"
      className={`flex-1 overflow-y-auto p-6 ${dropping ? "bg-t4s-blue/5 outline-2 outline-dashed outline-t4s-blue" : ""}`}
      onDragOver={(e) => {
        if (!hasFiles(e.dataTransfer)) return;
        e.preventDefault();
        setDropping(true);
      }}
      onDragLeave={() => setDropping(false)}
      onDrop={(e) => {
        if (!hasFiles(e.dataTransfer)) return;
        e.preventDefault();
        setDropping(false);
        void intake(filesFrom(e.dataTransfer));
      }}
    >
      <div className="mx-auto max-w-4xl">
        <div className="flex items-start justify-between gap-4">
          <div>
            <p className="font-mono text-sm text-t4s-blue">{current.code}</p>
            <h2 className="text-2xl font-semibold">{current.title}</h2>
            <div className="mt-2">
              <StatusBadge status={current.status} />
            </div>
            <IssueChips caseId={current.id} onOpenPending={setPendingKey} />
          </div>
          <div className="flex gap-2">
            <button type="button" className={btn.secondary} onClick={() => onEdit(current.id)}>
              <Pencil size={16} /> Editar
            </button>
            <button
              type="button"
              className={btn.secondary}
              disabled={busy}
              onClick={async () => {
                setBusy(true);
                try {
                  await duplicateCase(current.id);
                } finally {
                  setBusy(false);
                }
              }}
            >
              <Copy size={16} /> Duplicar
            </button>
            <button type="button" className={btn.secondary} onClick={() => setReporting(true)}>
              <Bug size={16} /> Reportar en GitHub
            </button>
            <button type="button" aria-label="Eliminar caso" className={btn.ghost} onClick={() => setConfirming(true)}>
              <Trash2 size={16} />
            </button>
          </div>
        </div>
        <dl className="mt-6 grid grid-cols-2 gap-x-6 gap-y-4 rounded-xl border border-t4s-line bg-white p-5">
          {activeFields(project.fields).map((f) => (
            <div key={f.id} className={f.type === "longtext" ? "col-span-2" : ""}>
              <dt className="text-xs font-medium uppercase tracking-wide text-t4s-gray">{f.label}</dt>
              <dd className="mt-1 whitespace-pre-wrap text-sm">{displayValue(f, current.values[f.id] ?? "")}</dd>
            </div>
          ))}
        </dl>
        <EvidenceGallery caseRecord={current} />
      </div>
      {reporting && (
        <ReportIssueDialog
          caseRecord={current}
          onClose={() => setReporting(false)}
          onOpenAccountSettings={onOpenAccountSettings}
          onOpenProjectGithub={onOpenProjectGithub}
          onAssisted={setPendingKey}
        />
      )}
      {pendingKey && <AssistedPastePanel pendingKey={pendingKey} onClose={closePanel} />}
      {confirming && (
        <ConfirmDialog
          title={`Eliminar ${current.code}`}
          message={
            current.evidenceCount
              ? `El caso y sus ${current.evidenceCount} evidencias se eliminarán de forma permanente. Su código no se reutilizará.`
              : "El caso se eliminará de forma permanente. Su código no se reutilizará."
          }
          confirmLabel="Eliminar caso"
          onCancel={() => setConfirming(false)}
          onConfirm={async () => {
            setConfirming(false);
            await deleteCase(current.id);
          }}
        />
      )}
    </section>
  );
}
