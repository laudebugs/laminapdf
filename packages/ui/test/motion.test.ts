// suppressMotion: the root's data-motion-suppressed, from a switch of look to two frames later.

import { describe, expect, it } from "vitest";
import { suppressMotion } from "../src/motion.ts";

/** The element suppressMotion takes, named without the DOM types this Node test can't use. */
type Root = NonNullable<Parameters<typeof suppressMotion>[0]>;

/** A root element whose window runs animation frames when `frame()` is called. */
function root() {
  const attrs = new Set<string>();
  let queue: (() => void)[] = [];
  const view = {
    requestAnimationFrame: (f: () => void) => {
      queue.push(f);
      return queue.length;
    },
  };
  const element = {
    ownerDocument: { defaultView: view },
    setAttribute: (name: string) => attrs.add(name),
    removeAttribute: (name: string) => attrs.delete(name),
  } as unknown as Root;
  const frame = () => {
    const due = queue;
    queue = [];
    for (const f of due) f();
  };
  return { element, frame, suppressed: () => attrs.has("data-motion-suppressed") };
}

describe("suppressMotion", () => {
  it("marks the root at once and clears it after two frames", () => {
    const r = root();
    suppressMotion(r.element);
    expect(r.suppressed()).toBe(true);
    r.frame();
    expect(r.suppressed()).toBe(true);
    r.frame();
    expect(r.suppressed()).toBe(false);
  });

  it("keeps the mark for two frames after the latest call", () => {
    const r = root();
    suppressMotion(r.element);
    r.frame();
    suppressMotion(r.element);
    r.frame();
    expect(r.suppressed()).toBe(true);
    r.frame();
    expect(r.suppressed()).toBe(false);
  });

  it("does nothing without a document or a window", () => {
    expect("document" in globalThis).toBe(false);
    expect(() => suppressMotion()).not.toThrow();
    const attrs: string[] = [];
    const element = {
      ownerDocument: { defaultView: null },
      setAttribute: (name: string) => attrs.push(name),
    } as unknown as Root;
    suppressMotion(element);
    expect(attrs).toEqual([]);
  });
});
