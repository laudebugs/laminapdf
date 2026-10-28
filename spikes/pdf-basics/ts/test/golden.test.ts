// SPEC case 1 (golden, byte-identical) and case 2 (raw writer parity).
import { describe, expect, test } from "vitest";
import { dict, flateDecode, int, open, ref, serialize, stream, writeIncremental } from "../src/index.js";
import { corpusFile, golden, latin1 } from "./helpers.js";
import { helloContent, helloPdf, imagingPdf, rawWriterContent, rawWriterPdf } from "./recipes.js";

describe("case 1: golden fixtures are reproduced byte-for-byte", () => {
  test("hello.pdf (CrLf xref EOL)", () => {
    expect(latin1(helloPdf())).toBe(latin1(golden("hello.pdf")));
  });
  test("imaging-demo.pdf (SpaceLf xref EOL, content builder, circle helper)", () => {
    expect(latin1(imagingPdf())).toBe(latin1(golden("imaging-demo.pdf")));
  });
  test("bonus: hello-incremental.pdf rebuilt with the incremental writer", () => {
    const out = writeIncremental(golden("hello.pdf"), [{ num: 5, body: stream(dict(), helloContent("Hello, Update")) }]);
    expect(latin1(out)).toBe(latin1(corpusFile("hello-incremental.pdf")));
  });
});

// Decoded content of raw_writer.py's stream, transcribed from the script.
const RAW_CONTENT =
  "0.2 0.4 0.8 RG 2 w 72 600 468 120 re S\n" +
  "BT /F2 24 Tf 90 690 Td (Hello from lamina) Tj ET\n" +
  "BT /F1 12 Tf 14 TL 90 660 Td\n" +
  "(This one-page document was produced by hand-emitting PDF objects:) '\n" +
  "(a catalog, a page tree, one page, a font and a content stream.) '\n" +
  "ET";

describe("case 2: raw_writer.py parity", () => {
  const ours = open(rawWriterPdf());
  const theirs = open(corpusFile("raw.pdf"));

  test("content builder emits the script's content", () => {
    expect(latin1(rawWriterContent())).toBe(RAW_CONTENT);
  });

  test("same objects; stream compared by decoded content", () => {
    expect(ours.objectNumbers()).toEqual(theirs.objectNumbers());
    for (const n of ours.objectNumbers()) {
      const a = ours.getObject(n);
      const b = theirs.getObject(n);
      if (a.kind === "stream" && b.kind === "stream") {
        const strip = (s: typeof a) => {
          const e = new Map(s.dict.entries);
          e.set("Length", int(0));
          return serialize({ kind: "dict", entries: e });
        };
        expect(latin1(strip(a))).toBe(latin1(strip(b)));
        expect(latin1(flateDecode(a.data))).toBe(RAW_CONTENT);
        expect(latin1(flateDecode(b.data))).toBe(RAW_CONTENT);
      } else {
        expect(latin1(serialize(a)), `object ${n}`).toBe(latin1(serialize(b)));
      }
    }
    expect(latin1(serialize(ours.trailer()))).toBe(latin1(serialize(theirs.trailer())));
  });

  test("byte layout identical outside the Flate payload", () => {
    // Replace the payload + /Length digits by placeholders, then fix offsets by re-deriving them.
    const norm = (b: Uint8Array) => {
      const s = latin1(b);
      const start = s.indexOf("stream\n", s.indexOf("6 0 obj")) + 7;
      const end = s.indexOf("\nendstream", start);
      return {
        before: s.slice(0, start).replace(/\/Length \d+/, "/Length #"),
        after: s.slice(end).replace(/\d{10} 00000 n/g, "OFFSET").replace(/startxref\n\d+/, "startxref\n#"),
      };
    };
    expect(norm(rawWriterPdf())).toEqual(norm(corpusFile("raw.pdf")));
    expect(ours.resolve(ref(7)).kind).toBe("dict");
  });
});
