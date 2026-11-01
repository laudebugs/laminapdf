// An archival letter, written as PDF/A-2b and as PDF/A-2u that is also PDF/UA-1.
import { Builder, Font, Image, PAGE_SIZES, type Profile, type Surface } from "@laminapdf/sdk";

const MARGIN = 56;
const BODY = 11;
const BAND_OPACITY = 0.35;
const PHOTO_ALT = "A photograph of the sprockets, packed in a box";
const LETTER =
  "Thank you for your order of 12 sprockets. This letter confirms the delivery date and the price we agreed. " +
  "Keep it: PDF/A files look the same in any viewer, now and in the future.";

/** PDF/A-2b, or PDF/A-2u (whose text maps to Unicode) with PDF/UA-1. */
const PROFILES: Record<"2b" | "2u", Profile[]> = {
  "2b": ["PDF/A-2b"],
  "2u": ["PDF/A-2u", "PDF/UA-1"],
};

/**
 * The letter's bytes. Needs `init({ text: true })`.
 *
 * @param level - `2b` or `2u`.
 * @param notoSans - the bytes of the Noto Sans font file.
 * @param photo - the bytes of a JPEG photo.
 */
export function pdfAArchive(level: "2b" | "2u", notoSans: Uint8Array, photo: Uint8Array): Uint8Array {
  using b = new Builder({ profiles: PROFILES[level], tagged: true });
  b.metadata({ title: "Order confirmation 42", author: "Acme Widgets Ltd" });
  b.lang("en-GB");
  const font = Font.fromBytes(notoSans);
  b.font(font);
  const page = b.page("A4");
  draw(page.surface(), font, Image.jpeg(photo));
  page.finish();
  using doc = b.finish();
  return doc.save();
}

function draw(s: Surface, font: Font, photo: Image): void {
  // A translucent band across the top: decoration, and transparency, which PDF/A-2 allows.
  const [width] = PAGE_SIZES.A4;
  s.artifact("layout", (s) => {
    s.save().opacity(BAND_OPACITY, 1).fillColor([0.12, 0.47, 0.78]);
    s.path((p) => p.rect([0, 0, width, 70])).fill();
    s.restore();
  });
  s.tag("H1", (s) => s.text("Order confirmation 42", { font, size: 22, at: [MARGIN, 110] }));
  s.tag("P", (s) => s.textBox(LETTER, { font, size: BODY, box: [MARGIN, 130, 300, 120] }));
  s.image(photo, [380, 130, 160, 120], PHOTO_ALT);
  s.tag("P", (s) => s.text("Acme Widgets Ltd, 1 Example Street, London", { font, size: BODY, at: [MARGIN, 300] }));
}
