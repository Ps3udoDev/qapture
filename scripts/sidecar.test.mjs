import test from "node:test";
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
