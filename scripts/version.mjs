// pnpm release <X.Y.Z> [--since <ref>] [--no-tag]: sincroniza la versión en los 4 archivos, agrega
// la sección al CHANGELOG con los commits desde `--since` (por defecto, el último tag), hace el
// commit `chore(release): vX.Y.Z` y crea el tag. Con `--no-tag` no crea el tag (flujo con `main`
// protegido: el tag se pone después del merge del PR). No hace push: imprime los pasos siguientes.
import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import process from "node:process";
import { bump, changelogSection, checkGreater, insertSection, parseReleaseArgs, readVersions } from "./release-lib.mjs";

const root = new URL("..", import.meta.url);
const git = (...args) => execFileSync("git", args, { cwd: root, encoding: "utf8" }).trim();
const fail = (msg) => {
  console.error(msg);
  process.exit(1);
};

let parsed;
try {
  parsed = parseReleaseArgs(process.argv.slice(2));
} catch (e) {
  fail(e.message);
}
const { version, noTag } = parsed;
let { since } = parsed;
if (git("status", "--porcelain") !== "") fail("El árbol de trabajo no está limpio. Haz commit o descarta los cambios antes del release.");
if (git("tag", "--list", `v${version}`) !== "") fail(`El tag v${version} ya existe.`);

const paths = {
  pkg: "package.json",
  cargoToml: "src-tauri/Cargo.toml",
  tauriConf: "src-tauri/tauri.conf.json",
  cargoLock: "src-tauri/Cargo.lock",
};
const current = Object.fromEntries(Object.entries(paths).map(([k, p]) => [k, readFileSync(new URL(p, root), "utf8")]));
const currentVersion = readVersions(current).pkg;
if (currentVersion === null) fail(`${paths.pkg} no tiene versión.`);
const notGreater = checkGreater(version, currentVersion);
if (notGreater) fail(notGreater);

if (since === null) {
  try {
    since = git("describe", "--tags", "--abbrev=0");
  } catch {
    since = null; // Sin tags: se toman todos los commits.
  }
}
const range = since ? [`${since}..HEAD`] : ["HEAD"];
const subjects = git("log", "--format=%s", ...range).split("\n").filter(Boolean);

const next = bump(current, version);
for (const [k, p] of Object.entries(paths)) writeFileSync(new URL(p, root), next[k]);

const date = new Date().toLocaleDateString("sv"); // AAAA-MM-DD en hora local
const changelogPath = new URL("CHANGELOG.md", root);
const changelog = readFileSync(changelogPath, "utf8");
writeFileSync(changelogPath, insertSection(changelog, changelogSection(version, date, subjects)));

git("add", ...Object.values(paths), "CHANGELOG.md");
git("commit", "-m", `chore(release): v${version}`);

if (noTag) {
  console.log(`Commit de release v${version} listo (sin tag ni push).`);
  console.log("Pasos siguientes:");
  console.log(`  git push -u origin HEAD           # rama release/v${version}`);
  console.log("  Abre el PR a main y fusiónalo con merge commit.");
  console.log("  Después, en main:");
  console.log(`  git checkout main && git pull && pnpm check-version v${version}`);
  console.log(`  git tag v${version} && git push origin v${version}`);
} else {
  git("tag", `v${version}`);
  console.log(`Release v${version} listo (commit y tag locales, sin push).`);
  console.log("Para publicarlo:");
  console.log("  git push");
  console.log(`  git push origin v${version}`);
}
