import { describe, expect, it } from "vitest";
import { filesFrom, hasFiles, readBytes, splitAccepted } from "./files";

const file = (type: string, bytes = [1, 2]) => new File([new Uint8Array(bytes)], "x", { type });

describe("archivos entrantes", () => {
  it("separa imágenes aceptadas de otros archivos", () => {
    const { images, rejected } = splitAccepted([file("image/png"), file("application/pdf"), file("image/webp")]);
    expect([images.length, rejected]).toEqual([2, 1]);
  });

  it("lee archivos desde files o desde items y detecta arrastres de archivos", () => {
    const png = file("image/png");
    expect(filesFrom({ files: [png], items: [], types: ["Files"] } as unknown as DataTransfer)).toEqual([png]);
    const fromItems = { files: [], items: [{ kind: "file", getAsFile: () => png }, { kind: "string", getAsFile: () => null }], types: ["Files"] };
    expect(filesFrom(fromItems as unknown as DataTransfer)).toEqual([png]);
    expect(filesFrom(null)).toEqual([]);
    expect(hasFiles({ types: ["text/plain"] } as unknown as DataTransfer)).toBe(false);
  });

  it("lee los bytes", async () => {
    expect([...(await readBytes(file("image/png", [7, 8, 9])))]).toEqual([7, 8, 9]);
  });
});
