// Read a PDF without changing it: its pages and their sizes, its title and authors, the text of its
// first page, and the warnings opening it raised. Reading the text needs `init({ text: true })` or
// `init({ extract: true })` before the file is opened.
import { Document } from "@laminapdf/sdk";

/** What a file holds, as an app's file list or search index might show it. */
export interface Summary {
  /** Each page's displayed size in points (1/72 inch), and its label when the file names one. */
  readonly pages: readonly { readonly width: number; readonly height: number; readonly label?: string }[];
  readonly title: string | undefined;
  readonly authors: readonly string[];
  readonly firstPageText: string;
  /** Codes of what opening and reading had to repair. */
  readonly warnings: readonly string[];
}

/** Opens `bytes` and reads what {@link Summary} lists. */
export function readDocument(bytes: Uint8Array): Summary {
  using doc = Document.open(bytes);
  // `pages` walks the page tree once; `pageInfo(i)` reads one page.
  const pages = doc.pages.map(({ width, height, label }) => ({
    width,
    height,
    ...(label === undefined ? {} : { label }),
  }));
  // Info and XMP metadata read together: the two places a PDF keeps its title.
  const { title, authors } = doc.metadata();
  let firstPageText = "";
  if (doc.pageCount > 0) {
    using first = doc.page(0);
    firstPageText = first.text().text();
  }
  // Last, since reading the pages and their text can add warnings.
  return { pages, title, authors, firstPageText, warnings: doc.warnings.map((w) => w.code) };
}
