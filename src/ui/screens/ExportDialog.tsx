import { useEffect, useState, type FormEvent } from "react";
import type { ExportRequest } from "../../application/use-cases";
import type { FormErrors } from "../../domain/cases";
import { emptyDocVersion, formatIsoDate, latestVersion, suggestNextVersion, todayIso } from "../../domain/doc-versions";
import { IMAGE_WIDTH, progressLabel, progressPercent, type ExportKind } from "../../domain/export";
import { Dialog, DialogFooter } from "../components/Dialog";
import { FormField } from "../components/FormField";
import { btn, inputCls } from "../components/styles";
import { useWorkspace } from "../store/context";

const NEW = "__nueva__";
const KINDS: { kind: ExportKind; label: string; hint: string }[] = [
  { kind: "project", label: "Matriz con evidencias", hint: "Carátula, matriz, capturas y resumen." },
  { kind: "template", label: "Plantilla vacía", hint: "Carátula y las columnas del proyecto, sin casos." },
];

/** UC-16 / UC-17: opciones del export; «Guardar como» lo abre Rust al confirmar. */
export function ExportDialog({ onClose }: { onClose: () => void }) {
  const project = useWorkspace((s) => s.project);
  const docVersions = useWorkspace((s) => s.docVersions);
  const exporting = useWorkspace((s) => s.exporting);
  const exportExcel = useWorkspace((s) => s.exportExcel);
  const testerName = useWorkspace((s) => s.testerName);
  const exportDefaults = useWorkspace((s) => s.settings?.settings.export);
  const [kind, setKind] = useState<ExportKind>("project");
  const [tester, setTester] = useState(testerName);
  const [versionId, setVersionId] = useState(() => latestVersion(docVersions)?.id ?? NEW);
  const [newVersion, setNewVersion] = useState(() => ({
    version: suggestNextVersion(docVersions),
    changes: docVersions.length ? "" : "Versión inicial",
  }));
  const [includeEvidence, setIncludeEvidence] = useState(true);
  const [includeSummary, setIncludeSummary] = useState(exportDefaults?.includeSummary ?? true);
  const [errors, setErrors] = useState<FormErrors>({});

  useEffect(() => {
    // Si la versión nueva ya quedó registrada (p. ej., se canceló «Guardar como»), se elige esa.
    if (versionId !== NEW) return;
    const typed = newVersion.version.trim().toLowerCase();
    const saved = docVersions.find((v) => v.version.toLowerCase() === typed);
    if (saved) setVersionId(saved.id);
  }, [docVersions, versionId, newVersion.version]);

  if (!project) return null;
  const busy = exporting !== null;

  async function submit(e: FormEvent) {
    e.preventDefault();
    const isNew = kind === "project" && versionId === NEW;
    const request: ExportRequest = {
      kind,
      options: {
        tester,
        docVersionId: kind === "project" && !isNew ? versionId : null,
        includeEvidence,
        includeSummary,
        imageMaxWidth: exportDefaults?.imageMaxWidth ?? IMAGE_WIDTH.default,
      },
      newVersion: isNew ? { ...emptyDocVersion(todayIso(), tester.trim()), version: newVersion.version, changes: newVersion.changes } : undefined,
    };
    const result = await exportExcel(request);
    if (result.ok) onClose();
    else setErrors(result.errors);
  }

  return (
    <Dialog title="Exportar Excel" kicker={project.name} onClose={busy ? () => {} : onClose}>
      <form onSubmit={submit} noValidate className="grid gap-4">
        <fieldset>
          <legend className="mb-2 text-sm font-medium">Qué exportar</legend>
          <div role="radiogroup" className="grid grid-cols-2 gap-2">
            {KINDS.map((k) => (
              <label key={k.kind} className={`cursor-pointer rounded-lg border p-3 text-sm ${kind === k.kind ? "border-t4s-blue bg-t4s-blue/5" : "border-t4s-line"}`}>
                <input type="radio" name="export-kind" className="mr-2" checked={kind === k.kind} onChange={() => setKind(k.kind)} disabled={busy} />
                <span className="font-medium">{k.label}</span>
                <span className="mt-1 block text-xs text-t4s-gray">{k.hint}</span>
              </label>
            ))}
          </div>
        </fieldset>
        <FormField label="Tester" required htmlFor="export-tester" error={errors.tester}>
          <input id="export-tester" className={inputCls} value={tester} onChange={(e) => setTester(e.target.value)} disabled={busy} placeholder="Nombre que aparece en la carátula" />
        </FormField>
        {kind === "project" && (
          <>
            <FormField label="Versión del documento" required htmlFor="export-version" error={errors.docVersionId}>
              <select id="export-version" className={inputCls} value={versionId} onChange={(e) => setVersionId(e.target.value)} disabled={busy}>
                {[...docVersions].reverse().map((v) => (
                  <option key={v.id} value={v.id}>
                    {`${v.version} · ${formatIsoDate(v.date)}`}
                  </option>
                ))}
                <option value={NEW}>Nueva versión…</option>
              </select>
            </FormField>
            {versionId === NEW && (
              <div className="grid grid-cols-3 gap-4">
                <FormField label="Número de versión" required htmlFor="export-new-version" error={errors.version}>
                  <input
                    id="export-new-version"
                    className={inputCls}
                    value={newVersion.version}
                    onChange={(e) => setNewVersion((v) => ({ ...v, version: e.target.value }))}
                    disabled={busy}
                  />
                </FormField>
                <FormField label="Cambios" htmlFor="export-new-changes" error={errors.changes} className="col-span-2">
                  <input
                    id="export-new-changes"
                    className={inputCls}
                    value={newVersion.changes}
                    onChange={(e) => setNewVersion((v) => ({ ...v, changes: e.target.value }))}
                    disabled={busy}
                  />
                </FormField>
              </div>
            )}
            <fieldset className="flex gap-6 text-sm">
              <legend className="sr-only">Hojas opcionales</legend>
              <label className="flex items-center gap-2">
                <input type="checkbox" checked={includeEvidence} onChange={(e) => setIncludeEvidence(e.target.checked)} disabled={busy} />
                Incluir hoja Evidencias
              </label>
              <label className="flex items-center gap-2">
                <input type="checkbox" checked={includeSummary} onChange={(e) => setIncludeSummary(e.target.checked)} disabled={busy} />
                Incluir hoja Resumen
              </label>
            </fieldset>
          </>
        )}
        {busy && (
          <div>
            <div
              role="progressbar"
              aria-label="Avance de la exportación"
              aria-valuemin={0}
              aria-valuemax={100}
              aria-valuenow={progressPercent(exporting)}
              className="h-2 overflow-hidden rounded-full bg-t4s-cream"
            >
              <div className="h-full bg-t4s-blue transition-all" style={{ width: `${progressPercent(exporting)}%` }} />
            </div>
            <p className="mt-1 text-xs text-t4s-gray">{progressLabel(exporting)}</p>
          </div>
        )}
        {errors._form && (
          <p role="alert" className="text-sm text-red-700">
            {errors._form}
          </p>
        )}
        <DialogFooter>
          <button type="button" className={btn.secondary} onClick={onClose} disabled={busy}>
            Cancelar
          </button>
          <button type="submit" className={btn.primary} disabled={busy}>
            {kind === "project" ? "Exportar Excel" : "Exportar plantilla"}
          </button>
        </DialogFooter>
      </form>
    </Dialog>
  );
}
