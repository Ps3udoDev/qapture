import type { FieldDef } from "../../domain/fields";
import { isRoutingStale, parseRepo, validateProjectGithub, type DeviceCode, type GhAccount, type GhRepo, type ProjectGithub, type ProjectGithubInput } from "../../domain/github";
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
  /** Simula `gh:connected`: guarda la cuenta y avisa. */
  emitConnected(account: GhAccount): void;
  /** Simula `gh:connect-failed`. */
  emitConnectFailed(message: string): void;
}

/** Replica en memoria los comandos `gh_*` y sus eventos. `fields` da las columnas de un proyecto. */
export function createMemoryGithub(fields: (projectId: string) => FieldDef[] = () => []): MemoryGithub {
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
  const m: MemoryGithub = {
    repos: [],
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
