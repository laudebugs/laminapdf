// A report whose title, paragraphs and photo each start below wherever the one before ended. Each
// text box reports its `bottom`, and the next element starts a gap below it, so a summary that
// wraps moves everything after it and empty notes take no space. The body continues on new pages
// from the top margin. Needs `init({ text: true })`.
import { Builder, type BuilderOptions, Font, Image, PAGE_SIZES, type Surface, type TextLayout } from "@laminapdf/sdk";

const [PAGE_WIDTH, PAGE_HEIGHT] = PAGE_SIZES.A5;
const MARGIN = 48;
const WIDTH = PAGE_WIDTH - 2 * MARGIN;
const BOTTOM = PAGE_HEIGHT - MARGIN;
const TITLE = 18;
const BODY = 11;
const GAP = 10;
/** The photo's height, for a 3:2 photo across the text column. */
const PHOTO_HEIGHT = WIDTH / 1.5;

/** What the report says. Empty `notes` take no space; blank lines separate the body's paragraphs. */
export interface Report {
  readonly title: string;
  readonly summary: string;
  readonly notes: string;
  readonly body: string;
}

/**
 * The report's bytes, with `photo` (a JPEG) below its notes.
 *
 * @param options - can fix the file ID, the producer and the creation date.
 */
export function elementFlow(photo: Uint8Array, report: Report, options: BuilderOptions = {}): Uint8Array<ArrayBuffer> {
  using b = new Builder(options);
  const fonts = { bold: Font.standard("Helvetica-Bold"), regular: Font.standard("Helvetica") };
  const jpeg = Image.jpeg(photo);
  let page = b.page("A5");
  let y = header(page.surface(), fonts, jpeg, report);
  let [rest, newPage] = [report.body.trimEnd(), false];
  for (;;) {
    const layout = page.surface().textBox(rest, { font: fonts.regular, size: BODY, box: area(y) });
    // A clipped box still drew its first lines: the rest goes on a new page, from its top.
    const cut = layout.overflow;
    if (cut === undefined) break;
    if (cut === 0 && newPage) throw new Error("a line of the body is taller than the page");
    [rest, y, newPage] = [rest.slice(cut).trimStart(), MARGIN, true];
    page.finish();
    page = b.page("A5");
  }
  page.finish();
  using doc = b.finish();
  return doc.save();
}

/**
 * The title with a rule under it, the summary, the notes and the photo, each below the one before;
 * returns where the body starts. Text that doesn't fit throws `text-overflow`.
 */
function header(s: Surface, fonts: { bold: Font; regular: Font }, photo: Image, report: Report): number {
  const fit = (text: string, font: Font, size: number, top: number) =>
    below(s.textBox(text, { font, size, box: area(top), overflow: "error" }), top);
  let y = fit(report.title, fonts.bold, TITLE, MARGIN);
  // A decoration half a gap above the summary. It's stroked: a fill colour would colour the text too.
  s.strokeColor([0.6])
    .path((p) => p.rect([MARGIN, y - GAP / 2, WIDTH, 0]))
    .stroke();
  for (const text of [report.summary, report.notes]) y = fit(text, fonts.regular, BODY, y);
  if (y + PHOTO_HEIGHT > BOTTOM) throw new Error("the photo doesn't fit below the notes");
  s.image(photo, [MARGIN, y, WIDTH, PHOTO_HEIGHT]);
  return y + PHOTO_HEIGHT + GAP;
}

/** Where the next element starts: a gap below `layout`'s last drawn line, or `y` if it drew none. */
function below(layout: TextLayout, y: number): number {
  return layout.bottom === undefined ? y : layout.bottom + GAP;
}

/** The rest of the page's text column, from `y` down to the bottom margin. */
function area(y: number): [number, number, number, number] {
  return [MARGIN, y, WIDTH, Math.max(BOTTOM - y, 0)];
}
