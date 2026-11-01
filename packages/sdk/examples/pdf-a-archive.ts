// An archival letter, written twice: as PDF/A-2b, and as PDF/A-2u that is also PDF/UA-1. Both
// embed their font, use only RGB and gray colour (covered by the sRGB output intent the profile
// writes), place a JPEG photo and a translucent band, and carry the identification in their XMP
// packet. The TS twin of crates/lamina/examples/pdf_a_archive.rs.
import { Builder, Font, Image } from "@laminapdf/sdk";

const BODY =
  "Thank you for your order of 12 sprockets. This letter confirms the delivery date and the price we agreed. " +
  "We keep a copy in our archive, and you can keep this one: PDF/A files look the same in any viewer, now and in the future.";

/**
 * The document's bytes. Needs `init({ text: true })`.
 *
 * @param level - `2b` (PDF/A-2b) or `2u` (PDF/A-2u and PDF/UA-1).
 * @param notoSans - `fixtures/fonts/NotoSans-Regular.ttf`.
 * @param photo - `fixtures/images/photo-rgb.jpg`.
 */
export function pdfAArchive(level: "2b" | "2u", notoSans: Uint8Array, photo: Uint8Array): Uint8Array {
  const b = new Builder({
    id: new TextEncoder().encode("lamina-ex-pdfa-2"),
    producer: "lamina examples",
    creationDate: "D:20261031120000Z",
    ...(level === "2b" ? { profiles: ["PDF/A-2b"], tagged: true } : { profiles: ["PDF/A-2u", "PDF/UA-1"] }),
  });
  b.metadata({ title: "Order confirmation 42", author: "Acme Widgets Ltd" }).lang("en-GB");
  const font = Font.fromBytes(notoSans);
  b.font(font);
  const jpeg = Image.jpeg(photo);
  const page = b.page("A4");
  const s = page.surface();
  s.artifact("layout", (s) => {
    s.save()
      .opacity(0.35, 1)
      .fillColor([0.12, 0.47, 0.78])
      .path((p) => p.rect([0, 0, 595.28, 70]))
      .fill()
      .restore();
  });
  s.text("Order confirmation 42", { font, size: 22, at: [56, 110], tag: "H1" });
  s.tag("P", (s) => s.textBox(BODY, { font, size: 11, box: [56, 130, 300, 120] }));
  s.image(jpeg, [380, 130, 160, 120], "A photograph of the sprockets, packed in a box");
  s.tag("P", (s) => s.text("Acme Widgets Ltd, 1 Example Street, London", { font, size: 9, at: [56, 300] }));
  page.finish();
  return b.finish().save();
}
