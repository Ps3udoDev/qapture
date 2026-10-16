/** `[u32 LE largo][JSON utf-8][bytes]`: el formato que espera `commands/frame.rs`. */
export function encodeFrame(meta: unknown, payload: Uint8Array): Uint8Array {
  const json = new TextEncoder().encode(JSON.stringify(meta));
  const out = new Uint8Array(4 + json.length + payload.length);
  new DataView(out.buffer).setUint32(0, json.length, true);
  out.set(json, 4);
  out.set(payload, 4 + json.length);
  return out;
}
