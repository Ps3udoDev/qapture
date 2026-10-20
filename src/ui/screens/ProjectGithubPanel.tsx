import { useEffect, useMemo, useState } from "react";
import type { FormErrors } from "../../domain/cases";
import { activeFields } from "../../domain/fields";
import { DEFAULT_LABELS, INSTALL_URL, validateProjectGithub, type GhRepo, type ProjectGithub, type ProjectGithubInput } from "../../domain/github";
import { btn, inputCls } from "../components/styles";
import { useWorkspace } from "../store/context";

interface Row {
  fullName: string;
  isDefault: boolean;
  labels: string;
}

const sectionCls = "grid gap-3 border-b border-t4s-line pb-4";
const selectCls = `${inputCls} max-w-xs`;
const splitLabels = (text: string) =>
  text
    .split(",")
    .map((l) => l.trim())
    .filter((l) => l !== "");

async function copy(text: string): Promise<boolean> {
  try {
    if (!navigator.clipboard) return false;
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    return false;
  }
}

function Err({ message }: { message?: string }) {
  return message ? (
    <p role="alert" className="text-xs text-red-700">
      {message}
    </p>
  ) : null;
}

/** Pestaña «GitHub» de Configurar proyecto: repos vinculados, ruteo por columna y repo de evidencias. */
export function ProjectGithubPanel({ onOpenAccountSettings }: { onOpenAccountSettings: () => void }) {
  const project = useWorkspace((s) => s.project);
  const account = useWorkspace((s) => s.github.account);
  const accessible = useWorkspace((s) => s.github.repos);
  const loadAccount = useWorkspace((s) => s.loadGithubAccount);
  const loadRepos = useWorkspace((s) => s.loadGithubRepos);
  const loadProject = useWorkspace((s) => s.loadProjectGithub);
  const save = useWorkspace((s) => s.saveProjectGithub);
  const pushToast = useWorkspace((s) => s.pushToast);

  const [loaded, setLoaded] = useState<ProjectGithub | null>(null);
  const [rows, setRows] = useState<Row[]>([]);
  const [routingFieldId, setRoutingFieldId] = useState<string | null>(null);
  const [routingMap, setRoutingMap] = useState<Record<string, string>>({});
  const [evidenceRepo, setEvidenceRepo] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [errors, setErrors] = useState<FormErrors>({});
  const [busy, setBusy] = useState(false);

  const projectId = project?.id;
  const fields = useMemo(() => activeFields(project?.fields ?? []), [project?.fields]);
  const listFields = fields.filter((f) => f.type === "list");
  const hasAccount = account !== null;

  useEffect(() => {
    void loadAccount();
  }, [loadAccount]);

  useEffect(() => {
    if (hasAccount) void loadRepos();
  }, [hasAccount, loadRepos]);

  useEffect(() => {
    if (!projectId) return;
    let cancelled = false;
    void loadProject(projectId).then((config) => {
      if (cancelled || !config) return;
      setLoaded(config);
      setRows(config.repos.map((r) => ({ fullName: r.fullName, isDefault: r.isDefault, labels: r.defaultLabels.join(", ") })));
      // Una columna que ya no sirve se descarta; en una Lista vigente solo se conservan las opciones que existen.
      const field = listFields.find((f) => f.id === config.routingFieldId);
      setRoutingFieldId(field ? field.id : null);
      setRoutingMap(field ? Object.fromEntries(Object.entries(config.routingMap).filter(([option]) => field.options.includes(option))) : {});
      setEvidenceRepo(config.evidenceRepo);
    });
    return () => {
      cancelled = true;
    };
    // `listFields` se deriva del proyecto; solo se recarga al cambiar de proyecto.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [projectId, loadProject]);

  if (!project) return null;

  if (!hasAccount) {
    return (
      <div className="grid gap-3">
        <p className="text-sm">Conecta tu cuenta de GitHub en Ajustes › GitHub para vincular repositorios.</p>
        <div>
          <button type="button" className={btn.primary} onClick={onOpenAccountSettings}>
            Abrir Ajustes
          </button>
        </div>
      </div>
    );
  }

  const repoList: GhRepo[] = accessible ?? [];
  const find = (fullName: string) => repoList.find((r) => r.fullName.toLowerCase() === fullName.toLowerCase());
  const linked = new Set(rows.map((r) => r.fullName.toLowerCase()));
  const candidates = repoList.filter((r) => !linked.has(r.fullName.toLowerCase()) && r.fullName.toLowerCase().includes(query.trim().toLowerCase()));
  const owners = [...new Set(candidates.map((r) => r.owner))];
  const routingField = listFields.find((f) => f.id === routingFieldId) ?? null;

  const addRepo = (fullName: string) => {
    if (!fullName) return;
    setRows((prev) => [...prev, { fullName, isDefault: prev.length === 0, labels: DEFAULT_LABELS.join(", ") }]);
    setQuery("");
  };
  const removeRepo = (fullName: string) => {
    setRows((prev) => prev.filter((r) => r.fullName !== fullName));
    setRoutingMap((prev) => Object.fromEntries(Object.entries(prev).filter(([, target]) => target !== fullName)));
  };
  const changeRouting = (id: string) => {
    setRoutingFieldId(id === "" ? null : id);
    setRoutingMap({});
  };
  const mapOption = (option: string, target: string) =>
    setRoutingMap((prev) => {
      const next = { ...prev };
      if (target === "") delete next[option];
      else next[option] = target;
      return next;
    });
  const notFound = async () => {
    if (await copy(INSTALL_URL)) pushToast("success", "Pide a un owner de la organización que instale Qapture en ese repositorio. Enlace copiado.");
    else pushToast("error", "No se pudo copiar el enlace.");
  };

  async function submit() {
    const input: ProjectGithubInput = {
      repos: rows.map((r) => ({ fullName: r.fullName, isDefault: r.isDefault, defaultLabels: splitLabels(r.labels) })),
      routingFieldId,
      routingMap: routingFieldId === null ? {} : routingMap,
      evidenceRepo,
    };
    const found = validateProjectGithub(input, fields);
    setErrors(found);
    if (Object.keys(found).length > 0) return;
    setBusy(true);
    const result = await save(project!.id, input);
    setBusy(false);
    if (!result.ok) return setErrors(result.errors);
    pushToast("success", "Configuración de GitHub guardada.");
    setLoaded((prev) => (prev ? { ...prev, ...input, routingStale: false } : prev));
  }

  const evidence = evidenceRepo ? find(evidenceRepo) : undefined;

  return (
    <div className="grid gap-4">
      <section className={sectionCls} aria-label="Repositorios vinculados">
        <h3 className="text-sm font-semibold">Repositorios vinculados</h3>
        {rows.length === 0 && <p className="text-sm text-t4s-gray">Aún no hay repositorios vinculados.</p>}
        {rows.map((row) => {
          const info = find(row.fullName);
          return (
            <div key={row.fullName} className="grid gap-1 rounded-md border border-t4s-line p-3">
              <div className="flex flex-wrap items-center gap-3">
                <span className="text-sm font-medium">{row.fullName}</span>
                <label className="flex items-center gap-1 text-sm">
                  <input
                    type="radio"
                    name="gh-default-repo"
                    aria-label={`Predeterminado ${row.fullName}`}
                    checked={row.isDefault}
                    onChange={() => setRows((prev) => prev.map((r) => ({ ...r, isDefault: r.fullName === row.fullName })))}
                  />
                  Predeterminado
                </label>
                <input
                  aria-label={`Etiquetas de ${row.fullName}`}
                  className={`${inputCls} max-w-xs`}
                  value={row.labels}
                  onChange={(e) => setRows((prev) => prev.map((r) => (r.fullName === row.fullName ? { ...r, labels: e.target.value } : r)))}
                />
                <button type="button" className={btn.ghost} aria-label={`Quitar ${row.fullName}`} onClick={() => removeRepo(row.fullName)}>
                  Quitar
                </button>
              </div>
              {info && !info.canWriteIssues && <p className="text-xs text-amber-800">Tu cuenta no puede crear issues en este repositorio.</p>}
            </div>
          );
        })}
        <Err message={errors.repos} />
        <Err message={errors.labels} />
        <div className="flex flex-wrap items-end gap-2">
          <input aria-label="Buscar repositorio" placeholder="Buscar repositorio" className={`${inputCls} max-w-[12rem]`} value={query} onChange={(e) => setQuery(e.target.value)} />
          <select aria-label="Agregar repositorio" className={selectCls} value="" onChange={(e) => addRepo(e.target.value)}>
            <option value="">Agregar repositorio…</option>
            {owners.map((owner) => (
              <optgroup key={owner} label={owner}>
                {candidates
                  .filter((r) => r.owner === owner)
                  .map((r) => (
                    <option key={r.fullName} value={r.fullName}>
                      {r.private ? "🔒 " : ""}
                      {r.fullName}
                    </option>
                  ))}
              </optgroup>
            ))}
          </select>
          <button type="button" className="text-sm text-t4s-blue underline hover:text-t4s-charcoal" onClick={() => void notFound()}>
            ¿No ves tu repo?
          </button>
        </div>
      </section>

      <section className={sectionCls} aria-label="Ruteo por columna">
        <h3 className="text-sm font-semibold">Ruteo por columna (opcional)</h3>
        <select aria-label="Columna de ruteo" className={selectCls} value={routingFieldId ?? ""} onChange={(e) => changeRouting(e.target.value)}>
          <option value="">Sin ruteo</option>
          {listFields.map((f) => (
            <option key={f.id} value={f.id}>
              {f.label}
            </option>
          ))}
        </select>
        {loaded?.routingStale && <p className="text-xs text-amber-800">La columna o alguna opción del ruteo cambió; revísalo.</p>}
        {routingField &&
          routingField.options.map((option) => (
            <div key={option} className="flex items-center gap-3">
              <span className="w-40 text-sm">{option}</span>
              <select aria-label={`Repo para ${option}`} className={selectCls} value={routingMap[option] ?? ""} onChange={(e) => mapOption(option, e.target.value)}>
                <option value="">Predeterminado</option>
                {rows.map((r) => (
                  <option key={r.fullName} value={r.fullName}>
                    {r.fullName}
                  </option>
                ))}
              </select>
            </div>
          ))}
        <Err message={errors.routing} />
      </section>

      <section className="grid gap-3" aria-label="Repo de evidencias">
        <h3 className="text-sm font-semibold">Repo de evidencias (opcional)</h3>
        <select aria-label="Repo de evidencias" className={selectCls} value={evidenceRepo ?? ""} onChange={(e) => setEvidenceRepo(e.target.value === "" ? null : e.target.value)}>
          <option value="">Ninguno: pegar capturas a mano</option>
          {evidenceRepo && !evidence && <option value={evidenceRepo}>{evidenceRepo}</option>}
          {repoList.map((r) => (
            <option key={r.fullName} value={r.fullName}>
              {r.private ? "🔒 " : ""}
              {r.fullName}
            </option>
          ))}
        </select>
        {evidence && !evidence.canWriteContents && <p className="text-xs text-amber-800">Tu cuenta no puede subir archivos a este repositorio; se usará el modo asistido.</p>}
        <Err message={errors.evidenceRepo} />
      </section>

      {errors._form && (
        <p role="alert" className="text-sm text-red-700">
          {errors._form}
        </p>
      )}
      <div className="flex justify-end border-t border-t4s-line pt-4">
        <button type="button" className={btn.primary} disabled={busy} onClick={() => void submit()}>
          Guardar
        </button>
      </div>
    </div>
  );
}
