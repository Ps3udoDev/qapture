export type AppErrorCode = "validation" | "not_found" | "db" | "io" | "capture" | "file_locked" | "export" | "corrupt" | "internal" | "contract"
  | "gh_auth" | "gh_forbidden" | "gh_not_found" | "gh_rate_limited" | "gh_offline" | "gh_keyring" | "gh_validation" | "gh_conflict";

export class AppError extends Error {
  readonly code: AppErrorCode;
  readonly detail: string;

  constructor(code: AppErrorCode, message: string, detail = "") {
    super(message);
    this.name = "AppError";
    this.code = code;
    this.detail = detail;
  }
}

export const isValidationError = (e: unknown): e is AppError =>
  e instanceof AppError && (e.code === "validation" || e.code === "not_found");
