// Split a PDF into chunks for retrieval: whole paragraphs and tables up to a token budget, a new
// chunk at each heading, each with the headings above it and the pages it came from.
import { Document } from "@laminapdf/sdk";

/** Each chunk of `bytes`: its headings, its pages, its size and its text. Needs `init({ extract: true })`. */
export function chunksOf(bytes: Uint8Array): string {
  using doc = Document.open(bytes);
  // A small budget, so this short file gives several chunks; 400 to 800 suits most indexes.
  const chunks = doc.extract({ format: "chunks", chunk: { maxTokens: 60 } });
  return chunks
    .map((chunk) => {
      const pages = chunk.pages.map((p) => p + 1).join(", ");
      return (
        `${chunk.id} under "${chunk.headingPath.join(" > ")}"\n` +
        `pages ${pages}, about ${chunk.tokensApprox} tokens\n` +
        `${chunk.text.trimEnd()}\n\n`
      );
    })
    .join("");
}
