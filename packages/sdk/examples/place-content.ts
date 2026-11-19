// Brand a report: a pale band behind its title, the company's logo over the band's right end,
// and its wordmark, a page of another PDF kept as vectors and text, at the foot of the page.
// Each is placed as page content, not as a comment: it prints and every reader draws it. The
// boxes are in page space (points from the page's top-left), as a viewer reports where a
// pointer went. In this tagged report the logo and the wordmark are figures with alternate
// text, and the band is decoration, which screen readers skip.
import { Document } from "@laminapdf/sdk";

// Where each goes on the example's input, the quarterly report, in points from the page's
// top-left: for your own file, take them from your viewer's pointer.
/** The band behind the title: `[x0, y0, x1, y1]`. */
const BAND = [44, 68, 551, 110] as const;
/** The logo, over the band's right end, as wide again as it is tall, as the image is. */
const LOGO = [455, 65, 551, 113] as const;
/** The wordmark, at the foot of the page, the size of its page. */
const WORDMARK = [56, 740, 176, 800] as const;

/** The matrix that stretches an image's or a page's unit square to the box `[x0, y0, x1, y1]`. */
function to([x0, y0, x1, y1]: readonly [number, number, number, number]) {
  return [x1 - x0, 0, 0, y1 - y0, x0, y0] as const;
}

/**
 * The document branded: a band, a logo and a wordmark on its first page, saved incrementally.
 * `at` is when it was branded, as a PDF date.
 */
export function placeContent(
  bytes: Uint8Array,
  logo: Uint8Array,
  wordmark: Uint8Array,
  at: string,
): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);

  // Behind the page's own content, so the title stays on top of it.
  doc.content.add(0, {
    id: "title-band",
    object: {
      type: "shape",
      path: { type: "rect", rect: [...BAND], radius: 6 },
      fill: [0.89, 0.93, 0.98],
    },
    matrix: [1, 0, 0, 1, 0, 0],
    layer: "behind",
    alt: "decorative",
    modified: at,
  });

  doc.content.add(0, {
    id: "logo",
    object: { type: "image", blob: doc.addBlob(logo) },
    matrix: to(LOGO),
    alt: { text: "Northwind Traders" },
    modified: at,
  });

  doc.content.add(0, {
    id: "wordmark",
    object: { type: "form", blob: doc.addBlob(wordmark), page: 0 },
    matrix: to(WORDMARK),
    alt: { text: "Northwind, the company's wordmark" },
    modified: at,
  });

  return doc.save({ mode: "incremental" });
}
