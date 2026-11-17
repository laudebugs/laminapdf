// What the Attachments panel (O14) says about a file: its kind, whether it is a program, the badge
// for its relationship and what it belongs to, the header's summary, and PDF dates both ways.

import { describe, expect, it } from "vitest";
import {
  type AttachmentData,
  attachmentKind,
  attachmentsSummary,
  isProgram,
  relationshipBadge,
} from "../src/reader/attachments.js";
import { pdfDateOf } from "../src/reader/properties-form.js";

/** `overrides` may also set a field to `undefined`, which leaves it out. */
type Overrides = { readonly [K in keyof AttachmentData]?: AttachmentData[K] | undefined };

function file(overrides: Overrides = {}): AttachmentData {
  const all = {
    id: "e4.0",
    name: "readings.csv",
    mime: "text/csv",
    size: 12_288,
    hasChecksum: true,
    embedded: true,
    holders: [],
    ...overrides,
  };
  return Object.fromEntries(Object.entries(all).filter(([, v]) => v !== undefined)) as unknown as AttachmentData;
}

const pageName = (index: number) => String(index + 1);

describe("attachments", () => {
  it("names a spreadsheet by its name's ending", () => {
    expect(attachmentKind(file({ name: "repairs.xlsx", mime: undefined }))).toBe("Spreadsheet");
  });

  it("names a PDF by its MIME type", () => {
    expect(attachmentKind(file({ name: "minutes", mime: "application/pdf" }))).toBe("PDF");
  });

  it("names a CSV file data", () => {
    expect(attachmentKind(file())).toBe("Data");
  });

  it("takes a file as a program by its ending or its MIME type, whatever its name says", () => {
    expect([
      isProgram(file({ name: "setup.EXE", mime: undefined })),
      isProgram(file({ name: "invoice.pdf", mime: "application/x-msdownload" })),
      isProgram(file()),
    ]).toEqual([true, true, false]);
  });

  it("badges an associated file with its relationship and page", () => {
    const sourced = file({ relationship: "source", holders: [{ kind: "embeddedFiles" }, { kind: "page", page: 2 }] });
    expect(relationshipBadge(sourced, pageName)).toBe("Source of page 3");
  });

  it("gives no badge to a file only in the document's list or of no stated relationship", () => {
    const listed = file({ relationship: "data", holders: [{ kind: "embeddedFiles" }] });
    const unspecified = file({ relationship: "unspecified", holders: [{ kind: "document" }] });
    expect([relationshipBadge(listed, pageName), relationshipBadge(unspecified, pageName)]).toEqual([
      undefined,
      undefined,
    ]);
  });

  it("sums the files the header counts", () => {
    expect(attachmentsSummary([file({ size: 1024 }), file({ id: "e6.0", size: 2048 })])).toBe("2 files · 3.0 KB");
  });

  it("writes a file's own time as a UTC PDF date", () => {
    expect(pdfDateOf(new Date("2026-10-27T08:12:00.000Z"))).toBe("D:20261027081200Z");
  });
});
