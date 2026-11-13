// doc.outline(), doc.namedDestination() and doc.pageLinks(): the links-and-outlines golden's outline against
// pypdf's facts (fixtures/build/facts.json, as crates/laminapdf/tests/navigation.rs reads them),
// views in page space, and the base module's refusal.
import { Document, init, isLaminaPdfError, loadExtract, type OutlineNode, type PageLink } from "@laminapdf/sdk";
import { beforeAll, describe, expect, it } from "vitest";
import { read, readText, thrown } from "./helpers.js";

const FILE = "fixtures/golden/links-and-outlines.pdf";

beforeAll(() => init());

describe("outline in the base module", () => {
  it("throws extract-module-not-loaded", () => {
    const doc = Document.open(read(FILE));
    expect(
      isLaminaPdfError(
        thrown(() => doc.outline()),
        "extract-module-not-loaded",
      ),
    ).toBe(true);
    doc.free();
  });
});

describe("outline and named destinations", () => {
  beforeAll(() => loadExtract());

  /** (title, depth, page) per item in document order, as pypdf lists them. */
  function flatten(nodes: OutlineNode[], depth = 0): [string, number, number | null][] {
    return nodes.flatMap((n) => [
      [n.title, depth, n.target?.page ?? null] as [string, number, number | null],
      ...flatten(n.children, depth + 1),
    ]);
  }

  const facts = JSON.parse(readText("fixtures/build/facts.json")).files["links-and-outlines.pdf"].pypdf as {
    outline: { title: string; depth: number; page: number | null }[];
    destinations: Record<string, number>;
  };

  it("matches pypdf's outline", () => {
    using doc = Document.open(read(FILE));
    expect(flatten(doc.outline())).toEqual(facts.outline.map((i) => [i.title, i.depth, i.page]));
  });

  it("matches pypdf's named destinations", () => {
    using doc = Document.open(read(FILE));
    const pages = Object.fromEntries(Object.keys(facts.destinations).map((n) => [n, doc.namedDestination(n)?.page]));
    expect(pages).toEqual(facts.destinations);
  });

  it("gives views in page space", () => {
    using doc = Document.open(read(FILE));
    expect([doc.namedDestination("contents")?.view, doc.namedDestination("appendix")?.view]).toEqual([
      { kind: "fit" },
      { kind: "fit-width", y: 72 },
    ]);
  });

  it("is undefined for an unknown name", () => {
    using doc = Document.open(read(FILE));
    expect(doc.namedDestination("nowhere")).toBeUndefined();
  });
});

describe("page links", () => {
  beforeAll(() => loadExtract());

  const facts = JSON.parse(readText("fixtures/links/expected.json")).files["links.pdf"] as Record<string, unknown>[][];

  /** A link as fixtures/links/expected.json describes it. */
  function fact(link: PageLink): Record<string, unknown> {
    switch (link.kind) {
      case "dest":
      case "named":
        return { page: link.target?.page ?? null };
      case "uri":
        return { uri: link.uri };
      case "remote":
        return { file: link.file ?? null };
      case "named-action":
        return { named: link.name };
      case "unsupported":
        return { action: link.action };
    }
  }

  it("matches pypdf's links on every page", () => {
    using doc = Document.open(read("fixtures/links/links.pdf"));
    const expected = facts.map((page) =>
      page.filter((l) => l.hidden === false && l.rect !== null).map(({ hidden: _h, rect: _r, ...rest }) => rest),
    );
    expect(Array.from({ length: doc.pageCount }, (_, i) => doc.pageLinks(i).map(fact))).toEqual(expected);
  });

  it("gives rectangles in page space", () => {
    using doc = Document.open(read("fixtures/links/links.pdf"));
    expect(doc.pageLinks(0)[0]?.rect).toEqual([72, 38, 400, 56]);
  });
});

describe("properties", () => {
  beforeAll(() => loadExtract());

  it("matches pypdf's facts for the corpus's outline and encrypted files", () => {
    const facts = JSON.parse(readText("fixtures/corpus/expected.json")) as Record<
      string,
      {
        properties: {
          title: string | null;
          producer: string | null;
          encryption_revision: number | null;
          fonts: string[];
        };
      }
    >;
    for (const name of ["outline.pdf", "enc.pdf"]) {
      using doc = Document.open(read(`fixtures/corpus/${name}`));
      const p = doc.properties();
      const fonts = [...new Set(p.fonts.flatMap((f) => (f.name === undefined ? [] : [f.name])))].sort();
      expect({
        title: p.title ?? null,
        producer: p.producer ?? null,
        encryption_revision: p.encryption?.revision ?? null,
        fonts,
      }).toEqual({
        title: facts[name]?.properties.title,
        producer: facts[name]?.properties.producer,
        encryption_revision: facts[name]?.properties.encryption_revision,
        fonts: facts[name]?.properties.fonts,
      });
    }
  });
});

describe("editing bookmarks and named destinations", () => {
  beforeAll(() => loadExtract());

  const OUTLINE = "fixtures/corpus/outline.pdf";

  it("lists each bookmark's ID and the name it goes to", () => {
    using doc = Document.open(read(OUTLINE));
    expect(doc.outline().map((n) => [n.id, n.destinationName ?? null])).toEqual([
      ["o11.0", null],
      ["o12.0", "chapter2"],
      ["o13.0", "appendix"],
    ]);
  });

  it("lists the named destinations of both stores, sorted", () => {
    using doc = Document.open(read(OUTLINE));
    expect(doc.namedDestinations().map((d) => [d.name, d.target?.page])).toEqual([
      ["appendix", 2],
      ["chapter2", 1],
    ]);
  });

  it("adds a bookmark at a place in page space and reads it back there", () => {
    using doc = Document.open(read(OUTLINE));
    const destination = doc.destinationAt(1, { kind: "at", x: 36, y: 72 });
    const edited = doc.apply([
      {
        op: "addOutlineItem",
        args: { at: "last", title: "Here", target: { destination }, style: { color: "#c00000" } },
      },
    ]);
    const [id] = edited.createdOutlineItems;
    const added = doc.outline().find((n) => n.id === id);
    expect([added?.title, added?.color, added?.target]).toEqual([
      "Here",
      "#c00000",
      { page: 1, view: { kind: "at", x: 36, y: 72 } },
    ]);
  });

  it("refuses a web address that isn't http, https or mailto", () => {
    using doc = Document.open(read(OUTLINE));
    const err = thrown(() =>
      doc.apply([{ op: "addOutlineItem", args: { at: "last", title: "x", target: { uri: "javascript:alert(1)" } } }]),
    );
    expect(isLaminaPdfError(err, "link-uri-refused")).toBe(true);
  });

  it("reports what still names a deleted destination", () => {
    using doc = Document.open(read(OUTLINE));
    const edited = doc.apply([{ op: "deleteNamedDestination", args: { name: "appendix" } }]);
    expect(edited.destinationUses).toEqual([{ kind: "outlineItem", item: "o13.0" }]);
  });
});
