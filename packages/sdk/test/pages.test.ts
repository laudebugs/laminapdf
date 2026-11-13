// The page ops: insertBlankPages, deletePages, movePages, rotatePages, reversePages and
// duplicatePages on
// fixtures/pages/, each saved and reopened, their `Edited` reports, the transaction handle's
// twins and the refusal to delete every page. The names start with the capability each test
// proves (C.01 and the rest); they become capability tags once `cargo xtask capabilities` runs
// TypeScript tests.
import { Document, init, isLaminaPdfError, loadExtract } from "@laminapdf/sdk";
import { beforeAll, describe, expect, it } from "vitest";
import { read, thrown } from "./helpers.js";

beforeAll(async () => {
  await init();
  await loadExtract();
});

const LETTER = { width: 612, height: 792 };
const A5 = { width: 420, height: 595 };

function open(name: string): Document {
  return Document.open(read(`fixtures/pages/${name}.pdf`));
}

/** The page IDs in order. */
function ids(doc: Document): string[] {
  return doc.pages.map((p) => p.id);
}

/** Each page's marker number: "Page n" drawn by fixtures/pages/make.py, read back as text. */
function markers(doc: Document): string[] {
  return doc.pages.map((_, i) => doc.page(i).text().text().trim());
}

/** The document saved (default options) and reopened. */
function reopened(doc: Document): Document {
  return Document.open(doc.save());
}

describe("insertBlankPages", () => {
  it("C.01 (pdf, round-trip): inserts blank pages after a page, with new IDs, kept after a save", () => {
    using doc = open("labels");
    const before = ids(doc);
    const edited = doc.insertBlankPages({ after: before[3] as string }, 2, A5, { rotate: 90 });
    const order = edited.pageOrder ?? [];
    expect(order).toHaveLength(12);
    expect(order.slice(0, 4)).toEqual(before.slice(0, 4));
    expect(order.slice(6)).toEqual(before.slice(4));
    const added = order.slice(4, 6);
    expect(added.some((id) => before.includes(id))).toBe(false);
    using again = reopened(doc);
    expect(again.pageCount).toBe(12);
    // A5 turned a quarter shows landscape.
    expect(again.pages.slice(4, 6).map((p) => [p.width, p.height, p.rotation])).toEqual([
      [595, 420, 90],
      [595, 420, 90],
    ]);
    // The new pages continue the arabic range they land in, after "1"; every other page keeps its label.
    expect(again.pages.map((p) => p.label)).toEqual([
      "i",
      "ii",
      "iii",
      "1",
      "2",
      "3",
      "2",
      "3",
      "4",
      "A-1",
      "A-2",
      "Back cover",
    ]);
  });

  it("inserts upright pages at the start and the end", () => {
    using doc = open("references");
    doc.insertBlankPages("start", 1, LETTER);
    doc.insertBlankPages("end", 1, LETTER);
    using again = reopened(doc);
    expect(again.pageCount).toBe(7);
    expect(markers(again)).toEqual(["", "Page 1", "Page 2", "Page 3", "Page 4", "Page 5", ""]);
    expect(again.pages[0]?.rotation).toBe(0);
  });

  it("refuses an anchor that isn't a page", () => {
    using doc = open("references");
    expect(
      isLaminaPdfError(
        thrown(() => doc.insertBlankPages({ before: "p999.0" }, 1, LETTER)),
        "unknown-page",
      ),
    ).toBe(true);
    expect(doc.pageCount).toBe(5);
  });
});

describe("deletePages", () => {
  it("retargets what pointed at a deleted page to the next page, and reports each reference", () => {
    using doc = open("references");
    const [, p2, p3, p4] = ids(doc);
    const edited = doc.deletePages([p3 as string]);
    expect(edited.pageOrder).toEqual(ids(doc));
    expect(edited.pageOrder).not.toContain(p3);
    const retargeted = edited.references.filter((r) => r.outcome === "retargeted");
    expect(retargeted.length).toBeGreaterThan(0);
    expect(new Set(retargeted.map((r) => r.to))).toEqual(new Set([p4]));
    expect(edited.references.every((r) => r.page === p3)).toBe(true);
    const kinds = new Set(edited.references.map((r) => r.kind));
    for (const kind of ["outlineItem", "link", "namedDestination", "openAction"] as const) {
      expect(kinds).toContain(kind);
    }
    expect(edited.references.find((r) => r.kind === "namedDestination" && r.name === "chap3")?.to).toBe(p4);

    using again = reopened(doc);
    expect(markers(again)).toEqual(["Page 1", "Page 2", "Page 4", "Page 5"]);
    expect(ids(again)).not.toContain(p3);
    // The two bookmarks to page 3 now go to page 4 (index 2), the one to page 2 is unchanged.
    expect(again.outline().map((n) => [n.title, n.target?.page])).toEqual([
      ["Dest to page 3", 2],
      ["GoTo page 3", 2],
      ["Page 2", 1],
    ]);
    expect(again.namedDestination("chap3")?.page).toBe(2);
    expect(p2).toBe(ids(again)[1]);
  });

  it('removes what pointed at a deleted page with references: "remove"', () => {
    using doc = open("references");
    const p3 = ids(doc)[2] as string;
    const edited = doc.deletePages([p3], { references: "remove" });
    expect(edited.references.length).toBeGreaterThan(0);
    expect(edited.references.every((r) => r.outcome === "removed" && r.to === undefined)).toBe(true);
    expect(edited.stillInEarlierRevisions).toBe(true);
    using again = reopened(doc);
    expect(again.pageCount).toBe(4);
    expect(again.namedDestination("chap3")).toBeUndefined();
    expect(again.outline().map((n) => [n.title, n.target?.page])).toEqual([
      ["Dest to page 3", undefined],
      ["GoTo page 3", undefined],
      ["Page 2", 1],
    ]);
  });

  it("keeps every surviving page's label", () => {
    using doc = open("labels");
    const before = doc.pages;
    doc.deletePages([before[1]?.id as string, before[7]?.id as string]);
    using again = reopened(doc);
    expect(again.pages.map((p) => p.label)).toEqual(["i", "iii", "1", "2", "3", "4", "A-2", "Back cover"]);
  });

  it("refuses to delete every page with no-pages-left, and changes nothing", () => {
    using doc = open("references");
    const all = ids(doc);
    expect(
      isLaminaPdfError(
        thrown(() => doc.deletePages(all)),
        "no-pages-left",
      ),
    ).toBe(true);
    expect(ids(doc)).toEqual(all);
    expect(doc.hasUnsavedChanges).toBe(false);
  });

  it("is undone as one step", () => {
    using doc = open("references");
    const all = ids(doc);
    doc.deletePages([all[0] as string, all[4] as string]);
    expect(doc.pageCount).toBe(3);
    doc.undo();
    expect(ids(doc)).toEqual(all);
  });
});

describe("movePages", () => {
  it("C.04 (pdf, round-trip): moves pages in the order given, keeping their IDs, kept after a save", () => {
    using doc = open("references");
    const [p1, p2, p3, p4, p5] = ids(doc) as [string, string, string, string, string];
    const edited = doc.movePages([p5, p4], { before: p2 });
    expect(edited.pageOrder).toEqual([p1, p5, p4, p2, p3]);
    expect(edited.references).toEqual([]);
    using again = reopened(doc);
    expect(ids(again)).toEqual([p1, p5, p4, p2, p3]);
    expect(markers(again)).toEqual(["Page 1", "Page 5", "Page 4", "Page 2", "Page 3"]);
    // Bookmarks follow their pages.
    expect(again.outline().map((n) => n.target?.page)).toEqual([4, 4, 3]);
  });

  it("keeps each page's label as it moves", () => {
    using doc = open("labels");
    const before = doc.pages;
    doc.movePages([before[9]?.id as string], "start");
    using again = reopened(doc);
    expect(again.pages[0]?.id).toBe(before[9]?.id);
    expect(again.pages.map((p) => p.label)).toEqual([before[9], ...before.slice(0, 9)].map((p) => p?.label));
  });
});

describe("rotatePages", () => {
  it("C.08 (pdf, round-trip): sets and turns several pages' rotation, kept after a save", () => {
    using doc = open("boxes");
    const pages = doc.pages;
    const all = pages.map((p) => p.id);
    const before = pages.map((p) => p.rotation);
    doc.rotatePages(all, 90, { mode: "turn" });
    using turned = reopened(doc);
    expect(turned.pages.map((p) => p.rotation)).toEqual(before.map((r) => (r + 90) % 360));
    expect(ids(turned)).toEqual(all);

    turned.rotatePages(all.slice(0, 2), 180);
    using set = reopened(turned);
    expect(set.pages.map((p) => p.rotation)).toEqual([180, 180, ...before.slice(2).map((r) => (r + 90) % 360)]);
  });
});

describe("reversePages", () => {
  it("C.13 (pdf, round-trip): reverses the whole document, kept after a save", () => {
    using doc = open("references");
    const all = ids(doc);
    const edited = doc.reversePages(all);
    expect(edited.pageOrder).toEqual([...all].reverse());
    using again = reopened(doc);
    expect(ids(again)).toEqual([...all].reverse());
    expect(markers(again)).toEqual(["Page 5", "Page 4", "Page 3", "Page 2", "Page 1"]);
  });

  it("reverses some pages among the places they hold, leaving the others", () => {
    using doc = open("references");
    const [p1, p2, p3, p4, p5] = ids(doc) as [string, string, string, string, string];
    expect(doc.reversePages([p4, p2, p1]).pageOrder).toEqual([p4, p2, p3, p1, p5]);
  });
});

describe("duplicatePages", () => {
  it("C.03 (pdf, round-trip): copies a form page to the end with new fields, kept after a save", () => {
    using doc = open("forms");
    const [p1, p2] = ids(doc) as [string, string];
    const edited = doc.duplicatePages([p1], "end");
    const order = edited.pageOrder ?? [];
    expect(order.slice(0, 2)).toEqual([p1, p2]);
    expect(order).toHaveLength(3);
    const copy = order[2] as string;
    expect([p1, p2]).not.toContain(copy);
    expect((edited.fields ?? []).map((f) => [f.outcome, f.name, f.page])).toEqual([
      ["renamed", "name#2", copy],
      ["renamed", "agree#2", copy],
    ]);
    using again = reopened(doc);
    expect(markers(again)).toEqual(["Page 1", "Page 2", "Page 1"]);
  });

  it("leaves a signature's widget out of the copy and says so", () => {
    using doc = open("forms");
    const [, p2] = ids(doc) as [string, string];
    const fields = doc.duplicatePages([p2], "start").fields ?? [];
    expect(fields.map((f) => [f.outcome, f.name])).toEqual([
      ["notCopied", undefined],
      ["renamed", "agree#2"],
    ]);
  });

  it("lists a copy whose tags it couldn't clone whole", () => {
    using doc = open("tagged");
    const [p1] = ids(doc) as [string];
    const edited = doc.duplicatePages([p1], "end");
    expect(edited.tagsIncomplete).toEqual([edited.pageOrder?.[2]]);
  });

  it("is undone as one step", () => {
    using doc = open("forms");
    const before = ids(doc);
    doc.duplicatePages(before, { after: before[0] as string });
    expect(doc.pageCount).toBe(4);
    doc.undo();
    expect(ids(doc)).toEqual(before);
  });
});

describe("the transaction handle", () => {
  it("collects the page ops into one edit that one undo reverts", () => {
    using doc = open("references");
    const [p1, p2, p3, , p5] = ids(doc) as [string, string, string, string, string];
    const all = ids(doc);
    doc.transact((tx) => {
      tx.deletePages([p3]);
      tx.movePages([p5], "start");
      tx.insertBlankPages("end", 1, LETTER);
      tx.rotatePages([p1, p2], 90, { mode: "turn" });
      tx.reversePages([p1, p2]);
      tx.duplicatePages([p1], "end");
    });
    expect(doc.pageCount).toBe(6);
    expect(ids(doc).slice(0, 3)).toEqual([p5, p2, p1]);
    expect(doc.pages.slice(1, 3).map((p) => p.rotation)).toEqual([90, 90]);
    doc.undo();
    expect(ids(doc)).toEqual(all);
    expect(doc.pages.map((p) => p.rotation)).toEqual([0, 0, 0, 0, 0]);
  });
});

describe("pages from other documents", () => {
  it("C.02 (pdf, round-trip): copies a page in from a source, renaming a field whose name is taken", () => {
    using doc = open("forms");
    const { id: source, pages } = doc.addSource(read("fixtures/pages/references.pdf"));
    expect(source).toMatch(/^[0-9a-f]{64}$/);
    expect(doc.addSource(read("fixtures/pages/references.pdf")).id).toBe(source);
    expect(pages).toHaveLength(5);
    const edited = doc.insertPagesFrom(source, [pages[2] as string], "end");
    expect(edited.pageOrder).toHaveLength(3);
    expect((edited.fields ?? []).map((f) => [f.outcome, f.name])).toEqual([["renamed", "name#2"]]);
    expect(edited.tagsDropped).toHaveLength(1);
    using again = reopened(doc);
    expect(markers(again)).toEqual(["Page 1", "Page 2", "Page 3"]);
  });

  it("removes a copied link whose page didn't come along", () => {
    using doc = open("forms");
    const { id: source, pages } = doc.addSource(read("fixtures/pages/references.pdf"));
    const [, p2] = pages as [string, string];
    const edited = doc.insertPagesFrom(source, [p2], "start", { annotations: true });
    expect(edited.linksRemoved).toEqual([edited.pageOrder?.[0]]);
  });

  it("C.16 (pdf, round-trip): replaces a page, keeping the bookmarks that pointed at it", () => {
    using doc = open("references");
    const before = ids(doc);
    const { id: source, pages } = doc.addSource(read("fixtures/pages/forms.pdf"));
    const [first] = pages as [string];
    const edited = doc.replacePages([before[2] as string], source, [first]);
    const copy = edited.pageOrder?.[2] as string;
    expect(before).not.toContain(copy);
    expect(edited.references.some((r) => r.outcome === "retargeted" && r.to === copy)).toBe(true);
    using again = reopened(doc);
    expect(again.outline()[0]?.target?.page).toBe(2);
  });

  it("C.05 (pdf, write): extracts pages into a new document with their labels", () => {
    using doc = open("labels");
    const pages = ids(doc).slice(2, 5);
    using part = doc.extractPages(pages);
    using again = Document.open(part.save());
    expect(again.pageCount).toBe(3);
    expect(again.pages.map((p) => p.label)).toEqual(doc.pages.slice(2, 5).map((p) => p.label));
  });

  it("H.10 (pdf, write): exports ranges as one document each", () => {
    using doc = open("references");
    const pages = ids(doc);
    const parts = doc.exportRanges([pages.slice(0, 2), pages.slice(2)]);
    try {
      expect(parts.map((p) => p.pageCount)).toEqual([2, 3]);
    } finally {
      for (const part of parts) part.free();
    }
  });

  it("refuses a source it wasn't given", () => {
    using doc = open("forms");
    const missing = "0".repeat(64);
    expect(thrown(() => doc.insertPagesFrom(missing, ["p3.0"], "end"))).toMatchObject({
      code: "unknown-source",
    });
  });
});
