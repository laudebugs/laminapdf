// Photos into a PDF, one A4 page each, fitted inside the margins.
//
// Each JPEG is embedded as it is, without decoding or compressing it again. A page turns to
// landscape for a wide photo, and a photo that its camera marked as taken sideways is drawn
// upright.
import { Image, type ImagesToPdfOptions, imagesToPdf } from "@laminapdf/sdk";

/** The space around each photo, in millimetres. */
const MARGIN_MM = 15;
/** Points in an inch, and millimetres in an inch. */
const POINTS_PER_INCH = 72;
const MM_PER_INCH = 25.4;

/**
 * The document's bytes, one page per photo.
 *
 * @param photos - the JPEGs, one per page, in order.
 * @param options - can fix the file ID, the producer and the creation date.
 */
export function imagesToPdfBytes(
  photos: readonly Uint8Array[],
  options: Omit<ImagesToPdfOptions, "fit" | "margin"> = {},
): Uint8Array<ArrayBuffer> {
  using doc = imagesToPdf(
    photos.map((photo) => Image.jpeg(photo)),
    { ...options, fit: "A4", margin: (MARGIN_MM * POINTS_PER_INCH) / MM_PER_INCH },
  );
  return doc.save();
}
