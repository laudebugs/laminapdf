// The inspector's reads: conformance(), fonts(), colorProfiles(), objects(), object(),
// streamBytes(), referencesTo(), objectSyntax() and revisions(), on committed fixtures and a
// decompression bomb built here. A test's title starts with the capability it proves (its tag:
// capability J.09, column pdf), which `cargo xtask capabilities` counts.
import { deflateSync } from "node:zlib";
import { Document, init, isLaminaPdfError, loadExtract } from "@laminapdf/sdk";
import { beforeAll, describe, expect, it } from "vitest";
import { read, thrown } from "./helpers.js";

beforeAll(async () => {
  await init();
  await loadExtract();
});

/** A one-page file whose content stream (object 4) inflates to `size` zero bytes. */
function bomb(size: number): Uint8Array {
  const data = deflateSync(new Uint8Array(size));
  const head = new TextEncoder().encode(
    "%PDF-1.7\n1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n" +
      "2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj\n" +
      "3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Contents 4 0 R >> endobj\n" +
      `4 0 obj << /Length ${data.length} /Filter /FlateDecode >>\nstream\n`,
  );
  const tail = new TextEncoder().encode("\nendstream\nendobj\ntrailer << /Root 1 0 R >>\n%%EOF\n");
  const out = new Uint8Array(head.length + data.length + tail.length);
  out.set(head);
  out.set(data, head.length);
  out.set(tail, head.length + data.length);
  return out;
}

describe("inspection", () => {
  it("[cap J.07 pdf read] reads the PDF/A claim and the header version, claimed and not verified", () => {
    using doc = Document.open(read("fixtures/golden/examples/pdf_a_archive-2b.pdf"));
    const claims = doc.conformance();
    expect(claims.pdfa).toEqual({ part: 2, conformance: "B" });
    expect(claims.headerVersion).toBe("1.7");
    expect(claims.pdfua).toBeUndefined();
  });

  it("[cap J.08 pdf read] lists each font with its ToUnicode coverage, and the output intent's ICC profile", () => {
    using doc = Document.open(read("fixtures/golden/examples/multilingual.pdf"));
    const fonts = doc.fonts();
    expect(fonts.length).toBeGreaterThan(0);
    for (const font of fonts) expect([font.embedded, font.subset, font.toUnicode]).toEqual([true, true, true]);
    const covered = fonts.flatMap((f) => (f.toUnicodeCoverage === undefined ? [] : [f.toUnicodeCoverage]));
    expect(covered.length).toBeGreaterThan(0);
    for (const coverage of covered) expect(coverage.mapped).toBe(coverage.codes);
    expect(doc.fonts([0]).length).toBeLessThanOrEqual(fonts.length);
    using archive = Document.open(read("fixtures/golden/examples/pdf_a_archive-2b.pdf"));
    const colour = archive.colorProfiles();
    const [intent] = colour.outputIntents;
    expect(intent?.subtype).toBe("GTS_PDFA1");
    const profile = colour.profiles.find((p) => p.object === intent?.profile);
    expect([profile?.colourSpace, profile?.outputIntent]).toEqual(["RGB", true]);
  });

  it("[cap J.09 pdf read] lists the objects, reads one with what refers to it, and its revisions", () => {
    using doc = Document.open(read("fixtures/corpus/hello-incremental.pdf"));
    const objects = doc.objects();
    const catalog = objects.find((o) => o.typeName === "Catalog");
    expect(catalog?.kind).toBe("dictionary");
    const pages = objects.find((o) => o.typeName === "Pages");
    if (pages === undefined) throw new Error("no page tree");
    const object = doc.object(pages.num);
    expect(object?.value.type).toBe("dictionary");
    expect(doc.referencesTo(pages.num).some((r) => r.from === catalog?.num && r.path === "/Pages")).toBe(true);
    expect(doc.objectSyntax(pages.num)).toMatch(/^\d+ 0 obj\n<</);
    const revisions = doc.revisions();
    expect(revisions).toHaveLength(2);
    expect(revisions[1]?.changed.length).toBeGreaterThan(0);
    expect(doc.object(100_000)).toBeUndefined();
    const trailer = doc.trailer();
    expect(trailer.type === "dictionary" && trailer.entries.some(([key]) => key === "Root")).toBe(true);
  });

  it("[cap J.09 pdf read] refuses to decode a stream past the limit, and still reads it raw", () => {
    using doc = Document.open(bomb(64 * 1024 * 1024 + 1));
    expect(
      isLaminaPdfError(
        thrown(() => doc.streamBytes(4, "decoded", 1024)),
        "limit-exceeded",
      ),
    ).toBe(true);
    const raw = doc.streamBytes(4, "raw", 16);
    expect(raw?.bytes).toHaveLength(16);
    expect(doc.streamBytes(1, "raw", 16)).toBeUndefined();
  });

  it("reads the first bytes of a decoded stream and its whole length", () => {
    using doc = Document.open(bomb(1000));
    const start = doc.streamBytes(4, "decoded", 10);
    expect([start?.bytes.length, start?.total]).toEqual([10, 1000]);
  });
});
