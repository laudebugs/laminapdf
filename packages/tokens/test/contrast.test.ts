import { describe, expect, it } from "vitest";
import { contexts, fixedContext, flatten, type Measured, measure, type Token } from "../scripts/model.ts";
import { composite, contrastRatio, hex } from "../src/contrast.ts";
import { loadTokens, tokensJson } from "./helpers.ts";

const ratio = (a: string, b: string) => contrastRatio(hex(a), hex(b));

/** Every declared pair in every context, plus the fixed palettes. */
function everyPair(tokens: Map<string, Token>): Measured[] {
  const pairs = contexts(tokens).flatMap(({ ctx, accent }) => measure(tokens, ctx, accent));
  return [...pairs, ...measure(tokens, fixedContext(tokens), "teal")];
}

/** The pairs below their minimum that nothing waives, one line each. */
function failing(pairs: Measured[]): string[] {
  return pairs
    .filter((p) => p.ratio !== undefined && p.ratio < p.min && p.waived === undefined)
    .map((p) => `${p.context}: ${p.fg} on ${p.on} is ${p.ratio?.toFixed(2)}, needs ${p.kind} ≥ ${p.min}`);
}

describe("contrast maths (WCAG 2.x)", () => {
  it("measures the known ratios", () => {
    expect(ratio("#000000", "#FFFFFF")).toBeCloseTo(21, 5);
    expect(ratio("#FFFFFF", "#000000")).toBeCloseTo(21, 5);
    expect(ratio("#767676", "#FFFFFF")).toBeCloseTo(4.54, 2);
    expect(ratio("#777777", "#777777")).toBe(1);
  });

  it("reproduces the design language's numbers", () => {
    expect(ratio("#1F1D1A", "#F7F4EE")).toBeCloseTo(15.32, 2); // text on ground
    expect(ratio("#1F5C63", "#FFFFFF")).toBeCloseTo(7.58, 2); // accent on white
    expect(ratio("#6B645A", "#ECE7DE")).toBeCloseTo(4.74, 2); // muted on gutter
    expect(ratio("#E6E1D8", "#22211F")).toBeCloseTo(12.35, 2); // Dark page ink
  });

  it("composites a translucent colour as a screen shows it", () => {
    const hover = composite({ r: 31, g: 29, b: 26, a: 0.06 }, hex("#F7F4EE"));
    expect(hover).toEqual(hex("#EAE7E1")); // §2.1: hover "= #EAE7E1 on ground"
    expect(composite({ r: 9, g: 9, b: 9, a: 1 }, hex("#FFFFFF"))).toEqual(hex("#090909"));
    expect(composite({ r: 9, g: 9, b: 9, a: 0 }, hex("#FFFFFF"))).toEqual(hex("#FFFFFF"));
  });
});

describe("declared pairs", () => {
  const tokens = loadTokens();
  const pairs = everyPair(tokens);

  it("reads every theme with every accent, and every page appearance", () => {
    const labels = new Set(pairs.map((p) => p.context));
    for (const accent of ["teal", "blue", "plum", "green"]) {
      for (const where of ["light", "dark", "page original", "page sepia", "page dark"]) {
        expect(labels).toContain(`${where} · ${accent}`);
      }
    }
    expect(labels).toContain("high-contrast");
    expect(labels).toContain("page high-contrast");
  });

  it("passes WCAG 2.2 AA: text ≥ 4.5, boundaries and focus ≥ 3", () => {
    expect(failing(pairs)).toEqual([]);
  });

  it("asks for at least the WCAG minimum of each kind", () => {
    for (const p of pairs)
      expect(p.min, `${p.context}: ${p.fg} on ${p.on}`).toBeGreaterThanOrEqual(p.kind === "text" ? 4.5 : 3);
  });

  it("gives a reason for every waived pair", () => {
    for (const p of pairs.filter((x) => x.waived !== undefined)) expect(p.waived).not.toBe("");
  });

  it("fails a text token changed to an unreadable value", () => {
    const doc = tokensJson() as { theme: { light: { muted: { $value: unknown } } } };
    doc.theme.light.muted.$value = "{primitive.color.warm.607}"; // control-border's #8A8174
    const broken = everyPair(flatten(doc as unknown as Record<string, unknown>));
    expect(failing(broken)).toContain("light · teal: muted on ground is 3.50, needs text ≥ 4.5");
  });
});
