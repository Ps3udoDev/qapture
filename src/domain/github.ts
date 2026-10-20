import type { FormErrors } from "./cases";
import { charLength, type FieldDef } from "./fields";

export const INSTALL_URL = "https://github.com/apps/qapture-qa/installations/new";
export const REVOKE_URL = "https://github.com/settings/apps/authorizations";
export const DEFAULT_LABELS = ["bug", "qapture"];

export interface GhAccount {
  login: string;
  name: string | null;
  avatarDataUrl: string | null;
  connectedAt: string;
  needsReconnect: boolean;
}

export interface DeviceCode {
  userCode: string;
  verificationUri: string;
  expiresIn: number;
  interval: number;
}

export interface GhRepo {
  owner: string;
  name: string;
  fullName: string;
  private: boolean;
  canWriteIssues: boolean;
  canWriteContents: boolean;
}

export interface LinkedRepo {
  fullName: string;
  isDefault: boolean;
  defaultLabels: string[];
}

export interface ProjectGithubInput {
  repos: LinkedRepo[];
  routingFieldId: string | null;
  routingMap: Record<string, string>;
  evidenceRepo: string | null;
}

export interface ProjectGithub extends ProjectGithubInput {
  routingStale: boolean;
}

const OWNER_RE = /^[A-Za-z0-9](?:[A-Za-z0-9-]{0,38})$/;
const NAME_RE = /^[A-Za-z0-9._-]{1,100}$/;
const MAX_LABELS = 10;
const MAX_LABEL_CHARS = 50;

export function parseRepo(value: string): { owner: string; repo: string } | null {
  const parts = value.split("/");
  if (parts.length !== 2) return null;
  const [owner, repo] = parts;
  if (!OWNER_RE.test(owner) || !NAME_RE.test(repo) || repo === "." || repo === "..") return null;
  return { owner, repo };
}

/** Igual que `eq_ignore_ascii_case` de Rust: solo pliega A-Z. */
const asciiLower = (s: string) => s.replace(/[A-Z]/g, (c) => c.toLowerCase());
const sameRepo = (a: string, b: string) => asciiLower(a) === asciiLower(b);
const invalidRepo = (value: string) => `«${value}» no es un repositorio válido (usa dueño/repositorio).`;

const labelsOk = (labels: string[]) =>
  labels.length <= MAX_LABELS &&
  labels.every((l) => {
    const n = charLength(l);
    return n >= 1 && n <= MAX_LABEL_CHARS && !l.includes(",");
  });

/** Orden por punto de código (el de bytes UTF-8 del `BTreeMap` de Rust), para que el primer error coincida. */
const byCodePoint = (a: string, b: string) => {
  const x = [...a];
  const y = [...b];
  for (let i = 0; i < Math.min(x.length, y.length); i++) {
    const d = x[i].codePointAt(0)! - y[i].codePointAt(0)!;
    if (d !== 0) return d;
  }
  return x.length - y.length;
};

/**
 * Espejo de `project_config::validate` (Rust). Una clave por sección y, dentro de cada una, el
 * primer error; el primer valor del objeto es el mismo mensaje que devolvería Rust.
 */
export function validateProjectGithub(input: ProjectGithubInput, fields: FieldDef[]): FormErrors {
  const errors: FormErrors = {};
  const names: string[] = [];
  let reposError: string | null = null;
  for (const linked of input.repos) {
    const value = linked.fullName.trim();
    if (!parseRepo(value)) {
      reposError = invalidRepo(value);
      break;
    }
    names.push(value);
  }
  if (!reposError) {
    const dup = names.find((name, i) => names.slice(0, i).some((p) => sameRepo(p, name)));
    if (dup !== undefined) reposError = `El repositorio «${dup}» está repetido.`;
  }
  if (!reposError && input.repos.length > 0 && input.repos.filter((r) => r.isDefault).length !== 1) {
    reposError = "Marca un repositorio como predeterminado.";
  }
  if (reposError) errors.repos = reposError;

  if (!input.repos.every((r) => labelsOk(r.defaultLabels.map((l) => l.trim())))) {
    errors.labels = "Las etiquetas deben tener entre 1 y 50 caracteres, sin comas (máximo 10).";
  }

  if (input.routingFieldId !== null) {
    const field = fields.find((f) => f.id === input.routingFieldId && !f.archived);
    const options = Object.keys(input.routingMap).sort(byCodePoint);
    if (!field) errors.routing = "La columna de ruteo no existe en este proyecto.";
    else if (field.type !== "list") errors.routing = "La columna de ruteo debe ser de tipo Lista.";
    else {
      const bad = options.find((o) => !field.options.includes(o));
      if (bad !== undefined) errors.routing = `«${bad}» no es una opción de «${field.label}».`;
      else if (!reposError) {
        const linkedNames = input.repos.map((r) => r.fullName.trim());
        for (const option of options) {
          const target = input.routingMap[option].trim();
          if (!linkedNames.some((n) => sameRepo(n, target))) {
            errors.routing = `«${target}» no está vinculado al proyecto.`;
            break;
          }
        }
      }
    }
  }

  const evidence = input.evidenceRepo?.trim() ?? "";
  if (evidence !== "" && !parseRepo(evidence)) errors.evidenceRepo = invalidRepo(evidence);
  return errors;
}

/** Espejo de `project_config::is_stale`: la columna de ruteo guardada ya no sirve. */
export function isRoutingStale(config: ProjectGithubInput, fields: FieldDef[]): boolean {
  if (config.routingFieldId === null) return false;
  const field = fields.find((f) => f.id === config.routingFieldId);
  if (!field) return true;
  return field.archived || field.type !== "list" || Object.keys(config.routingMap).some((o) => !field.options.includes(o));
}
