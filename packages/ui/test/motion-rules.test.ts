// The motion and state rules of design language §7.0 (motion-rules.ts), checked against
// motion-baseline.json, which lists what breaks them today and only shrinks: a new offender
// fails, and so does a listed one that is gone (remove it from the list). Entries are counted,
// so a second copy of a listed offence in the same file is new too.

import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { offences, sheets } from "./motion-rules.ts";

const BASELINE: Record<string, string[]> = JSON.parse(
  readFileSync(new URL("./motion-baseline.json", import.meta.url), "utf8"),
);

/** How many times each entry appears in `list`. */
const counts = (list: readonly string[]) => {
  const out = new Map<string, number>();
  for (const entry of list) out.set(entry, (out.get(entry) ?? 0) + 1);
  return out;
};

/** The entries of `a` beyond those of `b`, per file, as `file: entry (×n)`. */
function beyond(a: Map<string, string[]>, b: Map<string, string[]>): string[] {
  return [...a].flatMap(([file, list]) => {
    const theirs = counts(b.get(file) ?? []);
    return [...counts(list)]
      .filter(([entry, n]) => n > (theirs.get(entry) ?? 0))
      .map(([entry, n]) => `${file}: ${entry} (${theirs.get(entry) ?? 0} → ${n})`);
  });
}

describe("motion and state rules (design language §7.0)", () => {
  const now = new Map<string, string[]>();
  for (const [file, css] of sheets()) {
    const list = offences(css).map(([kind, what]) => `${kind}: ${what}`);
    if (list.length > 0) now.set(file, list.sort());
  }
  const baseline = new Map(Object.entries(BASELINE));

  it("adds no new break of a rule", () => {
    expect(beyond(now, baseline)).toEqual([]);
  });

  it("lists in the baseline only what still breaks a rule", () => {
    expect(beyond(baseline, now)).toEqual([]);
  });
});

describe("the motion rules' checker", () => {
  it("finds literal durations, easings and action cursors", () => {
    expect(offences(".a { transition: color 120ms ease-out; cursor: pointer !important; }")).toEqual([
      ["duration", "transition: 120ms"],
      ["easing", "transition: ease-out"],
      ["cursor", "pointer"],
    ]);
  });

  it("finds a transition of all, and a :hover outside the guard, once per rule", () => {
    expect(
      offences(".a:hover { color: x; background: y; } .b { transition: all var(--paper-duration-state-in); }"),
    ).toEqual([
      ["hover", ".a:hover"],
      ["transition-all", "transition: all var(--paper-duration-state-in)"],
    ]);
  });

  it("takes only the whole guard: hover and a fine pointer, not negated", () => {
    for (const media of ["(hover: hover)", "not all and (hover: hover) and (pointer: fine)", "(any-hover: hover)"]) {
      expect(offences(`@media ${media} { .a:hover { color: x; } }`), media).toEqual([["hover", ".a:hover"]]);
    }
  });

  it("passes tokens, no time, and :hover inside the guard", () => {
    expect(
      offences(
        "@media (hover: hover) and (pointer: fine) { .a:hover { color: x; } } .b { transition: visibility 0s linear; cursor: var(--paper-cursor-action); }",
      ),
    ).toEqual([]);
  });
});
