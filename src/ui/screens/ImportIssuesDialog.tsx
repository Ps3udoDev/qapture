import { Dialog, DialogFooter } from "../components/Dialog";
import { btn } from "../components/styles";
import { useWorkspace } from "../store/context";

export function ImportIssuesDialog() {
  const issues = useWorkspace((s) => s.importIssues);
  const dismiss = useWorkspace((s) => s.dismissImportIssues);
  if (!issues) return null;
  return (
    <Dialog title="No se pudo importar" onClose={dismiss}>
      <p className="text-sm">{issues.title}</p>
      {issues.items.length > 0 && (
        <ul className="mt-3 list-disc space-y-1 pl-5 text-sm">
          {issues.items.map((item, index) => (
            <li key={index}>{item}</li>
          ))}
        </ul>
      )}
      <DialogFooter>
        <button type="button" className={btn.primary} onClick={dismiss}>
          Entendido
        </button>
      </DialogFooter>
    </Dialog>
  );
}
