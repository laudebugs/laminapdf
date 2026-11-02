// The smallest document: one A4 page with a line of text in a standard 14 font.
import { Builder, type BuilderOptions, Font } from "@laminapdf/sdk";

/**
 * The document's bytes. Needs `init({ text: true })`.
 *
 * @param options - can fix the file ID, the producer and the creation date, so that the same
 *   input gives the same bytes on every run.
 */
export function hello(options: BuilderOptions = {}): Uint8Array<ArrayBuffer> {
  using b = new Builder(options);
  const page = b.page("A4");
  // Page space: points from the top-left corner, y down; `at[1]` is the text's baseline.
  page.surface().text("Hello, world!", { font: Font.standard("Helvetica"), size: 24, at: [72, 96] });
  page.finish();
  using doc = b.finish();
  return doc.save();
}
