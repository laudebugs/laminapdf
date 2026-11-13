---
title: "Using Document"
description: "The tasks a Document is for, from reading pages to editing and saving, each with a short example, and the mistakes to avoid."
version: "0.2.0"
url: "https://docs.laminapdf.com/reference/document-usage/"
---

# Using Document

The tasks a Document is for, from reading pages to editing and saving, each with a short example, and the mistakes to avoid.

A `Document` is an open PDF: you read it, edit it and save it. Each task below has a short example and a link to the guide that goes further. The [`Document` reference](https://docs.laminapdf.com/reference/sdk/classes/document/) lists every method and option.

## Open a file and read it

`Document.open(bytes)` reads a file from bytes you already hold, and every read, edit and save on the document it returns is synchronous. This example opens a file and reads what an app's file list would show: each page's size, the title and authors, the first page's text and the warnings the file raised:

`packages/sdk/examples/read-document.ts`:

```ts
// Read a PDF without changing it: its pages and their sizes, its title and authors, the text of its
// first page, and the warnings opening it raised. The text is read only when the text or extract
// module was loaded (`init({ text: true })` or `init({ extract: true })`) before the file opened.
import { Document } from "@laminapdf/sdk";

/** What a file holds, as an app's file list or search index might show it. */
export interface Summary {
  /** Each page's displayed size in points (1/72 inch), and its label when the file names one. */
  readonly pages: readonly { readonly width: number; readonly height: number; readonly label?: string }[];
  readonly title: string | undefined;
  readonly authors: readonly string[];
  /** `undefined` without a page, or when the file opened before the text or extract module loaded. */
  readonly firstPageText: string | undefined;
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
  let firstPageText: string | undefined;
  if (doc.canExtractText && doc.pageCount > 0) {
    using first = doc.page(0);
    firstPageText = first.text().text();
  }
  // Last, since reading the pages and their text can add warnings.
  return { pages, title, authors, firstPageText, warnings: doc.warnings.map((w) => w.code) };
}
```

- `doc.pages` reads every page's facts in one pass, and `doc.pageInfo(i)` reads one. Sizes are in points (1/72 inch), as the page is displayed.
- `doc.metadata()` reads the title, authors, subject, keywords and dates. A PDF keeps these twice, and the result also lists the fields where the two copies disagree.
- `doc.outline()`, `doc.pageLinks(i)` and `doc.properties()` read the bookmarks, a page's links, and what a properties dialog shows. Like the text reads below, they need the text or extract module.
- `doc.warnings` lists what the file needed repaired as it was read. A warning means the file opened, but not exactly as written; [errors and warnings](../../pages/concepts/errors-and-warnings.md) explains each kind.

A file that needs a password, or that comes from someone you don't trust, takes options: `Document.open(bytes, { password, limits })`. [Open files you don't trust](../../pages/guides/untrusted-files.md) has an example that turns each failure into a value your app can show.

## Read a page's text, or the whole document's

`doc.page(i).text()` returns a page's text by line, with each glyph's position for search highlights and selection. `doc.extract({ format: "markdown" })` reads the whole document in reading order, as Markdown, plain text, JSON or chunks for retrieval. Both need the text or extract module loaded before the file is opened; pages, metadata, warnings, edits and saves work in every module. [Extract text for an LLM](../../pages/recipes/extract-markdown.md) shows the four formats.

## Edit it in a transaction

An edit is an op, such as `rotatePage` or `setMetadata`, and `doc.transact(tx => …)` applies a group of them together: all of them, or none. This example turns a scan's landscape pages upright and saves the edit:

`packages/sdk/examples/rotate-pages.ts`:

```ts
// Turn a scan's landscape pages upright, in one transaction, and save the edit. A page is named by
// its ID, which stays the same when other pages move.
import { Document, type Rotation } from "@laminapdf/sdk";

/** A quarter turn clockwise from `degrees`. */
function quarterTurn(degrees: number): Rotation {
  return ((degrees + 90) % 360) as Rotation;
}

/** `bytes` with every page wider than it is tall turned a quarter clockwise. Needs `init()`. */
export function rotateLandscapePages(bytes: Uint8Array): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);
  // A page's width and height are as displayed, with its current rotation applied.
  const landscape = doc.pages.filter((page) => page.width > page.height);
  // One transaction: every page turns, or none does, and one `undo()` reverts them all.
  doc.transact((tx) => {
    for (const page of landscape) tx.rotatePage(page.id, quarterTurn(page.rotation));
  });
  // The default save mode appends the edit after the original bytes when the file allows it, and
  // writes the whole file again when it doesn't, as for a file that needed repair. An edited
  // encrypted file is refused (`encrypted-base`) unless you pass `removeEncryption: true`.
  return doc.save();
}
```

`doc.undo()` reverts the newest transaction as a whole, and `doc.redo()` applies it again. Every op is also plain JSON, so `doc.apply(ops)` and `doc.applyLog(log)` replay edits your app stored. [Edit a PDF](../../pages/guides/edit-a-pdf.md) covers undo, metadata and stored edits.

## Save it

`doc.save()` returns the file's bytes. The `mode` option chooses between a revision appended to the original bytes (`incremental`), the whole file written again (`fullRewrite`), and the first of the two the file allows (`auto`, the default). This example generates a draft, edits it and saves the edit incrementally:

**TypeScript**

`packages/sdk/examples/generate-then-edit.ts`:

```ts
// Generate a document, then open its bytes, edit them and save incrementally. The edit changes
// the metadata in one transaction, and the incremental save keeps the generated bytes as they
// are, with one revision appended.
import { Builder, type BuilderOptions, Document, Font } from "@laminapdf/sdk";

/**
 * The generated file followed by the edit's revision. Needs `init({ text: true })`.
 *
 * @param options - can fix the file ID, the producer and the creation date.
 * @param editedAt - the modification date to record, as a PDF date string.
 */
export function generateThenEdit(options: BuilderOptions = {}, editedAt?: string): Uint8Array<ArrayBuffer> {
  return edit(generate(options), editedAt);
}

/** A one-page draft, saved (a new document is always saved as a full rewrite). */
export function generate(options: BuilderOptions = {}): Uint8Array<ArrayBuffer> {
  using b = new Builder(options);
  const page = b.page("A4");
  page.surface().text("Draft report", { font: Font.standard("Helvetica"), size: 20, at: [72, 96] });
  page.finish();
  b.metadata({ title: "Draft report" });
  using doc = b.finish();
  return doc.save();
}

/** Opens `bytes`, edits the metadata in one transaction (which `undo` could revert) and saves incrementally. */
function edit(bytes: Uint8Array, editedAt?: string): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);
  doc.transact((tx) =>
    tx.setMetadata(
      { title: "Final report", authors: ["Ada Lovelace"] },
      editedAt === undefined ? {} : { at: editedAt },
    ),
  );
  return doc.save({ mode: "incremental" });
}
```

**Rust**

`crates/laminapdf/examples/generate_then_edit.rs`:

````rust
//! Generate a document, then open its bytes, edit them and save incrementally.
//!
//! The edit changes the metadata with an op (undoable and serialisable), and the
//! incremental save keeps the generated bytes as they are, with one revision appended.
//!
//! ```sh
//! cargo run -p laminapdf --example generate_then_edit -- [out-dir]   # writes <out-dir>/generate_then_edit.pdf
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{
    BuildOptions, Builder, Document, Metadata, MetadataPatch, Op, OpenOptions, PageSize, PdfDate, PointPt, Pt,
    SaveMode, SaveOptions, SetMetadata, Standard14,
};

fn main() -> Result<(), Box<dyn Error>> {
    let out = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let bytes = build(BuildOptions::new(), None)?;
    std::fs::write(out.join("generate_then_edit.pdf"), bytes)?;
    Ok(())
}

/// The generated file followed by the edit's revision. `edited_at` is recorded as the
/// modification date; `options` can fix the file ID, the producer and the creation date.
pub(crate) fn build(options: BuildOptions, edited_at: Option<PdfDate>) -> Result<Vec<u8>, Box<dyn Error>> {
    let generated = generate(options)?;
    edit(generated, edited_at)
}

/// A one-page draft, saved (a new document is always saved as a full rewrite).
pub(crate) fn generate(options: BuildOptions) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut b = Builder::new(options);
    let helvetica = b.standard_font(Standard14::Helvetica)?;
    let mut page = b.page(PageSize::A4);
    page.surface().text(
        "Draft report",
        &helvetica,
        Pt::new(20.0),
        PointPt::from_points(72.0, 96.0),
    )?;
    page.finish()?;
    b.metadata(Metadata::default().with_title("Draft report"));
    Ok(b.finish()?.save(&SaveOptions::new())?.bytes)
}

/// Opens `bytes`, edits the metadata (the Info dictionary and the XMP packet together) in one
/// transaction (which `undo` could revert) and saves incrementally: `bytes` unchanged, then the
/// appended revision.
fn edit(bytes: Vec<u8>, edited_at: Option<PdfDate>) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;
    let patch = MetadataPatch::new()
        .title(Some("Final report"))
        .authors(&["Ada Lovelace"]);
    doc.apply(vec![Op::from(SetMetadata::new(patch, edited_at))])?;
    let options = SaveOptions::new().with_mode(SaveMode::Incremental);
    Ok(doc.save(&options)?.bytes)
}
````

**Result:** [generate_then_edit.pdf](https://docs.laminapdf.com/examples/pdf/generate_then_edit.pdf)

A signed, repaired or encrypted file limits how it can be saved. `doc.plan(options)` says how a save would go, or why it would be refused, before anything is written. [Save a PDF](../../pages/guides/save-a-pdf.md) explains each case.

## Open a large file from a server

`await Document.openRange(httpRangeSource(url))` downloads only the parts of a file it reads. A read that needs bytes that haven't arrived throws `data-pending`, so wrap each read in `await doc.ensure(() => …)`, which fetches them and runs the read again. [Open large files by ranges](../../pages/guides/open-large-files.md) has the example.

## Mistakes to avoid

- **A document you don't free holds its memory.** A document lives in WebAssembly memory, which the garbage collector frees late. Open it with `using doc = …` so it is freed at the end of the block, or call `doc.free()`.
- **A document stays in the module it opened in.** A file opened before `init({ text: true })` or `loadExtract()` can't read text, the outline, links, properties or the structure tree: those calls fail with `extract-module-not-loaded`. Load the module first; `doc.canExtractText` says whether a document can.
- **A page index can change; its ID doesn't.** Ops name pages by `id`, as in `tx.rotatePage(page.id, 90)`. `doc.page(i)` returns a handle that follows its page by ID, so it still points at the right page after pages move.
- **A transaction's callback is synchronous.** `transact` applies the ops when the callback returns, so reads inside it see the document before the edit. An `async` callback is refused, and editing or saving the same document inside it fails with `nested-transaction`.
- **Saving marks the document saved.** After `doc.save()`, later edits and saves build on its output, and `hasUnsavedChanges` is false. For a download that shouldn't count, use `doc.saveCopy()`, which leaves the edits unsaved.
- **Removed metadata stays in earlier revisions.** An [incremental save](../../pages/reference/glossary.md#incremental-save) keeps the original bytes, old values included. To remove them from the file, save with `mode: "fullRewrite"`. `removeMetadata`'s result sets `stillInEarlierRevisions` when old values remain in earlier revisions.
- **An edited encrypted file needs a decision.** Every encrypted file counts, including one that opens without a password (`doc.isEncrypted`). It saves only as a [full rewrite](../../pages/reference/glossary.md#full-rewrite) without its encryption, and only with `removeEncryption: true`. Without it, every mode fails with `encrypted-base`.
- **Treat the file's text as untrusted.** Titles, authors and page text come from whoever made the file. Escape them before you put them in a web page. Lamina PDF's own error messages never contain the file's text.

Next: [using Builder](../../pages/reference/builder-usage.md).
