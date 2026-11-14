---
title: "Resize pages"
description: "Put every page of a PDF on new paper, scaling the content to fit or extending the page around it, and turn pages to portrait or landscape."
version: "0.4.0"
url: "https://docs.laminapdf.com/recipes/resize-pages/"
---

# Resize pages

Put every page of a PDF on new paper, scaling the content to fit or extending the page around it, and turn pages to portrait or landscape.

You'll put every page of a document on Letter paper, scaling each page's content to fit. Use it to print a document made on A4 in North America, to make a mixed stack of scans one size, or to give slides the same paper as the report they go in.

A page keeps its proportions. An A4 page is a little taller and narrower than Letter, so it is scaled down to Letter's height and centred, with a thin margin on each side:

**TypeScript**

`packages/sdk/examples/resize-pages.ts`:

```ts
// Make every page Letter, scaling each page's content to fit.
//
// A page keeps its proportions: an A4 page is scaled down to Letter's height and centred, with
// a little room on each side. The bookmarks, links and named places that point into a page move
// with its content, so they still land on the same words.
import { Document } from "@laminapdf/sdk";

/** Letter, in points. */
const LETTER = { width: 612, height: 792 };

/** The document with every page on Letter paper, its content scaled to fit. Needs `init()`. */
export function resizePages(bytes: Uint8Array): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);
  const pages = doc.pages.map((page) => page.id);
  doc.resizePages(pages, LETTER);
  return doc.save();
}
```

**Rust**

`crates/laminapdf/examples/resize_pages.rs`:

````rust
//! Make every page Letter, scaling each page's content to fit.
//!
//! A page keeps its proportions: an A4 page is scaled down to Letter's height and centred, with
//! a little room on each side. The bookmarks, links and named places that point into a page move
//! with its content, so they still land on the same words.
//!
//! ```sh
//! cargo run -p laminapdf --example resize_pages -- in.pdf [out-dir]
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{Document, OpenOptions, PageSize, ResizePages, SaveOptions};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let input = PathBuf::from(args.next().ok_or("usage: resize_pages in.pdf [out-dir]")?);
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let resized = resize(std::fs::read(input)?)?;
    std::fs::write(out.join("resize_pages.pdf"), resized)?;
    Ok(())
}

/// The document with every page on Letter paper, its content scaled to fit.
pub(crate) fn resize(bytes: Vec<u8>) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;
    let pages = doc.page_infos()?.into_iter().map(|page| page.id).collect();
    doc.apply(vec![ResizePages::scale(pages, PageSize::LETTER).into()])?;
    Ok(doc.save(&SaveOptions::new())?.bytes)
}
````

**Result:** [resize_pages.pdf](https://docs.laminapdf.com/examples/pdf/resize_pages.pdf)

The input has an A4 page and a Letter page. In the result both are Letter. The first page's text is a little smaller and its highlight still covers the same words, and the second page is as it was.

## How it works

- `resizePages` (`ResizePages` in Rust) takes the new paper's width and height in points. The size is the paper's before any rotation, so a page turned on its side stays turned.
- The page's content moves as one piece, and everything that points into the page moves with it. That covers links, notes, highlights and form fields, bookmarks and named places, the view the document opens at, and the boxes a screen reader uses to find a figure. A bookmark that showed a paragraph at the top of the window still does.
- The page shows its whole new paper: a crop box goes, and the bleed, trim and art boxes move with the content.
- **Undo:** one `undo()` puts every page back.

## What to change

- **Keep the content's size:** pass `{ mode: "extend", anchor: "topLeft" }` (`ResizePages::extend` in Rust). The paper grows around the content, or is cut down to the part of it the new paper covers. The anchor says where the old page sits: `"center"` by default, or a corner or an edge.
- **Turn pages to portrait or landscape:** `setOrientation(ids, "landscape")` turns each page shown the other way a quarter turn clockwise, and leaves the rest alone. It turns the page as readers show it (`/Rotate`). For a printer or tool that ignores that, pass `"swapBox"`: the paper's sides swap and the content, annotations and bookmarks turn into it.
- **Signed forms:** resizing or turning a page that shows a signed field breaks the signature, so saving refuses the edit unless you allow the save to break it.
- **Files a resize refuses:** to move a page's content, Lamina PDF reads it. A page whose content Lamina PDF can't decode is refused with `unsupported-filter`, and the document is left as it was.

Next: [crop pages](../../pages/recipes/crop-pages.md), to hide part of a page instead.
