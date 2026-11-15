---
title: "Crop pages"
description: "Crop every page of a PDF by the same margins whatever its size or rotation, and see which pages still hold the content the crop hides."
version: "0.4.0"
url: "https://docs.laminapdf.com/recipes/crop-pages/"
---

# Crop pages

Crop every page of a PDF by the same margins whatever its size or rotation, and see which pages still hold the content the crop hides.

You'll crop half an inch from each side of every page. Use it to trim the white border of a scan, to cut printer's marks off a proof, or to make slides fit a phone's screen.

The margins are measured on the page as a reader sees it. A page turned on its side loses the same strip from its visible top as an upright page does, so one set of margins suits a document of mixed sizes and rotations. The program prints each page whose crop hides content:

**TypeScript**

`packages/sdk/examples/crop-pages.ts`:

```ts
// Crop every page by half an inch on each side, whatever its size or rotation.
//
// Margins are measured on the page as it shows, so a page turned on its side loses the same
// strip from its visible top as an upright one. Cropping only hides content: the edit lists the
// pages whose hidden content stays in the file, where text extraction still finds it.
import { Document } from "@laminapdf/sdk";

/** The document with 36 points cropped from each side of every page. Needs `init()`. */
export function cropPages(bytes: Uint8Array): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);
  const pages = doc.pages.map((page) => page.id);
  const edited = doc.cropPages(pages, { top: 36, right: 36, bottom: 36, left: 36 });
  for (const page of edited.croppedContentRemains ?? []) {
    console.warn(`page ${page} hides content that is still in the file`);
  }
  return doc.save();
}
```

**Rust**

`crates/laminapdf/examples/crop_pages.rs`:

````rust
//! Crop every page by half an inch on each side, whatever its size or rotation.
//!
//! Margins are measured on the page as it shows, so a page turned on its side loses the same
//! strip from its visible top as an upright one. Cropping only hides content: the edit lists the
//! pages whose hidden content stays in the file, where text extraction still finds it.
//!
//! ```sh
//! cargo run -p laminapdf --example crop_pages -- in.pdf [out-dir]
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{CropMargins, CropPages, Document, OpenOptions, SaveOptions};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let input = PathBuf::from(args.next().ok_or("usage: crop_pages in.pdf [out-dir]")?);
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let cropped = crop(std::fs::read(input)?)?;
    std::fs::write(out.join("crop_pages.pdf"), cropped)?;
    Ok(())
}

/// The document with 36 points cropped from each side of every page.
pub(crate) fn crop(bytes: Vec<u8>) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;
    let pages = doc.page_infos()?.into_iter().map(|page| page.id).collect();
    let edited = doc.apply(vec![CropPages::new(pages, CropMargins::all(36.0)).into()])?;
    for page in &edited.cropped_content_remains {
        eprintln!("page {page} hides content that is still in the file");
    }
    Ok(doc.save(&SaveOptions::new())?.bytes)
}
````

**Result:** [crop_pages.pdf](https://docs.laminapdf.com/examples/pdf/crop_pages.pdf)

The input has four pages, one at each rotation, and the second is drawn at twice the usual scale. In the result, each page shows 72 points less across and 72 less down, and its "Page n" label sits the same distance from its visible corner.

## How it works

- `cropPages` (`CropPages` in Rust) sets each page's crop box: the part of the page a reader shows and prints. It takes `top`, `right`, `bottom` and `left` in points, and a side you leave out is 0.
- Cropping hides content but keeps it. Text outside the crop box stays in the file, and copying or extracting text still finds it. The edit's `croppedContentRemains` lists the pages where that is true.
- **Signed forms:** cropping a page that shows a signed field breaks the signature, so saving refuses the edit unless you allow the save to break it.
- **Undo:** one `undo()` puts every page's crop box back.

## What to change

- **Set a box exactly:** `setPageBoxes` (`SetPageBoxes` in Rust) writes any of a page's five boxes in the page's own coordinates: `media` (the paper), `crop`, `bleed`, `trim` and `art`. `null` removes a box, so the page uses its default, which is the media box for the crop box and the crop box for the others. Each box must lie inside the media box.
- **Crop only some pages:** pass their IDs. To give pages different margins, crop each group in its own call inside one `transact`, and one undo reverts them all.
- **Private content:** a crop isn't a redaction. Anyone with another tool can read what it hides, so don't crop a page to remove something private.

Next: [organise pages](../../pages/recipes/organise-pages.md), to delete, move and rotate them too.
