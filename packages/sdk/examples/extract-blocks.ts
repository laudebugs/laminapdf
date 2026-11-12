// List a PDF's blocks with where they sit: each heading, paragraph, list item and table, its page
// and its box in points from the page's top-left corner.
import { Document, type ExtractedBlock } from "@laminapdf/sdk";

/**
 * One line per block: its page number, its kind, its box (left, top, width, height) and its text.
 * Needs `init({ extract: true })`.
 */
export function blocksOf(bytes: Uint8Array): string {
  using doc = Document.open(bytes);
  const extraction = doc.extract({ format: "json" });
  return extraction.blocks
    .map((block) => {
      const box = block.bbox.map((v) => Math.round(v)).join(" ");
      const text = block.text.replace(/[\t\n]/g, " ");
      return `page ${block.page + 1} ${kind(block)} [${box}] ${text}\n`;
    })
    .join("");
}

/** The block's kind as the JSON names it, with a heading's level. */
function kind(block: ExtractedBlock): string {
  return block.type === "heading" ? `heading ${block.level}` : block.type.replaceAll("-", " ");
}
