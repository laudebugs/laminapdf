// The attachment edits and reads: attachFile, updateAttachment, deleteAttachment, attachments(),
// attachmentBytes() and collection(), saved and reopened. A test's title starts with the
// capability it proves (its tag: capability J.05, column pdf), which `cargo xtask capabilities`
// counts.
import { Document, init, isLaminaPdfError, type LaminaPdfError } from "@laminapdf/sdk";
import { beforeAll, describe, expect, it } from "vitest";
import { read, thrown } from "./helpers.js";

beforeAll(() => init());

const SAVED_AT = "D:20261117090000Z";
const XML = new TextEncoder().encode('<?xml version="1.0"?>\n<invoice number="42"/>\n');

/** The LaminaPdfError `f` throws. */
function failure(f: () => unknown): LaminaPdfError {
  const error = thrown(f);
  if (!isLaminaPdfError(error)) throw error;
  return error;
}

function associated(): Document {
  return Document.open(read("fixtures/attachments/associated.pdf"));
}

function savedAndReopened(doc: Document): Document {
  return Document.open(doc.save({ mode: "incremental" }));
}

async function sha256(bytes: Uint8Array): Promise<string> {
  const digest = await crypto.subtle.digest("SHA-256", new Uint8Array(bytes));
  return Array.from(new Uint8Array(digest), (b) => b.toString(16).padStart(2, "0")).join("");
}

describe("attachments", () => {
  it("[cap J.05 pdf round-trip] [cap M.09 pdf round-trip] attaches a file that reopens byte for byte as the document's associated file", async () => {
    using doc = Document.open(read("fixtures/corpus/hello.pdf"));
    const edited = doc.attachFile("invoice.xml", XML, "text/xml", {
      attachTo: "document",
      relationship: "alternative",
      modified: SAVED_AT,
    });
    using again = savedAndReopened(doc);
    const [file] = again.attachments();
    expect(file?.id).toBe(edited.createdAttachments?.[0]);
    expect([file?.name, file?.mime, file?.relationship, file?.modified]).toEqual([
      "invoice.xml",
      "text/xml",
      "alternative",
      SAVED_AT,
    ]);
    expect(file?.holders.map((h) => h.kind)).toEqual(["embeddedFiles", "document"]);
    const bytes = again.attachmentBytes(file?.id ?? "");
    expect([await sha256(bytes.bytes), bytes.checksum]).toEqual([await sha256(XML), "matches"]);
  });

  it("[cap H.09 pdf read] [cap J.05 pdf read] reads each file's holders and bytes, and flags a checksum that doesn't match", () => {
    using doc = associated();
    const [notes, readings] = doc.attachments();
    expect(notes?.holders.map((h) => [h.kind, h.page])).toEqual([
      ["embeddedFiles", undefined],
      ["fileAttachment", 0],
    ]);
    expect(readings?.holders.map((h) => [h.kind, h.page])).toEqual([
      ["embeddedFiles", undefined],
      ["page", 1],
    ]);
    expect(doc.attachmentBytes(notes?.id ?? "").checksum).toBe("matches");
    expect(doc.attachmentBytes(readings?.id ?? "").checksum).toBe("mismatch");
  });

  it("renames and describes an attachment, leaving its bytes", () => {
    using doc = associated();
    const readings = doc.attachments()[1];
    const before = doc.attachmentBytes(readings?.id ?? "").bytes;
    doc.updateAttachment(readings?.id ?? "", { name: "readings-2026.csv", description: null, relationship: "source" });
    const after = doc.attachments().find((a) => a.id === readings?.id);
    expect([after?.name, after?.key, after?.description, after?.relationship]).toEqual([
      "readings-2026.csv",
      "readings-2026.csv",
      undefined,
      "source",
    ]);
    expect(doc.attachmentBytes(readings?.id ?? "").bytes).toEqual(before);
  });

  it("deletes an attachment and says when an annotation still holds it", () => {
    using doc = associated();
    const [notes] = doc.attachments();
    const edited = doc.deleteAttachment(notes?.id ?? "");
    expect(edited.stillInEarlierRevisions).toBe(true);
    expect(edited.warnings?.map((w) => [w.code, w.data])).toEqual([
      ["attachment-still-held", { attachments: [{ attachment: notes?.id, annotation: "a6.0" }] }],
    ]);
    expect(
      doc
        .attachments()
        .find((a) => a.id === notes?.id)
        ?.holders.map((h) => h.kind),
    ).toEqual(["fileAttachment"]);
  });

  it("refuses an attachment the document doesn't hold", () => {
    using doc = associated();
    expect(failure(() => doc.deleteAttachment("e999.0")).code).toBe("unknown-attachment");
    expect(failure(() => doc.attachmentBytes("e999.0")).code).toBe("unknown-attachment");
  });

  it("warns when a PDF/A-2 file gets an attachment", () => {
    using doc = Document.open(read("fixtures/golden/examples/pdf_a_archive-2b.pdf"));
    doc.attachFile("notes.txt", XML, "text/plain", { attachTo: "document", modified: SAVED_AT });
    expect(doc.plan().conformanceWarnings.map((w) => w.code)).toEqual(["pdfa-attachment-forbidden"]);
  });

  it("[cap J.06 pdf read] reads a portfolio's schema, sort and files", () => {
    using doc = Document.open(read("fixtures/attachments/portfolio.pdf"));
    const collection = doc.collection();
    expect(collection?.schema.map((f) => [f.key, f.kind, f.visible])).toEqual([
      ["name", "fileName", true],
      ["desc", "description", true],
      ["owner", "text", true],
      ["due", "date", true],
      ["pages", "number", false],
    ]);
    expect([collection?.sort, collection?.view, collection?.initial]).toEqual([
      [
        ["owner", true],
        ["due", false],
      ],
      "details",
      "report.pdf",
    ]);
    expect(collection?.files.map((f) => [f.key, f.values.owner])).toEqual([
      ["notes.txt", "Review"],
      ["report.pdf", "Finance"],
    ]);
  });

  it("finds no portfolio in a file without a collection", () => {
    using doc = associated();
    expect(doc.collection()).toBeUndefined();
  });

  it("attaches a file inside a transaction from the op attachFileOp makes", () => {
    using doc = associated();
    const op = doc.attachFileOp("invoice.xml", XML, "text/xml", { attachTo: "embeddedFiles" });
    const readings = doc.attachments()[1]?.id ?? "";
    doc.transact((tx) => {
      tx.apply({ op: "attachFile", args: op });
      tx.deleteAttachment(readings);
    });
    expect(doc.attachments().map((a) => a.name)).toEqual(["invoice.xml", "notes.txt"]);
  });
});
