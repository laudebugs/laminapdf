// doc.initialView(), doc.threads() and doc.setInitialView(): the reads in page space, the edit
// through a save and back, the refusal to replace a script, and the base module's refusal.
import { Document, init, isLaminaPdfError, loadExtract } from "@laminapdf/sdk";
import { beforeAll, describe, expect, it } from "vitest";
import { openOn } from "../examples/set-initial-view.js";
import { read, thrown } from "./helpers.js";

/**
 * A classic PDF of `objects` (numbered from 1; object 1 the catalog) with its xref table: two
 * 600 × 800 pages (3, 4) under the page tree (2) when the caller lists them so.
 */
function pdf(objects: readonly string[]): Uint8Array {
  let body = "%PDF-1.7\n";
  const offsets: number[] = [];
  objects.forEach((object, i) => {
    offsets.push(body.length);
    body += `${i + 1} 0 obj\n${object}\nendobj\n`;
  });
  const xref = body.length;
  body += `xref\n0 ${objects.length + 1}\n0000000000 65535 f \n`;
  body += offsets.map((o) => `${String(o).padStart(10, "0")} 00000 n \n`).join("");
  body += `trailer\n<< /Size ${objects.length + 1} /Root 1 0 R >>\nstartxref\n${xref}\n%%EOF\n`;
  return new TextEncoder().encode(body);
}

const PAGES = [
  "<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 /MediaBox [0 0 600 800] >>",
  "<< /Type /Page /Parent 2 0 R >>",
  "<< /Type /Page /Parent 2 0 R /Rotate 90 >>",
];

beforeAll(() => init());

describe("initial view in the base module", () => {
  it("throws extract-module-not-loaded", () => {
    using doc = Document.open(pdf([`<< /Type /Catalog /Pages 2 0 R >>`, ...PAGES]));
    expect(
      isLaminaPdfError(
        thrown(() => doc.initialView()),
        "extract-module-not-loaded",
      ),
    ).toBe(true);
  });
});

describe("initial view", () => {
  beforeAll(() => loadExtract());

  it("I.06 (pdf, round-trip): reads where a document opens in page space", () => {
    using doc = Document.open(
      pdf([`<< /Type /Catalog /Pages 2 0 R /OpenAction [3 0 R /XYZ 72 700 2] /PageMode /UseOutlines >>`, ...PAGES]),
    );
    expect(doc.initialView()).toEqual({
      open: { kind: "page", target: { page: 0, view: { kind: "at", x: 72, y: 100, zoom: 2 } } },
      pageMode: "outlines",
      preferences: {
        hideToolbar: false,
        hideMenubar: false,
        hideWindowUi: false,
        fitWindow: false,
        centerWindow: false,
        displayDocTitle: false,
      },
    });
  });

  it("I.06 (pdf, round-trip): an edit survives a save and undoes", () => {
    using doc = Document.open(pdf([`<< /Type /Catalog /Pages 2 0 R >>`, ...PAGES]));
    const edited = doc.setInitialView({
      open: { page: doc.page(1).id, zoom: "fitPage" },
      pageLayout: "twoPageRight",
      preferences: { fitWindow: true },
    });
    expect(edited.changed).toBe(true);
    using reopened = Document.open(doc.save());
    const view = reopened.initialView();
    expect([view.open, view.pageLayout, view.preferences.fitWindow]).toEqual([
      { kind: "page", target: { page: 1, view: { kind: "fit" } } },
      "twoPageRight",
      true,
    ]);
    doc.undo();
    expect(doc.initialView().open).toEqual({ kind: "default" });
  });

  it("I.11 (pdf, round-trip): sets the window preferences and removes them with null", () => {
    using doc = Document.open(pdf([`<< /Type /Catalog /Pages 2 0 R /PageLayout /OneColumn >>`, ...PAGES]));
    doc.setInitialView({ pageLayout: null, preferences: { displayDocTitle: true, centerWindow: true } });
    using reopened = Document.open(doc.save());
    const view = reopened.initialView();
    expect([view.pageLayout, view.preferences.displayDocTitle, view.preferences.centerWindow]).toEqual([
      undefined,
      true,
      true,
    ]);
  });

  it("refuses to replace an open action that runs a script", () => {
    using doc = Document.open(
      pdf([`<< /Type /Catalog /Pages 2 0 R /OpenAction << /S /JavaScript /JS (1) >> >>`, ...PAGES]),
    );
    expect(doc.initialView().open).toEqual({ kind: "action", action: "JavaScript" });
    const e = thrown(() => doc.setInitialView({ open: { page: doc.page(0).id } }));
    expect(isLaminaPdfError(e, "open-action-not-destination")).toBe(true);
  });

  it("the example opens on the page at its width with the bookmarks shown", () => {
    using doc = Document.open(openOn(read("fixtures/corpus/outline.pdf"), 1));
    const view = doc.initialView();
    expect([
      view.open.kind === "page" && view.open.target.page,
      view.pageMode,
      view.preferences.displayDocTitle,
    ]).toEqual([1, "outlines", true]);
  });
});

describe("article threads", () => {
  beforeAll(() => loadExtract());

  it("I.09 (pdf, read): lists each thread's parts as rectangles in page space", () => {
    using doc = Document.open(
      pdf([
        `<< /Type /Catalog /Pages 2 0 R /Threads [5 0 R] >>`,
        ...PAGES,
        "<< /Type /Thread /F 6 0 R /I << /Title (Backlog) >> >>",
        "<< /Type /Bead /T 5 0 R /N 7 0 R /V 7 0 R /P 3 0 R /R [50 600 300 700] >>",
        "<< /Type /Bead /N 6 0 R /V 6 0 R /P 4 0 R /R [0 0 600 100] >>",
      ]),
    );
    expect(doc.threads()).toEqual([
      {
        title: "Backlog",
        parts: [
          { page: 0, rect: [50, 100, 300, 200] },
          // Turned a quarter clockwise, the page's bottom strip is its left edge as displayed.
          { page: 1, rect: [0, 0, 100, 600] },
        ],
      },
    ]);
  });

  it("is empty without any", () => {
    using doc = Document.open(pdf([`<< /Type /Catalog /Pages 2 0 R >>`, ...PAGES]));
    expect(doc.threads()).toEqual([]);
  });
});
