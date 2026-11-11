import { useState } from "react";
import {
  buildProfile,
  columnLetter,
  DEFAULT_STATUS_MAP,
  fold,
  QAPTURE_STATUS_TARGETS,
  statusValues,
  suggestTarget,
  type CoverCells,
  type FieldRole,
  type GuionInspection,
  type GuionPreview,
  type GuionSheet,
  type ImportProfile,
  type ProfileDefinition,
  type Target,
} from "../../domain/guion";
import { Dialog, DialogFooter } from "../components/Dialog";
import { btn, inputCls as input } from "../components/styles";
import { useStoreApi, useWorkspace } from "../store/context";

type FieldTarget = Extract<Target, { kind: "field" }>;
type FieldType = FieldTarget["type"];

const EMPTY_COVER: CoverCells = { executionDate: null, responsible: null, observations: null };

const suggestions = (sheet: GuionSheet, row: number): (Target | null)[] =>
  (sheet.rows[row - 1] ?? []).map((header, i) =>
    suggestTarget(header, sheet.rows.slice(row).map((r) => r[i] ?? "")),
  );

const newField = (header: string): FieldTarget => ({
  kind: "field",
  key: fold(header).replace(/[^a-z0-9]+/g, "_").replace(/^_|_$/g, "").slice(0, 40) || "campo",
  label: header,
  type: "text",
  options: [],
  role: null,
});

/** Valores distintos de la columna debajo de los encabezados, para una lista nueva. */
const columnOptions = (sheet: GuionSheet, row: number, column: number): string[] => [
  ...new Set(sheet.rows.slice(row).map((r) => r[column]?.trim() ?? "").filter(Boolean)),
];

const targetFor = (value: string, header: string): Target | null => {
  if (value === "none") return null;
  if (value === "field") return newField(header);
  return { kind: value as "code" | "title" | "status" };
};

const EMPTY_STATUS = "(Vacío)";

export function ImportGuionDialog() {
  const wizard = useWorkspace((s) => s.guionWizard);
  return wizard ? <GuionWizard key={wizard.inspection.token} {...wizard} /> : null;
}

interface WizardProps {
  inspection: GuionInspection;
  preview: GuionPreview | null;
  profiles: ImportProfile[];
}

function GuionWizard({ inspection, preview, profiles }: WizardProps) {
  const store = useStoreApi();
  const busy = useWorkspace((s) => s.dataBusy);
  const previewGuion = useWorkspace((s) => s.previewGuion);
  const importGuion = useWorkspace((s) => s.importGuion);
  const close = useWorkspace((s) => s.closeGuionImport);
  const deleteProfile = useWorkspace((s) => s.deleteImportProfile);
  const initialProfile = profiles.find((p) => p.id === inspection.suggestedProfileId);
  const initialSheet =
    inspection.sheets.find((s) => s.name === initialProfile?.definition.sheet) ?? inspection.sheets[0];
  const [step, setStep] = useState<1 | 2 | 3>(1);
  const [sheetName, setSheetName] = useState(initialSheet?.name ?? "");
  const [profileId, setProfileId] = useState(initialProfile?.id ?? "");
  const [headerRow, setHeaderRow] = useState(initialSheet?.suggestedHeaderRow?.toString() ?? "");
  const [targets, setTargets] = useState<(Target | null)[]>(() =>
    initialSheet ? suggestions(initialSheet, initialSheet.suggestedHeaderRow ?? 0) : [],
  );
  const [statusImport, setStatusImport] = useState({ ...DEFAULT_STATUS_MAP.import });
  const [useCover, setUseCover] = useState(true);
  const [definition, setDefinition] = useState<ProfileDefinition | null>(null);
  const [fromProfile, setFromProfile] = useState(false);
  const [projectName, setProjectName] = useState(inspection.fileName.replace(/\.xlsx$/i, ""));
  const [saveProfile, setSaveProfile] = useState(false);
  const [profileName, setProfileName] = useState("");
  const [error, setError] = useState("");
  const [confirmDelete, setConfirmDelete] = useState(false);
  const sheet = inspection.sheets.find((s) => s.name === sheetName);
  const selectedProfile = profiles.find((p) => p.id === profileId);
  const row = Number(headerRow);
  const validRow = Number.isInteger(row) && row >= 1;
  const headers = validRow ? (sheet?.rows[row - 1] ?? []) : [];
  const statusColumn = targets.findIndex((target) => target?.kind === "status");
  const statuses = sheet && validRow && statusColumn >= 0 ? statusValues(sheet, row, statusColumn) : [];
  const cover = sheet?.cover ?? EMPTY_COVER;
  const hasCover = Object.values(cover).some(Boolean);
  const canImport =
    !busy &&
    !!preview &&
    preview.errors.length === 0 &&
    !!projectName.trim() &&
    !(saveProfile && !profileName.trim());

  function resetSheet(next: GuionSheet) {
    setSheetName(next.name);
    setHeaderRow(next.suggestedHeaderRow?.toString() ?? "");
    setTargets(suggestions(next, next.suggestedHeaderRow ?? 0));
    setStatusImport({ ...DEFAULT_STATUS_MAP.import });
    setUseCover(true);
    setError("");
  }
  function chooseSheet(name: string) {
    const found = inspection.sheets.find((s) => s.name === name);
    if (found) resetSheet(found);
    setProfileId("");
  }
  function changeRow(value: string) {
    setHeaderRow(value);
    setTargets(sheet ? suggestions(sheet, Number(value)) : []);
    setStatusImport({ ...DEFAULT_STATUS_MAP.import });
  }
  function updateTarget(index: number, target: Target | null) {
    setTargets((previous) => previous.map((old, i) => (i === index ? target : old)));
  }
  function changeType(index: number, target: FieldTarget, type: FieldType) {
    const options = type === "list" && sheet ? columnOptions(sheet, row, index) : [];
    updateTarget(index, { ...target, type, options });
  }
  function changeStatus(status: string, value: string) {
    setStatusImport((previous) => {
      const result = { ...previous };
      if (value === "unknown") delete result[fold(status)];
      else result[fold(status)] = value;
      return result;
    });
  }
  async function showPreview(next: ProfileDefinition, saved: boolean) {
    if (busy) return;
    setError("");
    const previous = store.getState().guionWizard?.preview;
    await previewGuion(next);
    const current = store.getState().guionWizard;
    // Solo una respuesta nueva de este mismo archivo abre la vista previa.
    if (current?.inspection.token === inspection.token && current.preview && current.preview !== previous) {
      setDefinition(next);
      setFromProfile(saved);
      setStep(3);
    }
  }
  async function next() {
    if (selectedProfile) {
      const matching = inspection.sheets.find((s) => s.name === selectedProfile.definition.sheet);
      if (!matching) {
        setError(`El guion no tiene la hoja «${selectedProfile.definition.sheet}».`);
        return;
      }
      setSheetName(matching.name);
      await showPreview(selectedProfile.definition, true);
    } else setStep(2);
  }
  async function chooseProfile(id: string) {
    setProfileId(id);
    const saved = profiles.find((p) => p.id === id);
    if (!saved) return;
    const matching = inspection.sheets.find((s) => s.name === saved.definition.sheet);
    if (!matching) {
      setError(`El guion no tiene la hoja «${saved.definition.sheet}».`);
      return;
    }
    resetSheet(matching);
    await showPreview(saved.definition, true);
  }
  function previewMapping() {
    if (!sheet) return;
    const profile = buildProfile(sheet, row, targets, statusImport, useCover ? cover : EMPTY_COVER);
    void showPreview(profile, false);
  }
  async function submit() {
    if (!definition || !canImport) return;
    setError("");
    const result = await importGuion({
      token: inspection.token,
      profile: definition,
      projectName,
      saveProfileAs: saveProfile ? profileName : null,
    });
    if (!result.ok) setError(result.errors._form ?? "");
  }
  async function removeProfile(profile: ImportProfile) {
    await deleteProfile(profile.id);
    if (!store.getState().guionWizard?.profiles.some((p) => p.id === profile.id)) {
      setProfileId("");
      setConfirmDelete(false);
    }
  }
  function back() {
    setError("");
    setStep(step === 3 && !fromProfile ? 2 : 1);
  }
  const onClose = () => {
    if (busy) return;
    if (confirmDelete) setConfirmDelete(false);
    else close();
  };

  const stepTitle = step === 1 ? "Archivo y hoja" : step === 2 ? "Columnas" : "Vista previa";
  return (
    <>
      <Dialog title="Importar guion de pruebas" kicker={`Paso ${step} de 3`} wide onClose={onClose}>
        <h3 className="mb-4 font-semibold">{stepTitle}</h3>
        <fieldset disabled={busy} className="space-y-4 disabled:opacity-70">
          {step === 1 && (
            <>
              <p className="text-sm">{inspection.fileName}</p>
              <label className="block text-sm">
                Hoja
                <select className={`${input} mt-1`} value={sheetName} onChange={(e) => chooseSheet(e.target.value)}>
                  {inspection.sheets.map((s) => (
                    <option key={s.name}>{s.name}</option>
                  ))}
                </select>
              </label>
              {!sheet && (
                <p role="alert" className="text-sm text-red-700">
                  El guion no tiene hojas visibles.
                </p>
              )}
              <label className="block text-sm">
                Perfil
                <select
                  className={`${input} mt-1`}
                  value={profileId}
                  onChange={(e) => void chooseProfile(e.target.value)}
                >
                  <option value="">Sin perfil</option>
                  {profiles.map((p) => (
                    <option key={p.id} value={p.id}>
                      {p.name}
                    </option>
                  ))}
                </select>
              </label>
              {selectedProfile && (
                <button type="button" className={btn.danger} onClick={() => setConfirmDelete(true)}>
                  Borrar perfil
                </button>
              )}
            </>
          )}
          {step === 2 && sheet && (
            <>
              <label className="block text-sm">
                Fila de encabezados
                <input
                  type="number"
                  min={1}
                  step={1}
                  className={`${input} mt-1`}
                  value={headerRow}
                  onChange={(e) => changeRow(e.target.value)}
                />
              </label>
              {sheet.suggestedHeaderRow === null && (
                <p className="text-sm text-amber-800">No se encontró la fila de encabezados. Elígela a mano.</p>
              )}
              <div className="overflow-x-auto">
                <table className="w-full text-left text-sm">
                  <thead>
                    <tr>
                      <th>Columna</th>
                      <th>Encabezado</th>
                      <th>Destino</th>
                      <th>Tipo</th>
                      <th>Rol para exportar</th>
                    </tr>
                  </thead>
                  <tbody>
                    {headers.map((header, i) => {
                      if (!header.trim()) return null;
                      const target = targets[i];
                      return (
                        <tr key={i} className="border-t border-t4s-line">
                          <td className="p-2">{columnLetter(i)}</td>
                          <td className="p-2">{header}</td>
                          <td className="p-2">
                            <select
                              aria-label={`Destino de ${header}`}
                              className={input}
                              value={target?.kind ?? "none"}
                              onChange={(e) => updateTarget(i, targetFor(e.target.value, header))}
                            >
                              <option value="code">Código</option>
                              <option value="title">Título</option>
                              <option value="status">Estado</option>
                              <option value="field">Campo nuevo</option>
                              <option value="none">No importar</option>
                            </select>
                          </td>
                          <td className="p-2">
                            {target?.kind === "field" && (
                              <select
                                aria-label={`Tipo de ${header}`}
                                className={input}
                                value={target.type}
                                onChange={(e) => changeType(i, target, e.target.value as FieldType)}
                              >
                                <option value="text">Texto</option>
                                <option value="longtext">Texto largo</option>
                                <option value="list">Lista</option>
                                <option value="date">Fecha</option>
                              </select>
                            )}
                          </td>
                          <td className="p-2">
                            {target?.kind === "field" && (
                              <select
                                aria-label={`Rol de ${header}`}
                                className={input}
                                value={target.role ?? ""}
                                onChange={(e) =>
                                  updateTarget(i, { ...target, role: (e.target.value || null) as FieldRole | null })
                                }
                              >
                                <option value="">Ninguno</option>
                                <option value="executionDate">Fecha de ejecución</option>
                                <option value="tester">Tester</option>
                                <option value="conditions">Condiciones</option>
                              </select>
                            )}
                          </td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              </div>
              {statuses.length > 0 && (
                <section>
                  <h4 className="mb-2 font-medium">Estados</h4>
                  <table className="w-full text-left text-sm">
                    <thead>
                      <tr>
                        <th>Valor del guion</th>
                        <th>Estado en Qapture</th>
                      </tr>
                    </thead>
                    <tbody>
                      {statuses.map((status) => (
                        <tr key={status}>
                          <td className="p-2">{status || EMPTY_STATUS}</td>
                          <td className="p-2">
                            <select
                              className={input}
                              aria-label={`Estado para ${status || EMPTY_STATUS}`}
                              value={statusImport[fold(status)] ?? "unknown"}
                              onChange={(e) => changeStatus(status, e.target.value)}
                            >
                              <option value="unknown">Sin equivalencia (avisar)</option>
                              {QAPTURE_STATUS_TARGETS.map((target) => (
                                <option key={target.value} value={target.value}>
                                  {target.label}
                                </option>
                              ))}
                            </select>
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </section>
              )}
              {hasCover && (
                <div className="space-y-2 text-sm">
                  <p>
                    Carátula: fecha en {cover.executionDate ?? "—"} · responsable en {cover.responsible ?? "—"} ·
                    observaciones en {cover.observations ?? "—"}
                  </p>
                  <label className="flex items-center gap-2">
                    <input type="checkbox" checked={useCover} onChange={(e) => setUseCover(e.target.checked)} />
                    Usar estas celdas
                  </label>
                </div>
              )}
            </>
          )}
          {step === 3 && preview && (
            <>
              <p className="font-medium">{preview.cases.length} casos</p>
              <div className="max-h-64 overflow-auto">
                <table className="w-full text-left text-sm">
                  <thead>
                    <tr>
                      <th>Fila</th>
                      <th>Código</th>
                      <th>Título</th>
                      <th>Estado</th>
                    </tr>
                  </thead>
                  <tbody>
                    {preview.cases.map((c, i) => (
                      <tr key={i} className="border-t border-t4s-line">
                        <td className="p-2">{c.row}</td>
                        <td className="p-2">{c.code}</td>
                        <td className="p-2">{c.title}</td>
                        <td className="p-2">
                          {c.status}
                          {c.conditioned ? " · Condicionado" : ""}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
              {preview.errors.length > 0 && (
                <div role="alert" className="text-sm text-red-700">
                  <h4 className="font-medium">Errores</h4>
                  <ul className="list-disc pl-5">
                    {preview.errors.map((issue, i) => (
                      <li key={i}>{issue}</li>
                    ))}
                  </ul>
                </div>
              )}
              {preview.warnings.length > 0 && (
                <section className="text-sm text-amber-800">
                  <h4 className="font-medium">Avisos</h4>
                  <ul className="list-disc pl-5">
                    {preview.warnings.map((warning, i) => (
                      <li key={i}>{warning}</li>
                    ))}
                  </ul>
                </section>
              )}
              <label className="block text-sm">
                Nombre del proyecto
                <input
                  className={`${input} mt-1`}
                  maxLength={150}
                  value={projectName}
                  onChange={(e) => setProjectName(e.target.value)}
                />
              </label>
              <label className="flex items-center gap-2 text-sm">
                <input type="checkbox" checked={saveProfile} onChange={(e) => setSaveProfile(e.target.checked)} />
                Guardar como perfil
              </label>
              {saveProfile && (
                <label className="block text-sm">
                  Nombre del perfil
                  <input
                    className={`${input} mt-1`}
                    maxLength={100}
                    value={profileName}
                    onChange={(e) => setProfileName(e.target.value)}
                  />
                </label>
              )}
            </>
          )}
          {error && (
            <p role="alert" className="whitespace-pre-line text-sm text-red-700">
              {error}
            </p>
          )}
        </fieldset>
        <DialogFooter>
          <button type="button" className={btn.secondary} disabled={busy} onClick={onClose}>
            Cancelar
          </button>
          {step > 1 && (
            <button type="button" className={btn.secondary} disabled={busy} onClick={back}>
              Atrás
            </button>
          )}
          {step === 1 && (
            <button type="button" className={btn.primary} disabled={busy || !sheet} onClick={() => void next()}>
              Siguiente
            </button>
          )}
          {step === 2 && (
            <button type="button" className={btn.primary} disabled={busy || !sheet || !validRow} onClick={previewMapping}>
              Vista previa
            </button>
          )}
          {step === 3 && (
            <button type="button" className={btn.primary} disabled={!canImport} onClick={() => void submit()}>
              Importar
            </button>
          )}
        </DialogFooter>
      </Dialog>
      {confirmDelete && selectedProfile && (
        <Dialog
          title="Borrar perfil"
          onClose={() => {
            if (!busy) setConfirmDelete(false);
          }}
        >
          <p className="text-sm">
            ¿Borrar el perfil «{selectedProfile.name}»? Los proyectos importados conservan su configuración.
          </p>
          <DialogFooter>
            <button type="button" className={btn.secondary} disabled={busy} onClick={() => setConfirmDelete(false)}>
              Cancelar
            </button>
            <button
              type="button"
              className={btn.danger}
              disabled={busy}
              onClick={() => void removeProfile(selectedProfile)}
            >
              Borrar
            </button>
          </DialogFooter>
        </Dialog>
      )}
    </>
  );
}
