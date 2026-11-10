import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { FirstDraw } from "./first-draw.js";

/** Whether `promise` has settled after the queued microtasks run. */
async function settled(promise: Promise<unknown>): Promise<boolean> {
  let done = false;
  promise.then(
    () => (done = true),
    () => (done = true),
  );
  await vi.advanceTimersByTimeAsync(0);
  return done;
}

describe("FirstDraw", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it("opens on its own document's first render", async () => {
    const gate = new FirstDraw(2, 2000);
    const waiting = gate.wait();
    expect(await settled(waiting)).toBe(false);
    gate.drawn(2);
    expect(await settled(waiting)).toBe(true);
  });

  it("stays closed for a render of an earlier document", async () => {
    const gate = new FirstDraw(2, 2000);
    gate.drawn(1);
    expect(await settled(gate.wait())).toBe(false);
  });

  it("opens once after the wait for a caller that never renders, then lets every call through", async () => {
    const gate = new FirstDraw(2, 2000);
    await vi.advanceTimersByTimeAsync(2000);
    expect(await settled(gate.wait())).toBe(true);
  });

  it("rejects a call aborted while it waited", async () => {
    const gate = new FirstDraw(2, 2000);
    const abort = new AbortController();
    const waiting = gate.wait(abort.signal);
    abort.abort();
    gate.drawn(2);
    await expect(waiting).rejects.toThrow();
  });
});
