import { readFileSync } from "node:fs";
import { applyAppearance, parseAppearance } from "@laminapdf/ui";
import { describe, expect, it } from "vitest";

// public/appearance.js sets the look before the first paint; the app sets it again once it runs.
// For every stored value, both must set the same attributes, or the reader flashes on load.
const SCRIPT = readFileSync(new URL("../public/appearance.js", import.meta.url), "utf8");
const KEY = "lamina.appearance";

type Root = { attrs: Map<string, string>; setAttribute(n: string, v: string): void; removeAttribute(n: string): void };

function root(): Root {
  const attrs = new Map<string, string>();
  return {
    attrs,
    setAttribute: (name, value) => attrs.set(name, value),
    removeAttribute: (name) => attrs.delete(name),
  };
}

/** The attributes the script sets with `stored` in localStorage (`throws`: storage blocked). */
function prePaint(stored: string | null | "throws"): Record<string, string> {
  const element = root();
  const storage = {
    getItem: (key: string) => {
      if (stored === "throws") throw new Error("blocked");
      return key === KEY ? stored : null;
    },
  };
  new Function("document", "localStorage", SCRIPT)({ documentElement: element }, storage);
  return Object.fromEntries(element.attrs);
}

function app(stored: string | null): Record<string, string> {
  const element = root();
  applyAppearance(element as unknown as HTMLElement, parseAppearance(stored));
  return Object.fromEntries(element.attrs);
}

describe("the pre-paint script", () => {
  it("sets what the app sets, for every stored value", () => {
    const values = [
      null,
      "{",
      "null",
      '{"theme":"dark","accent":"blue"}',
      '{"theme":"high-contrast","accent":"plum","motion":"reduce"}',
      '{"theme":"light","accent":"green","motion":"system"}',
      '{"theme":"system","accent":"teal"}',
      '{"theme":"sepia","accent":"red","motion":"fast"}',
    ];
    for (const stored of values) expect(prePaint(stored), String(stored)).toEqual(app(stored));
  });

  it("sets the default look when storage is blocked", () => {
    expect(prePaint("throws")).toEqual(app(null));
  });

  it("is loaded by index.html before the app's module", () => {
    const html = readFileSync(new URL("../index.html", import.meta.url), "utf8");
    const script = html.indexOf('<script src="/appearance.js"></script>');
    expect(script).toBeGreaterThan(0);
    expect(script).toBeLessThan(html.indexOf('<script type="module"'));
  });
});
