/** F6: ajustes de la aplicación. Espejo de `settings.rs` y `system/shortcuts.rs`. */
import type { FormErrors } from "./cases";
import { IMAGE_WIDTH } from "./export";
import { charLength } from "./fields";

export const SHORTCUT_ACTIONS = ["toggle", "capture", "paste", "newCase"] as const;
export type ShortcutAction = (typeof SHORTCUT_ACTIONS)[number];

export interface Settings {
  profile: { tester: string; role: string; email: string };
  shortcuts: Record<ShortcutAction, string>;
  system: { autostart: boolean; startHidden: boolean; closeToTray: boolean; escHides: boolean };
  export: { defaultFolder: string; imageMaxWidth: number; includeSummary: boolean };
}

export interface ShortcutFailure {
  action: ShortcutAction;
  accelerator: string;
}

export interface SettingsSnapshot {
  settings: Settings;
  /** Atajos que el SO no dejó registrar al arrancar. */
  shortcutFailures: ShortcutFailure[];
  appVersion: string;
  dataDir: string;
  /** Datos junto al exe (`portable.flag`): sin inicio con Windows. */
  portable: boolean;
}

export type LinkTarget = "releases" | "docs";

export const DEFAULT_SETTINGS: Settings = {
  profile: { tester: "", role: "", email: "" },
  shortcuts: { toggle: "Ctrl+Alt+Q", capture: "Ctrl+Alt+S", paste: "Ctrl+Alt+V", newCase: "Ctrl+Alt+N" },
  system: { autostart: false, startHidden: true, closeToTray: true, escHides: false },
  export: { defaultFolder: "", imageMaxWidth: IMAGE_WIDTH.default, includeSummary: true },
};

export const SHORTCUT_LABEL: Record<ShortcutAction, string> = {
  toggle: "Mostrar/ocultar",
  capture: "Capturar pantalla",
  paste: "Pegar evidencia",
  newCase: "Nuevo caso",
};

const RESERVED = ["Alt+F4", "Ctrl+Alt+Delete", "Win+L", "Alt+Tab", "Ctrl+Esc", "Win+D", "Win+E", "Win+R", "Win+Tab"];

const MODIFIER_NAMES: Record<string, "Ctrl" | "Alt" | "Shift" | "Win"> = {
  ctrl: "Ctrl",
  control: "Ctrl",
  alt: "Alt",
  shift: "Shift",
  win: "Win",
  super: "Win",
  meta: "Win",
};

const isValidKey = (key: string) => /^[A-Za-z0-9]$/.test(key) || /^[Ff]0*([1-9]|1\d|2[0-4])$/.test(key);

/** Forma canónica (`Ctrl`, `Alt`, `Shift`, `Win` y la tecla) o `null`. Igual que `shortcuts::normalize`. */
export function normalizeAccelerator(raw: string): string | null {
  const on = { Ctrl: false, Alt: false, Shift: false, Win: false };
  let key: string | null = null;
  for (const part of raw.split("+").map((p) => p.trim())) {
    const modifier = MODIFIER_NAMES[part.toLowerCase()];
    if (modifier) {
      if (on[modifier]) return null;
      on[modifier] = true;
    } else if (key === null && isValidKey(part)) {
      key = part.length > 1 ? `F${Number(part.slice(1))}` : part.toUpperCase();
    } else {
      return null;
    }
  }
  if (key === null) return null;
  const order = ["Ctrl", "Alt", "Shift", "Win"] as const;
  return [...order.filter((m) => on[m]), key].join("+");
}

interface KeyEventLike {
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
  metaKey: boolean;
  code: string;
}

/** Combinación canónica desde un `KeyboardEvent`; `null` si solo hay modificadores o la tecla no sirve. */
export function formatAccelerator(e: KeyEventLike): string | null {
  const letter = /^Key([A-Z])$/.exec(e.code);
  const digit = /^Digit(\d)$/.exec(e.code);
  const fn = /^F(\d{1,2})$/.exec(e.code);
  let key: string | null = null;
  if (letter) key = letter[1];
  else if (digit) key = digit[1];
  else if (fn && Number(fn[1]) >= 1 && Number(fn[1]) <= 24) key = `F${Number(fn[1])}`;
  if (!key) return null;
  const mods = [e.ctrlKey && "Ctrl", e.altKey && "Alt", e.shiftKey && "Shift", e.metaKey && "Win"].filter(Boolean);
  return [...mods, key].join("+");
}

const isValidEmail = (email: string) => {
  const parts = email.split("@");
  return parts.length === 2 && parts[0] !== "" && parts[1].includes(".") && !/\s/.test(email);
};

const tooLong = (label: string, value: string, max: number): string | null =>
  charLength(value.trim()) > max ? `«${label}» supera ${max} caracteres.` : null;

/** Espejo de `settings::validate` y `shortcuts::validate`; un error por campo (`profile.email`, `shortcuts.capture`…). */
export function validateSettings(s: Settings): FormErrors {
  const errors: FormErrors = {};
  const tester = tooLong("Nombre del tester", s.profile.tester, 100);
  if (tester) errors["profile.tester"] = tester;
  const role = tooLong("Cargo", s.profile.role, 100);
  if (role) errors["profile.role"] = role;
  const email = s.profile.email.trim();
  const emailError = tooLong("Correo", email, 100) ?? (email && !isValidEmail(email) ? "«Correo» no es válido." : null);
  if (emailError) errors["profile.email"] = emailError;

  const width = s.export.imageMaxWidth;
  if (!Number.isInteger(width) || width < IMAGE_WIDTH.min || width > IMAGE_WIDTH.max) {
    errors["export.imageMaxWidth"] = `El ancho máximo de imagen debe estar entre ${IMAGE_WIDTH.min} y ${IMAGE_WIDTH.max} px.`;
  }

  const canonical: Partial<Record<ShortcutAction, string>> = {};
  for (const action of SHORTCUT_ACTIONS) {
    const label = SHORTCUT_LABEL[action];
    const accel = normalizeAccelerator(s.shortcuts[action]);
    const key = `shortcuts.${action}`;
    if (!accel) errors[key] = `«${label}»: combinación no válida.`;
    else if (!accel.split("+").slice(0, -1).some((m) => m === "Ctrl" || m === "Alt" || m === "Win")) errors[key] = `«${label}»: usa al menos Ctrl, Alt o Win.`;
    else if (RESERVED.includes(accel)) errors[key] = `«${label}»: Windows reserva ${accel}.`;
    else canonical[action] = accel;
  }
  SHORTCUT_ACTIONS.forEach((later, j) => {
    const accel = canonical[later];
    const key = `shortcuts.${later}`;
    const earlier = SHORTCUT_ACTIONS.slice(0, j).find((a) => canonical[a] === accel);
    if (accel && earlier && !errors[key]) {
      errors[key] = `«${SHORTCUT_LABEL[earlier]}» y «${SHORTCUT_LABEL[later]}» usan la misma combinación (${accel}).`;
    }
  });
  return errors;
}

/** Mensaje del rechazo del SO que devuelve `save_settings`. */
export const refusalMessage = (accelerator: string, action: ShortcutAction) =>
  `No se pudo registrar ${accelerator} para «${SHORTCUT_LABEL[action]}»: otra aplicación ya la usa. Se conservan los atajos anteriores.`;

/** Toast de arranque cuando un atajo no se pudo registrar. */
export const failureMessage = (f: ShortcutFailure) =>
  `No se pudo registrar ${f.accelerator} («${SHORTCUT_LABEL[f.action]}»): otra aplicación ya la usa. Cámbialo en Ajustes › Atajos.`;
