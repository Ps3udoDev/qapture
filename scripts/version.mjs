// pnpm release <X.Y.Z> [--since <ref>]: sincroniza la versión en los 4 archivos, agrega la sección
// al CHANGELOG con los commits desde `--since` (por defecto, el último tag), hace el commit
// `chore(release): vX.Y.Z` y crea el tag. No hace push: imprime los comandos para hacerlo.
import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import process from "node:process";
import { bump, changelogSection, insertSection } from "./release-lib.mjs";

const root = new URL("..", import.meta.url);
const git = (...args) => execFileSync("git", args, { cwd: root, encoding: "utf8" }).trim();
const fail = (msg) => {
  console.error(msg);
  process.exit(1);
};

const args = process.argv.slice(2);
const sinceAt = args.indexOf("--since");
let since = null;
if (sinceAt !== -1) {
  since = args[sinceAt + 1];
  if (!since) fail("--since necesita una referencia (tag o commit).");
  args.splice(sinceAt, 2);
}
const version = args[0];
if (!version || args.length !== 1) fail("Uso: pnpm release <X.Y.Z> [--since <ref>]");
if (!/^\d+\.\d+\.\d+$/.test(version)) {
  fail(`La versión debe ser X.Y.Z (el MSI no admite prereleases): ${version}.`);
}
if (git("status", "--porcelain") !== "") fail("El árbol de trabajo no está limpio. Haz commit o descarta los cambios antes del release.");
if (git("tag", "--list", `v${version}`) !== "") fail(`El tag v${version} ya existe.`);

if (since === null) {
  try {
    since = git("describe", "--tags", "--abbrev=0");
  } catch {
    since = null; // Sin tags: se toman todos los commits.
  }
}
const range = since ? [`${since}..HEAD`] : ["HEAD"];
const subjects = git("log", "--format=%s", ...range).split("\n").filter(Boolean);

const paths = {
  pkg: "package.json",
  cargoToml: "src-tauri/Cargo.toml",
  tauriConf: "src-tauri/tauri.conf.json",
  cargoLock: "src-tauri/Cargo.lock",
};
const current = Object.fromEntries(Object.entries(paths).map(([k, p]) => [k, readFileSync(new URL(p, root), "utf8")]));
const next = bump(current, version);
for (const [k, p] of Object.entries(paths)) writeFileSync(new URL(p, root), next[k]);

const date = new Date().toLocaleDateString("sv"); // AAAA-MM-DD en hora local
const changelogPath = new URL("CHANGELOG.md", root);
const changelog = readFileSync(changelogPath, "utf8");
writeFileSync(changelogPath, insertSection(changelog, changelogSection(version, date, subjects)));

git("add", ...Object.values(paths), "CHANGELOG.md");
git("commit", "-m", `chore(release): v${version}`);
git("tag", `v${version}`);

console.log(`Release v${version} listo (commit y tag locales, sin push).`);
console.log("Para publicarlo:");
console.log("  git push");
console.log(`  git push origin v${version}`);
