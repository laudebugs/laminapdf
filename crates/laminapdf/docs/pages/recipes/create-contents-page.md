---
title: "Add a contents page"
description: "Add a page to a PDF that lists its bookmarks, each with the page label readers see and a link to that page, as one edit."
version: "0.5.0"
url: "https://docs.laminapdf.com/recipes/create-contents-page/"
---

# Add a contents page

Add a page to a PDF that lists its bookmarks, each with the page label readers see and a link to that page, as one edit.

You'll add a contents page to the front of a PDF, made from its bookmarks. Each entry shows its page's label and opens that page when clicked. You need this for a long report or a merged file that has bookmarks but no contents page to print.

The page is made from the bookmarks only: Lamina PDF never guesses headings from the text. One edit lays it out, inserts it and links its entries:

**TypeScript**

`packages/sdk/examples/create-contents-page.ts`:

```ts
// Add a clickable contents page to a PDF, made from its bookmarks.
// The page lists the bookmarks that go to a page, two levels deep, each with the label its page
// shows and a link to it, and gets a bookmark of its own. It's one edit, so one undo takes it out
// again, and it saves incrementally.
import { Document, Font } from "@laminapdf/sdk";

/**
 * The document with a contents page before the first page, set in the font `font` (embedded, so
 * a PDF/A or PDF/UA-1 file keeps its claim), saved incrementally. Needs `init({ text: true })`.
 */
export function addContents(bytes: Uint8Array, font: Uint8Array): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);
  doc.createContentsPage({ font: Font.fromBytes(font), bookmark: true });
  return doc.save({ mode: "incremental" });
}
```

**Rust**

`crates/laminapdf/examples/create_contents_page.rs`:

````rust
//! Add a clickable contents page to a PDF, made from its bookmarks.
//!
//! The page lists the bookmarks that go to a page, two levels deep, each with the label its page
//! shows and a link to it, and gets a bookmark of its own. It's one edit, so one undo takes it
//! out again, and it saves incrementally.
//!
//! ```sh
//! cargo run -p laminapdf --example create_contents_page -- in.pdf font.ttf [out-dir]
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{ContentsFonts, ContentsOptions, Document, Font, OpenOptions, SaveOptions};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let usage = "usage: create_contents_page in.pdf font.ttf [out-dir]";
    let input = PathBuf::from(args.next().ok_or(usage)?);
    let font = PathBuf::from(args.next().ok_or(usage)?);
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let edited = add_contents(std::fs::read(input)?, std::fs::read(font)?)?;
    std::fs::write(out.join("create_contents_page.pdf"), edited)?;
    Ok(())
}

/// `bytes` with a contents page before the first page, set in the font `font` (embedded, so a
/// PDF/A or PDF/UA-1 file keeps its claim), saved incrementally.
pub(crate) fn add_contents(bytes: Vec<u8>, font: Vec<u8>) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;
    let options = ContentsOptions::new().with_bookmark(true);
    let fonts = ContentsFonts::new().with_font(Font::from_bytes(font)?);
    doc.create_contents_page(&options, &fonts)?;
    Ok(doc.save(&SaveOptions::new())?.bytes)
}
````

**Result:** [create_contents_page.pdf](https://docs.laminapdf.com/examples/pdf/create_contents_page.pdf)

The result is the original bytes, unchanged, followed by one revision. The new first page lists the bookmarks under a "Contents" heading, and a "Contents" bookmark at the top of the outline opens it.

## How it works

- The page lists each bookmark that goes to a page, two levels deep. A bookmark that opens a web address, or goes nowhere, is left out, but the bookmarks inside it are still listed.
- Each entry ends with the label its page shows once the contents page is in. Say a file numbers its front matter i, ii and its body 1, 2. A contents page at the front joins the front matter as "i", so the old "i" and "ii" print as "ii" and "iii", and the body keeps 1 and 2. In a file without labels, the numbers move up by one for each contents page.
- The entries are links to the bookmarks' places, and each link's description is the bookmark's title, which screen readers read.
- In a tagged file, the page is tagged too: a `TOC` element with a `TOCI` for each entry joins the document's structure tree.
- The font is embedded and subset. Without one, the page is set in Helvetica, which isn't embedded, so a file that claims PDF/A or PDF/UA-1 refuses it with [`font-not-embedded`](../../errors/font-not-embedded.md). A file with no bookmark that goes to a page fails with [`no-bookmarks`](../../errors/no-bookmarks.md).
- It's one edit: one undo removes the page, its links and its bookmark.

## What to change

- **One level:** pass `levels: 1` (`with_levels(1)` in Rust) to list only the top-level bookmarks.
- **Another place:** pass `at: "end"`, or `{ after: id }` with a page's ID (`with_at` in Rust).
- **Another heading:** pass `title` (`with_title`), in the document's own language.
- **Text size:** pass `size` (`with_size`), from 4 to 72 points; entries are 11 points by default, and the heading half again as large. A contents page longer than ten pages fails with [`invalid-op`](../../errors/invalid-op.md): list one level, or use a smaller size.
- **Characters the font lacks:** add `fallbacks` (`with_fallback`), fonts tried in order. A title no font can draw fails with [`missing-glyph`](../../errors/missing-glyph.md).

Next: [edit the bookmarks](../../pages/recipes/edit-bookmarks.md) the page is made from.
