import type { StoreApi } from "zustand/vanilla";
import type { FormErrors } from "../../domain/cases";
import type { DeviceCode, GhAccount, GhRepo, ProjectGithub, ProjectGithubInput } from "../../domain/github";
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
}

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

  return {
    github: { account: null, flow: null, repos: null, status: "idle" },

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
  };
}
