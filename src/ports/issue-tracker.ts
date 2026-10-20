import type { CaseIssueLink, DeviceCode, GhAccount, GhRepo, ProjectGithub, ProjectGithubInput, RepoMeta, ReportInput, ReportResult } from "../domain/github";
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
  /** Etiquetas, milestones y asignados del repo. */
  repoMeta(repo: string): Promise<RepoMeta>;
  /** Vínculos caso-issue de todo el proyecto. */
  projectLinks(projectId: string): Promise<CaseIssueLink[]>;
  createIssue(input: ReportInput): Promise<ReportResult>;
  commentIssue(input: ReportInput): Promise<ReportResult>;
  /** Pone el PNG de la captura en el portapapeles. */
  copyEvidence(evidenceId: string): Promise<void>;
  /** Abre en el navegador el issue vinculado. */
  openIssue(caseId: string, repo: string): Promise<void>;
  /** Evento `gh:connected`. */
  onConnected(handler: (account: GhAccount) => void): Promise<Unlisten>;
  /** Evento `gh:connect-failed`. */
  onConnectFailed(handler: (message: string) => void): Promise<Unlisten>;
}
