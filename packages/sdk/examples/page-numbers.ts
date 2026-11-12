// A document of several pages, each with a heading and a footer that reads "Page 2 of 3". A page
// takes no more drawing once it is finished, so the page count is known before the first page is
// drawn.
import { Builder, type BuilderOptions, Font, PAGE_SIZES } from "@laminapdf/sdk";

const MARGIN = 72;
const HEADING_SIZE = 20;
const FOOTER_SIZE = 10;
const [, PAGE_HEIGHT] = PAGE_SIZES.A4;

/**
 * One A4 page per heading in `headings`. Needs `init({ text: true })`.
 *
 * @param options - can fix the file ID, the producer and the creation date.
 */
export function pageNumbers(headings: readonly string[], options: BuilderOptions = {}): Uint8Array<ArrayBuffer> {
  using b = new Builder(options);
  const font = Font.standard("Helvetica");
  headings.forEach((heading, index) => {
    const page = b.page("A4");
    const s = page.surface();
    // `at` is the baseline's start, in points from the page's top-left corner.
    s.text(heading, { font, size: HEADING_SIZE, at: [MARGIN, MARGIN + HEADING_SIZE] });
    const footer = `Page ${index + 1} of ${headings.length}`;
    s.text(footer, { font, size: FOOTER_SIZE, at: [MARGIN, PAGE_HEIGHT - MARGIN / 2] });
    // A page joins the document only once it is finished.
    page.finish();
  });
  b.metadata({ title: headings[0] ?? "Untitled" });
  using doc = b.finish();
  return doc.save();
}
