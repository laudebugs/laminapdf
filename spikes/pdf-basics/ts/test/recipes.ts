/** Rebuild the golden fixtures with the library (shared by tests and scripts). */
import {
  array,
  bool,
  ContentBuilder,
  dict,
  fixed,
  flateEncode,
  int,
  name,
  num,
  PdfWriter,
  raw,
  ref,
  str,
  stream,
} from "../src/index.js";

const N = name;
const nums = (...v: number[]) => array(v.map(num));

/** docs/research/pdf/examples/01-file-structure/hello.py */
export function helloContent(text = "Hello, World"): Uint8Array {
  return new ContentBuilder().BT().Tf("F1", 24).Td(72, 720).Tj(text).ET().toBytes();
}

export function helloPdf(): Uint8Array {
  const w = new PdfWriter();
  w.add(dict([["Type", N("Catalog")], ["Pages", ref(2)]]));
  w.add(dict([["Type", N("Pages")], ["Kids", array([ref(3)])], ["Count", int(1)]]));
  // The fixture's page dict is hand-formatted across three lines: emit it verbatim.
  w.add(
    raw(
      "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792]\n" +
        "   /Resources << /Font << /F1 4 0 R >> >>\n" +
        "   /Contents 5 0 R >>",
    ),
  );
  w.add(dict([["Type", N("Font")], ["Subtype", N("Type1")], ["BaseFont", N("Helvetica")]]));
  w.add(stream(dict(), helloContent()));
  return w.toBytes(dict([["Root", ref(1)]]), { version: "1.7", xrefEol: "CrLf" });
}

/** docs/research/pdf/examples/02-imaging-model/make_imaging_demo.py */
export function imagingContent(): Uint8Array {
  const rad = 30 * (Math.PI / 180); // CPython math.radians: x * (pi / 180)
  const c = Math.cos(rad);
  const s = Math.sin(rad);
  const F2 = (v: number) => fixed(v, 2);
  const F6 = (v: number) => fixed(v, 6);
  return new ContentBuilder()
    .comment("1. filled rectangle (DeviceRGB)")
    .q().rg(F2(0.2), F2(0.4), F2(0.8)).re(72, 600, 200, 120).f().Q()
    .comment("2. stroked Bezier circle, round joins, dashed")
    .q().RG(F2(0.85), F2(0.1), F2(0.1)).w(6).j(1).d([18, 6], 0)
    .circle(430, 660, 70)
    .S().Q()
    .comment("3. axial gradient clipped to a rectangle")
    .q().re(72, 380, 468, 150).inline((b) => b.W().n()).sh("Sh1").Q()
    .comment("4. rotated square, 30 deg about its centre (306,220), 50% opacity")
    .q().gs("GS1").cm(1, 0, 0, 1, 306, 220).cm(F6(c), F6(s), F6(-s), F6(c), 0, 0)
    .rg(0, 0.6, 0.3).RG(0, 0, 0).w(2).re(-60, -60, 120, 120).B().Q()
    .toBytes(true);
}

export function imagingPdf(): Uint8Array {
  const w = new PdfWriter();
  w.add(dict([["Type", N("Catalog")], ["Pages", ref(2)]]));
  w.add(dict([["Type", N("Pages")], ["Kids", array([ref(3)])], ["Count", int(1)]]));
  w.add(
    dict([
      ["Type", N("Page")],
      ["Parent", ref(2)],
      ["MediaBox", nums(0, 0, 612, 792)],
      ["Contents", ref(4)],
      ["Resources", dict([["Shading", dict([["Sh1", ref(5)]])], ["ExtGState", dict([["GS1", ref(7)]])]])],
    ]),
  );
  w.add(stream(dict(), imagingContent()));
  w.add(
    dict([
      ["ShadingType", int(2)],
      ["ColorSpace", N("DeviceRGB")],
      ["Coords", nums(72, 0, 540, 0)],
      ["Function", ref(6)],
      ["Extend", array([bool(true), bool(true)])],
    ]),
  );
  w.add(
    dict([
      ["FunctionType", int(2)],
      ["Domain", nums(0, 1)],
      ["C0", nums(1, 0.85, 0.1)],
      ["C1", nums(0.5, 0, 0.6)],
      ["N", int(1)],
    ]),
  );
  w.add(dict([["Type", N("ExtGState")], ["ca", num(0.5)], ["CA", num(0.5)]]));
  return w.toBytes(dict([["Root", ref(1)]]), { version: "1.7", xrefEol: "SpaceLf" });
}

/** docs/research/pdf/examples/05-generation-approaches/raw_writer.py */
export const RAW_TITLE = "Hello from lamina";
export const RAW_PARA = [
  "This one-page document was produced by hand-emitting PDF objects:",
  "a catalog, a page tree, one page, a font and a content stream.",
];

export function rawWriterContent(): Uint8Array {
  const b = new ContentBuilder()
    .inline((x) => x.RG(0.2, 0.4, 0.8).w(2).re(72, 600, 468, 120).S())
    .inline((x) => x.BT().Tf("F2", 24).Td(90, 690).Tj(RAW_TITLE).ET())
    .inline((x) => x.BT().Tf("F1", 12).TL(14).Td(90, 660));
  for (const l of RAW_PARA) b.quote(l);
  return b.ET().toBytes();
}

export function rawWriterPdf(): Uint8Array {
  const w = new PdfWriter();
  w.add(dict([["Type", N("Catalog")], ["Pages", ref(2)]]));
  w.add(dict([["Type", N("Pages")], ["Kids", array([ref(3)])], ["Count", int(1)]]));
  w.add(
    dict([
      ["Type", N("Page")],
      ["Parent", ref(2)],
      ["MediaBox", nums(0, 0, 612, 792)],
      ["Resources", dict([["Font", dict([["F1", ref(4)], ["F2", ref(5)]])]])],
      ["Contents", ref(6)],
    ]),
  );
  w.add(dict([["Type", N("Font")], ["Subtype", N("Type1")], ["BaseFont", N("Helvetica")]]));
  w.add(dict([["Type", N("Font")], ["Subtype", N("Type1")], ["BaseFont", N("Helvetica-Bold")]]));
  // /Length placeholder first keeps the script's key order: << /Length N /Filter /FlateDecode >>
  w.add(stream(dict([["Length", int(0)], ["Filter", N("FlateDecode")]]), flateEncode(rawWriterContent())));
  w.add(dict([["Title", str(RAW_TITLE)], ["Producer", str("raw_writer.py")]]));
  return w.toBytes(dict([["Root", ref(1)], ["Info", ref(7)]]), { version: "1.7", xrefEol: "SpaceLf" });
}

/** Hand-assembles a file byte by byte (latin1 string), recording each object's offset. */
export class HandPdf {
  s: string;
  readonly offsets = new Map<number, number>();
  constructor(header = "%PDF-1.7\n") {
    this.s = header;
  }
  get length(): number {
    return this.s.length;
  }
  obj(num: number, body: string): number {
    const at = this.s.length;
    this.offsets.set(num, at);
    this.s += `${num} 0 obj\n${body}\nendobj\n`;
    return at;
  }
  /** A stream object; /Length is the exact data length. */
  streamObj(num: number, dictBody: string, data: string): number {
    return this.obj(num, `<< ${dictBody} /Length ${data.length} >>\nstream\n${data}\nendstream`);
  }
  raw(text: string): this {
    this.s += text;
    return this;
  }
  bytes(): Uint8Array {
    const out = new Uint8Array(this.s.length);
    for (let i = 0; i < this.s.length; i++) out[i] = this.s.charCodeAt(i) & 0xff;
    return out;
  }
}

const latin1Of = (b: Uint8Array): string => {
  let s = "";
  for (const x of b) s += String.fromCharCode(x);
  return s;
};

/**
 * One-page file whose only xref section is a Flate xref stream with a PNG Up predictor
 * (/DecodeParms << /Predictor 12 /Columns 4 >>, /W [1 2 1]) — the pdfTeX/Acrobat shape.
 */
export function xrefStreamPredictorPdf(): Uint8Array {
  const p = new HandPdf("%PDF-1.5\n");
  p.obj(1, "<< /Type /Catalog /Pages 2 0 R >>");
  p.obj(2, "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 612 792] >>");
  p.obj(3, "<< /Type /Page /Parent 2 0 R /Resources << >> /Contents 4 0 R >>");
  p.streamObj(4, "", latin1Of(helloContent("predictor")));
  const xrefAt = p.length;
  const rows: number[][] = [[0, 0, 0, 255]];
  for (let n = 1; n <= 4; n++) rows.push([1, (p.offsets.get(n)! >> 8) & 0xff, p.offsets.get(n)! & 0xff, 0]);
  rows.push([1, (xrefAt >> 8) & 0xff, xrefAt & 0xff, 0]);
  const enc: number[] = [];
  let prev = [0, 0, 0, 0];
  for (const r of rows) {
    enc.push(2, ...r.map((b, i) => (b - prev[i]! + 256) & 0xff));
    prev = r;
  }
  const data = latin1Of(flateEncode(Uint8Array.from(enc)));
  p.streamObj(5, "/Type /XRef /Size 6 /W [1 2 1] /Root 1 0 R /Filter /FlateDecode /DecodeParms << /Predictor 12 /Columns 4 >>", data);
  p.raw(`startxref\n${xrefAt}\n%%EOF\n`);
  return p.bytes();
}
