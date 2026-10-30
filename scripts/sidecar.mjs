import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

export function hostTriple(output) {
  const triple = /^host:\s*(\S+)\s*$/m.exec(output)?.[1];
  if (!triple) throw new Error("rustc -vV no devolvió host");
  return triple;
}
export function binaryName(triple, suffixed = true) {
  return `qapture-mcp${suffixed ? `-${triple}` : ""}${triple.includes("windows") ? ".exe" : ""}`;
}
export function buildSidecar() {
  const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
  const crate = join(root, "src-tauri");
  const target = process.env.TAURI_ENV_TARGET_TRIPLE;
  const triple = target || hostTriple(execFileSync("rustc", ["-vV"], { encoding: "utf8" }));
  execFileSync("cargo", ["build", "--release", "--bin", "qapture-mcp", ...(target ? ["--target", target] : [])], { cwd: crate, stdio: "inherit" });
  const output = join(crate, "binaries");
  mkdirSync(output, { recursive: true });
  const targetDir = process.env.CARGO_TARGET_DIR ? resolve(crate, process.env.CARGO_TARGET_DIR) : join(crate, "target");
  copyFileSync(join(targetDir, ...(target ? [target] : []), "release", binaryName(triple, false)), join(output, binaryName(triple)));
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) buildSidecar();
