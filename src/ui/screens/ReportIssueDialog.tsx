import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { CaseRecord } from "../../domain/cases";
import { todayIso } from "../../domain/doc-versions";
import type { Evidence } from "../../domain/evidence";
import type { ProjectGithub, RepoMeta, ReportInput } from "../../domain/github";
import { buildIssue, buildRetestComment } from "../../domain/issue-template";
import { suggestRepo } from "../../domain/repo-routing";
import { Dialog, DialogFooter } from "../components/Dialog";
import { btn, inputCls } from "../components/styles";
import { useStoreApi, useWorkspace } from "../store/context";
import { EvidencePicker, IssueMetaFields } from "./ReportIssueFields";

type Phase = "loading" | "no-account" | "no-repos" | "ready";
const TITLE = "Reportar en GitHub";
const sameRepo = (a: string, b: string) => a.toLowerCase() === b.toLowerCase();

export function ReportIssueDialog({ caseRecord, onClose, onOpenAccountSettings, onOpenProjectGithub, onAssisted }: {
  caseRecord: CaseRecord;
  onClose: () => void;
  onOpenAccountSettings: () => void;
  onOpenProjectGithub: () => void;
  /** Reporte en modo asistido con capturas pendientes; recibe la clave `caseId|owner/repo`. */
  onAssisted: (pendingKey: string) => void;
}) {
  const store = useStoreApi();
  const project = useWorkspace((s) => s.project)!;
  const evidence = useWorkspace((s) => s.evidence);
  const account = useWorkspace((s) => s.github.account);
  const links = useWorkspace((s) => s.github.links[project.id]);
  const imageUrl = useWorkspace((s) => s.imageUrl);
  const readImage = useWorkspace((s) => s.readImage);
  const testerName = useWorkspace((s) => s.testerName);
  const role = useWorkspace((s) => s.settings?.settings.profile.role ?? "");
  const appVersion = useWorkspace((s) => s.settings?.appVersion ?? "");
  const loadGithubAccount = useWorkspace((s) => s.loadGithubAccount);
  const loadProjectGithub = useWorkspace((s) => s.loadProjectGithub);
  const loadProjectLinks = useWorkspace((s) => s.loadProjectLinks);
  const loadRepoMeta = useWorkspace((s) => s.loadRepoMeta);
  const reportIssue = useWorkspace((s) => s.reportIssue);
  const commentIssue = useWorkspace((s) => s.commentIssue);
  const openIssue = useWorkspace((s) => s.openIssue);

  const [phase, setPhase] = useState<Phase>("loading");
  const [config, setConfig] = useState<ProjectGithub | null>(null);
  const [repo, setRepo] = useState("");
  const [meta, setMeta] = useState<RepoMeta | null>(null);
  const [title, setTitle] = useState("");
  const [body, setBody] = useState("");
  const [labels, setLabels] = useState<string[]>([]);
  const [assignees, setAssignees] = useState<string[]>([]);
  const [milestone, setMilestone] = useState<number | null>(null);
  const [selected, setSelected] = useState<Set<string>>(() => new Set(evidence.map((e) => e.id)));
  const [sizes, setSizes] = useState<Record<string, number>>({});
  const [preview, setPreview] = useState(false);
  const [progress, setProgress] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const sending = useRef(false);

  const existing = useMemo(() => (repo ? links?.find((l) => l.caseId === caseRecord.id && sameRepo(l.fullName, repo)) : undefined), [links, caseRecord.id, repo]);
  const chosen = useMemo(() => evidence.filter((e) => selected.has(e.id)), [evidence, selected]);
  const tester = testerName().trim() || account?.login || "";

  useEffect(() => {
    let active = true;
    void (async () => {
      await loadGithubAccount();
      if (!active) return;
      if (!store.getState().github.account) return setPhase("no-account");
      await loadProjectLinks(project.id);
      const cfg = await loadProjectGithub(project.id);
      if (!active) return;
      if (!cfg || cfg.repos.length === 0) return setPhase("no-repos");
      setConfig(cfg);
      setRepo(suggestRepo(cfg, project.fields, caseRecord) ?? cfg.repos[0].fullName);
      setPhase("ready");
    })();
    return () => {
      active = false;
    };
    // Solo al abrir: la configuración del proyecto se lee una vez.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    let active = true;
    for (const e of evidence) {
      readImage(e.renderedPath)
        .then((bytes) => {
          if (active) setSizes((s) => ({ ...s, [e.id]: bytes.byteLength }));
        })
        .catch(() => undefined);
    }
    return () => {
      active = false;
    };
  }, [evidence, readImage]);

  const compose = useCallback(
    (picked: Evidence[]) => {
      const evidences = picked.map((e) => ({ description: e.description }));
      const issue = buildIssue({ caseRecord, fields: project.fields, project, evidences, tester, role, appVersion, today: todayIso() });
      return existing ? { title: issue.title, body: buildRetestComment({ caseRecord, evidences, tester, today: todayIso() }) } : issue;
    },
    [existing, caseRecord, project, tester, role, appVersion],
  );

  // Al cambiar de repo (o de modo crear/comentar) se recargan los metadatos y se rearma el texto.
  const mode = existing ? `${existing.fullName}#${existing.issueNumber}` : "new";
  useEffect(() => {
    if (phase !== "ready" || !repo) return;
    let active = true;
    setMeta(null);
    void loadRepoMeta(repo).then((m) => {
      if (active) setMeta(m);
    });
    const built = compose(chosen);
    setBody(built.body);
    setTitle(built.title);
    // Milestones y asignados son de cada repo: no se arrastran al cambiar.
    setMilestone(null);
    setAssignees([]);
    if (!existing) {
      setLabels(config?.repos.find((r) => sameRepo(r.fullName, repo))?.defaultLabels ?? []);
    }
    return () => {
      active = false;
    };
    // El cambio de selección no regenera el texto (se hace con «Regenerar descripción»).
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [phase, repo, mode]);

  const close = () => {
    if (!sending.current) onClose();
  };

  async function submit() {
    if (sending.current || !account) return;
    sending.current = true;
    setError(null);
    const verb = existing ? "Comentando…" : "Creando issue…";
    setProgress(chosen.length > 0 ? "Subiendo capturas…" : verb);
    const input: ReportInput = {
      caseId: caseRecord.id,
      repo,
      title,
      body,
      labels: existing ? [] : labels,
      assignees: existing ? [] : assignees,
      milestone: existing ? null : milestone,
      evidenceIds: chosen.map((e) => e.id),
    };
    const outcome = existing ? await commentIssue(input) : await reportIssue(input);
    sending.current = false;
    setProgress(null);
    if (outcome.ok && outcome.result) {
      const r = outcome.result;
      onClose();
      if (r.uploadMode === "assisted" && r.pendingEvidence.length > 0) {
        onAssisted(`${caseRecord.id}|${r.link.fullName}`);
        void openIssue(caseRecord.id, r.link.fullName);
      }
      return;
    }
    if (!outcome.ok) setError(outcome.errors._form ?? null);
  }

  if (phase === "no-account" || phase === "no-repos" || phase === "loading") {
    return (
      <Dialog title={TITLE} kicker={caseRecord.code} onClose={onClose}>
        {phase === "loading" && <p className="text-sm text-t4s-gray">Cargando…</p>}
        {phase === "no-account" && (
          <Notice text="Conecta tu cuenta de GitHub en Ajustes › GitHub." action="Abrir Ajustes" onAction={() => { onClose(); onOpenAccountSettings(); }} />
        )}
        {phase === "no-repos" && (
          <Notice text="Este proyecto no tiene repositorios vinculados." action="Vincular repositorio" onAction={() => { onClose(); onOpenProjectGithub(); }} />
        )}
      </Dialog>
    );
  }

  const busy = progress !== null;
  const login = account?.login ?? "";
  return (
    <Dialog wide title={existing ? `Comentar en ${existing.fullName}#${existing.issueNumber}` : TITLE} kicker={caseRecord.code} onClose={close}>
      <form
        noValidate
        className="space-y-4"
        onSubmit={(e) => {
          e.preventDefault();
          void submit();
        }}
      >
        <div>
          <label className="block text-sm font-medium" htmlFor="ri-repo">
            Repositorio
          </label>
          <select id="ri-repo" className={`${inputCls} mt-1`} value={repo} disabled={busy} onChange={(e) => setRepo(e.target.value)}>
            {config?.repos.map((r) => (
              <option key={r.fullName}>{r.fullName}</option>
            ))}
          </select>
        </div>
        {!existing && (
          <div>
            <label className="block text-sm font-medium" htmlFor="ri-title">
              Título
            </label>
            <input id="ri-title" className={`${inputCls} mt-1`} value={title} onChange={(e) => setTitle(e.target.value)} />
          </div>
        )}
        <div>
          <div className="flex items-end justify-between">
            <label className="text-sm font-medium" htmlFor="ri-body">
              Descripción
            </label>
            <div role="tablist" className="flex gap-1">
              {(["Editar", "Vista previa"] as const).map((t) => (
                <button
                  key={t}
                  type="button"
                  role="tab"
                  aria-selected={preview === (t === "Vista previa")}
                  onClick={() => setPreview(t === "Vista previa")}
                  className={`border-b-2 px-2 py-1 text-xs ${preview === (t === "Vista previa") ? "border-t4s-blue font-medium text-t4s-blue" : "border-transparent text-t4s-gray"}`}
                >
                  {t}
                </button>
              ))}
            </div>
          </div>
          {preview ? (
            <pre data-testid="issue-preview" className="mt-1 max-h-72 overflow-y-auto whitespace-pre-wrap rounded-md border border-t4s-line bg-t4s-cream/40 p-3 font-mono text-xs">
              {body}
            </pre>
          ) : (
            <textarea id="ri-body" rows={12} className={`${inputCls} mt-1 font-mono text-xs`} value={body} onChange={(e) => setBody(e.target.value)} />
          )}
        </div>
        {!existing && (
          <IssueMetaFields meta={meta} labels={labels} assignees={assignees} milestone={milestone} onLabels={setLabels} onAssignees={setAssignees} onMilestone={setMilestone} />
        )}
        <EvidencePicker
          evidence={evidence}
          selected={selected}
          sizes={sizes}
          urlFor={(e) => imageUrl(e.renderedPath, e.updatedAt)}
          onToggle={(id, on) =>
            setSelected((s) => {
              const next = new Set(s);
              if (on) next.add(id);
              else next.delete(id);
              return next;
            })
          }
          onRegenerate={() => setBody(compose(chosen).body)}
        />
        <p className="text-sm text-t4s-gray">
          {existing ? `Se comentará en ${existing.fullName}#${existing.issueNumber} como @${login}.` : `Se creará en ${repo} como @${login}.`}
        </p>
        {error && (
          <p role="alert" className="text-sm text-red-700">
            {error}
          </p>
        )}
        <DialogFooter>
          {progress && <span className="mr-auto self-center text-sm text-t4s-gray">{progress}</span>}
          <button type="button" className={btn.secondary} disabled={busy} onClick={close}>
            Cancelar
          </button>
          <button type="submit" className={btn.primary} disabled={busy}>
            {existing ? "Comentar" : "Crear issue"}
          </button>
        </DialogFooter>
      </form>
    </Dialog>
  );
}

function Notice({ text, action, onAction }: { text: string; action: string; onAction: () => void }) {
  return (
    <div className="space-y-4">
      <p className="text-sm">{text}</p>
      <DialogFooter>
        <button type="button" className={btn.primary} onClick={onAction}>
          {action}
        </button>
      </DialogFooter>
    </div>
  );
}
