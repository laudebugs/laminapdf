---
title: "Merge and split PDFs"
description: "Merge several PDFs into one with a bookmark for each file, or split a PDF into one file per chapter."
version: "0.4.0"
url: "https://docs.laminapdf.com/recipes/merge-and-split/"
---

# Merge and split PDFs

Merge several PDFs into one with a bookmark for each file, or split a PDF into one file per chapter.

You'll merge two PDFs into one, then split a book into one file per chapter. Use the first to send a report with its appendices as one file, and the second to send each reader only the chapter they need.

## Merge files into one

`Document.merge` (`Document::merge` in Rust) takes each file's bytes and a name for it, and returns a new document with every file's pages in order:

**TypeScript**

`packages/sdk/examples/merge-documents.ts`:

```ts
// Merge PDFs into one, with a bookmark for each file.
//
// Each file's own bookmarks go under a bookmark named after it, its form fields keep working
// (renamed when two files use one name), and its pages keep their labels.
import { Document } from "@laminapdf/sdk";

/** One document of `files` (each a name and its bytes), in order, with a bookmark per file. Needs `init()`. */
export function mergeDocuments(files: readonly (readonly [string, Uint8Array])[]): Uint8Array<ArrayBuffer> {
  using merged = Document.merge(files.map(([title, bytes]) => ({ bytes, title })));
  return merged.save();
}
```

**Rust**

`crates/laminapdf/examples/merge_documents.rs`:

````rust
//! Merge PDFs into one, with a bookmark for each file.
//!
//! Each file's own bookmarks go under a bookmark named after it, its form fields keep working
//! (renamed when two files use one name), and its pages keep their labels.
//!
//! ```sh
//! cargo run -p laminapdf --example merge_documents -- a.pdf b.pdf [out-dir]
//! ```

use std::error::Error;
use std::path::{Path, PathBuf};

use laminapdf::{Document, MergeInput, MergeOptions, SaveOptions};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<PathBuf> = std::env::args().skip(1).map(PathBuf::from).collect();
    let [first, second, rest @ ..] = args.as_slice() else {
        return Err("usage: merge_documents a.pdf b.pdf [out-dir]".into());
    };
    let out = rest.first().cloned().unwrap_or_else(|| "target/examples-out".into());
    std::fs::create_dir_all(&out)?;
    let files = [
        (name(first), std::fs::read(first)?),
        (name(second), std::fs::read(second)?),
    ];
    std::fs::write(out.join("merge_documents.pdf"), merge(files.to_vec())?)?;
    Ok(())
}

fn name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// One document of `files` (each a name and its bytes), in order, with a bookmark per file.
pub(crate) fn merge(files: Vec<(String, Vec<u8>)>) -> Result<Vec<u8>, Box<dyn Error>> {
    let inputs = files
        .into_iter()
        .map(|(name, bytes)| MergeInput::new(bytes).with_title(name))
        .collect();
    let mut merged = Document::merge(inputs, MergeOptions::default())?;
    Ok(merged.save(&SaveOptions::new())?.bytes)
}
````

**Result:** [merge_documents.pdf](https://docs.laminapdf.com/examples/pdf/merge_documents.pdf)

The merged file has six pages: the five of `references.pdf`, then the one of `hello.pdf`. Its bookmarks start with one per file, named as you named the file, and `references.pdf`'s own bookmarks sit under its bookmark.

## Split a file by chapter

`split` cuts a document where each top-level bookmark's page begins and returns one new document per chapter:

**TypeScript**

`packages/sdk/examples/split-document.ts`:

```ts
// Split a PDF into one file per chapter.
//
// The file is cut where each top-level bookmark's page begins, and each part keeps the
// bookmarks of its chapter.
import { Document } from "@laminapdf/sdk";

/** One file per chapter of `bytes`: a part from each top-level bookmark's page to the next. Needs `init()`. */
export function splitDocument(bytes: Uint8Array): Uint8Array<ArrayBuffer>[] {
  using doc = Document.open(bytes);
  const chapters = doc.split({ bookmarks: 1 });
  try {
    return chapters.map((chapter) => chapter.save());
  } finally {
    for (const chapter of chapters) chapter.free();
  }
}
```

**Rust**

`crates/laminapdf/examples/split_document.rs`:

````rust
//! Split a PDF into one file per chapter.
//!
//! The file is cut where each top-level bookmark's page begins, and each part keeps the
//! bookmarks of its chapter.
//!
//! ```sh
//! cargo run -p laminapdf --example split_document -- book.pdf [out-dir]
//! ```

use std::error::Error;
use std::num::NonZeroUsize;
use std::path::PathBuf;

use laminapdf::{Document, OpenOptions, SaveOptions, SplitBy};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let input = PathBuf::from(args.next().ok_or("usage: split_document book.pdf [out-dir]")?);
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    for (i, part) in split(std::fs::read(input)?)?.into_iter().enumerate() {
        std::fs::write(out.join(format!("split_document-{}.pdf", i + 1)), part)?;
    }
    Ok(())
}

/// One file per chapter of `bytes`: a part from each top-level bookmark's page to the next.
pub(crate) fn split(bytes: Vec<u8>) -> Result<Vec<Vec<u8>>, Box<dyn Error>> {
    let doc = Document::open_bytes(bytes, OpenOptions::new())?;
    let chapters = doc.split(&SplitBy::Bookmarks {
        level: NonZeroUsize::MIN,
    })?;
    let mut files = Vec::with_capacity(chapters.len());
    for mut chapter in chapters {
        files.push(chapter.save(&SaveOptions::new())?.bytes);
    }
    Ok(files)
}
````

**Result:** [split_document-1.pdf](https://docs.laminapdf.com/examples/pdf/split_document-1.pdf)

The book has three chapters of four pages, so the program makes three files. Each keeps its chapter's bookmark and the two section bookmarks under it.

## How it works

- Each file's pages come in as copied pages do: with their annotations, form fields and page labels, and with the named destinations their links use. [Copy pages between PDFs](../../pages/recipes/copy-pages.md) explains the rules.
- When two files use one form field name, the later file's field becomes `<name>#2`. A named destination that two files use is renamed the same way, and the bookmarks and links that used it follow the new name.
- Layers keep the visibility each file gave them.
- A merged file is a new document, so saving it writes a whole file with no password. An encrypted file needs its `password` beside its bytes.
- A part of a split holds only what its pages use, as a document from `extractPages` does.

## Merge tagged PDFs

Merged pages keep their tags when the result can stay tagged throughout: when you merge into a tagged document, or make a new document from files that are all tagged. Each file's elements go under a `Part` element named after it, so a screen reader reads the files in order. When one of the files is untagged, a new document comes out untagged, since a tagged file must tag all its content.

On a document that claims PDF/UA-1, merging a file without that claim warns with [`imported-pdfua-unchecked`](../../errors/imported-pdfua-unchecked.md). A document that claims a PDF/A level warns with [`imported-pdfa-unchecked`](../../errors/imported-pdfa-unchecked.md) when a merged file doesn't claim the same level, so check the saved file with a PDF/A validator.

## What to change

- **Keep the files' bookmarks at the top level:** pass `{ outline: "keep" }` as the options. `{ outline: "none" }` leaves them out.
- **Merge into an open document:** add each file with `addSource`, then call `doc.mergeDocuments([a.id, b.id], "end")`. The edit's `renamedDestinations` lists each renamed destination, and one `undo()` takes the whole merge back.
- **Merge only some pages:** pass `{ source: a.id, pages: [...] }` in place of the source's ID. Bookmarks to pages left behind are dropped, and the edit's `bookmarksDropped` counts them.
- **Split by page count:** `split({ count: 10 })` makes parts of ten pages. `split({ ranges: [[a, b], [c]] })` makes one part per list of page IDs.
- **Split at sections:** `split({ bookmarks: 2 })` cuts at the second level of bookmarks.

Next: [organise pages](../../pages/recipes/organise-pages.md), to put the merged pages in the order you want.
