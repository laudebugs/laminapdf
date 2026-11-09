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
