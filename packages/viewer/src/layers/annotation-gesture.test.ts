import { describe, expect, it } from "vitest";
import type { PreviewShape } from "./annotation-geometry.js";
import { AnnotationGesture, type GestureHost } from "./annotation-gesture.js";

/** A host that records what the gesture asks, in order, and lets each promise settle on cue. */
function fakeHost(options: { readonly failEdit?: boolean } = {}) {
  const log: string[] = [];
  const pending: (() => void)[] = [];
  const later = (what: string) =>
    new Promise<void>((resolve) => {
      pending.push(() => {
        log.push(`${what} drawn`);
        resolve();
      });
    });
  const host: GestureHost = {
    hideAnnotations: (page, places) => {
      log.push(`hide ${page} [${places.join(",")}]`);
      return later("hidden");
    },
    whenDrawn: (page) => {
      log.push(`wait ${page}`);
      return later("edit");
    },
    applyEdit: async (ops) => {
      log.push(`apply ${ops.length}`);
      if (options.failEdit) throw new Error("refused");
      return { pages: [], history: { steps: [], position: 0, saved: 0 } } as never;
    },
    preview: (page, shapes) =>
      log.push(shapes === undefined ? `preview ${page} gone` : `preview ${page} ${shapes.length}`),
  };
  /** Settles the oldest pending promise and lets its continuations run. */
  const settle = async () => {
    pending.shift()?.();
    await new Promise((resolve) => setTimeout(resolve, 0));
  };
  return { host, log, settle };
}

const SHAPE: PreviewShape = { d: "M0 0 L1 1", stroke: "red" };

describe("AnnotationGesture", () => {
  it("shows its preview only once the page is drawn without what it moves", async () => {
    const { host, log, settle } = fakeHost();
    const gesture = new AnnotationGesture(host, 2, [4]);
    gesture.update([SHAPE]);
    expect([gesture.state, log]).toEqual(["starting", ["hide 2 [4]"]]);
    await settle();
    expect([gesture.state, log.slice(1)]).toEqual(["active", ["hidden drawn", "preview 2 1"]]);
  });

  it("removes the preview on commit only after the new page is drawn", async () => {
    const { host, log, settle } = fakeHost();
    const gesture = new AnnotationGesture(host, 0, [1]);
    await settle();
    gesture.update([SHAPE, SHAPE]);
    const committed = gesture.commit([{ op: "updateAnnot", args: { id: "a1.0" } } as never]);
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(log.at(-1)).toBe("wait 0");
    expect(log).not.toContain("preview 0 gone");
    await settle();
    await committed;
    expect(log.slice(-2)).toEqual(["edit drawn", "preview 0 gone"]);
    expect(gesture.state).toBe("committed");
  });

  it("shows the annotations again and removes the preview on cancel", async () => {
    const { host, log, settle } = fakeHost();
    const gesture = new AnnotationGesture(host, 0, [1]);
    await settle();
    gesture.update([SHAPE]);
    const cancelled = gesture.cancel();
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(log.at(-1)).toBe("hide 0 []");
    await settle();
    await cancelled;
    expect(log.slice(-2)).toEqual(["hidden drawn", "preview 0 gone"]);
    expect(gesture.state).toBe("cancelled");
  });

  it("cancelled before the page is drawn without its annotations, never shows a preview", async () => {
    const { host, log, settle } = fakeHost();
    const gesture = new AnnotationGesture(host, 0, [1]);
    gesture.update([SHAPE]);
    const cancelled = gesture.cancel();
    await settle();
    await settle();
    await cancelled;
    expect(log.filter((line) => line.startsWith("preview 0 1"))).toEqual([]);
    expect(log.at(-1)).toBe("preview 0 gone");
  });

  it("a failed edit shows the annotations again, removes the preview and rejects", async () => {
    const { host, log, settle } = fakeHost({ failEdit: true });
    const gesture = new AnnotationGesture(host, 0, [1]);
    await settle();
    const failure = gesture.commit([]).then(
      () => undefined,
      (e: unknown) => e,
    );
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(log.at(-1)).toBe("hide 0 []");
    await settle();
    expect(await failure).toEqual(new Error("refused"));
    expect(log.at(-1)).toBe("preview 0 gone");
  });

  it("a new mark hides nothing and draws its preview at once", async () => {
    const { host, log } = fakeHost();
    const gesture = new AnnotationGesture(host, 3, []);
    await gesture.ready();
    gesture.update([SHAPE]);
    expect(log).toEqual(["preview 3 0", "preview 3 1"]);
  });
});
