// Print a summary of a review: a new PDF that lists every comment, with each page drawn small
// beside its comments and their numbers marked on it.
// The summary is tagged and conforms to PDF/UA-1, so a screen reader reads it in order. The
// reviewed file doesn't change.
import { Document, Font } from "@laminapdf/sdk";

/**
 * A summary of the comments on `bytes`, set in the font `font`, which the summary embeds. Needs
 * `init({ text: true })`.
 */
export function summariseComments(bytes: Uint8Array, font: Uint8Array): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);
  using summary = doc.commentSummary({ font: Font.fromBytes(font) });
  // A page whose fonts the file doesn't embed is listed without its drawing.
  for (const warning of summary.warnings) {
    console.warn(warning.code);
  }
  return summary.save();
}
