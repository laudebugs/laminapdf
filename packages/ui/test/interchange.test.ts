import { describe, expect, it } from "vitest";
import {
  duplicates,
  exportName,
  type FlattenAnswer,
  flattenAction,
  flattenScopes,
  type ImportReport,
  importFailure,
  mismatch,
  needsOverride,
  notDrawn,
  pageList,
  paperSize,
  scopeCount,
  skipped,
  startChoice,
  startFlatten,
  withFormat,
} from "../src/comment/interchange.ts";
import { summaryWords } from "../src/comment/summary-words.ts";

const answer = (change: Partial<FlattenAnswer> = {}): FlattenAnswer => ({
  scope: "all",
  history: false,
  understood: false,
  ...change,
});
const none = { approval: 0, certified: false };

describe("Export's choices", () => {
  it("start Markdown on highlights and notes and the other formats on every comment", () => {
    expect(startChoice("markdown").scope).toBe("highlightsAndNotes");
    expect(startChoice("xfdf").scope).toBe("all");
  });

  it("move the scope with the format only while it is the format's own start", () => {
    expect(withFormat(startChoice("xfdf"), "markdown").scope).toBe("highlightsAndNotes");
    expect(withFormat({ ...startChoice("xfdf"), scope: "shown" }, "markdown").scope).toBe("shown");
  });

  it("name the file after the document, and Markdown's after its highlights", () => {
    expect(exportName("Q3-board-report", "xfdf")).toBe("Q3-board-report.xfdf");
    expect(exportName("Q3-board-report", "fdf")).toBe("Q3-board-report.fdf");
    expect(exportName("Q3-board-report", "markdown")).toBe("Q3-board-report – highlights.md");
  });

  it("count the comments each scope names", () => {
    const counts = { notes: 11, all: 14, shown: 3 };
    expect(["highlightsAndNotes", "all", "shown"].map((s) => scopeCount(counts, s as never))).toEqual([11, 14, 3]);
  });
});

describe("an import's summary", () => {
  const summary = (warnings: ImportReport["warnings"]): ImportReport => ({
    added: 6,
    matched: 2,
    changed: 1,
    deleted: 0,
    pages: [0, 2, 10, 15],
    warnings,
  });

  it("lists its pages as a sentence", () => {
    expect(pageList([15, 0, 10, 2, 2])).toBe("1, 3, 11 and 16");
    expect(pageList([3])).toBe("4");
  });

  it("picks out what is skipped, the file it was made for and shared IDs", () => {
    const s = summary([
      { code: "interchange-skipped", kind: "Sound", reason: "unsupported", page: 11 },
      { code: "interchange-document-mismatch", file: "Q2-board-report.pdf" },
      { code: "annot-name-duplicate", page: 1, name: "a" },
    ]);
    expect([skipped(s).length, mismatch(s), duplicates(s)]).toEqual([1, "Q2-board-report.pdf", 1]);
    expect(mismatch(summary([]))).toBeUndefined();
  });

  it("read a malformed file's format and line from the error", () => {
    expect(importFailure({ code: "interchange-malformed", data: { format: "XFDF", line: 41 } })).toEqual({
      state: "failed",
      format: "XFDF",
      line: 41,
    });
    expect(importFailure(new Error("x"))).toEqual({ state: "failed", format: undefined, line: undefined });
  });
});

describe("Flatten", () => {
  const counts = { all: 14, page: 3, shown: 14, selected: 0 };

  it("offers Selected only with a selection, and starts on it", () => {
    expect(flattenScopes(counts)).toEqual(["all", "page", "shown"]);
    expect(flattenScopes({ ...counts, selected: 2 })).toContain("selected");
    expect(startFlatten({ ...counts, selected: 2 })).toBe("selected");
    expect(startFlatten(counts)).toBe("all");
  });

  it("flattens in place on an unsigned file, and on a signed one, whose signature stays valid", () => {
    expect(flattenAction(none, answer(), 14)).toEqual({ action: "flatten", enabled: true });
    expect(flattenAction({ approval: 1, certified: false }, answer(), 14)).toEqual({
      action: "flatten",
      enabled: true,
    });
  });

  it("offers only a copy on a certified file until the override is ticked", () => {
    const certified = { approval: 0, certified: true };
    expect(flattenAction(certified, answer(), 14)).toEqual({ action: "copy", enabled: true });
    expect(needsOverride(certified, false)).toBe(true);
    expect(flattenAction(certified, answer({ understood: true }), 14)).toEqual({ action: "flatten", enabled: true });
  });

  it("makes removing the history a rewrite, which a signed file needs the override for", () => {
    const signed = { approval: 1, certified: false };
    expect(flattenAction(none, answer({ history: true }), 14)).toEqual({ action: "rewrite", enabled: true });
    expect(needsOverride(signed, true)).toBe(true);
    expect(flattenAction(signed, answer({ history: true }), 14).enabled).toBe(false);
    expect(flattenAction(signed, answer({ history: true, understood: true }), 14).enabled).toBe(true);
  });

  it("can't flatten nothing", () => {
    expect(flattenAction(none, answer({ scope: "page" }), 0).enabled).toBe(false);
  });
});

describe("the summary", () => {
  it("names the pages it couldn't draw from its warnings", () => {
    expect(
      notDrawn([
        { code: "summary-page-not-drawn", data: { page: 2, reason: "fonts-not-embedded" } },
        { code: "summary-page-not-drawn", data: { page: 4, reason: "odd" } },
        { code: "other" },
      ]),
    ).toEqual([
      { page: 2, reason: "fonts-not-embedded" },
      { page: 4, reason: undefined },
    ]);
  });

  it("takes the document's paper or A4 or Letter in points", () => {
    expect(paperSize("document")).toBe("document");
    expect(paperSize("letter")).toEqual([612, 792]);
  });

  it("writes its own words from the catalogue: a template for each phrase, and every kind and state", () => {
    const words = summaryWords();
    expect(words.page).toBe("Page {label}");
    expect(words.commentsOther).toBe("{count} comments");
    expect(words.kinds?.Text).toBe("Note");
    expect(Object.keys(words.kinds ?? {})).toHaveLength(17);
    expect(words.states).toEqual({
      Accepted: "Accepted",
      Rejected: "Rejected",
      Cancelled: "Cancelled",
      Completed: "Completed",
    });
  });
});
