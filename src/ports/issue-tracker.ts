import type { DeviceCode, GhAccount, GhRepo, ProjectGithub, ProjectGithubInput } from "../domain/github";
import type { Unlisten } from "./updater";

/** Cuenta, repos y configuración GitHub por proyecto (F8a). El token nunca sale de Rust. */
export interface IssueTracker {
  /** `null` si no hay cuenta conectada. */
  account(): Promise<GhAccount | null>;
  /** Inicia el Device Flow; el resultado llega por `onConnected` / `onConnectFailed`. */
  connectStart(): Promise<DeviceCode>;
  connectCancel(): Promise<void>;
  disconnect(): Promise<void>;
  listRepos(): Promise<GhRepo[]>;
  getProjectGithub(projectId: string): Promise<ProjectGithub>;
  setProjectGithub(projectId: string, config: ProjectGithubInput): Promise<ProjectGithub>;
  /** Evento `gh:connected`. */
  onConnected(handler: (account: GhAccount) => void): Promise<Unlisten>;
  /** Evento `gh:connect-failed`. */
  onConnectFailed(handler: (message: string) => void): Promise<Unlisten>;
}
