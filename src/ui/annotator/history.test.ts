import { describe, expect, it } from "vitest";
import { canRedo, canUndo, commit, createHistory, redo, undo } from "./history";

describe("historial", () => {
  it("deshace y rehace; un cambio nuevo descarta el futuro", () => {
    let h = createHistory(0);
    expect(canUndo(h)).toBe(false);
    h = commit(commit(h, 1), 2);
    h = undo(h);
    expect([h.present, canRedo(h)]).toEqual([1, true]);
    h = redo(h);
    expect(h.present).toBe(2);
    h = commit(undo(h), 9);
    expect([h.present, canRedo(h)]).toEqual([9, false]);
    expect(undo(undo(undo(h))).present).toBe(0);
  });

  it("ignora cambios idénticos y respeta el límite", () => {
    const h = createHistory("a");
    expect(commit(h, "a")).toBe(h);
    let long = createHistory(0);
    for (let i = 1; i <= 5; i++) long = commit(long, i, 3);
    expect(long.past).toEqual([2, 3, 4]);
  });
});
