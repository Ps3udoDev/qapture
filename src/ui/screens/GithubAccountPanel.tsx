import { useCallback, useEffect, useRef, useState } from "react";
import { ConfirmDialog } from "../components/ConfirmDialog";
import { btn } from "../components/styles";
import { useWorkspace } from "../store/context";

const linkCls = "w-fit text-left text-sm text-t4s-blue underline hover:text-t4s-charcoal";

/** Segundos que le quedan al código; se calcula con la hora de inicio, no acumulando ticks. */
const remaining = (startedAt: number, expiresIn: number) => Math.max(0, expiresIn - Math.floor((Date.now() - startedAt) / 1000));
const clock = (seconds: number) => `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
/** El front corta el flujo al llegar a 0 (antes que Rust), así que el aviso lo da el front. */
const MSG_EXPIRED = "El código venció. Vuelve a pulsar «Conectar con GitHub».";

function Countdown({ startedAt, expiresIn, onExpired }: { startedAt: number; expiresIn: number; onExpired: () => void }) {
  const [left, setLeft] = useState(() => remaining(startedAt, expiresIn));
  const fired = useRef(false);
  useEffect(() => {
    const tick = () => {
      const next = remaining(startedAt, expiresIn);
      setLeft(next);
      if (next === 0 && !fired.current) {
        fired.current = true;
        onExpired();
      }
    };
    tick();
    const id = setInterval(tick, 1000);
    return () => clearInterval(id);
  }, [startedAt, expiresIn, onExpired]);
  return <p className="text-sm text-t4s-gray">El código vence en {clock(left)}.</p>;
}

async function copy(text: string): Promise<boolean> {
  try {
    if (!navigator.clipboard) return false;
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    return false;
  }
}

/** Pestaña «GitHub» de Ajustes: conectar con Device Flow, estado de la cuenta, reconectar y desconectar. */
export function GithubAccountPanel() {
  const github = useWorkspace((s) => s.github);
  const loadAccount = useWorkspace((s) => s.loadGithubAccount);
  const connect = useWorkspace((s) => s.connectGithub);
  const cancel = useWorkspace((s) => s.cancelGithubConnect);
  const disconnect = useWorkspace((s) => s.disconnectGithub);
  const openLink = useWorkspace((s) => s.openLink);
  const pushToast = useWorkspace((s) => s.pushToast);
  const [confirming, setConfirming] = useState(false);
  const { account, flow, status } = github;

  useEffect(() => {
    void loadAccount();
  }, [loadAccount]);

  const expire = useCallback(() => {
    void cancel();
    pushToast("error", MSG_EXPIRED);
  }, [cancel, pushToast]);

  const copyCode = async () => {
    if (!flow) return;
    if (await copy(flow.code.userCode)) pushToast("success", "Código copiado.");
    else pushToast("error", "No se pudo copiar el código.");
  };

  if (flow) {
    return (
      <div className="grid gap-3">
        <p className="text-sm">Ingresa este código en GitHub:</p>
        <p className="font-mono text-3xl font-semibold tracking-widest text-t4s-charcoal" aria-label="Código de GitHub">
          {flow.code.userCode}
        </p>
        <div className="flex flex-wrap gap-2">
          <button type="button" className={btn.secondary} onClick={() => void copyCode()}>
            Copiar código
          </button>
          <button type="button" className={btn.primary} onClick={() => void openLink("githubDevice")}>
            Abrir GitHub
          </button>
        </div>
        <Countdown startedAt={flow.startedAt} expiresIn={flow.code.expiresIn} onExpired={expire} />
        <div>
          <button type="button" className={btn.secondary} onClick={() => void cancel()}>
            Cancelar
          </button>
        </div>
      </div>
    );
  }

  if (!account) {
    return (
      <div className="grid gap-3">
        <p className="text-sm">Conecta tu cuenta para reportar casos como issues e importar issues como casos.</p>
        <div>
          <button type="button" className={btn.primary} disabled={status === "loading"} onClick={() => void connect()}>
            Conectar con GitHub
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="grid gap-4">
      <div className="flex items-center gap-3">
        {account.avatarDataUrl ? (
          <img src={account.avatarDataUrl} alt="" className="h-12 w-12 rounded-full border border-t4s-line" />
        ) : (
          <div aria-hidden className="h-12 w-12 rounded-full border border-t4s-line bg-t4s-cream" />
        )}
        <div className="grid">
          <span className="text-sm font-medium">@{account.login}</span>
          {account.name && <span className="text-sm text-t4s-gray">{account.name}</span>}
        </div>
      </div>

      {account.needsReconnect && (
        <div role="alert" className="flex flex-wrap items-center gap-3 rounded border border-amber-300 bg-amber-50 px-3 py-2 text-sm text-amber-900">
          <span>La sesión de GitHub venció o fue revocada.</span>
          <button type="button" className={btn.primary} disabled={status === "loading"} onClick={() => void connect()}>
            Reconectar
          </button>
        </div>
      )}

      <div className="grid gap-1">
        <button type="button" className={linkCls} onClick={() => void openLink("githubInstall")}>
          Instalar o configurar Qapture en una cuenta u organización
        </button>
        <button type="button" className={linkCls} onClick={() => void openLink("githubRevoke")}>
          Revocar acceso en GitHub
        </button>
      </div>

      <div>
        <button type="button" className={btn.secondary} onClick={() => setConfirming(true)}>
          Desconectar
        </button>
      </div>

      {confirming && (
        <ConfirmDialog
          title="Desconectar GitHub"
          message="Se borrará el acceso guardado en este equipo. Tus proyectos conservan los repos vinculados."
          confirmLabel="Desconectar"
          onCancel={() => setConfirming(false)}
          onConfirm={async () => {
            setConfirming(false);
            await disconnect();
          }}
        />
      )}
    </div>
  );
}
