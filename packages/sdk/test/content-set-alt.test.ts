// Alternative text for the figure an image belongs to: doc.content.setAlt on a small tagged
// file, saved and reopened, and refused where no figure owns the object. Needs the extract module
// for the structure tree.
import { Document, init, isLaminaPdfError, type StructureNode } from "@laminapdf/sdk";
import { beforeAll, describe, expect, it } from "vitest";
import { thrown } from "./helpers.js";

beforeAll(() => init({ extract: true }));

const CONTENT = [
  "/P <</MCID 0>> BDC BT /F1 12 Tf 20 150 Td (Hello) Tj ET EMC",
  "/Figure <</MCID 1>> BDC q 40 0 0 40 100 20 cm /Im1 Do Q EMC",
].join("\n");

/** A one-page tagged file: a paragraph and a figure around a one-pixel image. */
function tagged(figureAlt: string): Uint8Array {
  const stream = (dict: string, body: string) => `<< ${dict} /Length ${body.length} >>\nstream\n${body}\nendstream`;
  const objects = [
    "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 7 0 R /MarkInfo << /Marked true >> >>",
    "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
    "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Contents 4 0 R /StructParents 0 /Resources << /Font << /F1 5 0 R >> /XObject << /Im1 6 0 R >> >> >>",
    stream("", CONTENT),
    "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
    stream("/Type /XObject /Subtype /Image /Width 1 /Height 1 /ColorSpace /DeviceGray /BitsPerComponent 8", "\u0080"),
    "<< /Type /StructTreeRoot /K [8 0 R] /ParentTree 11 0 R /ParentTreeNextKey 1 >>",
    "<< /Type /StructElem /S /Document /P 7 0 R /K [9 0 R 10 0 R] >>",
    "<< /Type /StructElem /S /P /P 8 0 R /Pg 3 0 R /K 0 >>",
    `<< /Type /StructElem /S /Figure /P 8 0 R /Pg 3 0 R /K 1 ${figureAlt} >>`,
    "<< /Nums [0 [9 0 R 10 0 R]] >>",
  ];
  let out = "%PDF-1.7\n";
  const offsets = objects.map((body, i) => {
    const at = out.length;
    out += `${i + 1} 0 obj\n${body}\nendobj\n`;
    return at;
  });
  const xref = out.length;
  out += `xref\n0 ${objects.length + 1}\n0000000000 65535 f \n`;
  for (const at of offsets) {
    out += `${String(at).padStart(10, "0")} 00000 n \n`;
  }
  out += `trailer\n<< /Size ${objects.length + 1} /Root 1 0 R >>\nstartxref\n${xref}\n%%EOF\n`;
  return Uint8Array.from(out, (c) => c.charCodeAt(0));
}

function figures(nodes: readonly StructureNode[] | undefined): StructureNode[] {
  return (nodes ?? []).flatMap((n) => [
    ...(n.kind === "Figure" ? [n] : []),
    ...figures(n.kids.flatMap((k) => (k.type === "node" ? [k] : []))),
  ]);
}

function image(doc: Document) {
  const found = doc.content.objects(0).find((o) => o.kind === "image");
  if (!found) {
    throw new Error("no image");
  }
  return found.object;
}

describe("alternative text for a figure", () => {
  it("describes an image, replacing the text its figure had, and the text reopens", () => {
    using doc = Document.open(tagged("/Alt (A grey square)"));
    doc.content.setAlt(0, image(doc), "A dark square");
    using again = Document.open(doc.save({ mode: "incremental" }));
    expect(figures(again.structureTree()).map((f) => f.alt)).toEqual(["A dark square"]);
  });

  it("adds text to a figure that had none, and undo takes it away again", () => {
    using doc = Document.open(tagged(""));
    doc.content.setAlt(0, image(doc), "A square");
    expect(figures(doc.structureTree()).map((f) => f.alt)).toEqual(["A square"]);
    doc.undo();
    expect(figures(doc.structureTree()).map((f) => f.alt)).toEqual([undefined]);
  });

  it("refuses empty text, and an object no figure owns, with invalid-op", () => {
    using doc = Document.open(tagged("/Alt (A grey square)"));
    const text = doc.content.objects(0).find((o) => o.kind === "text");
    for (const [object, alt] of [
      [image(doc), "  "],
      [image(doc), "x".repeat(2001)],
      [text?.object ?? image(doc), "Hello"],
    ] as const) {
      const e = thrown(() => doc.content.setAlt(0, object, alt));
      expect(isLaminaPdfError(e) && e.code).toBe("invalid-op");
    }
  });

  it("refuses a name that no longer matches with content-object-stale", () => {
    using doc = Document.open(tagged(""));
    const object = { ...image(doc), hash: "0000000000000000" };
    const e = thrown(() => doc.content.setAlt(0, object, "A square"));
    expect(isLaminaPdfError(e) && e.code).toBe("content-object-stale");
  });
});
