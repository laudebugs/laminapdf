import type { ContentTextRun, ObjectRef, TextEditReport } from "@laminapdf/sdk";
import { describe, expect, it } from "vitest";
import {
  cycleRuns,
  editTextOp,
  followRuns,
  fontFacts,
  hitRun,
  lastFirst,
  mergedRefs,
  neighbourRun,
  type PlanHost,
  planText,
  readableFont,
  reasonOf,
  runsWithin,
  setTextStyleOp,
  standardFor,
  styleOf,
  styleStep,
  substitutedCharacters,
  substituteFont,
  substituteOrder,
  underlineOf,
} from "./text-runs.ts";

/** A run of `text` at `bounds`, the `start`th glyph of text object `ordinal`. */
function run(
  ordinal: number,
  bounds: [number, number, number, number],
  text = "word",
  extra: Partial<ContentTextRun> = {},
  start = 0,
): ContentTextRun {
  return {
    run: { object: { ordinal, hash: `h${ordinal}` } as unknown as ObjectRef, start, end: start + text.length },
    text,
    bounds,
    fontName: "Helvetica",
    fontSize: 12,
    line: ordinal,
    readOnly: undefined,
    ...extra,
  } as unknown as ContentTextRun;
}

describe("which run a click lands on", () => {
  it("takes the smallest run under the point, and the one drawn last of two alike", () => {
    const line = run(0, [0, 0, 300, 20], "a whole line");
    const word = run(1, [100, 2, 140, 18], "word");
    const twin = run(2, [100, 2, 140, 18], "word");
    expect(hitRun([line, word], { x: 110, y: 10 })).toBe(word);
    expect(hitRun([line, word, twin], { x: 110, y: 10 })).toBe(twin);
    expect(hitRun([line, word], { x: 10, y: 10 })).toBe(line);
  });

  it("leaves a little slop around a run and nothing for a run with no area", () => {
    const word = run(1, [100, 100, 140, 112]);
    expect(hitRun([word], { x: 141, y: 106 })).toBe(word);
    expect(hitRun([word], { x: 150, y: 106 })).toBeUndefined();
    expect(hitRun([run(2, [10, 10, 10, 20], "")], { x: 10, y: 15 })).toBeUndefined();
  });

  it("lists the runs inside a block's box", () => {
    const a = run(0, [10, 10, 50, 20]);
    const b = run(1, [10, 30, 50, 40]);
    const far = run(2, [400, 400, 450, 410]);
    expect(runsWithin([a, b, far], [0, 0, 100, 100])).toEqual([a, b]);
  });
});

describe("a pick through an edit", () => {
  it("follows its run to where the engine puts it, even as the run's name changes", () => {
    const before = run(3, [72, 100, 150, 112], "Hello");
    const after = [run(2, [72, 40, 150, 52], "Title"), run(3, [72, 100, 190, 112], "Hello there")];
    // The renamed run is longer but starts in the same place.
    expect(followRuns([before], after)).toEqual([after[1]]);
  });

  it("follows every part when one run is split by a substitution", () => {
    const before = run(3, [72, 100, 192, 112], "Hello wörld");
    const parts = [
      run(3, [72, 100, 130, 112], "Hello w"),
      run(4, [130, 100, 140, 112], "ö"),
      run(5, [140, 100, 192, 112], "rld"),
    ];
    expect(followRuns([before], parts)).toEqual(parts);
  });

  it("follows a run whose size changed by the old run's centre, and nothing for a deleted run", () => {
    const before = run(3, [72, 100, 150, 112]);
    const bigger = run(3, [72, 92, 200, 116]);
    expect(followRuns([before], [bigger])).toEqual([bigger]);
    expect(followRuns([before], [run(9, [300, 300, 340, 312])])).toEqual([]);
  });
});

describe("stepping through the runs by key", () => {
  it("skips what can't be edited and wraps", () => {
    const a = run(0, [0, 0, 10, 10]);
    const locked = run(1, [0, 20, 10, 30], "x", { readOnly: "type3-font" });
    const b = run(2, [0, 40, 10, 50]);
    expect(cycleRuns([a, locked, b], undefined, 1)).toBe(a);
    expect(cycleRuns([a, locked, b], undefined, -1)).toBe(b);
    expect(cycleRuns([a, locked, b], a, 1)).toBe(b);
    expect(cycleRuns([a, locked, b], b, 1)).toBe(a);
    expect(cycleRuns([locked], undefined, 1)).toBeUndefined();
  });
});

describe("what a run's font says", () => {
  it("reads weight and slant from the font's name, and the subset tag off", () => {
    expect(readableFont("ABCDEF+Source-Serif-Bold")).toBe("Source Serif Bold");
    expect(fontFacts("Helvetica-BoldOblique")).toEqual({ bold: true, italic: true });
    expect(fontFacts("ABCDEF+Times-Roman")).toEqual({ bold: false, italic: false });
    expect(fontFacts("Calibri-Italic")).toEqual({ bold: false, italic: true });
  });

  it("names the standard font nearest to it", () => {
    expect(standardFor("Times-Roman")).toBe("times");
    expect(standardFor("ABCDEF+Garamond")).toBe("times");
    expect(standardFor("Courier-Bold")).toBe("courier");
    expect(standardFor("Arial-BoldMT")).toBe("helvetica");
    expect(standardFor("NotoSans-Regular")).toBe("helvetica");
  });
});

describe("the ops text edits make", () => {
  const ref = run(4, [0, 0, 10, 10]).run;

  it("names the run, the text and, when asked, the fonts", () => {
    expect(editTextOp("p1", ref, "new")).toEqual({ op: "editText", args: { page: "p1", run: ref, text: "new" } });
    expect(editTextOp("p1", ref, "new", ["helvetica", "blob-1"])).toEqual({
      op: "editText",
      args: { page: "p1", run: ref, text: "new", fonts: ["helvetica", "blob-1"] },
    });
    // No fonts means none are allowed, not an empty list.
    expect(editTextOp("p1", ref, "", [])).toEqual({ op: "editText", args: { page: "p1", run: ref, text: "" } });
  });

  it("sets only what a change names", () => {
    expect(styleOf({ bold: true })).toEqual({ bold: true });
    expect(styleOf({ size: 14, color: [0.1, 0.2, 0.3] })).toEqual({ size: 14, color: [0.1, 0.2, 0.3] });
    expect(styleOf({ underline: true, align: "center" })).toEqual({ underline: true, align: "center" });
    expect(styleOf({ bold: false })).toEqual({ bold: false });
    expect(setTextStyleOp("p1", [ref], styleOf({ italic: true }))).toEqual({
      op: "setTextStyle",
      args: { page: "p1", runs: [ref], italic: true },
    });
  });

  it("names a step by its one field", () => {
    expect(styleStep({ bold: true })).toBe("bold");
    expect(styleStep({ bold: false })).toBe("unbold");
    expect(styleStep({ italic: false })).toBe("unitalic");
    expect(styleStep({ underline: true })).toBe("underline");
    expect(styleStep({ color: [0, 0, 0] })).toBe("colour");
    expect(styleStep({ size: 9, bold: true })).toBe("style");
  });

  it("edits several runs last drawn first, so an earlier name isn't renumbered", () => {
    const a = run(1, [0, 0, 1, 1], "a");
    const b = run(2, [0, 0, 1, 1], "b");
    const c = run(2, [0, 0, 1, 1], "c", {}, 5);
    expect(lastFirst([a, b, c])).toEqual([c, b, a]);
  });
});

/** The engine as a fake: a run's own font lacks "č"; the chain draws it in Times; no font has "你". */
function engine(
  lacking: readonly string[] = ["č"],
  nothing: readonly string[] = ["你"],
): PlanHost & { calls: unknown[] } {
  const calls: unknown[] = [];
  const report = (fidelity: string, substitutions: object[] = []) =>
    ({ fidelity, substitutions, grewBy: 0 }) as unknown as TextEditReport;
  const miss = (ch: string) =>
    Object.assign(
      new Error(`the font has no glyph for U+${ch.codePointAt(0)?.toString(16).toUpperCase().padStart(4, "0")}`),
      {
        code: "missing-glyph",
      },
    );
  return {
    calls,
    plan: async (_page, _run, text, fonts) => {
      calls.push(fonts);
      const bad = [...text].find((c) => nothing.includes(c));
      if (bad !== undefined) throw miss(bad);
      const gap = [...text].find((c) => lacking.includes(c));
      if (gap === undefined) return report("exact");
      if (fonts === undefined) throw miss(gap);
      return report("substituted", [{ text: gap, font: "Times-Roman", kind: "substitute" }]);
    },
    chain: async (primary, text) => ({
      fonts: [primary, "times"],
      missing: [...text].filter((c) => nothing.includes(c)),
    }),
  };
}

describe("what an edit would do, before it is made", () => {
  it("says nothing when the run's own font draws every character", async () => {
    const plan = await planText(engine(), 0, run(0, [0, 0, 10, 10]), "plain");
    expect(plan.kind).toBe("ok");
  });

  it("names the characters and the font that draws them, asking for fonts only after the run's own fell short", async () => {
    const host = engine();
    const plan = await planText(host, 0, run(0, [0, 0, 10, 10]), "Čech and č");
    // The first plan names no font: the second does.
    expect(host.calls).toEqual([undefined, ["helvetica", "times"]]);
    expect(plan.kind).toBe("substituted");
    if (plan.kind !== "substituted") return;
    expect(substitutedCharacters(plan.substitutions)).toEqual(["č"]);
    expect(substituteFont(plan.substitutions)).toBe("Times-Roman");
    expect(plan.fonts).toEqual(["helvetica", "times"]);
  });

  it("picks the chain by the run's own look", async () => {
    const host = engine();
    await planText(host, 0, run(0, [0, 0, 10, 10], "x", { fontName: "ABCDEF+Garamond" }), "č");
    expect(host.calls[1]).toEqual(["times", "times"]);
  });

  it("names a character no font draws, from the registry or else from the engine's message", async () => {
    const plan = await planText(engine(), 0, run(0, [0, 0, 10, 10]), "你好");
    expect(plan).toEqual({ kind: "missing", characters: ["你"] });
    const quiet = engine(["č"], ["你"]);
    const bare: PlanHost = { ...quiet, chain: async (primary) => ({ fonts: [primary], missing: [] }) };
    expect(await planText(bare, 0, run(0, [0, 0, 10, 10]), "你")).toEqual({ kind: "missing", characters: ["你"] });
  });

  it("gives the engine's reason for text that can't be edited in place, from the run or from the refusal", async () => {
    expect(await planText(engine(), 0, run(0, [0, 0, 10, 10], "x", { readOnly: "type3-font" }), "y")).toEqual({
      kind: "refused",
      reason: "type3-font",
    });
    const refusing: PlanHost = {
      plan: async () => {
        throw Object.assign(new Error("content-not-editable"), {
          code: "content-not-editable",
          data: { reason: "actual-text" },
        });
      },
      chain: async () => ({ fonts: [], missing: [] }),
    };
    expect(await planText(refusing, 0, run(0, [0, 0, 10, 10]), "y")).toEqual({
      kind: "refused",
      reason: "actual-text",
    });
    expect(reasonOf(new Error("content-not-editable: the reason is vertical-text"))).toBe("vertical-text");
  });
});

describe("the line under a run, and the fonts that stand in", () => {
  it("finds a thin path that spans the run just under it, and not a rule that only lies near", () => {
    const text = run(0, [72, 100, 200, 112], "Hello");
    const under = { kind: "path", bounds: [72, 111, 200, 113] };
    const rule = { kind: "path", bounds: [72, 111, 400, 112] };
    const box = { kind: "path", bounds: [72, 111, 200, 140] };
    expect(underlineOf(text, [rule, box] as never)).toBeUndefined();
    expect(underlineOf(text, [rule, under] as never)).toBe(under);
  });

  it("puts the registry's fonts before the standard font, which can have letters it can't write", () => {
    expect(substituteOrder(["helvetica", "noto-1"])).toEqual(["noto-1", "helvetica"]);
    expect(substituteOrder(["helvetica"])).toEqual(["helvetica"]);
    expect(substituteOrder([])).toEqual([]);
  });

  it("stops at the ends instead of wrapping", () => {
    const a = run(0, [0, 0, 10, 10]);
    const b = run(1, [0, 20, 10, 30]);
    expect(neighbourRun([a, b], a, 1)).toBe(b);
    expect(neighbourRun([a, b], b, 1)).toBeUndefined();
    expect(neighbourRun([a, b], a, -1)).toBeUndefined();
  });
});

describe("deleting several runs in one step", () => {
  it("joins runs that touch in one text object, which an edit to either would rename, and goes last first", () => {
    const a = run(1, [0, 0, 1, 1], "a");
    const piece = run(2, [0, 0, 1, 1], "Č", {}, 0);
    const rest = run(2, [0, 0, 1, 1], "ech", {}, 1);
    const refs = mergedRefs([a, piece, rest]);
    expect(refs.map((r) => [r.object.ordinal, r.start, r.end])).toEqual([
      [2, 0, 4],
      [1, 0, 1],
    ]);
  });
});
