// Put the same marks on every page of a document: a hideable CONFIDENTIAL watermark, a header with
// the file's name and the date, a footer that numbers the pages, and a Bates number in the corner
// of each page.
//
// The marks become part of each page: they print and every reader draws them. The watermark sits
// in an optional content group, so a reader's layers panel can switch it off, and in a tagged file
// like this PDF/A-2 and PDF/UA-1 archive every mark is an artifact that screen readers skip. A header's `{page}` and
// `{pages}` are filled when the edit is applied; the Bates numbers count from where you say
// (number a second file from where the first ended). The marks stay changeable after a save:
// `doc.content.pageMarks(index)` lists them by the IDs below.
import { Document } from "@laminapdf/sdk";

/**
 * The document with its pages marked, saved incrementally, all in the font `font` (a TrueType or
 * OpenType file), which PDF/A and PDF/UA files need embedded. `at` is when the marks were made, as a
 * PDF date; `file` and `date` are what the header's `{file}` and `{date}` stand for, as you want
 * them written.
 */
export function addPageMarks(
  bytes: Uint8Array,
  font: Uint8Array,
  file: string,
  date: string,
  at: string,
): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);
  const fonts = [doc.addBlob(font)];
  // Every page, counted from 0.
  const pages = Array.from({ length: doc.pageCount }, (_, i) => i);

  // A red CONFIDENTIAL across each page, turned 45 degrees and faint, which a reader can hide.
  doc.content.addWatermark(pages, {
    id: "confidential",
    mark: { text: { text: "CONFIDENTIAL", fonts, size: 90, color: [0.8, 0.1, 0.1] } },
    opacity: 0.2,
    angle: 45,
    hideable: true,
    modified: at,
  });

  // The file's name and the date at the top right, and the page's number at the bottom centre,
  // clear of this report's own header at the top left and page number at the bottom right: slots in
  // the order top left, centre, right, then bottom left, centre, right.
  doc.content.setHeaderFooter(pages, {
    id: "folio",
    slots: [null, null, "{file} · {date}", null, "Page {page} of {pages}", null],
    fonts,
    size: 9,
    color: [0.35],
    date,
    file,
    modified: at,
  });

  // ACME-000101, ACME-000102 and on, at the bottom left.
  doc.content.applyBates(pages, {
    id: "bates",
    prefix: "ACME-",
    start: 101,
    digits: 6,
    slot: "bottomLeft",
    fonts,
    size: 9,
    modified: at,
  });

  return doc.save({ mode: "incremental" });
}
