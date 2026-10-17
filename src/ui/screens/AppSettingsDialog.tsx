import { useState } from "react";
import type { FormErrors } from "../../domain/cases";
import { IMAGE_WIDTH } from "../../domain/export";
import { DEFAULT_SETTINGS, SHORTCUT_ACTIONS, SHORTCUT_LABEL, type Settings } from "../../domain/settings";
import { Dialog, DialogFooter } from "../components/Dialog";
import { FormField } from "../components/FormField";
import { btn, inputCls } from "../components/styles";
import { useWorkspace } from "../store/context";
import { BackupsDialog } from "./BackupsDialog";
import { ShortcutRecorder } from "./ShortcutRecorder";

const TABS = ["Perfil", "Atajos", "Sistema", "Exportación", "Datos", "Acerca de"] as const;
type Tab = (typeof TABS)[number];

const tabOf = (key: string): Tab => {
  if (key.startsWith("profile.")) return "Perfil";
  if (key.startsWith("export.")) return "Exportación";
  return "Atajos";
};

function Check({ label, checked, onChange }: { label: string; checked: boolean; onChange: (v: boolean) => void }) {
  return (
    <label className="flex items-center gap-2 text-sm">
      <input type="checkbox" checked={checked} onChange={(e) => onChange(e.target.checked)} /> {label}
    </label>
  );
}

/** Ajustes de la aplicación (F6): un solo «Guardar» para todas las pestañas. */
export function AppSettingsDialog({ onClose }: { onClose: () => void }) {
  const snapshot = useWorkspace((s) => s.settings);
  const saveSettings = useWorkspace((s) => s.saveSettings);
  const pickExportFolder = useWorkspace((s) => s.pickExportFolder);
  const openDataFolder = useWorkspace((s) => s.openDataFolder);
  const openLink = useWorkspace((s) => s.openLink);
  const importLegacy = useWorkspace((s) => s.importLegacy);
  const busyData = useWorkspace((s) => s.dataBusy);
  const [draft, setDraft] = useState<Settings>(() => structuredClone(snapshot?.settings ?? DEFAULT_SETTINGS));
  const [tab, setTab] = useState<Tab>("Perfil");
  const [errors, setErrors] = useState<FormErrors>({});
  const [busy, setBusy] = useState(false);
  const [backups, setBackups] = useState(false);

  const patch = <K extends keyof Settings>(section: K, change: Partial<Settings[K]>) => setDraft((d) => ({ ...d, [section]: { ...d[section], ...change } }));
  const failed = new Set((snapshot?.shortcutFailures ?? []).map((f) => f.action));

  async function save() {
    setBusy(true);
    const result = await saveSettings(draft);
    setBusy(false);
    if (result.ok) return onClose();
    setErrors(result.errors);
    const first = Object.keys(result.errors)[0];
    if (first) setTab(tabOf(first));
  }

  return (
    <>
      <Dialog title="Ajustes" kicker="Qapture" onClose={backups ? () => undefined : onClose}>
        <div role="tablist" aria-label="Secciones de ajustes" className="mb-4 flex flex-wrap gap-1 border-b border-t4s-line">
          {TABS.map((t) => (
            <button
              key={t}
              type="button"
              role="tab"
              aria-selected={tab === t}
              onClick={() => setTab(t)}
              className={`-mb-px border-b-2 px-3 py-2 text-sm font-medium ${tab === t ? "border-t4s-blue text-t4s-blue" : "border-transparent text-t4s-gray hover:text-t4s-charcoal"}`}
            >
              {t}
            </button>
          ))}
        </div>

        <div role="tabpanel" aria-label={tab} className="grid gap-4">
          {tab === "Perfil" && (
            <>
              <FormField label="Nombre del tester" htmlFor="set-tester" error={errors["profile.tester"]}>
                <input id="set-tester" className={inputCls} value={draft.profile.tester} onChange={(e) => patch("profile", { tester: e.target.value })} />
              </FormField>
              <FormField label="Cargo" htmlFor="set-role" error={errors["profile.role"]}>
                <input id="set-role" className={inputCls} value={draft.profile.role} onChange={(e) => patch("profile", { role: e.target.value })} />
              </FormField>
              <FormField label="Correo" htmlFor="set-email" error={errors["profile.email"]}>
                <input id="set-email" className={inputCls} value={draft.profile.email} onChange={(e) => patch("profile", { email: e.target.value })} />
              </FormField>
            </>
          )}

          {tab === "Atajos" && (
            <>
              <p className="text-sm text-t4s-gray">Clic en una combinación y presiona la nueva. Esc cancela; Retroceso restaura la combinación por defecto.</p>
              {SHORTCUT_ACTIONS.map((action) => (
                <div key={action} role="group" aria-label={SHORTCUT_LABEL[action]} className="grid gap-1">
                  <div className="flex items-center justify-between gap-4">
                    <span className="text-sm font-medium">{SHORTCUT_LABEL[action]}</span>
                    <div className="flex items-center gap-3">
                      {failed.has(action) && <span className="text-xs text-red-700">No registrado</span>}
                      <ShortcutRecorder
                        label={SHORTCUT_LABEL[action]}
                        value={draft.shortcuts[action]}
                        fallback={DEFAULT_SETTINGS.shortcuts[action]}
                        onChange={(v) => patch("shortcuts", { [action]: v })}
                      />
                    </div>
                  </div>
                  {errors[`shortcuts.${action}`] && (
                    <p role="alert" className="text-xs text-red-700">
                      {errors[`shortcuts.${action}`]}
                    </p>
                  )}
                </div>
              ))}
            </>
          )}

          {tab === "Sistema" && (
            <>
              <Check label="Iniciar con Windows" checked={draft.system.autostart} onChange={(v) => patch("system", { autostart: v })} />
              <Check label="Iniciar oculto" checked={draft.system.startHidden} onChange={(v) => patch("system", { startHidden: v })} />
              <Check label="Cerrar oculta en la bandeja" checked={draft.system.closeToTray} onChange={(v) => patch("system", { closeToTray: v })} />
              <Check label="Esc oculta la ventana" checked={draft.system.escHides} onChange={(v) => patch("system", { escHides: v })} />
            </>
          )}

          {tab === "Exportación" && (
            <>
              <FormField label="Carpeta por defecto" htmlFor="set-folder" error={errors["export.defaultFolder"]}>
                <div className="flex gap-2">
                  <input id="set-folder" readOnly className={inputCls} value={draft.export.defaultFolder} placeholder="Se pregunta al exportar" />
                  <button
                    type="button"
                    className={btn.secondary}
                    onClick={async () => {
                      const folder = await pickExportFolder();
                      if (folder) patch("export", { defaultFolder: folder });
                    }}
                  >
                    Elegir…
                  </button>
                  <button type="button" className={btn.ghost} disabled={!draft.export.defaultFolder} onClick={() => patch("export", { defaultFolder: "" })}>
                    Quitar
                  </button>
                </div>
              </FormField>
              <FormField label="Ancho máximo de imagen (px)" htmlFor="set-width" error={errors["export.imageMaxWidth"]}>
                <input
                  id="set-width"
                  type="number"
                  min={IMAGE_WIDTH.min}
                  max={IMAGE_WIDTH.max}
                  className={inputCls}
                  value={Number.isNaN(draft.export.imageMaxWidth) ? "" : draft.export.imageMaxWidth}
                  onChange={(e) => patch("export", { imageMaxWidth: e.target.value === "" ? Number.NaN : Number(e.target.value) })}
                />
              </FormField>
              <Check label="Incluir hoja Resumen" checked={draft.export.includeSummary} onChange={(v) => patch("export", { includeSummary: v })} />
            </>
          )}

          {tab === "Datos" && (
            <>
              <div>
                <button type="button" className={btn.secondary} onClick={() => void openDataFolder()}>
                  Abrir carpeta de datos
                </button>
                <p className="mt-1 break-all text-xs text-t4s-gray">{snapshot?.dataDir}</p>
              </div>
              <div className="flex gap-2">
                <button type="button" className={btn.secondary} onClick={() => setBackups(true)}>
                  Respaldos…
                </button>
                <button type="button" className={btn.secondary} disabled={busyData} onClick={() => void importLegacy()}>
                  Importar Bitácora v1…
                </button>
              </div>
            </>
          )}

          {tab === "Acerca de" && (
            <>
              <p className="text-sm font-medium">Qapture {snapshot?.appVersion}</p>
              <div className="flex gap-2">
                <button type="button" className={btn.secondary} onClick={() => void openLink("releases")}>
                  Releases
                </button>
                <button type="button" className={btn.secondary} onClick={() => void openLink("docs")}>
                  Documentación
                </button>
              </div>
            </>
          )}

          {errors._form && (
            <p role="alert" className="text-sm text-red-700">
              {errors._form}
            </p>
          )}
        </div>

        <DialogFooter>
          <button type="button" className={btn.secondary} onClick={onClose}>
            Cancelar
          </button>
          <button type="button" className={btn.primary} disabled={busy} onClick={() => void save()}>
            Guardar
          </button>
        </DialogFooter>
      </Dialog>
      {backups && <BackupsDialog onClose={() => setBackups(false)} />}
    </>
  );
}
