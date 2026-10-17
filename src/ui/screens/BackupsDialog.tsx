import { DatabaseBackup } from "lucide-react";
import { useEffect, useState } from "react";
import { BACKUP_KIND_LABEL, formatBackupDate, formatSize, type Backup } from "../../domain/data";
import { ConfirmDialog } from "../components/ConfirmDialog";
import { Dialog } from "../components/Dialog";
import { btn } from "../components/styles";
import { useWorkspace } from "../store/context";

export function BackupTable({ backups, disabled, onRestore }: { backups: Backup[]; disabled: boolean; onRestore: (backup: Backup) => void }) {
  if (backups.length === 0) return <p className="text-sm text-t4s-gray">Todavía no hay respaldos en esta computadora.</p>;
  return (
    <table className="w-full text-sm">
      <thead>
        <tr className="text-left text-xs uppercase tracking-wide text-t4s-gray">
          <th className="py-2 font-medium">Fecha</th>
          <th className="font-medium">Tipo</th>
          <th className="font-medium">Tamaño</th>
          <th>
            <span className="sr-only">Acciones</span>
          </th>
        </tr>
      </thead>
      <tbody>
        {backups.map((b) => (
          <tr key={b.fileName} className="border-t border-t4s-line">
            <td className="py-2">{formatBackupDate(b.createdAt)}</td>
            <td>{BACKUP_KIND_LABEL[b.kind]}</td>
            <td>{formatSize(b.sizeBytes)}</td>
            <td className="text-right">
              <button type="button" className={btn.ghost} disabled={disabled} onClick={() => onRestore(b)}>
                Restaurar
              </button>
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

const restoreWarning = (b: Backup) =>
  `Qapture volverá al estado del ${formatBackupDate(b.createdAt)}. Antes guarda una copia del estado actual en Respaldos. ` +
  "Las capturas que no estén en el respaldo se mueven a la carpeta «evidence-sin-uso»; no se borran.";

export function BackupsDialog({ onClose }: { onClose: () => void }) {
  const backups = useWorkspace((s) => s.backups);
  const busy = useWorkspace((s) => s.dataBusy);
  const loadBackups = useWorkspace((s) => s.loadBackups);
  const createBackup = useWorkspace((s) => s.createBackup);
  const restoreBackup = useWorkspace((s) => s.restoreBackup);
  const [pending, setPending] = useState<Backup | null>(null);

  useEffect(() => {
    void loadBackups();
  }, [loadBackups]);

  return (
    <>
      <Dialog title="Respaldos" kicker="Datos" onClose={onClose}>
        <p className="text-sm text-t4s-gray">
          Qapture copia la base de datos una vez al día y conserva las últimas 7 copias diarias. Las capturas no se copian: siguen en la carpeta de datos.
        </p>
        <div className="my-4">
          <button type="button" className={btn.secondary} disabled={busy} onClick={() => void createBackup()}>
            <DatabaseBackup size={16} /> Respaldar ahora
          </button>
        </div>
        <BackupTable backups={backups} disabled={busy} onRestore={setPending} />
      </Dialog>
      {pending && (
        <ConfirmDialog
          title="Restaurar respaldo"
          message={restoreWarning(pending)}
          confirmLabel="Restaurar"
          onCancel={() => setPending(null)}
          onConfirm={async () => {
            const ok = await restoreBackup(pending.fileName);
            setPending(null);
            if (ok) onClose();
          }}
        />
      )}
    </>
  );
}
