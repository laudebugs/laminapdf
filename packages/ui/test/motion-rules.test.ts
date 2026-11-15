// The motion and state rules of design language §7.0 (motion-rules.ts), checked against
// motion-baseline.json, which lists what breaks them today and only shrinks: a new offender
// fails, and so does a listed one that is gone (remove it from the list). Entries are counted,
// so a second copy of a listed offence in the same file is new too.

import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { declarations, offences, sheets } from "./motion-rules.ts";

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

describe("the state layer", () => {
  const ui = sheets().filter(([file]) => file.startsWith("packages/ui/"));
  /** A selector list split at its top-level commas, whitespace collapsed. */
  const selectorsOf = (list: string) => {
    const out: string[] = [];
    let depth = 0;
    let current = "";
    for (const ch of list.replace(/\s+/g, " ")) {
      if (ch === "(") depth++;
      if (ch === ")") depth--;
      if (ch === "," && depth === 0) {
        out.push(current.trim());
        current = "";
      } else current += ch;
    }
    return [...out, current.trim()];
  };
  /** The compound a selector styles: its last one, outside parentheses; none for a pseudo-element. */
  const subject = (selector: string) => {
    let depth = 0;
    let start = 0;
    for (let i = 0; i < selector.length; i++) {
      const ch = selector[i];
      if (ch === "(") depth++;
      if (ch === ")") depth--;
      if (depth === 0 && (ch === " " || ch === ">" || ch === "+" || ch === "~")) start = i + 1;
    }
    const last = selector.slice(start);
    return last.includes("::") ? "" : last;
  };
  // Every class that draws the layer: the subjects of rules setting background-image to it.
  const layered = new Set(
    ui.flatMap(([, css]) =>
      declarations(css)
        .filter(({ decl }) => /^background-image\s*:.*--ui-state-layer/.test(decl))
        .flatMap(({ selector }) => selectorsOf(selector))
        .flatMap((s) => [...subject(s).matchAll(/\.(paper-[\w-]+)/g)].map((m) => m[1] ?? "")),
    ),
  );

  it("knows the controls it draws on", () => {
    expect([...layered]).toEqual(
      expect.arrayContaining(["paper-button", "paper-swatch", "paper-outline-link", "paper-checkbox-box"]),
    );
  });

  it("is never wiped by a `background` shorthand on one of those controls", () => {
    // The shorthand resets background-image, so the control's hover and press would vanish.
    const found = ui.flatMap(([file, css]) =>
      declarations(css)
        .filter(({ decl }) => /^background\s*:/.test(decl))
        .flatMap(({ selector }) => selectorsOf(selector))
        .filter((s) => [...layered].some((c) => new RegExp(String.raw`\.${c}(?![\w-])`).test(subject(s))))
        .map((s) => `${file}: ${s}`),
    );
    expect(found).toEqual([]);
  });
});
