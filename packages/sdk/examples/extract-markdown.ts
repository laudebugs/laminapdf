// Extract a PDF's text for an LLM: Markdown in reading order, JSON with a box for every block,
// and chunks sized for retrieval.
//
// Tables drawn with lines come out as Markdown tables, and text a reader can't see (white,
// invisible or off the page) is left out of all three.
import { type Chunk, Document, type Extraction } from "@laminapdf/sdk";

/** What a PDF gives an LLM or a search index. */
export interface ForAnLlm {
  /** The text in reading order, with a `<!-- page N -->` comment before each page. */
  readonly markdown: string;
  /** Every block with its page and box, the text left out, the file's warnings and the chunks. */
  readonly json: Extraction;
  /** About 400 tokens each, a new one at each heading. */
  readonly chunks: readonly Chunk[];
}

/** The Markdown, the JSON and the chunks of `bytes`. Needs `init({ extract: true })`. */
export function extractForAnLlm(bytes: Uint8Array): ForAnLlm {
  using doc = Document.open(bytes);
  // Page markers let a reader cite the page a passage came from.
  const markdown = doc.extract({ format: "markdown", pageMarkers: true });
  // The JSON carries the same chunks, and each block's place in the Markdown.
  const json = doc.extract({ format: "json", pageMarkers: true, chunk: { maxTokens: 400 } });
  return { markdown, json, chunks: json.chunks ?? [] };
}
