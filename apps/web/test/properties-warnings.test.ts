// The properties dialog warns under a field before Apply (O04) by rules of its own, since the SDK
// reports conformance warnings only from a save plan, after an edit. These hold the dialog's rules
// to the SDK's own: the same edit, made through the SDK, plans the warnings the form shows.

import { readFileSync } from "node:fs";
import { Document, init, type MetadataKey } from "@laminapdf/sdk";
import { fieldWarnings, formFrom } from "@laminapdf/ui/dialogs";
import { beforeAll, describe, expect, it } from "vitest";

const EXAMPLES = new URL("../../../fixtures/golden/examples/", import.meta.url);

/** The warning codes the SDK plans after `edit`, and the fields the form warns under for the same edit. */
function both(file: string, edit: { authors?: string[]; remove?: MetadataKey[] }) {
  using doc = Document.open(new Uint8Array(readFileSync(new URL(file, EXAMPLES))));
  const metadata = doc.metadata();
  const form = {
    ...formFrom(metadata),
    ...(edit.authors === undefined ? {} : { authors: edit.authors.join("; ") }),
    removed: (edit.remove ?? []).filter((k) => k !== "modified"),
  };
  if (edit.authors !== undefined) doc.setMetadata({ authors: edit.authors });
  if (edit.remove !== undefined) doc.removeMetadata(edit.remove);
  const planned = doc
    .plan()
    .conformanceWarnings.map((w) => w.code)
    .filter((c) => c !== "pdfa-info-mismatch")
    .sort();
  const shown = Object.values(fieldWarnings(metadata, form)).map((w) =>
    w?.id === "properties.edit.warning.title" ? "pdfua-title-missing" : "pdfa-one-author",
  );
  return { planned, shown: shown.sort() };
}

describe("the dialog's conformance warnings", () => {
  beforeAll(async () => {
    await init();
  });

  it("match the SDK's when a PDF/UA file loses its title", () => {
    const { planned, shown } = both("pdf_ua_report.pdf", { remove: ["title"] });
    expect([planned, shown]).toEqual([["pdfua-title-missing"], ["pdfua-title-missing"]]);
  });

  it("match the SDK's when a PDF/A-2 file gets two authors: none, as only PDF/A-1 limits them", () => {
    const { planned, shown } = both("pdf_a_archive-2b.pdf", { authors: ["Ada Lovelace", "Charles Babbage"] });
    expect([planned, shown]).toEqual([[], []]);
  });

  it("match the SDK's when nothing breaks a claim", () => {
    const { planned, shown } = both("pdf_ua_report.pdf", { authors: ["Ada Lovelace"] });
    expect([planned, shown]).toEqual([[], []]);
  });
});
