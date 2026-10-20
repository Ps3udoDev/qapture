import type { StoreApi } from "zustand/vanilla";
import type { FormErrors } from "../../domain/cases";
import type { CaseIssueLink, DeviceCode, GhAccount, GhRepo, ProjectGithub, ProjectGithubInput, ReportInput, ReportResult } from "../../domain/github";
import { AppError, isValidationError } from "../../ports/errors";
import type { Repositories } from "../../ports/repositories";
import type { SliceHelpers } from "./evidence-slice";
import type { FormResult, WorkspaceState } from "./workspace";

export interface GithubState {
  account: GhAccount | null;
  /** Device Flow en curso; `startedAt` (ms) fija la cuenta regresiva del código. */
  flow: { code: DeviceCode; startedAt: number } | null;
  repos: GhRepo[] | null;
  status: "idle" | "loading";
  /** Vínculos caso-issue por proyecto. */
  links: Record<string, CaseIssueLink[]>;
  /** Capturas sin pegar del modo asistido; clave `caseId|owner/repo`. Solo en memoria. */
  pending: Record<string, PendingEvidence>;
}

export interface PendingEvidence {
  issueUrl: string;
  /** `n` es la posición 1..N dentro de las capturas elegidas. */
  items: { evidenceId: string; n: number; done: boolean }[];
  reason: string;
}

export type ReportOutcome = FormResult & { result?: ReportResult };

export interface GithubSlice {
  github: GithubState;
  loadGithubAccount(): Promise<void>;
  connectGithub(): Promise<void>;
  cancelGithubConnect(): Promise<void>;
  disconnectGithub(): Promise<void>;
  loadGithubRepos(): Promise<void>;
  /** Suscribe `gh:connected` y `gh:connect-failed`; devuelve la función que las cancela. */
  listenGithub(): Promise<() => void>;
  loadProjectGithub(projectId: string): Promise<ProjectGithub | null>;
  saveProjectGithub(projectId: string, input: ProjectGithubInput): Promise<FormResult>;
  loadProjectLinks(projectId: string): Promise<void>;
  reportIssue(input: ReportInput): Promise<ReportOutcome>;
  commentIssue(input: ReportInput): Promise<ReportOutcome>;
  copyPendingEvidence(key: string, evidenceId: string): Promise<void>;
  markPendingDone(key: string, evidenceId: string, done: boolean): void;
  dismissPending(key: string): void;
  openIssue(caseId: string, repo: string): Promise<void>;
}

type Set = StoreApi<WorkspaceState>["setState"];
type Get = StoreApi<WorkspaceState>["getState"];

export function createGithubSlice(repos: Repositories, set: Set, get: Get, h: SliceHelpers): GithubSlice {
  const patch = (change: Partial<GithubState>) => set((s) => ({ github: { ...s.github, ...change } }));

  /** Informa el error y, si la sesión ya no sirve, recarga la cuenta para mostrar «Reconectar». */
  const fail = async (error: unknown) => {
    h.report(error);
    if (error instanceof AppError && error.code === "gh_auth") {
      try {
        patch({ account: await repos.github.account() });
      } catch {
        // El error original ya se mostró.
      }
    }
  };

  const loadLinks = async (projectId: string) => {
    try {
      const links = await repos.github.projectLinks(projectId);
      set((s) => ({ github: { ...s.github, links: { ...s.github.links, [projectId]: links } } }));
    } catch (error) {
      await fail(error);
    }
  };

  /** Crea o comenta; recarga los vínculos, guarda los pendientes del modo asistido y avisa. */
  const report = async (input: ReportInput, run: (input: ReportInput) => Promise<ReportResult>, toast: (r: ReportResult) => string): Promise<ReportOutcome> => {
    const projectId = get().project?.id;
    try {
      const result = await run(input);
      if (projectId) await loadLinks(projectId);
      if (result.uploadMode === "assisted" && result.pendingEvidence.length > 0) {
        const key = `${input.caseId}|${result.link.fullName}`;
        const pending: PendingEvidence = {
          issueUrl: result.commentUrl ?? result.link.htmlUrl,
          items: result.pendingEvidence.map((evidenceId, i) => ({ evidenceId, n: i + 1, done: false })),
          reason: result.assistedReason ?? "",
        };
        set((s) => ({ github: { ...s.github, pending: { ...s.github.pending, [key]: pending } } }));
      }
      get().pushToast("success", toast(result));
      return { ok: true, result };
    } catch (error) {
      if (error instanceof AppError && error.code === "gh_conflict") {
        if (projectId) await loadLinks(projectId);
        return { ok: false, errors: { _form: error.message } };
      }
      if (isValidationError(error) || (error instanceof AppError && error.code === "gh_validation")) {
        return { ok: false, errors: { _form: (error as AppError).message } };
      }
      await fail(error);
      return { ok: false, errors: {} as FormErrors };
    }
  };

  return {
    github: { account: null, flow: null, repos: null, status: "idle", links: {}, pending: {} },

    async loadGithubAccount() {
      try {
        patch({ account: await repos.github.account() });
      } catch (error) {
        h.report(error);
      }
    },

    async connectGithub() {
      patch({ status: "loading" });
      try {
        const code = await repos.github.connectStart();
        patch({ flow: { code, startedAt: Date.now() } });
      } catch (error) {
        await fail(error);
      } finally {
        patch({ status: "idle" });
      }
    },

    async cancelGithubConnect() {
      patch({ flow: null });
      try {
        await repos.github.connectCancel();
      } catch (error) {
        h.report(error);
      }
    },

    async disconnectGithub() {
      try {
        await repos.github.disconnect();
        patch({ account: null, repos: null, flow: null });
      } catch (error) {
        await fail(error);
      }
    },

    async loadGithubRepos() {
      patch({ status: "loading" });
      try {
        patch({ repos: await repos.github.listRepos() });
      } catch (error) {
        await fail(error);
      } finally {
        patch({ status: "idle" });
      }
    },

    async listenGithub() {
      const offs: (() => void)[] = [];
      try {
        offs.push(
          await repos.github.onConnected((account) => {
            patch({ account, flow: null });
            get().pushToast("success", `Conectado como @${account.login}.`);
          }),
        );
        offs.push(
          await repos.github.onConnectFailed((message) => {
            patch({ flow: null });
            get().pushToast("error", message);
          }),
        );
      } catch (error) {
        for (const off of offs) off();
        throw error;
      }
      return () => {
        for (const off of offs) off();
      };
    },

    async loadProjectGithub(projectId) {
      try {
        return await repos.github.getProjectGithub(projectId);
      } catch (error) {
        await fail(error);
        return null;
      }
    },

    async saveProjectGithub(projectId, input): Promise<FormResult> {
      try {
        await repos.github.setProjectGithub(projectId, input);
        return { ok: true };
      } catch (error) {
        if (isValidationError(error)) return { ok: false, errors: { _form: error.message } };
        await fail(error);
        return { ok: false, errors: {} as FormErrors };
      }
    },

    loadProjectLinks: loadLinks,

    reportIssue: (input) => report(input, (i) => repos.github.createIssue(i), (r) => `Issue #${r.link.issueNumber} creado en ${r.link.fullName}.`),

    commentIssue: (input) => report(input, (i) => repos.github.commentIssue(i), (r) => `Comentario agregado en ${r.link.fullName}#${r.link.issueNumber}.`),

    async copyPendingEvidence(key, evidenceId) {
      try {
        await repos.github.copyEvidence(evidenceId);
        const item = get().github.pending[key]?.items.find((i) => i.evidenceId === evidenceId);
        if (item) get().pushToast("success", `Captura ${item.n} copiada. Pégala con Ctrl+V en GitHub.`);
      } catch (error) {
        await fail(error);
      }
    },

    markPendingDone(key, evidenceId, done) {
      set((s) => {
        const entry = s.github.pending[key];
        if (!entry) return s;
        const items = entry.items.map((i) => (i.evidenceId === evidenceId ? { ...i, done } : i));
        const { [key]: _removed, ...rest } = s.github.pending;
        const pending = items.every((i) => i.done) ? rest : { ...s.github.pending, [key]: { ...entry, items } };
        return { github: { ...s.github, pending } };
      });
    },

    dismissPending(key) {
      set((s) => {
        const { [key]: _removed, ...rest } = s.github.pending;
        return { github: { ...s.github, pending: rest } };
      });
    },

    async openIssue(caseId, repo) {
      try {
        await repos.github.openIssue(caseId, repo);
      } catch (error) {
        await fail(error);
      }
    },
  };
}
