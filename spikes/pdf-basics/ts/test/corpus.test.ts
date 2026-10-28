// SPEC case 3: every corpus file opens and matches pypdf's facts.
import { describe, expect, test } from "vitest";
import { open, ref } from "../src/index.js";
import { corpusFile, corpusNames, expected, latin1 } from "./helpers.js";

const exp = expected();

describe("case 3: corpus", () => {
  test("expected.json covers every corpus file", () => {
    expect(Object.keys(exp).sort()).toEqual(corpusNames());
  });

  describe.each(corpusNames())("%s", (file) => {
    const e = exp[file]!;
    const bytes = corpusFile(file);
    const doc = open(bytes);

    test("size and header version", () => {
      expect(bytes.length).toBe(e.size);
      expect(doc.headerVersion()).toBe(e.header_version);
    });
    test("page_count", () => expect(doc.pageCount()).toBe(e.page_count));
    test("encrypted", () => expect(doc.isEncrypted()).toBe(e.encrypted));
    test("first_page_mediabox", () => expect(doc.pages()[0]?.mediaBox).toEqual(e.first_page_mediabox));
    test("root_keys", () => {
      expect([...doc.catalog().entries.keys()].map((k) => `/${k}`).sort()).toEqual(e.root_keys);
    });
    test("revisions (xref sections via /Prev) equals %%EOF count", () => {
      expect(doc.revisions()).toBe(e.revisions);
    });
    test("has_xref_stream", () => {
      const off = doc.startxrefOffset();
      const atXref = off !== undefined && latin1(bytes.subarray(off, off + 4)) === "xref";
      expect(off === undefined ? false : !atXref).toBe(e.has_xref_stream);
    });
  });

  test("hello-broken.pdf is repaired", () => {
    const doc = open(corpusFile("hello-broken.pdf"));
    expect(doc.wasRepaired()).toBe(true);
    expect(latin1(doc.pageContents(doc.pages()[0]!))).toContain("Hello, World");
  });

  test("intact files are not repaired", () => {
    for (const f of corpusNames()) if (f !== "hello-broken.pdf") expect(open(corpusFile(f)).wasRepaired(), f).toBe(false);
  });

  test("hello-incremental.pdf: 2 revisions, newest content wins", () => {
    const doc = open(corpusFile("hello-incremental.pdf"));
    expect(doc.revisions()).toBe(2);
    const text = latin1(doc.pageContents(doc.pages()[0]!));
    expect(text).toContain("Hello, Update");
    expect(text).not.toContain("Hello, World");
  });

  test("hello-xrefstm.pdf resolves objects from the object stream", () => {
    const doc = open(corpusFile("hello-xrefstm.pdf"));
    for (const n of [1, 2, 3, 4]) expect(doc.xrefEntry(n)).toEqual({ type: "compressed", stream: 6, index: n - 1 });
    const cat = doc.resolve(ref(1));
    expect(cat.kind === "dict" && cat.entries.get("Type")).toEqual({ kind: "name", name: "Catalog" });
    const font = doc.resolve(ref(4));
    expect(font.kind === "dict" && font.entries.get("BaseFont")).toEqual({ kind: "name", name: "Helvetica" });
    expect(latin1(doc.pageContents(doc.pages()[0]!))).toBe("BT /F1 24 Tf 72 720 Td (Hello, World) Tj ET");
  });

  test("xref streams with PNG predictors (pdflib.pdf)", () => {
    const doc = open(corpusFile("pdflib.pdf"));
    const t = doc.trailer();
    expect(t.entries.get("Type")).toEqual({ kind: "name", name: "XRef" });
    expect(doc.pageContents(doc.pages()[0]!).length).toBeGreaterThan(0);
  });
});
