---
title: "Copy pages between PDFs"
description: "Copy pages from another PDF into this one, replace a page with one from another file, or make a new PDF from some pages."
version: "0.3.0"
url: "https://docs.laminapdf.com/recipes/copy-pages/"
---

# Copy pages between PDFs

Copy pages from another PDF into this one, replace a page with one from another file, or make a new PDF from some pages.

You'll copy a page from one PDF into another, then make a new PDF from three pages of a longer one. Use the first to add a cover page, an appendix or a signed form from another file, and the second to send someone only the chapter they need.

## Copy a page in

Add the other file as a source first. Lamina PDF names the source by a hash of its bytes and lists its page IDs, and you pick the pages to copy from that list. The program copies the other file's third page to the end:

**TypeScript**

`packages/sdk/examples/insert-pages.ts`:

```ts
// Copy a page from another PDF into this one.
//
// The copy keeps its annotations and form fields. A field whose name the document already uses
// takes the next free `<name>#2`, and a link to a page that didn't come along is removed; the
// edit lists both.
import { Document } from "@laminapdf/sdk";

/** `bytes` with the third page of `other` added at the end. Needs `init()`. */
export function insertPages(bytes: Uint8Array, other: Uint8Array): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);
  const source = doc.addSource(other);
  const third = source.pages[2];
  if (third === undefined) {
    throw new Error("the other file has no third page");
  }
  const edited = doc.insertPagesFrom(source.id, [third], "end");
  for (const field of edited.fields ?? []) {
    if (field.outcome === "renamed") {
      console.warn(`a copied field was renamed ${field.name}`);
    }
  }
  console.warn(`${edited.linksRemoved?.length ?? 0} links removed`);
  return doc.save();
}
```

**Rust**

`crates/laminapdf/examples/insert_pages.rs`:

````rust
//! Copy a page from another PDF into this one.
//!
//! The copy keeps its annotations and form fields. A field whose name the document already uses
//! takes the next free `<name>#2`, and a link to a page that didn't come along is removed; the
//! edit lists both.
//!
//! ```sh
//! cargo run -p laminapdf --example insert_pages -- form.pdf other.pdf [out-dir]
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{Document, FieldOutcome, InsertPagesFrom, OpenOptions, PagePosition, SaveOptions};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let usage = "usage: insert_pages form.pdf other.pdf [out-dir]";
    let input = PathBuf::from(args.next().ok_or(usage)?);
    let other = PathBuf::from(args.next().ok_or(usage)?);
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let combined = insert(std::fs::read(input)?, std::fs::read(other)?)?;
    std::fs::write(out.join("insert_pages.pdf"), combined)?;
    Ok(())
}

/// `bytes` with the third page of `other` added at the end.
pub(crate) fn insert(bytes: Vec<u8>, other: Vec<u8>) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;
    let source = doc.add_source(other, None)?;
    let third = doc
        .source_pages(&source)?
        .into_iter()
        .nth(2)
        .ok_or("the other file has no third page")?;
    let insert = InsertPagesFrom::new(source, vec![third], PagePosition::End);
    let edited = doc.apply(vec![insert.into()])?;
    for field in &edited.fields {
        if let FieldOutcome::Renamed { name, .. } = &field.outcome {
            eprintln!("a copied field was renamed {name}");
        }
    }
    eprintln!("{} links removed", edited.links_removed.len());
    Ok(doc.save(&SaveOptions::new())?.bytes)
}
````

**Result:** [insert_pages.pdf](https://docs.laminapdf.com/examples/pdf/insert_pages.pdf)

The result has three pages, and the third is the copied one. Its text field was called `name`, which the form already used, so the copy's field is `name#2` and keeps a value of its own.

## Make a new PDF from pages

`extractPages` (`extract_pages` in Rust) takes page IDs and returns a new document holding only those pages:

**TypeScript**

`packages/sdk/examples/extract-pages.ts`:

```ts
// Make a new PDF from some pages of another.
//
// The new file holds only what its pages use, and its pages keep the labels they had (`iii`,
// `1`, `2` rather than 1, 2, 3).
import { Document } from "@laminapdf/sdk";

/** A new document of the third to the fifth page of `bytes`. Needs `init()`. */
export function extractPages(bytes: Uint8Array): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);
  const pages = doc.pages.slice(2, 5).map((page) => page.id);
  using part = doc.extractPages(pages);
  return part.save();
}
```

**Rust**

`crates/laminapdf/examples/extract_pages.rs`:

````rust
//! Make a new PDF from some pages of another.
//!
//! The new file holds only what its pages use, and its pages keep the labels they had (`iii`,
//! `1`, `2` rather than 1, 2, 3).
//!
//! ```sh
//! cargo run -p laminapdf --example extract_pages -- book.pdf [out-dir]
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{Document, OpenOptions, SaveOptions};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let input = PathBuf::from(args.next().ok_or("usage: extract_pages book.pdf [out-dir]")?);
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let part = extract(std::fs::read(input)?)?;
    std::fs::write(out.join("extract_pages.pdf"), part)?;
    Ok(())
}

/// A new document of the third to the fifth page of `bytes`.
pub(crate) fn extract(bytes: Vec<u8>) -> Result<Vec<u8>, Box<dyn Error>> {
    let doc = Document::open_bytes(bytes, OpenOptions::new())?;
    let pages: Vec<_> = doc.page_infos()?.into_iter().skip(2).take(3).map(|p| p.id).collect();
    let mut part = doc.extract_pages(&pages)?;
    Ok(part.save(&SaveOptions::new())?.bytes)
}
````

**Result:** [extract_pages.pdf](https://docs.laminapdf.com/examples/pdf/extract_pages.pdf)

The new file's pages keep the labels they showed in the book, `iii`, `1` and `2`, and it holds only the fonts and images those pages use.

## How it works

- A copy is a new page with a new ID, listed in the edit's `pageOrder`. The copy shares nothing with the source file, so changing one never changes the other.
- The copy's annotations come along. A link to a page that came too still works; a link to a page left behind is removed, and the edit's `linksRemoved` lists the page that held it.
- Form fields keep their names. A name the document already uses gets the next free number, `<name>#2` and on, and the edit's `fields` lists each renamed field.
- A signature's field isn't copied, since a signature signs one place in one file. The edit's `fields` lists it as `notCopied`.
- Page labels continue the numbering where the copies land, and every other page keeps its label.

## What to change

- **Replace a page:** `replacePages(targets, source, pages)` puts each source page in a target's place. Bookmarks, links and named destinations that pointed at the old page point at the new one.
- **Keep fields linked:** pass `{ fields: "link" }` as the last argument, and a copied field joins the document's field of the same name, so both show one value. Linking needs both fields to have the same type and value; otherwise the copy is renamed.
- **Leave annotations behind:** pass `{ annotations: false }` to copy only the page content and its form fields.
- **Split into several files:** `exportRanges([[a, b], [c]])` makes one document per list of pages, and [`split`](../../pages/recipes/merge-and-split.md) cuts a file by chapter or page count.
- **Tagged documents:** a copied page comes without its tags, and the edit's `tagsDropped` lists it. On a file that claims PDF/UA-1, saving then warns with [`imported-tags-dropped`](../../errors/imported-tags-dropped.md).
- **Undo:** one `undo()` removes every copied page and what came with it.

Next: [duplicate pages](../../pages/recipes/duplicate-pages.md), to copy a page within the same document.
