// The metadata API: doc.metadata(), setMetadata, removeMetadata, plan and wouldViolate, saved and
// reopened. The names start with the capability each test proves (J.01 and the rest); they become
// capability tags once `cargo xtask capabilities` runs TypeScript tests.
import { Document, init, isLaminaPdfError, type Op } from "@laminapdf/sdk";
import { beforeAll, describe, expect, it } from "vitest";
import { read, readText, thrown } from "./helpers.js";

beforeAll(() => init());

const SAVED_AT = "D:20261110093000Z";

function open(path = "fixtures/golden/metadata.pdf"): Document {
  return Document.open(read(path));
}

/** The document saved with the save-time stamp, reopened. */
function savedAndReopened(doc: Document, mode: "incremental" | "fullRewrite" = "incremental"): Document {
  return Document.open(doc.save({ mode, metadataDate: SAVED_AT }));
}

describe("metadata", () => {
  it("J.01 (pdf, round-trip): sets the title and authors in Info and XMP together", () => {
    for (const mode of ["incremental", "fullRewrite"] as const) {
      using doc = open();
      doc.setMetadata({ title: "Quarterly figures", authors: ["Ada Lovelace", "Charles Babbage"], subject: null });
      using again = savedAndReopened(doc, mode);
      const meta = again.metadata();
      expect([meta.title, meta.authors, meta.subject, meta.disagreements, again.info.author]).toEqual([
        "Quarterly figures",
        ["Ada Lovelace", "Charles Babbage"],
        undefined,
        [],
        "Ada Lovelace; Charles Babbage",
      ]);
    }
  });

  it("J.02 (pdf, round-trip): writes the dates, and the save stamps the modification date", () => {
    using doc = open();
    doc.setMetadata({ created: "D:20260101120000+01'00'" });
    using again = savedAndReopened(doc);
    const meta = again.metadata();
    expect([meta.created, meta.modified]).toEqual(["D:20260101120000+01'00'", SAVED_AT]);
  });

  it("J.03 (pdf, round-trip): writes the producer and the creator tool", () => {
    using doc = open();
    doc.setMetadata({ producer: "Lamina PDF", creator: "A writer" });
    using again = savedAndReopened(doc);
    const meta = again.metadata();
    expect([meta.producer, meta.creator, meta.disagreements]).toEqual(["Lamina PDF", "A writer", []]);
  });

  it("J.10 (pdf, round-trip): removes selected fields from both sides", () => {
    using doc = open();
    const edited = doc.removeMetadata(["keywords", "authors"]);
    expect(edited.stillInEarlierRevisions).toBe(true);
    using again = savedAndReopened(doc);
    const meta = again.metadata();
    expect([meta.keywords, meta.authors, again.info.keywords, meta.title]).toEqual([
      undefined,
      [],
      undefined,
      "Metadata golden: Info & XMP <in sync>",
    ]);
  });

  it("applies the frozen samples, the deprecated setInfo with them", () => {
    using doc = open();
    const sample = (name: string) => JSON.parse(readText(`fixtures/ops/v1/${name}.json`)) as Op;
    doc.apply([sample("set-info"), sample("set-metadata"), sample("remove-metadata")]);
    expect(doc.metadata().title).toBe("Quarterly figures");
  });

  it("undoing an edit saves the original bytes", () => {
    const bytes = read("fixtures/golden/metadata.pdf");
    using doc = Document.open(bytes);
    doc.setMetadata({ title: "Changed" });
    doc.undo();
    expect(doc.save({ metadataDate: SAVED_AT })).toEqual(bytes);
  });

  it("adds XMP metadata to a file without any", () => {
    using doc = open("fixtures/corpus/hello.pdf");
    expect(doc.metadata().packet).toBe("absent");
    doc.setMetadata({ title: "Hello" });
    expect(doc.metadata().packet).toBe("parsed");
  });
});

describe("plan and wouldViolate", () => {
  it("names the refusal on a certified file instead of throwing", () => {
    using doc = open("fixtures/signed/certified-p2.pdf");
    expect(doc.wouldViolate({ op: "setMetadata", args: { patch: { title: "x" } } } as Op)).toHaveLength(1);
    doc.setMetadata({ title: "x" });
    const plan = doc.plan();
    expect([plan.constraint, plan.refusal, plan.docMdp, plan.violations[0]?.restriction]).toEqual([
      "conflict",
      "would-invalidate-signatures",
      2,
      "docMdp",
    ]);
  });

  it("stamps an approval-signed file incrementally", () => {
    using doc = open("fixtures/signed/approval.pdf");
    doc.setMetadata({ title: "x" });
    const plan = doc.plan({ metadataDate: SAVED_AT });
    expect([plan.constraint, plan.mode, plan.metadata]).toEqual(["appendOnly", "incremental", "stamp"]);
    expect(doc.saveWithReport({ metadataDate: SAVED_AT }).report.metadataUpdated).toBe(true);
  });

  it("warns before a save that breaks a PDF/UA claim, and still saves", () => {
    using doc = open("fixtures/golden/examples/pdf_ua_report.pdf");
    doc.removeMetadata(["title"]);
    expect(doc.plan().conformanceWarnings.map((w) => w.code)).toEqual(["pdfua-title-missing"]);
    expect(doc.save().length).toBeGreaterThan(0);
  });

  it("refuses a metadataDate that is no date", () => {
    using doc = open();
    expect(
      isLaminaPdfError(
        thrown(() => doc.plan({ metadataDate: "yesterday" })),
        "invalid-argument",
      ),
    ).toBe(true);
  });
});
