// The XMP edits: setXmpProperty, removeXmpProperty, replaceXmpPacket and xmpPacket, saved and
// reopened. The names start with the capability each test proves (J.04).
import { Document, init, isLaminaPdfError, type LaminaPdfError } from "@laminapdf/sdk";
import { beforeAll, describe, expect, it } from "vitest";
import { read, thrown } from "./helpers.js";

beforeAll(() => init());

const SAVED_AT = "D:20261110093000Z";
const REVIEW = "https://example.com/ns/review/1.0/";
const PDFA = "fixtures/golden/examples/pdf_a_archive-2b.pdf";

/** The LaminaPdfError `f` throws. */
function failure(f: () => unknown): LaminaPdfError {
  const error = thrown(f);
  if (!isLaminaPdfError(error)) throw error;
  return error;
}

function open(path = "fixtures/golden/metadata.pdf"): Document {
  return Document.open(read(path));
}

function savedAndReopened(doc: Document): Document {
  return Document.open(doc.save({ mode: "incremental", metadataDate: SAVED_AT }));
}

describe("xmp", () => {
  it("J.04 (pdf, round-trip): sets custom properties of every shape", () => {
    using doc = open();
    doc.transact((tx) => {
      tx.setXmpProperty(REVIEW, "review", "status", { type: "text", value: "Approved" });
      tx.setXmpProperty(REVIEW, "review", "round", { type: "integer", value: 2 });
      tx.setXmpProperty(REVIEW, "review", "tags", {
        type: "bag",
        value: [
          { type: "text", value: "q3" },
          { type: "text", value: "board" },
        ],
      });
      tx.setXmpProperty(REVIEW, "review", "note", {
        type: "langAlt",
        value: [{ lang: "x-default", text: "Signed off" }],
      });
    });
    using again = savedAndReopened(doc);
    const xml = again.xmpPacket().xml ?? "";
    for (const part of ["<review:status>Approved</review:status>", "<review:round>2</review:round>", "<rdf:Bag>"]) {
      expect(xml).toContain(part);
    }
  });

  it("J.04 (pdf, round-trip): keeps the Info dictionary in step for a mirrored property", () => {
    using doc = open();
    doc.setXmpProperty("http://purl.org/dc/elements/1.1/", "dc", "title", { type: "text", value: "Minutes" });
    using again = savedAndReopened(doc);
    expect([again.info.title, again.metadata().title]).toEqual(["Minutes", "Minutes"]);
  });

  it("J.04 (pdf, round-trip): declares a custom property in a PDF/A file", () => {
    using doc = open(PDFA);
    expect(doc.xmpPacket().pdfa).toBe("2B");
    const edited = doc.setXmpProperty(REVIEW, "review", "round", { type: "integer", value: 2 });
    expect(edited.declared).toEqual([{ namespace: REVIEW, prefix: "review", name: "round", valueType: "Integer" }]);
  });

  it("removes a property and says the earlier revision still holds it", () => {
    using doc = open();
    doc.setXmpProperty(REVIEW, "review", "status", { type: "text", value: "Draft" });
    using saved = savedAndReopened(doc);
    const edited = saved.removeXmpProperty(REVIEW, "status");
    expect([edited.stillInEarlierRevisions, saved.xmpPacket().xml?.includes("review:status")]).toEqual([true, false]);
  });

  it("replaces the packet, and refuses one that doesn't parse", () => {
    using doc = open();
    const xml = `<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description rdf:about="" xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title><rdf:Alt><rdf:li xml:lang="x-default">Replaced</rdf:li></rdf:Alt></dc:title></rdf:Description></rdf:RDF></x:xmpmeta>`;
    doc.replaceXmpPacket(xml);
    expect([doc.xmpPacket().xml, doc.info.title]).toEqual([xml, "Replaced"]);
    const error = failure(() => doc.replaceXmpPacket("<x:xmpmeta><rdf:RDF/></x:xmpmeta>"));
    expect([error.code, error.data?.reason]).toEqual(["invalid-xmp-packet", "prefix"]);
  });

  it("refuses an edit of the PDF/A identification and a value of the wrong shape", () => {
    using doc = open(PDFA);
    const claim = failure(() =>
      doc.setXmpProperty("http://www.aiim.org/pdfa/ns/id/", "pdfaid", "part", { type: "integer", value: 3 }),
    );
    const shape = failure(() =>
      doc.setXmpProperty("http://purl.org/dc/elements/1.1/", "dc", "title", { type: "integer", value: 3 }),
    );
    expect([claim.code, shape.code, shape.data?.reason]).toEqual([
      "xmp-claim-read-only",
      "invalid-xmp-property",
      "shape",
    ]);
  });

  it("reads a file without XMP metadata as absent", () => {
    using doc = open("fixtures/corpus/hello.pdf");
    expect(doc.xmpPacket()).toEqual({ status: "absent" });
  });
});
