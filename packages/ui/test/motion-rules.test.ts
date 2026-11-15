// The motion and state rules of design language §7.0 (motion-rules.ts), checked against
// motion-baseline.json, which lists what breaks them today and only shrinks: a new offender
// fails, and so does a listed one that is gone (remove it from the list).

import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { offences, sheets } from "./motion-rules.ts";

const BASELINE: Record<string, string[]> = JSON.parse(
  readFileSync(new URL("./motion-baseline.json", import.meta.url), "utf8"),
);

describe("motion and state rules (design language §7.0)", () => {
  const now = new Map<string, string[]>();
  for (const [file, css] of sheets()) {
    const list = offences(css).map(([kind, what]) => `${kind}: ${what}`);
    if (list.length > 0) now.set(file, list.sort());
  }

  it("adds no new break of a rule", () => {
    const added = [...now].flatMap(([file, list]) =>
      list.filter((entry) => !(BASELINE[file] ?? []).includes(entry)).map((entry) => `${file}: ${entry}`),
    );
    expect(added).toEqual([]);
  });

  it("lists in the baseline only what still breaks a rule", () => {
    const gone = Object.entries(BASELINE).flatMap(([file, list]) =>
      list.filter((entry) => !(now.get(file) ?? []).includes(entry)).map((entry) => `${file}: ${entry}`),
    );
    expect(gone).toEqual([]);
  });

  it("finds what it should", () => {
    expect(offences(".a { transition: color 120ms ease-out; cursor: pointer; }")).toEqual([
      ["duration", "transition: 120ms"],
      ["easing", "transition: ease-out"],
      ["cursor", "cursor: pointer"],
    ]);
    expect(offences(".a:hover { color: red; } .b { transition: all var(--paper-duration-state-in); }")).toEqual([
      ["hover", ".a:hover"],
      ["transition-all", "transition: all var(--paper-duration-state-in)"],
    ]);
    expect(
      offences(
        "@media (hover: hover) and (pointer: fine) { .a:hover { color: x; } } .b { transition: visibility 0s linear; cursor: var(--paper-cursor-action); }",
      ),
    ).toEqual([]);
  });
});
