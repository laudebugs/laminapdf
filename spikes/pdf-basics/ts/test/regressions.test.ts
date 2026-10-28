// Regression tests for code-review findings (each failed before its fix).
import { describe, expect, test } from "vitest";
import { dict, flateDecode, flateEncode, name, open, PdfWriter, raw, ref, unpredict, type WriterObject, writePdf } from "../src/index.js";
import { expectPdfError, latin1 } from "./helpers.js";
import { HandPdf, xrefStreamPredictorPdf } from "./recipes.js";

const pad10 = (n: number): string => String(n).padStart(10, "0");
const rowBytes = (...rows: number[][]): string => String.fromCharCode(...rows.flat());

describe("xref loading", () => {
  test("hybrid /XRefStm fills entries the classic table marks free", () => {
    const p = new HandPdf("%PDF-1.5\n");
    p.obj(1, "<< /Type /Catalog /Pages 2 0 R >>");
    const members = "<< /Type /Pages /Kids [3 0 R] /Count 1 >> << /Type /Page /Parent 2 0 R >>";
    const header = "2 0 3 42 ";
    expect(members.indexOf("<< /Type /Page ")).toBe(42);
    p.streamObj(5, `/Type /ObjStm /N 2 /First ${header.length}`, header + members);
    const stmAt = p.length;
    const o1 = p.offsets.get(1)!;
    const o5 = p.offsets.get(5)!;
    // Type-2 rows for 2 and 3 (in object stream 5), type-1 rows for 1, 5 and the stream itself.
    const rows = rowBytes([0, 0, 0, 255], [1, o1 >> 8, o1 & 0xff, 0], [2, 0, 5, 0], [2, 0, 5, 1], [0, 0, 0, 0], [1, o5 >> 8, o5 & 0xff, 0], [1, stmAt >> 8, stmAt & 0xff, 0]);
    p.streamObj(6, "/Type /XRef /Size 7 /W [1 2 1]", rows);
    const table = p.length;
    // The classic table hides the compressed objects from pre-1.5 readers: 2 and 3 are `f`.
    p.raw("xref\n0 7\n0000000000 65535 f\r\n");
    p.raw(`${pad10(o1)} 00000 n\r\n0000000000 00001 f\r\n0000000000 00001 f\r\n0000000000 00001 f\r\n`);
    p.raw(`${pad10(o5)} 00000 n\r\n${pad10(stmAt)} 00000 n\r\n`);
    p.raw(`trailer\n<< /Size 7 /Root 1 0 R /XRefStm ${stmAt} >>\nstartxref\n${table}\n%%EOF\n`);
    const doc = open(p.bytes());
    expect(doc.wasRepaired()).toBe(false);
    expect(doc.xrefEntry(2)).toEqual({ type: "compressed", stream: 5, index: 0 });
    expect(doc.xrefEntry(1)?.type).toBe("offset"); // in-use table entry kept
    expect(doc.pageCount()).toBe(1);
  });

  test("duplicate number within one classic section: the last entry wins", () => {
    const p = new HandPdf();
    p.obj(1, "<< /Type /Catalog /Pages 2 0 R >>");
    p.obj(2, "<< /Type /Pages /Kids [] /Count 0 >>");
    const x = p.length;
    p.raw(`xref\n0 3\n0000000000 65535 f\r\n0000000000 00000 f\r\n${pad10(p.offsets.get(2)!)} 00000 n\r\n`);
    p.raw(`1 1\n${pad10(p.offsets.get(1)!)} 00000 n\r\n`);
    p.raw(`trailer\n<< /Size 3 /Root 1 0 R >>\nstartxref\n${x}\n%%EOF\n`);
    const doc = open(p.bytes());
    expect(doc.wasRepaired()).toBe(false);
    expect(doc.xrefEntry(1)?.type).toBe("offset");
  });

  test("duplicate number within one xref stream: the last row wins", () => {
    const p = new HandPdf("%PDF-1.5\n");
    const o1 = p.obj(1, "<< /Type /Catalog /Pages 2 0 R >>");
    const o2 = p.obj(2, "<< /Type /Pages /Kids [] /Count 0 >>");
    const x = p.length;
    const rows = rowBytes([0, 0, 0, 255], [0, 0, 0, 0], [1, 0, o2, 0], [1, x >> 8, x & 0xff, 0], [1, 0, o1, 0]);
    p.streamObj(3, "/Type /XRef /Size 4 /W [1 2 1] /Index [0 4 1 1] /Root 1 0 R", rows);
    p.raw(`startxref\n${x}\n%%EOF\n`);
    const doc = open(p.bytes());
    expect(doc.wasRepaired()).toBe(false);
    expect(doc.xrefEntry(1)).toEqual({ type: "offset", offset: o1, gen: 0 });
  });

  test("unknown xref-stream row type is free and shadows older revisions", () => {
    const p = new HandPdf("%PDF-1.5\n");
    p.obj(1, "<< /Type /Catalog /Pages 2 0 R >>");
    p.obj(2, "<< /Type /Pages /Kids [] /Count 0 >>");
    p.obj(3, "<< /Marker true >>");
    const x1 = p.length;
    p.raw(`xref\n0 4\n0000000000 65535 f\r\n`);
    for (const n of [1, 2, 3]) p.raw(`${pad10(p.offsets.get(n)!)} 00000 n\r\n`);
    p.raw(`trailer\n<< /Size 4 /Root 1 0 R >>\nstartxref\n${x1}\n%%EOF\n`);
    const x2 = p.length;
    p.streamObj(4, `/Type /XRef /Size 5 /W [1 2 1] /Index [3 2] /Root 1 0 R /Prev ${x1}`, rowBytes([7, 0, 0, 0], [1, x2 >> 8, x2 & 0xff, 0]));
    p.raw(`startxref\n${x2}\n%%EOF\n`);
    const doc = open(p.bytes());
    expect(doc.wasRepaired()).toBe(false);
    expect(doc.revisions()).toBe(2);
    expect(doc.xrefEntry(3)).toEqual({ type: "free" });
    expect(doc.getObject(3).kind).toBe("null");
  });
});

describe("repair", () => {
  test("newest trailer candidate by file offset wins (classic trailer after an xref stream)", () => {
    const p = new HandPdf("%PDF-1.5\n");
    p.obj(1, "<< /Type /Catalog /Pages 2 0 R >>");
    p.obj(2, "<< /Type /Pages /Kids [] /Count 0 >>");
    const x1 = p.length;
    p.streamObj(3, "/Type /XRef /Size 4 /W [1 2 1] /Root 1 0 R", rowBytes([0, 0, 0, 255]));
    p.raw(`startxref\n${x1}\n%%EOF\n`);
    // Incremental update with a classic section: a new catalog with one page.
    p.obj(4, "<< /Type /Catalog /Pages 5 0 R >>");
    p.obj(5, "<< /Type /Pages /Kids [6 0 R] /Count 1 >>");
    p.obj(6, "<< /Type /Page /Parent 5 0 R >>");
    p.raw(`xref\n0 1\n0000000000 65535 f\r\n4 3\n`);
    p.raw(`trailer\n<< /Size 7 /Root 4 0 R /Prev ${x1} >>\nstartxref\n999999\n%%EOF\n`); // damaged: bad startxref
    const doc = open(p.bytes());
    expect(doc.wasRepaired()).toBe(true);
    expect(doc.trailer().entries.get("Root")).toEqual(ref(4));
    expect(doc.pageCount()).toBe(1);
  });
});

describe("page tree", () => {
  const withCatalog = (catalog: string, extra?: (w: PdfWriter) => void): Uint8Array => {
    const w = new PdfWriter();
    w.add(raw(catalog));
    extra?.(w);
    return w.toBytes(dict([["Root", ref(1)]]));
  };
  test("catalog without /Pages → 0 pages", () => {
    expect(open(withCatalog("<< /Type /Catalog >>")).pageCount()).toBe(0);
  });
  test("/Pages root that does not load → 0 pages", () => {
    const doc = open(withCatalog("<< /Type /Catalog /Pages 2 0 R >>", (w) => w.add(raw("<< /Type /Pages /Kids [3 0 R >>"))));
    expect(doc.wasRepaired()).toBe(false);
    expect(doc.pageCount()).toBe(0);
  });
  test("/Pages root that is not a dict → 0 pages", () => {
    expect(open(withCatalog("<< /Type /Catalog /Pages 42 >>")).pageCount()).toBe(0);
  });
  test("a non-dict /Root with no catalog to adopt stays an error", () => {
    const w = new PdfWriter();
    w.add(raw("[1 2 3]"));
    w.add(raw("<< /Type /Catalog /Pages 3 0 R >>"));
    const doc = open(w.toBytes(dict([["Root", ref(1)]])));
    // Repair adopts the /Type /Catalog object; with no catalog at all, it fails.
    expect(doc.pageCount()).toBe(0);
    const w2 = new PdfWriter();
    w2.add(raw("[1 2 3]"));
    expectPdfError(() => open(w2.toBytes(dict([["Root", ref(1)]]))).pageCount());
  });
});

describe("parser / lexer", () => {
  test("spaces/tabs after `stream` before the EOL are skipped (`stream \\r\\n`)", () => {
    const w = new PdfWriter();
    w.add(raw("<< /Type /Catalog /Pages 2 0 R >>"));
    w.add(raw("<< /Type /Pages /Kids [] /Count 0 >>"));
    w.add(raw("<< /Length 3 >>\nstream \t\r\nabc\nendstream"));
    w.add(raw("<< /Length 3 >>\nstream \rxyz\nendstream"));
    const doc = open(w.toBytes(dict([["Root", ref(1)]])));
    const s3 = doc.getObject(3);
    const s4 = doc.getObject(4);
    expect(s3.kind === "stream" && latin1(s3.data)).toBe("abc");
    expect(s4.kind === "stream" && latin1(s4.data)).toBe("xyz");
  });
});

describe("filters", () => {
  test("PNG predictor with an enormous row is rejected with a PdfError (no huge allocation)", () => {
    const e = expectPdfError(() => unpredict(new Uint8Array([2, 1, 2, 3]), { predictor: 12, colors: 32, bitsPerComponent: 16, columns: 16777215 }), "Filter");
    expect(e.message).toMatch(/row length/);
  });
  test("PNG predictor: short final row is decoded as far as it goes", () => {
    expect([...unpredict(Uint8Array.from([0, 1, 2, 3, 2, 1]), { predictor: 12, colors: 1, bitsPerComponent: 8, columns: 3 })]).toEqual([1, 2, 3, 2]);
  });
  test("truncated deflate returns the partial output; nothing decodable still throws", () => {
    const src = new Uint8Array(200_000).map((_, i) => ((i * 7919) >> 3) ^ i % 13);
    const z = flateEncode(src);
    const part = flateDecode(z.subarray(0, z.length >> 1));
    expect(part.length).toBeGreaterThan(0);
    expect(part.length).toBeLessThan(src.length);
    expect(part).toEqual(src.subarray(0, part.length));
    expectPdfError(() => flateDecode(z.subarray(0, 4)), "Filter");
  });
});

describe("writer", () => {
  test("compact mode strips xref-stream keys from a trailer copied off an xref-stream file", () => {
    const doc = open(xrefStreamPredictorPdf());
    expect(doc.wasRepaired()).toBe(false);
    expect(doc.trailer().entries.has("DecodeParms")).toBe(true);
    const objs: WriterObject[] = doc
      .objectNumbers()
      .map((num) => ({ num, body: doc.getObject(num) }))
      .filter((o) => !(o.body.kind === "stream" && o.body.dict.entries.get("Type")?.kind === "name"));
    const out = writePdf(objs, doc.trailer(), { mode: "compact" });
    const again = open(out);
    expect(again.wasRepaired()).toBe(false);
    expect(again.pageCount()).toBe(1);
    // The new xref stream's dict is the trailer: no predictor parameters copied from the old one.
    expect(again.trailer().entries.has("DecodeParms")).toBe(false);
    expect(again.trailer().entries.get("Filter")).toEqual(name("FlateDecode"));
  });

  test("compact mode with many objects is not quadratic", () => {
    // Compare scaling rather than wall-clock time so the test is machine-independent:
    // 4x the objects should cost ~4x (linear), not ~16x (quadratic).
    const build = (n: number): { out: Uint8Array; ms: number } => {
      const w = new PdfWriter();
      w.add(dict([["Type", name("Catalog")]]));
      for (let i = 0; i < n; i++) w.add(dict([["I", { kind: "int", value: i }]]));
      let best = Number.POSITIVE_INFINITY;
      let out: Uint8Array = new Uint8Array();
      for (let run = 0; run < 3; run++) {
        const t0 = performance.now();
        out = w.toBytes(dict([["Root", ref(1)]]), { mode: "compact" });
        best = Math.min(best, performance.now() - t0);
      }
      return { out, ms: best };
    };
    const small = build(20_000);
    const large = build(80_000);
    expect(open(large.out).getObject(80_001)).toEqual(dict([["I", { kind: "int", value: 79_999 }]]));
    expect(large.ms / Math.max(small.ms, 1)).toBeLessThan(9);
  });
});
