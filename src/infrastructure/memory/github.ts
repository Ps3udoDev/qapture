import type { FieldDef } from "../../domain/fields";
import { isRoutingStale, parseRepo, validateProjectGithub, type CaseIssueLink, type DeviceCode, type GhAccount, type GhRepo, type ProjectGithub, type ProjectGithubInput, type RepoMeta, type ReportInput, type ReportResult } from "../../domain/github";
import { AppError } from "../../ports/errors";
import type { IssueTracker } from "../../ports/issue-tracker";

export interface MemoryGithub extends IssueTracker {
  /** Repos que devuelve `listRepos`. */
  repos: GhRepo[];
  /** Cuenta conectada (`null` = sin cuenta). */
  accountValue: GhAccount | null;
  /** Error con el que falla la siguiente llamada (se consume). */
  nextError: AppError | null;
  /** Código que devuelve `connectStart`. */
  deviceCode: DeviceCode;
  /** Veces que se pidió iniciar / cancelar la conexión. */
  starts: number;
  cancels: number;
  /** Vínculos caso-issue guardados. */
  links: CaseIssueLink[];
  /** Metadatos por repo (`owner/repo`) que devuelve `repoMeta`. */
  metas: Record<string, RepoMeta>;
  /** Fuerza el modo de subida de los reportes; `null` = `repo`. */
  forcedUpload: { mode: "repo" | "assisted"; reason?: string } | null;
  /** Veces que se copió una captura, y cuáles. */
  copies: number;
  copied: string[];
  /** Issues abiertos en el navegador. */
  opened: { caseId: string; repo: string }[];
  /** Simula `gh:connected`: guarda la cuenta y avisa. */
  emitConnected(account: GhAccount): void;
  /** Simula `gh:connect-failed`. */
  emitConnectFailed(message: string): void;
}

/** Replica en memoria los comandos `gh_*` y sus eventos. `fields` da las columnas de un proyecto. */
export function createMemoryGithub(fields: (projectId: string) => FieldDef[] = () => [], caseProject: (caseId: string) => string | undefined = () => undefined): MemoryGithub {
  const connected = new Set<(account: GhAccount) => void>();
  const failed = new Set<(message: string) => void>();
  const configs = new Map<string, ProjectGithubInput>();
  const copy = <T>(value: T): T => structuredClone(value);
  const take = () => {
    if (m.nextError) {
      const e = m.nextError;
      m.nextError = null;
      throw e;
    }
  };
  const view = (projectId: string): ProjectGithub => {
    const config = configs.get(projectId) ?? { repos: [], routingFieldId: null, routingMap: {}, evidenceRepo: null };
    return { ...copy(config), routingStale: isRoutingStale(config, fields(projectId)) };
  };
  let issueSeq = 0;
  const findLink = (caseId: string, repo: string) => m.links.find((l) => l.caseId === caseId && l.fullName.toLowerCase() === repo.trim().toLowerCase());
  const report = (input: ReportInput, link: CaseIssueLink, comment: boolean): ReportResult => {
    const forced = m.forcedUpload;
    const assisted = input.evidenceIds.length > 0 && forced?.mode === "assisted";
    return {
      link: copy(link),
      uploadMode: forced?.mode ?? "repo",
      assistedReason: forced?.mode === "assisted" ? (forced.reason ?? "No hay repo de evidencias configurado.") : null,
      pendingEvidence: assisted ? [...input.evidenceIds] : [],
      commentUrl: comment ? `${link.htmlUrl}#issuecomment-${++issueSeq}` : null,
    };
  };
  const m: MemoryGithub = {
    repos: [],
    links: [],
    metas: {},
    forcedUpload: null,
    copies: 0,
    copied: [],
    opened: [],
    accountValue: null,
    nextError: null,
    deviceCode: { userCode: "WDJB-MJHT", verificationUri: "https://github.com/login/device", expiresIn: 900, interval: 5 },
    starts: 0,
    cancels: 0,
    async account() {
      take();
      return m.accountValue ? copy(m.accountValue) : null;
    },
    async connectStart() {
      take();
      m.starts += 1;
      return copy(m.deviceCode);
    },
    async connectCancel() {
      m.cancels += 1;
    },
    async disconnect() {
      take();
      m.accountValue = null;
    },
    async listRepos() {
      take();
      return copy(m.repos);
    },
    async getProjectGithub(projectId) {
      take();
      return view(projectId);
    },
    async setProjectGithub(projectId, config) {
      take();
      const message = Object.values(validateProjectGithub(config, fields(projectId)))[0];
      if (message) throw new AppError("validation", message);
      const clean = copy(config);
      clean.repos = clean.repos.map((r) => ({ ...r, fullName: r.fullName.trim(), defaultLabels: r.defaultLabels.map((l) => l.trim()) }));
      const full = (name: string) => clean.repos.find((r) => r.fullName.toLowerCase() === name.trim().toLowerCase())?.fullName ?? name;
      clean.routingMap = clean.routingFieldId === null ? {} : Object.fromEntries(Object.entries(clean.routingMap).map(([option, target]) => [option, full(target)]));
      const evidence = clean.evidenceRepo?.trim() ?? "";
      clean.evidenceRepo = evidence === "" || !parseRepo(evidence) ? null : evidence;
      configs.set(projectId, clean);
      return view(projectId);
    },
    async repoMeta(repo) {
      take();
      return copy(m.metas[repo] ?? { labels: [], milestones: [], assignees: [] });
    },
    async projectLinks(projectId) {
      take();
      return copy(m.links.filter((l) => caseProject(l.caseId) === projectId));
    },
    async createIssue(input) {
      take();
      const existing = findLink(input.caseId, input.repo);
      if (existing) throw new AppError("gh_conflict", `El caso ya tiene un issue en ${existing.fullName} (#${existing.issueNumber}).`);
      const parsed = parseRepo(input.repo.trim());
      if (!parsed) throw new AppError("gh_validation", "El repositorio no es válido.");
      const number = ++issueSeq;
      const fullName = `${parsed.owner}/${parsed.repo}`;
      const link: CaseIssueLink = { caseId: input.caseId, ...parsed, fullName, issueNumber: number, htmlUrl: `https://github.com/${fullName}/issues/${number}`, direction: "exported", issueState: "open" };
      m.links.push(link);
      return report(input, link, false);
    },
    async commentIssue(input) {
      take();
      const link = findLink(input.caseId, input.repo);
      if (!link) throw new AppError("not_found", "El caso no tiene un issue en ese repositorio.");
      return report(input, link, true);
    },
    async copyEvidence(evidenceId) {
      take();
      m.copies += 1;
      m.copied.push(evidenceId);
    },
    async openIssue(caseId, repo) {
      take();
      if (!findLink(caseId, repo)) throw new AppError("not_found", "El caso no tiene un issue en ese repositorio.");
      m.opened.push({ caseId, repo });
    },
    async onConnected(handler) {
      connected.add(handler);
      return () => void connected.delete(handler);
    },
    async onConnectFailed(handler) {
      failed.add(handler);
      return () => void failed.delete(handler);
    },
    emitConnected(account) {
      m.accountValue = copy(account);
      for (const h of [...connected]) h(copy(account));
    },
    emitConnectFailed(message) {
      for (const h of [...failed]) h(message);
    },
  };
  return m;
}
