import { describe, expect, it } from "vitest";
import { flatten, measure, themeContext } from "../../scripts/model.ts";
import { contrastRatio, type Rgba } from "../../src/contrast.ts";
import doc from "../../tokens.json" with { type: "json" };
import "../../src/generated/tokens.css";

// The high-contrast theme maps tokens to CSS system colours, which only the browser resolves.
// Outside forced colours each engine uses its own defaults; every declared pair must pass with
// them, as it must with the OS palette under forced colours (U6f screenshots that mode).

const tokens = flatten(doc);
const ctx = themeContext(tokens, "high-contrast", "teal");

/** What `var(--paper-<name>)` resolves to here, as opaque RGBA. */
function resolved(host: HTMLElement, name: string): Rgba {
  const probe = document.createElement("span");
  probe.style.color = `var(--paper-${name})`;
  host.append(probe);
  const m = /^rgba?\((\d+), (\d+), (\d+)(?:, ([\d.]+))?\)$/.exec(getComputedStyle(probe).color);
  probe.remove();
  if (m === null) throw new Error(`--paper-${name} didn't resolve to an rgb() colour`);
  return { r: Number(m[1]), g: Number(m[2]), b: Number(m[3]), a: m[4] === undefined ? 1 : Number(m[4]) };
}

const short = (ref: string) => ref.replace(/^\{|\}$/g, "");

describe("the high-contrast theme in this browser", () => {
  const host = document.createElement("div");
  host.dataset.theme = "high-contrast";
  document.body.append(host);

  const pairs = measure(tokens, ctx, "teal");

  it("declares its pairs with system colours", () => {
    expect(pairs.length).toBeGreaterThan(10);
    expect(pairs.every((p) => p.ratio === undefined)).toBe(true);
  });

  it("passes every pair with the engine's system colours", () => {
    const failing: string[] = [];
    for (const p of pairs) {
      if (p.waived !== undefined) continue;
      const ratio = contrastRatio(resolved(host, p.fg), resolved(host, short(p.on)));
      if (ratio < p.min) failing.push(`${p.fg} on ${short(p.on)} is ${ratio.toFixed(2)}, needs ${p.kind} ≥ ${p.min}`);
    }
    expect(failing).toEqual([]);
  });

  it("uses system colours, not the placeholders in tokens.json", () => {
    expect(getComputedStyle(host).getPropertyValue("--paper-text").trim()).toBe("CanvasText");
  });
});
