// What the header's and the select's state tests share: the commands that press the real mouse and
// set the motion preference (vitest.browser.config.ts) and a reading of states.css's layer. The
// page loads the site kit's base.css as each site's layout does.
import "../../src/base.css";
import "../../src/states.css";
import { commands } from "vitest/browser";

declare module "vitest/browser" {
  interface BrowserCommands {
    /** Holds the mouse button down on the element `selector` names. */
    press: (selector: string) => Promise<void>;
    /** Lets the mouse button go, away from everything. */
    release: () => Promise<void>;
    /** Makes the page prefer reduced motion or not; null goes back to the browser's own setting. */
    reducedMotion: (value: "reduce" | "no-preference" | null) => Promise<void>;
  }
}

/** `value` as the browser computes a colour (`rgb(…)` or `rgba(…)`). */
export function resolvedColor(value: string): string {
  const probe = document.createElement("span");
  probe.style.color = value;
  document.body.append(probe);
  const resolved = getComputedStyle(probe).color;
  probe.remove();
  return resolved;
}

/** The state layer states.css draws over `element`'s fill: hover, pressed or none. */
export function layer(element: Element): string {
  return getComputedStyle(element).getPropertyValue("--site-state").trim();
}

/**
 * The layer while the mouse holds `selector`'s first element down, after waiting for it to settle
 * (a hover fade on the way in may still be under way), then the mouse is let go.
 */
export async function pressedLayer(selector: string): Promise<string> {
  const element = document.querySelector(selector);
  if (element === null) throw new Error(`no ${selector}`);
  await commands.press(selector);
  const pressed = resolvedColor("var(--paper-pressed)");
  for (let i = 0; i < 20 && layer(element) !== pressed; i += 1) await new Promise((r) => setTimeout(r, 25));
  const seen = layer(element);
  await commands.release();
  return seen;
}
