import test from "node:test";
import { readFileSync } from "node:fs";
import { basename } from "node:path";
import assert from "node:assert/strict";
import { binaryName, hostTriple } from "./sidecar.mjs";

test("parses the host triple from rustc -vV", () => {
  assert.equal(hostTriple("rustc 1.90.0\nhost: x86_64-unknown-linux-gnu\nrelease: 1.90.0\n"), "x86_64-unknown-linux-gnu");
  assert.throws(() => hostTriple("no host"));
});
test("names the windows binary with .exe", () => {
  assert.equal(binaryName("x86_64-pc-windows-msvc"), "qapture-mcp-x86_64-pc-windows-msvc.exe");
  assert.equal(binaryName("x86_64-pc-windows-msvc", false), "qapture-mcp.exe");
  assert.equal(binaryName("x86_64-unknown-linux-gnu"), "qapture-mcp-x86_64-unknown-linux-gnu");
});

// Tauri automatically includes the crate's src/bin targets. Registering the
// same bridge as externalBin produces duplicate WiX components (ICE30).
test("the crate bridge is not registered again as an external binary", () => {
  const config = JSON.parse(readFileSync(new URL("../src-tauri/tauri.conf.json", import.meta.url), "utf8"));
  assert.ok(!(config.bundle.externalBin ?? []).some((path) => basename(path) === "qapture-mcp"));
  assert.match(readFileSync(new URL("../src-tauri/Cargo.toml", import.meta.url), "utf8"), /name = "qapture-mcp"/);
});

// A bridge kept open by an agent session locks qapture-mcp.exe; the installer
// must close it before replacing files (F9a, checklist row 6).
test("the NSIS installer closes the bridge before installing and uninstalling", () => {
  const config = JSON.parse(readFileSync(new URL("../src-tauri/tauri.conf.json", import.meta.url), "utf8"));
  const hooks = config.bundle.windows.nsis.installerHooks;
  assert.equal(hooks, "./windows/hooks.nsh");
  const nsh = readFileSync(new URL(`../src-tauri/${hooks.slice(2)}`, import.meta.url), "utf8");
  for (const hook of ["NSIS_HOOK_PREINSTALL", "NSIS_HOOK_PREUNINSTALL"]) {
    const body = nsh.split(`!macro ${hook}`)[1]?.split("!macroend")[0] ?? "";
    assert.match(body, /RestartManager_ShutdownFile "\$INSTDIR\\qapture-mcp\.exe"/, hook);
  }
});
