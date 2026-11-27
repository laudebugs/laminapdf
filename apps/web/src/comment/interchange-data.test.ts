import { describe, expect, it } from "vitest";
import {
  type CommentScopes,
  exportOptions,
  flattenCounts,
  flattenOp,
  pagesOf,
  previewOf,
  scopeCounts,
  staysAmong,
  summaryCounts,
  summaryRequest,
} from "./interchange-data.ts";
import type { ThreadView } from "./threads.ts";

/** A thread with only what the dialogs read. */
const thread = (id: string, page: number, subtype: string, kind: ThreadView["kind"]): ThreadView =>
  ({ id, page, subtype, kind }) as ThreadView;

const threads = [
  thread("a1", 0, "Highlight", "markup"),
  thread("a2", 0, "Text", "notes"),
  thread("a3", 2, "Ink", "drawings"),
  thread("a4", 3, "Sound", "stamps"),
];

const scopes: CommentScopes = {
  all: threads,
  shown: [threads[0] as ThreadView, threads[2] as ThreadView],
  page: [threads[0] as ThreadView, threads[1] as ThreadView],
  selected: [],
};

const choice = (scope: "highlightsAndNotes" | "all" | "shown", format = "markdown" as const) => ({
  format,
  scope,
  include: { headings: true, authors: false, replies: true },
});

describe("the interchange dialogs' counts", () => {
  it("count what each export choice names", () => {
    expect(scopeCounts(scopes)).toEqual({ notes: 2, all: 4, shown: 2 });
  });

  it("count each flatten scope, and the pages and comments the summary lists", () => {
    expect(flattenCounts(scopes)).toEqual({ all: 4, page: 2, shown: 2, selected: 0 });
    expect(summaryCounts(scopes)).toEqual({
      all: { comments: 4, pages: 3 },
      shown: { comments: 2, pages: 2 },
    });
    expect(pagesOf(threads)).toEqual([0, 2, 3]);
  });

  it("say how many items on how many pages Markdown holds", () => {
    expect(previewOf("# x", scopes, choice("highlightsAndNotes"))).toEqual({ text: "# x", items: 2, pages: 1 });
    expect(previewOf("# x", scopes, choice("shown"))).toMatchObject({ items: 1, pages: 1 });
  });
});

describe("what the worker is asked", () => {
  it("exports the panel's comments as the pages they are on, and the notes as Markdown's own choice", () => {
    expect(exportOptions(scopes, choice("shown"), "r.pdf")).toEqual({
      format: "markdown",
      comments: "all",
      pages: [0, 2],
      file: "r.pdf",
      markdown: { headings: true, authors: false, replies: true },
    });
    expect(exportOptions(scopes, choice("highlightsAndNotes", "xfdf" as never), "r.pdf").comments).toBe(
      "highlightsAndNotes",
    );
  });

  it("flattens comments by ID as the view shows them, leaving a sound's kind to say it stays", () => {
    expect(flattenOp([threads[0] as ThreadView])).toEqual({
      op: "flattenAnnots",
      args: { annots: ["a1"], intent: "view" },
    });
    expect(staysAmong(threads)).toEqual(["Sound"]);
  });

  it("asks for the summary in the catalogue's words, listing the panel's comments when it is chosen", () => {
    const choose = {
      layout: "list",
      comments: "shown",
      replies: true,
      status: false,
      emptyPages: false,
      paper: "a4",
    } as const;
    const request = summaryRequest(scopes, choose, [new Uint8Array([1])], undefined, "en");
    expect(request).toMatchObject({ layout: "list", comments: ["a1", "a3"], status: false, language: "en" });
    expect(request.paper).toEqual([595.28, 841.89]);
    expect(request.words?.page).toBe("Page {label}");
    expect(summaryRequest(scopes, { ...choose, comments: "all" }, [], "T", "en").comments).toBeUndefined();
  });
});
