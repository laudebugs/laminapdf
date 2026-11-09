import { describe, expect, it } from "vitest";
import {
  choose,
  documentKey,
  type PageAppearanceChoices,
  parseChoices,
  resolve,
  undo,
  withDocument,
} from "./page-appearance.tsx";

const none: PageAppearanceChoices = { documents: {} };

describe("the stored page appearance choices", () => {
  it("read nothing from damaged or empty storage, and drop unknown names", () => {
    expect(parseChoices(null)).toEqual(none);
    expect(parseChoices("not json")).toEqual(none);
    expect(parseChoices('{"all":"night","documents":{"a":"sepia","b":"blue"}}')).toEqual({ documents: { a: "sepia" } });
  });

  it("keep at most 200 documents, the oldest going first", () => {
    let choices = none;
    for (let i = 0; i < 205; i++) choices = withDocument(choices, `d${i}`, "dark");
    expect(Object.keys(choices.documents)).toHaveLength(200);
    expect(choices.documents.d0).toBeUndefined();
    expect(choices.documents.d204).toBe("dark");
  });
});

describe("the page appearance shown", () => {
  it("is this document's, else all documents', else High contrast under forced colours, else Original", () => {
    const choices = { all: "sepia" as const, documents: { a: "dark" as const } };
    expect(resolve(choices, "a", false)).toBe("dark");
    expect(resolve(choices, "b", true)).toBe("sepia");
    expect(resolve(none, "b", true)).toBe("high-contrast");
    expect(resolve(none, undefined, false)).toBe("original");
  });
});

describe("choosing a page appearance", () => {
  const local = { key: "a", kept: true };
  const linked = { key: "l", kept: false };

  it("keeps a local document's choice in storage", () => {
    expect(choose(none, none, local, "dark", "document")).toEqual({
      stored: { documents: { a: "dark" } },
      session: none,
    });
  });

  it("keeps a linked document's choice for this tab only", () => {
    expect(choose(none, none, linked, "dark", "document")).toEqual({
      stored: none,
      session: { documents: { l: "dark" } },
    });
  });

  it("for all documents drops this document's own choice, so it follows", () => {
    const stored = { documents: { a: "sepia" as const } };
    expect(choose(stored, none, local, "dark", "document").stored.documents.a).toBe("dark");
    expect(choose(stored, none, local, "dark", "all")).toEqual({
      stored: { all: "dark", documents: {} },
      session: none,
    });
    const session = { documents: { l: "sepia" as const } };
    expect(choose(none, session, linked, "dark", "all")).toEqual({
      stored: { all: "dark", documents: {} },
      session: none,
    });
  });

  it("with no document open is for all documents", () => {
    expect(choose(none, none, { key: undefined, kept: false }, "sepia", "document").stored.all).toBe("sepia");
  });
});

describe("a document's key", () => {
  it("is the same for the same name and size, and differs otherwise", () => {
    const key = documentKey({ name: "report.pdf", size: 1234 });
    expect(key).toMatch(/^[0-9a-f]{16}$/);
    expect(documentKey({ name: "report.pdf", size: 1234 })).toBe(key);
    expect(documentKey({ name: "report.pdf", size: 1235 })).not.toBe(key);
    expect(documentKey({ name: "other.pdf", size: 1234 })).not.toBe(key);
  });
});

describe("undoing a choice", () => {
  const local = { key: "a", kept: true };

  it("brings back only what that choice changed, never a later choice", () => {
    const start = { stored: none, session: none };
    const sepia = choose(start.stored, start.session, local, "sepia", "all");
    const dark = choose(sepia.stored, sepia.session, local, "dark", "document");
    // Undoing the first (Sepia for all) keeps this document's later Dark.
    expect(undo(dark, start, sepia, local)).toEqual({
      stored: { all: undefined, documents: { a: "dark" } },
      session: none,
    });
    // Undoing the second brings back Sepia for all.
    expect(undo(dark, sepia, dark, local)).toEqual({ stored: { all: "sepia", documents: {} }, session: none });
  });
});
