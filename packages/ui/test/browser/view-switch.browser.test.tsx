// The Reader toggle's view transition (motion plan §4.3): switchView swaps the canvas's view inside a
// same-document view transition of duration-view, the new view rising a little as it fades in; under
// reduced motion it only fades; where view transitions are missing the switch is instant.

import { useState } from "react";
import { afterEach, describe, expect, it, onTestFinished } from "vitest";
import { commands } from "vitest/browser";
import { switchView } from "../../src/index.ts";
import { cleanup, renderIn } from "./render.tsx";

/** What the test presses: the switch between the two views. */
interface Control {
  toggle: () => void;
}

/** The canvas showing pages or the reading column; `control.toggle` switches between them. */
function Canvas({ control }: { control: Control }) {
  const [column, setColumn] = useState(false);
  control.toggle = () => switchView(() => setColumn((shown) => !shown));
  return (
    <main className="paper-reader-canvas" style={{ position: "relative", blockSize: "200px" }}>
      {column ? <div className="paper-reader-mode">Column</div> : <p>Pages</p>}
    </main>
  );
}

/** The canvas, rendered, and its switch. */
function canvas(): { host: HTMLElement; toggle: () => void } {
  const control: Control = { toggle: () => {} };
  const host = renderIn(<Canvas control={control} />);
  return { host, toggle: () => control.toggle() };
}

/** The running animation of the new view's snapshot. */
const newView = () =>
  document
    .getAnimations()
    .find((a) => (a.effect as KeyframeEffect | null)?.pseudoElement === "::view-transition-new(paper-view)");

/**
 * How far the new view rises from, in px: the second length of its first keyframe's translate
 * ("0px 8px", or "0px calc(8px)" in Firefox); none when it has one.
 */
function rise(animation: Animation): number {
  const [first] = (animation.effect as KeyframeEffect).getKeyframes();
  const lengths = [...String(first?.translate ?? "").matchAll(/(-?[\d.]+)px/g)].map((m) => Number(m[1]));
  return lengths[1] ?? 0;
}

const switching = () => document.documentElement.hasAttribute("data-view-switch");

// A transition still running would carry into the next test.
afterEach(async () => {
  await expect.poll(switching).toBe(false);
  cleanup();
});

describe("switchView", () => {
  it("fades the canvas to the other view over duration-view, the new view rising as it comes", async () => {
    await commands.reducedMotion("no-preference");
    onTestFinished(() => commands.reducedMotion(null));
    const { host, toggle } = canvas();
    toggle();
    await expect.poll(newView).toBeDefined();
    const animation = newView() as Animation;
    expect(animation.effect?.getTiming().duration).toBe(250);
    expect(rise(animation)).toBe(8);
    expect(host.textContent).toBe("Column");
    await animation.finished;
    await expect.poll(switching).toBe(false);
    // And back.
    toggle();
    await expect.poll(newView).toBeDefined();
    await expect.poll(() => host.textContent).toBe("Pages");
  });

  it("only cross-fades under reduced motion", async () => {
    await commands.reducedMotion("reduce");
    onTestFinished(() => commands.reducedMotion(null));
    const { toggle } = canvas();
    toggle();
    await expect.poll(newView).toBeDefined();
    const animation = newView() as Animation;
    expect(animation.effect?.getTiming().duration).toBe(250);
    expect(rise(animation)).toBe(0);
  });

  it("only cross-fades with Settings › Reduce motion on", async () => {
    document.documentElement.setAttribute("data-motion", "reduce");
    onTestFinished(() => document.documentElement.removeAttribute("data-motion"));
    const { toggle } = canvas();
    toggle();
    await expect.poll(newView).toBeDefined();
    expect(rise(newView() as Animation)).toBe(0);
  });

  it("switches at once where view transitions are missing", async () => {
    Object.defineProperty(document, "startViewTransition", { value: undefined, configurable: true });
    onTestFinished(() => {
      delete (document as { startViewTransition?: unknown }).startViewTransition;
    });
    const { host, toggle } = canvas();
    toggle();
    expect(switching()).toBe(false);
    await expect.poll(() => host.textContent).toBe("Column");
    expect(newView()).toBeUndefined();
  });
});
