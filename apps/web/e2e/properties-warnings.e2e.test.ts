// The properties dialog warns under a field before Apply (O04) by rules of its own, since the SDK
// reports conformance warnings only from a save plan, after an edit. No browser: it is in the e2e
// suite because that suite runs after the WebAssembly is built, which loading the SDK needs. These hold the dialog's rules
// and its field list to the SDK's: the same edit, made through the SDK, plans the warnings the form
// shows, and the form's fields are the SDK's metadata keys.

import { readFileSync } from "node:fs";
import { Document, init, type MetadataKey } from "@laminapdf/sdk";
import { type EditableField, fieldWarnings, formFrom, type MetadataField } from "@laminapdf/ui/testing";
import { beforeAll, describe, expect, it } from "vitest";

const EXAMPLES = new URL("../../../fixtures/golden/examples/", import.meta.url);

/** The SDK's code for each of the form's warnings. */
const CODE = {
  "properties.edit.warning.title": "pdfua-title-missing",
  "properties.edit.warning.oneAuthor": "pdfa-one-author",
} as const;

// The form's field names are the SDK's metadata keys, both ways: a compile-time check.
type Same<A, B> = [A] extends [B] ? ([B] extends [A] ? true : false) : false;
const fieldsAreKeys: Same<MetadataField, MetadataKey> = true;

const example = (name: string) => new Uint8Array(readFileSync(new URL(name, EXAMPLES)));

/** The PDF/A-2b example claiming PDF/A-1b instead: the part's digit swapped, its XMP unfiltered, the offsets kept. */
function pdfa1(): Uint8Array {
  const bytes = example("pdf_a_archive-2b.pdf");
  const text = new TextDecoder("latin1").decode(bytes);
  const at = text.indexOf("pdfaid:part>2<");
  if (at < 0) throw new Error("the example's XMP no longer names its PDF/A part as an element");
  bytes[at + "pdfaid:part>".length] = "1".charCodeAt(0);
  return bytes;
}

/** The warning codes the SDK plans after `edit`, and those the form shows under its fields for the same edit. */
function both(bytes: Uint8Array, edit: { authors?: string[]; remove?: EditableField[] }) {
  using doc = Document.open(bytes);
  const metadata = doc.metadata();
  const form = {
    ...formFrom(metadata),
    ...(edit.authors === undefined ? {} : { authors: edit.authors.join("; ") }),
    removed: edit.remove ?? [],
  };
  if (edit.authors !== undefined) doc.setMetadata({ authors: edit.authors });
  if (edit.remove !== undefined) doc.removeMetadata(edit.remove);
  const planned = doc
    .plan()
    .conformanceWarnings.map((w) => w.code)
    .filter((c) => c !== "pdfa-info-mismatch")
    .sort();
  const shown = Object.values(fieldWarnings(metadata, form))
    .map((w) => (w === undefined ? "" : CODE[w.id as keyof typeof CODE]))
    .sort();
  return { planned, shown };
}

describe("the dialog's conformance warnings", () => {
  beforeAll(async () => {
    await init();
  });

  it("cover every metadata key the SDK has", () => {
    expect(fieldsAreKeys).toBe(true);
  });

  it("match the SDK's when a PDF/UA file loses its title", () => {
    expect(both(example("pdf_ua_report.pdf"), { remove: ["title"] })).toEqual({
      planned: ["pdfua-title-missing"],
      shown: ["pdfua-title-missing"],
    });
  });

  it("match the SDK's when a PDF/A-1 file gets two authors", () => {
    expect(both(pdfa1(), { authors: ["Ada Lovelace", "Charles Babbage"] })).toEqual({
      planned: ["pdfa-one-author"],
      shown: ["pdfa-one-author"],
    });
  });

  it("match the SDK's when a PDF/A-2 file gets two authors: none, as only PDF/A-1 limits them", () => {
    expect(both(example("pdf_a_archive-2b.pdf"), { authors: ["Ada Lovelace", "Charles Babbage"] })).toEqual({
      planned: [],
      shown: [],
    });
  });

  it("match the SDK's when nothing breaks a claim", () => {
    expect(both(example("pdf_ua_report.pdf"), { authors: ["Ada Lovelace"] })).toEqual({ planned: [], shown: [] });
  });
});
