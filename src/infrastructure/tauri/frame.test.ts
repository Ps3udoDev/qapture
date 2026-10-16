import { describe, expect, it } from "vitest";
import { encodeFrame } from "./frame";

describe("encodeFrame", () => {
  it("antepone el largo del JSON en u32 little-endian", () => {
    const frame = encodeFrame({ description: "Recibo ✅" }, new Uint8Array([9, 8]));
    const len = new DataView(frame.buffer).getUint32(0, true);
    const json = new TextDecoder().decode(frame.slice(4, 4 + len));
    expect(JSON.parse(json)).toEqual({ description: "Recibo ✅" });
    expect([...frame.slice(4 + len)]).toEqual([9, 8]);
  });
});
