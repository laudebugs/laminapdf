// Photos into a PDF, one A4 page each, fitted inside the margins.
//
// Each JPEG is embedded as it is, without decoding or compressing it again. A page turns to
// landscape for a wide photo, and a photo that its camera marked as taken sideways is drawn
// upright.
import { type BuilderOptions, Image, imagesToPdf } from "@laminapdf/sdk";

/** The space around each photo, in millimetres. */
const MARGIN_MM = 15;

/**
 * The document's bytes, one page per photo.
 *
 * @param photos - the JPEGs, one per page, in order.
 * @param options - can fix the file ID, the producer and the creation date.
 */
export function imagesToPdfBytes(photos: readonly Uint8Array[], options: BuilderOptions = {}): Uint8Array<ArrayBuffer> {
  const { text: _text, ...build } = options;
  using doc = imagesToPdf(
    photos.map((photo) => Image.jpeg(photo)),
    { ...build, fit: "A4", margin: (MARGIN_MM * 72) / 25.4 },
  );
  return doc.save();
}
