// A report written in Markdown, turned into tagged pages that flow: headings, paragraphs with
// emphasis, code and links, a photo, lists and a task list, a block quote, a table that runs
// across pages, a code block, footnotes, and paragraphs in Arabic and Hindi.
//
// The front matter gives the title and language, a contents page lists the headings, and the
// output meets PDF/UA-1, so every font is embedded. The builder reads no files: the fonts and the
// one photo the Markdown names are passed in, and nothing is fetched.
import { Builder, type BuilderOptions, Font, Image } from "@laminapdf/sdk";

/** The fonts the report is set in: Latin, a bold face, Arabic and Devanagari. */
export interface ReportFonts {
  readonly regular: Uint8Array;
  /** A collection whose second face (index 1) is the bold one. */
  readonly bold: Uint8Array;
  readonly arabic: Uint8Array;
  readonly devanagari: Uint8Array;
}

/**
 * The report's bytes. Needs `init({ text: true })`.
 *
 * @param markdown - the report's Markdown.
 * @param fonts - the fonts to set it in.
 * @param photo - the JPEG the Markdown shows as `photo.jpg`.
 * @param options - can fix the file ID, the producer and the creation date.
 */
export function markdownToPdf(
  markdown: string,
  fonts: ReportFonts,
  photo: Uint8Array,
  options: BuilderOptions = {},
): Uint8Array<ArrayBuffer> {
  using b = new Builder({ ...options, profiles: ["PDF/UA-1"] });
  const report = b.markdown(markdown, {
    fonts: {
      regular: Font.fromBytes(fonts.regular),
      bold: Font.fromCollection(fonts.bold, 1),
      fallbacks: [Font.fromBytes(fonts.arabic), Font.fromBytes(fonts.devanagari)],
    },
    images: { "photo.jpg": Image.jpeg(photo) },
    // A5 pages, so the table runs across two of them, with the page number in the footer.
    page: { size: "A5", footer: { center: "Page {page} of {pages}" }, furnitureSize: 8 },
    contents: 2,
  });
  using doc = b.finish();
  const [warning] = doc.warnings;
  if (warning !== undefined) throw new Error(`the Markdown has something the PDF leaves out: ${warning.code}`);
  if (report.pages.length === 0) throw new Error("the report has no pages");
  return doc.save();
}
