import { TriangleAlert } from "lucide-react";
import { useState } from "react";
import { formatBackupDate, type Backup } from "../../domain/data";
import { ConfirmDialog } from "../components/ConfirmDialog";
import { btn } from "../components/styles";
import { useWorkspace } from "../store/context";
import { BackupTable } from "./BackupsDialog";

type Choice = { backup: Backup | null; thenImport: boolean };

/** Spec §8: la base no abre. Volver a un respaldo o empezar con una base vacía (e importar un paquete). */
export function RecoveryScreen() {
  const recovery = useWorkspace((s) => s.recovery);
  const backups = useWorkspace((s) => s.backups);
  const busy = useWorkspace((s) => s.dataBusy);
  const recover = useWorkspace((s) => s.recover);
  const [pending, setPending] = useState<Choice | null>(null);

  return (
    <main className="mx-auto flex w-full max-w-3xl flex-col gap-6 p-8">
      <header className="flex items-start gap-3">
        <TriangleAlert className="mt-1 shrink-0 text-red-700" size={28} aria-hidden />
        <div>
          <h1 className="text-xl font-semibold">Qapture no pudo abrir tus datos</h1>
          <p className="text-sm text-t4s-gray">
            La base de datos parece dañada. No se borra: queda en la carpeta de datos como «qapture-danada-…db» por si hace falta revisarla.
          </p>
        </div>
      </header>
      <section className="rounded-xl border border-t4s-line bg-white p-5">
        <h2 className="mb-3 font-semibold">Volver a un respaldo</h2>
        <BackupTable backups={backups} disabled={busy} onRestore={(backup) => setPending({ backup, thenImport: false })} />
      </section>
      <section className="rounded-xl border border-t4s-line bg-white p-5">
        <h2 className="mb-1 font-semibold">Empezar de nuevo</h2>
        <p className="mb-3 text-sm text-t4s-gray">Crea una base vacía. Si un compañero te compartió el proyecto, impórtalo desde su paquete .qapture.</p>
        <div className="flex flex-wrap gap-2">
          <button type="button" className={btn.primary} disabled={busy} onClick={() => setPending({ backup: null, thenImport: true })}>
            Empezar vacía e importar un paquete
          </button>
          <button type="button" className={btn.secondary} disabled={busy} onClick={() => setPending({ backup: null, thenImport: false })}>
            Empezar con una base vacía
          </button>
        </div>
      </section>
      {recovery?.detail && (
        <details className="text-xs text-t4s-gray">
          <summary>Detalle técnico</summary>
          <p className="mt-1 break-all">{recovery.detail}</p>
        </details>
      )}
      {pending && (
        <ConfirmDialog
          title={pending.backup ? "Restaurar respaldo" : "Empezar con una base vacía"}
          message={
            pending.backup
              ? `Qapture usará el respaldo del ${formatBackupDate(pending.backup.createdAt)}. Las capturas que no estén en ese respaldo se mueven a la carpeta «evidence-sin-uso».`
              : "Qapture creará una base vacía. Las capturas que había se mueven a la carpeta «evidence-sin-uso»; no se borran."
          }
          confirmLabel={pending.backup ? "Restaurar" : "Empezar vacía"}
          onCancel={() => setPending(null)}
          onConfirm={async () => {
            const choice = pending;
            setPending(null);
            await recover(choice.backup?.fileName ?? null, choice.thenImport);
          }}
        />
      )}
    </main>
  );
}
