// pnpm check-version [vX.Y.Z]: sale con código 1 si las 4 fuentes de versión no coinciden
// (o si el tag dado no corresponde). Lo usan CI y release.yml antes de compilar.
import { readFileSync } from "node:fs";
import process from "node:process";
import { checkVersions, readVersions } from "./release-lib.mjs";

const root = new URL("..", import.meta.url);
const read = (path) => readFileSync(new URL(path, root), "utf8");

const versions = readVersions({
  pkg: read("package.json"),
  cargoToml: read("src-tauri/Cargo.toml"),
  tauriConf: read("src-tauri/tauri.conf.json"),
  cargoLock: read("src-tauri/Cargo.lock"),
});
const errors = checkVersions(versions, process.argv[2]);

if (errors.length > 0) {
  for (const e of errors) console.error(e);
  process.exit(1);
}
console.log(`Versión ${versions.pkg} coherente en los 4 archivos.`);
