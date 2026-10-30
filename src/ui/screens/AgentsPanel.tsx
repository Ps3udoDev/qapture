import { useEffect, useState } from "react";
import { AGENT_GROUPS, RECOMMENDED } from "../../domain/agents";
import type { Settings } from "../../domain/settings";
import { ConfirmDialog } from "../components/ConfirmDialog";
import { btn, inputCls } from "../components/styles";
import { useStoreApi, useWorkspace } from "../store/context";
const CLIENTS = ["Claude Code", "Codex", "Cursor", "Claude Desktop"] as const;
export function AgentsPanel({
  value,
  onChange,
  onNavigate,
}: {
  onNavigate: () => void;
  value: Settings["agents"];
  onChange: (change: Partial<Settings["agents"]>) => void;
}) {
  const store = useStoreApi();
  const config = useWorkspace((s) => s.agentConfig);
  const activity = useWorkspace((s) => s.agentActivity);
  const load = useWorkspace((s) => s.loadAgents);
  const regenerate = useWorkspace((s) => s.regenerateAgentKey);
  const selectProject = useWorkspace((s) => s.selectProject);
  const selectCase = useWorkspace((s) => s.selectCase);
  const toast = useWorkspace((s) => s.pushToast);
  const [client, setClient] = useState<(typeof CLIENTS)[number]>("Claude Code");
  const [filter, setFilter] = useState("");
  const [confirm, setConfirm] = useState(false);
  useEffect(() => {
    void load(filter || undefined);
  }, [load, filter]);
  const text = config
    ? client === "Claude Code"
      ? config.claudeCode
      : client === "Codex"
        ? config.codex
        : config.json
    : "";
  async function goToCase(projectId: string, caseId: string) {
    await selectProject(projectId);
    const current = store.getState();
    if (current.project?.id !== projectId) return;
    if (!current.cases.some((item) => item.id === caseId)) {
      toast("error", "El caso ya no existe.");
      return;
    }
    selectCase(caseId);
    onNavigate();
  }
  return (
    <>
      <label className="flex items-center gap-2 text-sm font-medium">
        <input
          type="checkbox"
          checked={value.enabled}
          onChange={(e) => onChange({ enabled: e.target.checked })}
        />
        Permitir que agentes de IA usen Qapture (MCP)
      </label>
      <fieldset
        disabled={!value.enabled}
        className="grid gap-3 disabled:opacity-50"
      >
        <legend className="mb-2 text-sm font-medium">Permisos</legend>
        {AGENT_GROUPS.map((group) => (
          <div key={group.id} className="rounded border border-t4s-line p-3">
            <label className="flex items-center gap-2 text-sm font-medium">
              <input
                type="checkbox"
                checked={value[group.id]}
                onChange={(e) => onChange({ [group.id]: e.target.checked })}
              />
              {group.label}
            </label>
            <p className="mt-1 text-sm">{group.description}</p>
            <p className="mt-1 text-xs text-t4s-gray">{group.risk}</p>
            <p className="mt-1 break-words font-mono text-xs">
              {group.tools.join(", ")}
            </p>
            {group.id === "capture" && (
              <label className="mt-2 flex items-center gap-2 text-sm">
                <input
                  type="checkbox"
                  checked={value.confirmCapture}
                  disabled={!value.capture}
                  onChange={(e) =>
                    onChange({ confirmCapture: e.target.checked })
                  }
                />
                Pedir confirmación en cada captura
              </label>
            )}
            {group.id === "export" && (
              <label className="mt-2 flex items-center gap-2 text-sm">
                <input
                  type="checkbox"
                  checked={value.confirmExport}
                  disabled={!value.export}
                  onChange={(e) =>
                    onChange({ confirmExport: e.target.checked })
                  }
                />
                Pedir confirmación en cada exportación
              </label>
            )}
          </div>
        ))}
        <button
          type="button"
          className={btn.secondary}
          onClick={() => onChange(RECOMMENDED)}
        >
          Activar configuración recomendada
        </button>
      </fieldset>
      <p className="text-xs text-t4s-gray">
        Los permisos se aplican al guardar los ajustes.
      </p>
      <section className="grid gap-2" aria-label="Conectar un agente">
        <h3 className="text-sm font-medium">Conectar un agente</h3>
        {config && !config.listening && (
          <p role="alert" className="text-sm text-red-700">
            Qapture no pudo abrir el canal para agentes. Reinicia la app.
          </p>
        )}
        <div
          role="tablist"
          aria-label="Clientes MCP"
          className="flex flex-wrap gap-2"
        >
          {CLIENTS.map((c) => (
            <button
              type="button"
              role="tab"
              aria-selected={client === c}
              className={client === c ? btn.primary : btn.secondary}
              key={c}
              onClick={() => setClient(c)}
            >
              {c}
            </button>
          ))}
        </div>
        <pre className="max-h-48 overflow-auto whitespace-pre-wrap break-all rounded bg-gray-50 p-3 text-xs">
          {text || "Cargando configuración…"}
        </pre>
        <button
          type="button"
          className={btn.secondary}
          disabled={!text}
          onClick={() =>
            void navigator.clipboard
              .writeText(text)
              .then(() => toast("success", "Configuración copiada."))
              .catch(() =>
                toast("error", "No se pudo copiar la configuración."),
              )
          }
        >
          Copiar
        </button>
      </section>
      <section className="grid gap-2" aria-label="Actividad de agentes">
        <h3 className="text-sm font-medium">Actividad</h3>
        <label className="text-sm">
          Filtrar por cliente
          <input
            aria-label="Filtrar por cliente"
            className={inputCls}
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
          />
        </label>
        <button
          type="button"
          className={btn.secondary}
          onClick={() => void load(filter || undefined)}
        >
          Actualizar actividad
        </button>
        <div className="max-h-64 overflow-auto">
          <table className="w-full text-left text-xs">
            <thead>
              <tr>
                {["Fecha", "Cliente", "Herramienta", "Caso", "Resultado"].map(
                  (c) => (
                    <th className="p-1" key={c}>
                      {c}
                    </th>
                  ),
                )}
              </tr>
            </thead>
            <tbody>
              {activity.map((row) => (
                <tr key={row.id}>
                  <td className="p-1">{new Date(row.at).toLocaleString()}</td>
                  <td>{row.client}</td>
                  <td>{row.tool}</td>
                  <td>
                    {row.caseId && row.projectId ? (
                      <button
                        type="button"
                        className="text-t4s-blue underline"
                        onClick={() =>
                          void goToCase(row.projectId!, row.caseId!)
                        }
                      >
                        Ir al caso
                      </button>
                    ) : (
                      (row.caseId ?? "—")
                    )}
                  </td>
                  <td>
                    {row.ok ? "Correcto" : "Error"}
                    <span className="block text-t4s-gray">{row.detail}</span>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        {!activity.length && (
          <p className="text-sm text-t4s-gray">Sin actividad.</p>
        )}
      </section>
      <button
        type="button"
        className={btn.secondary}
        onClick={() => setConfirm(true)}
      >
        Regenerar clave
      </button>
      {confirm && (
        <ConfirmDialog
          title="Regenerar clave de agentes"
          message="Se desconectarán los agentes conectados. Tendrás que volver a conectarlos."
          confirmLabel="Regenerar clave"
          onCancel={() => setConfirm(false)}
          onConfirm={async () => {
            await regenerate();
            setConfirm(false);
          }}
        />
      )}
    </>
  );
}
