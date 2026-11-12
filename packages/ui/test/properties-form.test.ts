// The editable Description tab's logic (O04): the form from the metadata, what Apply changes, the
// warnings under a field, the earlier-versions notice, and PDF dates both ways.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  authorsOf,
  changeOf,
  type DocumentMetadataData,
  fieldWarnings,
  formFrom,
  keywordsOf,
  localInputToPdfDate,
  pdfDateInstant,
  pdfDateToLocalInput,
  removalStaysInEarlierVersions,
} from "../src/reader/properties-form.ts";

const META: DocumentMetadataData = {
  title: "Q3 Board Report",
  authors: ["Ada Lovelace"],
  subject: "Repairs and rent",
  keywords: "board, repairs",
  creator: "Report template 2.3",
  producer: "Northwind print service",
  created: "D:20261014102000Z",
  modified: "D:20261021174800Z",
  disagreements: [],
  packet: "parsed",
};

describe("the metadata form", () => {
  // The tests read local dates in one zone, half an hour off the hour, whatever the machine's.
  beforeEach(() => {
    vi.stubEnv("TZ", "Asia/Kolkata");
  });
  afterEach(() => {
    vi.unstubAllEnvs();
  });

  it("starts from the metadata: authors joined, keywords as chips, the date in local time", () => {
    expect(formFrom(META)).toEqual({
      title: "Q3 Board Report",
      authors: "Ada Lovelace",
      subject: "Repairs and rent",
      keywords: ["board", "repairs"],
      creator: "Report template 2.3",
      producer: "Northwind print service",
      created: "2026-10-14T15:50",
      removed: [],
    });
  });

  it("changes nothing when nothing was edited, the untouched date included", () => {
    expect(changeOf(META, formFrom(META))).toBeUndefined();
  });

  it("sets the fields that changed, and removes the ones emptied or marked", () => {
    const form = {
      ...formFrom(META),
      title: " Quarterly figures ",
      authors: "Ada Lovelace; Charles Babbage",
      subject: "",
    };
    expect(changeOf(META, { ...form, removed: ["creator"] })).toEqual({
      patch: { title: "Quarterly figures", authors: ["Ada Lovelace", "Charles Babbage"] },
      remove: ["subject", "creator"],
    });
  });

  it("writes a changed date with the zone it was entered in", () => {
    expect(changeOf(META, { ...formFrom(META), created: "2026-11-01T09:00" })?.patch).toEqual({
      created: "D:20261101090000+05'30'",
    });
  });

  it("brings disagreeing fields back in step in a PDF/A file, and leaves them alone elsewhere", () => {
    const pdfa = { ...META, pdfa: "2B", disagreements: ["title" as const] };
    expect(changeOf(pdfa, formFrom(pdfa))?.patch).toEqual({ title: "Q3 Board Report" });
    expect(changeOf({ ...pdfa, pdfa: undefined }, formFrom(pdfa))).toBeUndefined();
  });

  it("removes nothing the file doesn't hold", () => {
    const empty: DocumentMetadataData = { authors: [], disagreements: [], packet: "absent" };
    expect(changeOf(empty, { ...formFrom(empty), removed: ["title"] })).toBeUndefined();
  });

  it("warns under the author of a PDF/A-1 file with two, and under the title of a PDF/UA-1 file without one", () => {
    const two = { ...formFrom(META), authors: "Ada Lovelace; Charles Babbage", title: "" };
    expect(fieldWarnings({ ...META, pdfa: "1B", pdfua: 1 }, two)).toEqual({
      authors: { id: "properties.edit.warning.oneAuthor", values: { count: 2, level: "1B" } },
      title: { id: "properties.edit.warning.title" },
    });
    expect(fieldWarnings({ ...META, pdfa: "2B" }, two)).toEqual({});
    expect(fieldWarnings({ ...META, pdfua: 1 }, { ...formFrom(META), removed: ["title"] })).toEqual({
      title: { id: "properties.edit.warning.title" },
    });
  });

  it("says removed values stay in earlier versions unless the save rewrites the file", () => {
    const change = changeOf(META, { ...formFrom(META), removed: ["creator"] });
    expect([removalStaysInEarlierVersions(change, false), removalStaysInEarlierVersions(change, true)]).toEqual([
      true,
      false,
    ]);
    const set = changeOf(META, { ...formFrom(META), title: "New" });
    expect(removalStaysInEarlierVersions(set, false)).toBe(false);
  });
});

describe("the form's text", () => {
  it("splits authors at semicolons and keywords at commas and semicolons, once each", () => {
    expect(authorsOf(" Ada Lovelace ;; Charles Babbage ")).toEqual(["Ada Lovelace", "Charles Babbage"]);
    expect(keywordsOf("board, repairs; board ,, rent")).toEqual(["board", "repairs", "rent"]);
  });
});

describe("PDF dates", () => {
  beforeEach(() => {
    vi.stubEnv("TZ", "America/New_York");
  });
  afterEach(() => {
    vi.unstubAllEnvs();
  });

  it("reads each form of a PDF date as its instant", () => {
    expect(pdfDateInstant("D:20261014102000Z")?.toISOString()).toBe("2026-10-14T10:20:00.000Z");
    expect(pdfDateInstant("D:20261014102000+01'00'")?.toISOString()).toBe("2026-10-14T09:20:00.000Z");
    expect(pdfDateInstant("D:20261014102000-05'30")?.toISOString()).toBe("2026-10-14T15:50:00.000Z");
    expect(pdfDateInstant("D:2026")?.toISOString()).toBe("2026-01-01T05:00:00.000Z");
    expect(pdfDateInstant("14 October")).toBeUndefined();
  });

  it("goes to the date field and back with the reader's own zone, summer time included", () => {
    expect(pdfDateToLocalInput("D:20260714120000Z")).toBe("2026-07-14T08:00");
    expect(localInputToPdfDate("2026-07-14T08:00")).toBe("D:20260714080000-04'00'");
    expect(localInputToPdfDate("2026-12-14T08:00")).toBe("D:20261214080000-05'00'");
    expect([pdfDateToLocalInput(undefined), localInputToPdfDate("")]).toEqual(["", undefined]);
  });
});
