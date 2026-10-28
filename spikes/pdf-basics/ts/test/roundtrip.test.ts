// SPEC case 4: write → read → re-serialize → identical.
import { describe, expect, test } from "vitest";
import {
  array,
  bool,
  dict,
  hexStr,
  int,
  MAX_DEPTH,
  NULL,
  name,
  open,
  type PdfObject,
  PdfWriter,
  parseObject,
  real,
  ref,
  serialize,
  str,
  stream,
  type WriteOptions,
  writeIncremental,
} from "../src/index.js";
import { bytesOf, expectPdfError, latin1 } from "./helpers.js";

const allBytes = new Uint8Array(256).map((_, i) => i);

/** One of everything, plus escaping edge cases. Numbers 1..n. */
function sampleObjects(): PdfObject[] {
  return [
    NULL,
    bool(true),
    bool(false),
    int(0),
    int(-17),
    int(Number.MAX_SAFE_INTEGER),
    real(0.5),
    real(-0.25),
    real(123456.789),
    real(-1e-9), // formats as 0
    name("Type"),
    name(""),
    name("A B#C/D%E(F)G<H>I[J]K{L}M"),
    name("\x00\x7f\x80\xff~!"),
    str(""),
    str("(unbalanced ((( and ))) \\ backslash"),
    str("line\rfeed\ncarriage\ttab"),
    str(allBytes),
    hexStr(allBytes),
    hexStr(""),
    array([]),
    array([int(1), array([real(2.5), array([name("x")])]), ref(3, 7)]),
    dict(),
    dict([
      ["Z", int(1)],
      ["A", dict([["nested", array([bool(false), NULL])]])],
      ["1", str("integer-like key keeps order")],
      ["sp ace", hexStr("\x01\x02")],
    ]),
    ref(1, 0),
    ref(99999, 65535),
    stream(dict([["Type", name("XObject")]]), allBytes),
    stream(dict(), bytesOf("contains endstream and endobj keywords\nendstream\n")),
    stream(dict([["Length", int(999)], ["After", bool(true)]]), bytesOf("length kept in place")),
  ];
}

function nest(depth: number): PdfObject {
  let o: PdfObject = int(1);
  for (let i = 0; i < depth; i++) o = i % 2 === 0 ? array([o]) : dict([["k", o]]);
  return o;
}

function build(objs: PdfObject[], opts: WriteOptions): Uint8Array {
  const w = new PdfWriter();
  const cat = w.add(dict([["Type", name("Catalog")], ["Pages", ref(objs.length + 2)]]));
  for (const o of objs) w.add(o);
  w.add(dict([["Type", name("Pages")], ["Kids", array([])], ["Count", int(0)]]));
  return w.toBytes(dict([["Root", cat]]), opts);
}

const modes: Array<[string, WriteOptions]> = [
  ["classic CrLf", { xrefEol: "CrLf" }],
  ["classic SpaceLf", { xrefEol: "SpaceLf" }],
  ["compact (ObjStm + XRef stream)", { mode: "compact" }],
];

describe("case 4: round-trip", () => {
  test.each(modes)("all object types — %s", (_, opts) => {
    const objs = sampleObjects();
    const doc = open(build(objs, opts));
    expect(doc.wasRepaired()).toBe(false);
    objs.forEach((o, i) => {
      expect(latin1(serialize(doc.getObject(i + 2))), `object ${i + 2}`).toBe(latin1(serialize(o)));
    });
    expect(doc.pageCount()).toBe(0);
  });

  test("compact mode packs non-stream objects into an object stream", () => {
    const doc = open(build(sampleObjects(), { mode: "compact" }));
    expect(doc.xrefEntry(2)?.type).toBe("compressed");
    expect(doc.trailer().entries.get("Type")).toEqual(name("XRef"));
    expect(doc.headerVersion()).toBe("1.7");
    expect(open(build([], { mode: "compact", version: "1.4" })).headerVersion()).toBe("1.5");
  });

  test("streams can be Flate-encoded by the writer", () => {
    const data = bytesOf("q 1 0 0 1 0 0 cm Q\n".repeat(50));
    const w = new PdfWriter();
    w.add(stream(dict(), data));
    const cat = w.add(dict([["Type", name("Catalog")]]));
    const doc = open(w.toBytes(dict([["Root", cat]]), { compressStreams: true }));
    const got = doc.getObject(1);
    expect(got.kind === "stream" && latin1(serialize(got.dict))).toMatch(/^<< \/Filter \/FlateDecode \/Length \d+ >>$/);
    expect(got.kind === "stream" && latin1(doc.decodeStream(got))).toBe(latin1(data));
  });

  test("literal/hex strings, names and numbers re-parse to the same object", () => {
    for (const o of sampleObjects()) {
      if (o.kind === "stream") continue;
      expect(latin1(serialize(parseObject(serialize(o))))).toBe(latin1(serialize(o)));
    }
  });

  test(`nesting at the limit (${MAX_DEPTH}) round-trips; one deeper is rejected`, () => {
    const atLimit = nest(MAX_DEPTH);
    const doc = open(build([atLimit], {}));
    expect(latin1(serialize(doc.getObject(2)))).toBe(latin1(serialize(atLimit)));
    expectPdfError(() => parseObject(serialize(nest(MAX_DEPTH + 1))), "DepthLimit");
    // Compact mode too (the object is parsed out of the object stream).
    const c = open(build([atLimit], { mode: "compact" }));
    expect(latin1(serialize(c.getObject(2)))).toBe(latin1(serialize(atLimit)));
  });

  describe("incremental updates", () => {
    const base = (opts: WriteOptions) => build([str("v1"), int(10)], opts);

    test.each(modes)("2 and 3 revisions, newest wins — base %s", (_, opts) => {
      const r1 = base(opts);
      const r2 = writeIncremental(r1, [{ num: 2, body: str("v2") }, { num: 10, body: name("added") }]);
      expect(latin1(r2.subarray(0, r1.length))).toBe(latin1(r1)); // original untouched
      const r3 = writeIncremental(r2, [{ num: 2, body: str("v3") }, { num: 3, body: int(30) }], { xrefEol: "SpaceLf" });
      const d2 = open(r2);
      const d3 = open(r3);
      expect([d2.revisions(), d3.revisions()]).toEqual([2, 3]);
      expect(d2.wasRepaired() || d3.wasRepaired()).toBe(false);
      expect(latin1(serialize(d2.getObject(2)))).toBe("(v2)");
      expect(latin1(serialize(d3.getObject(2)))).toBe("(v3)");
      expect(latin1(serialize(d3.getObject(3)))).toBe("30");
      expect(latin1(serialize(d3.getObject(10)))).toBe("/added");
      expect(d3.trailer().entries.get("Size")).toEqual(int(11));
      expect(d3.trailer().entries.get("Prev")).toEqual(int(d2.startxrefOffset()!));
      expect(d3.catalog().entries.get("Type")).toEqual(name("Catalog"));
    });

    test("trailer keys are repeated, /Prev is not duplicated, extra keys merge", () => {
      const r1 = base({});
      const r2 = writeIncremental(r1, [{ num: 9, body: dict([["Title", str("t")]]) }], {
        trailer: dict([["Info", ref(9)]]),
      });
      const r3 = writeIncremental(r2, [{ num: 2, body: str("x") }]);
      const t = latin1(r3).split("trailer\n").at(-1)!;
      expect(t.startsWith(`<< /Size 10 /Root 1 0 R /Info 9 0 R /Prev ${open(r2).startxrefOffset()} >>`)).toBe(true);
    });

    test("refuses to append to a document that needed repair", () => {
      const broken = latin1(base({})).replace(/startxref\n\d+/, "startxref\n3");
      expectPdfError(() => writeIncremental(bytesOf(broken), [{ num: 2, body: NULL }]), "Xref");
    });
  });
});
