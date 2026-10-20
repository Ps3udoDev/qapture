import { ClipboardCopy, ExternalLink } from "lucide-react";
import { useEffect } from "react";
import { Dialog, DialogFooter } from "../components/Dialog";
import { btn } from "../components/styles";
import { useWorkspace } from "../store/context";

/** Modo asistido: el tester copia cada captura y la pega en GitHub. `pendingKey` = `caseId|owner/repo`. */
export function AssistedPastePanel({ pendingKey, onClose }: { pendingKey: string; onClose: () => void }) {
  const entry = useWorkspace((s) => s.github.pending[pendingKey]);
  const evidence = useWorkspace((s) => s.evidence);
  const imageUrl = useWorkspace((s) => s.imageUrl);
  const copy = useWorkspace((s) => s.copyPendingEvidence);
  const markDone = useWorkspace((s) => s.markPendingDone);
  const openIssue = useWorkspace((s) => s.openIssue);

  // Con todas marcadas como pegadas el slice borra el pendiente: el panel ya no tiene nada que mostrar.
  useEffect(() => {
    if (!entry) onClose();
  }, [entry, onClose]);
  if (!entry) return null;

  const split = pendingKey.indexOf("|");
  const caseId = pendingKey.slice(0, split);
  const repo = pendingKey.slice(split + 1);

  return (
    <Dialog title="Termina de adjuntar las capturas" kicker={repo} onClose={onClose}>
      <div className="space-y-4">
        {entry.reason && <p className="text-sm text-t4s-gray">{entry.reason}</p>}
        <p className="text-sm">Copia cada captura y pégala con Ctrl+V en un comentario del issue (o editando la descripción).</p>
        <ul className="space-y-2">
          {entry.items.map((item) => {
            const e = evidence.find((x) => x.id === item.evidenceId);
            return (
              <li key={item.evidenceId} className="flex items-center gap-3 rounded-md border border-t4s-line p-2">
                {e && <img src={imageUrl(e.renderedPath, e.updatedAt)} alt="" className="h-12 w-20 rounded object-cover" />}
                <span className="min-w-0 flex-1 truncate text-sm">
                  <span className="font-medium">Captura {item.n}</span>
                  {e?.description ? ` · ${e.description}` : ""}
                </span>
                <button type="button" className={btn.secondary} onClick={() => void copy(pendingKey, item.evidenceId)}>
                  <ClipboardCopy size={14} /> Copiar
                </button>
                <label className="inline-flex items-center gap-1.5 text-sm">
                  <input type="checkbox" checked={item.done} onChange={(ev) => markDone(pendingKey, item.evidenceId, ev.target.checked)} />
                  Pegada
                </label>
              </li>
            );
          })}
        </ul>
        <DialogFooter>
          <button type="button" className={btn.secondary} onClick={() => void openIssue(caseId, repo)}>
            <ExternalLink size={14} /> Abrir issue
          </button>
          <button type="button" className={btn.primary} onClick={onClose}>
            Cerrar
          </button>
        </DialogFooter>
      </div>
    </Dialog>
  );
}
