// SPEC case 5: hostile input yields a PdfError or a (repaired) document — never another exception, never a hang.
import { describe, expect, test } from "vitest";
import { array, dict, flateEncode, int, name, open, PdfWriter, ref, stream } from "../src/index.js";
import { bytesOf, corpusFile, corpusNames, exercise, expectPdfError, onlyPdfErrors, prng } from "./helpers.js";

/** Open + touch everything; returns whether it ended in a PdfError. Also enforces a time budget. */
function attempt(bytes: Uint8Array, label: string, budgetMs = 2000): "ok" | "error" {
  const t0 = performance.now();
  const r = onlyPdfErrors(() => exercise(open(bytes)), label);
  const dt = performance.now() - t0;
  if (dt > budgetMs) throw new Error(`${label}: took ${dt.toFixed(0)} ms`);
  return r;
}

describe("case 5: robustness", () => {
  test.each(corpusNames())("truncation of %s at many offsets", (file) => {
    const full = corpusFile(file);
    const rnd = prng(0x5eed ^ full.length);
    const offsets = new Set<number>();
    const step = Math.max(1, Math.floor(full.length / 600));
    for (let i = 0; i <= full.length; i += step) offsets.add(i);
    for (let i = 0; i < 100; i++) offsets.add(Math.floor(rnd() * full.length));
    for (let i = Math.max(0, full.length - 64); i <= full.length; i++) offsets.add(i); // the tail is where xref lives
    let errors = 0;
    for (const n of offsets) if (attempt(full.subarray(0, n), `${file}[0..${n}]`) === "error") errors++;
    expect(errors).toBeGreaterThan(0); // the empty prefix at least must fail
  });

  test.each(corpusNames())("random byte flips in %s (seeded)", (file) => {
    const full = corpusFile(file);
    const rnd = prng(0xc0ffee + full.length);
    for (let iter = 0; iter < 150; iter++) {
      const m = full.slice();
      const flips = 1 + Math.floor(rnd() * 8);
      for (let k = 0; k < flips; k++) {
        const at = Math.floor(rnd() * m.length);
        m[at] = rnd() < 0.5 ? m[at]! ^ (1 << Math.floor(rnd() * 8)) : Math.floor(rnd() * 256);
      }
      attempt(m, `${file} mutant #${iter}`);
    }
  });

  test("random garbage buffers (seeded)", () => {
    const rnd = prng(42);
    const alphabet = bytesOf("0123456789 \n\r<>[]()/%obj endobj stream endstream xref trailer startxref R f n ");
    for (let iter = 0; iter < 300; iter++) {
      const len = Math.floor(rnd() * 400);
      const b = new Uint8Array(len);
      for (let i = 0; i < len; i++) b[i] = rnd() < 0.7 ? alphabet[Math.floor(rnd() * alphabet.length)]! : Math.floor(rnd() * 256);
      attempt(b, `garbage #${iter}`);
    }
  });

  test("empty input", () => expectPdfError(() => open(new Uint8Array(0)), "Xref"));
  test("%PDF- only", () => expectPdfError(() => open(bytesOf("%PDF-"))));
  test("header and trailer garbage", () => expectPdfError(() => open(bytesOf("%PDF-1.7\nstartxref\n99999\n%%EOF\n"))));

  test("self-referencing /Kids → Cycle", () => {
    const pdf = new PdfWriter();
    pdf.add(dict([["Type", name("Catalog")], ["Pages", ref(2)]]));
    pdf.add(dict([["Type", name("Pages")], ["Kids", array([ref(2)])], ["Count", int(1)]]));
    const doc = open(pdf.toBytes(dict([["Root", ref(1)]])));
    expectPdfError(() => doc.pageCount(), "Cycle");
  });

  test("indirect page-tree cycle (Pages → Pages → first Pages) → Cycle", () => {
    const pdf = new PdfWriter();
    pdf.add(dict([["Type", name("Catalog")], ["Pages", ref(2)]]));
    pdf.add(dict([["Type", name("Pages")], ["Kids", array([ref(3)])]]));
    pdf.add(dict([["Type", name("Pages")], ["Kids", array([ref(2)])]]));
    expectPdfError(() => open(pdf.toBytes(dict([["Root", ref(1)]]))).pages(), "Cycle");
  });

  test("ref cycle `1 0 obj 2 0 R endobj 2 0 obj 1 0 R endobj`", () => {
    const body = "1 0 obj 2 0 R endobj 2 0 obj 1 0 R endobj";
    // Bare: no trailer, no catalog → open fails cleanly.
    expectPdfError(() => open(bytesOf(body)), "Xref");
    // With a trailer pointing into the cycle: resolve detects it.
    const withTrailer = bytesOf(`%PDF-1.7\n${body}\n3 0 obj << /Type /Catalog /Pages 1 0 R >> endobj\ntrailer << /Root 3 0 R >>\n`);
    const doc = open(withTrailer);
    expect(doc.wasRepaired()).toBe(true);
    expectPdfError(() => doc.resolve(ref(1)), "Cycle");
    expectPdfError(() => doc.pages(), "Cycle");
  });

  test("/Length referencing its own stream object", () => {
    const b = bytesOf("%PDF-1.7\n1 0 obj << /Type /Catalog /Length 1 0 R >>\nstream\nabc\nendstream\nendobj\ntrailer << /Root 1 0 R >>\n");
    attempt(b, "self length");
  });

  test("nesting depth 10 000 → DepthLimit, no stack overflow", () => {
    const deep = `${"[".repeat(10_000)}${"]".repeat(10_000)}`;
    const deepDict = `${"<< /A ".repeat(10_000)}1${" >>".repeat(10_000)}`;
    for (const d of [deep, deepDict]) {
      const b = bytesOf(`%PDF-1.7\n1 0 obj << /Type /Catalog /Pages 2 0 R /X ${d} >> endobj\ntrailer << /Root 1 0 R >>\n`);
      expect(attempt(b, "deep")).toBe("error");
    }
  });

  test("Flate bomb over the limit → LimitExceeded (default 64 MiB and configured)", () => {
    const bomb = flateEncode(new Uint8Array(65 * 1024 * 1024), 9);
    expect(bomb.length).toBeLessThan(200_000);
    const pdf = new PdfWriter();
    pdf.add(dict([["Type", name("Catalog")], ["Pages", ref(2)]]));
    pdf.add(dict([["Type", name("Pages")], ["Kids", array([ref(3)])], ["Count", int(1)]]));
    pdf.add(dict([["Type", name("Page")], ["Parent", ref(2)], ["Contents", ref(4)]]));
    pdf.add(stream(dict([["Filter", name("FlateDecode")]]), bomb));
    const bytes = pdf.toBytes(dict([["Root", ref(1)]]));
    const doc = open(bytes);
    expectPdfError(() => doc.pageContents(doc.pages()[0]!), "LimitExceeded");
    const small = open(bytes, { maxDecodedStreamSize: 1024 });
    expectPdfError(() => small.pageContents(small.pages()[0]!), "LimitExceeded");
  });

  test("Prev loop in xref chain terminates", () => {
    const pdf = new PdfWriter();
    pdf.add(dict([["Type", name("Catalog")], ["Pages", ref(2)]]));
    pdf.add(dict([["Type", name("Pages")], ["Kids", array([])], ["Count", int(0)]]));
    const s = Buffer.from(pdf.toBytes(dict([["Root", ref(1)]]))).toString("latin1");
    const off = /startxref\n(\d+)/.exec(s)![1]!;
    const looped = s.replace("/Root 1 0 R >>", `/Root 1 0 R /Prev ${off} >>`);
    const doc = open(bytesOf(looped));
    expect(doc.revisions()).toBe(1);
    expect(doc.pageCount()).toBe(0);
  });
});
