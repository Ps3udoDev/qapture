import { availableMessage, PORTABLE_MESSAGE, progressMessage } from "../../domain/update";
import { Dialog, DialogFooter } from "../components/Dialog";
import { btn } from "../components/styles";
import { useWorkspace } from "../store/context";

/** Aviso de una versión nueva. Se monta solo cuando hay una sin posponer (ver `isUpdatePromptOpen`). */
export function UpdateDialog() {
  const info = useWorkspace((s) => s.update.available);
  const status = useWorkspace((s) => s.update.status);
  const progress = useWorkspace((s) => s.update.progress);
  const installUpdate = useWorkspace((s) => s.installUpdate);
  const dismissUpdate = useWorkspace((s) => s.dismissUpdate);
  const openLink = useWorkspace((s) => s.openLink);
  if (!info) return null;
  const downloading = status === "downloading";

  return (
    <Dialog title="Nueva versión disponible" kicker="Qapture" onClose={downloading ? () => undefined : dismissUpdate}>
      <div className="grid gap-3">
        <p className="text-sm">{availableMessage(info)}</p>
        {info.notes && <p className="whitespace-pre-line text-sm text-t4s-gray">{info.notes}</p>}
        {info.portable && <p className="text-sm">{PORTABLE_MESSAGE}</p>}
        {downloading && (
          <div role="status" className="grid gap-1">
            <p className="text-sm">{progressMessage(progress ?? 0, progress === null ? null : 100)}</p>
            {progress !== null && (
              <div className="h-2 overflow-hidden rounded bg-t4s-line" aria-hidden="true">
                <div className="h-full bg-t4s-blue" style={{ width: `${progress}%` }} />
              </div>
            )}
          </div>
        )}
      </div>
      <DialogFooter>
        <button type="button" className={btn.secondary} disabled={downloading} onClick={dismissUpdate}>
          Después
        </button>
        {info.portable ? (
          <button type="button" className={btn.primary} onClick={() => void openLink("releases")}>
            Descargar
          </button>
        ) : (
          <button type="button" className={btn.primary} disabled={downloading} onClick={() => void installUpdate()}>
            Instalar y reiniciar
          </button>
        )}
      </DialogFooter>
    </Dialog>
  );
}
