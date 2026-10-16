export type AppErrorCode = "validation" | "not_found" | "db" | "io" | "capture" | "internal" | "contract";

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
