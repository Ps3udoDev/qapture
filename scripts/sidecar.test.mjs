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
