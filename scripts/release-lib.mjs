// Lógica pura del release: funciones sobre texto, sin leer ni escribir archivos.
// Las usan version.mjs (pnpm release), check-version.mjs y release-notes.mjs.

const SEMVER = /^\d+\.\d+\.\d+$/;
const FILE_NAMES = {
  pkg: "package.json",
  cargoToml: "src-tauri/Cargo.toml",
  tauriConf: "src-tauri/tauri.conf.json",
  cargoLock: "src-tauri/Cargo.lock",
};

// Primera `version = "…"` de la sección [package] de Cargo.toml.
const CARGO_TOML_VERSION = /(\[package\][^[]*?\nversion\s*=\s*")([^"]*)(")/;
// Primer `"version": "…"` de un JSON (package.json y tauri.conf.json lo tienen al nivel raíz).
const JSON_VERSION = /("version"\s*:\s*")([^"]*)(")/;
// La entrada `qapture` de Cargo.lock: el `version` va justo después de `name`.
const CARGO_LOCK_VERSION = /(\[\[package\]\]\r?\nname = "qapture"\r?\nversion = ")([^"]*)(")/;

const PATTERNS = {
  pkg: JSON_VERSION,
  cargoToml: CARGO_TOML_VERSION,
  tauriConf: JSON_VERSION,
  cargoLock: CARGO_LOCK_VERSION,
};

export function readVersions(files) {
  const out = {};
  for (const key of Object.keys(FILE_NAMES)) {
    out[key] = PATTERNS[key].exec(files[key])?.[2] ?? null;
  }
  return out;
}

export function checkVersions(versions, tag) {
  const errors = [];
  const reference = versions.pkg;
  if (reference === null || !SEMVER.test(reference)) {
    errors.push(`La versión debe ser X.Y.Z (el MSI no admite prereleases): ${reference}.`);
    return errors;
  }
  for (const key of ["cargoToml", "tauriConf", "cargoLock"]) {
    if (versions[key] !== reference) {
      errors.push(`${FILE_NAMES[key]} tiene ${versions[key]} y ${FILE_NAMES.pkg} ${reference}.`);
    }
  }
  if (tag !== undefined && tag !== `v${reference}`) {
    errors.push(`El tag ${tag} no coincide con la versión ${reference}.`);
  }
  return errors;
}

export function bump(files, version) {
  if (!SEMVER.test(version)) {
    throw new Error(`La versión debe ser X.Y.Z (el MSI no admite prereleases): ${version}.`);
  }
  const out = {};
  for (const key of Object.keys(FILE_NAMES)) {
    if (!PATTERNS[key].test(files[key])) {
      throw new Error(`No se encontró la versión en ${FILE_NAMES[key]}.`);
    }
    out[key] = files[key].replace(PATTERNS[key], (_, a, _old, c) => `${a}${version}${c}`);
  }
  return out;
}

function conventional(subject) {
  const m = /^(feat|fix)(?:\([^)]*\))?!?:\s*(.+)$/.exec(subject.trim());
  return m ? { type: m[1], text: m[2] } : null;
}

export function changelogSection(version, date, subjects) {
  const parsed = subjects.map(conventional).filter(Boolean);
  const heading = date ? `## ${version} — ${date}` : `## ${version}`;
  const groups = [
    ["Nuevo", "feat"],
    ["Correcciones", "fix"],
  ];
  const blocks = groups
    .map(([title, type]) => {
      const items = parsed.filter((p) => p.type === type).map((p) => `- ${p.text}`);
      return items.length ? `### ${title}\n${items.join("\n")}\n` : null;
    })
    .filter(Boolean);
  const body = blocks.length ? blocks.join("\n") : "- Sin cambios destacados.\n";
  return `${heading}\n\n${body}`;
}

// Pone la sección nueva justo debajo del título `# Cambios`, antes de las versiones anteriores.
export function insertSection(changelog, section) {
  const m = /^# [^\n]*\n\n?/.exec(changelog);
  if (!m) return `# Cambios\n\n${section}\n${changelog}`;
  const head = m[0].endsWith("\n\n") ? m[0] : `${m[0]}\n`;
  return `${head}${section}\n${changelog.slice(m[0].length)}`;
}

export function notesFor(changelog, version) {
  const lines = changelog.split("\n");
  const start = lines.findIndex((l) => l === `## ${version}` || l.startsWith(`## ${version} `));
  if (start === -1) return "";
  let end = lines.findIndex((l, i) => i > start && l.startsWith("## "));
  if (end === -1) end = lines.length;
  const body = lines.slice(start + 1, end).join("\n").trim();
  return body ? `${body}\n` : "";
}
