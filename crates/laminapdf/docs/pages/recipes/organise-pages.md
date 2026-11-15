---
title: "Organise pages"
description: "Delete, move, insert and rotate a PDF's pages in one edit, and see where the bookmarks and links that pointed at a deleted page now go."
version: "0.4.0"
url: "https://docs.laminapdf.com/recipes/organise-pages/"
---

# Organise pages

Delete, move, insert and rotate a PDF's pages in one edit, and see where the bookmarks and links that pointed at a deleted page now go.

You'll delete a PDF's third page, move its last page to the front, add a blank page at the end and turn two pages a quarter clockwise, in one edit. Use it for a page organiser, a scan out of order, or a pipeline that drops cover pages.

Each page has an ID, such as `p12.0`, that stays the same while other pages move, so each step names its pages by the IDs read before the edit:

**TypeScript**

`packages/sdk/examples/organise-pages.ts`:

```ts
// Organise a document's pages: delete one, move one to the front, add a blank page at the end
// and turn two pages a quarter clockwise, in one edit that one undo reverts.
//
// Pages are named by their IDs, which stay the same while other pages move. Deleting a page
// points the bookmarks, links and named destinations that went to it at the nearest page left,
// and the edit lists each one, so an app can say what moved.
import { Document } from "@laminapdf/sdk";

/** US Letter, in points. */
const LETTER = { width: 612, height: 792 };

/**
 * The document with its third page deleted, its last page moved to the front, a blank Letter
 * page added at the end and its first two pages turned a quarter clockwise. Needs `init()`.
 */
export function organisePages(bytes: Uint8Array): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);
  const ids = doc.pages.map((page) => page.id);
  const [first, second, third] = ids;
  const last = ids.at(-1);
  if (first === undefined || second === undefined || third === undefined || last === undefined) {
    throw new Error("the document needs at least three pages");
  }

  // One transaction: every op applies, in order, or none does. Each op names its pages by the
  // IDs read before the edit, which the moves and the deletion leave as they are.
  const edited = doc.apply([
    { op: "deletePages", args: { pages: [third] } },
    { op: "movePages", args: { pages: [last], to: "start" } },
    { op: "insertBlankPages", args: { at: "end", count: 1, size: LETTER, rotate: 0 } },
    { op: "rotatePages", args: { pages: [first, second], angle: 90, mode: "turn" } },
  ]);
  for (const reference of edited.references) {
    if (reference.outcome === "retargeted") {
      console.warn(`${reference.kind} now points to page ${reference.to}`);
    }
  }
  return doc.save();
}
```

**Rust**

`crates/laminapdf/examples/organise_pages.rs`:

````rust
//! Organise a document's pages: delete one, move one to the front, add a blank page at the end
//! and turn two pages a quarter clockwise, in one edit that one undo reverts.
//!
//! Pages are named by their IDs, which stay the same while other pages move. Deleting a page
//! points the bookmarks, links and named destinations that went to it at the nearest page left,
//! and the edit lists each one, so an app can say what moved.
//!
//! ```sh
//! cargo run -p laminapdf --example organise_pages -- in.pdf [out-dir]
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{
    DeletePages, Document, InsertBlankPages, MovePages, Op, OpenOptions, PagePosition, PageSize, ReferenceOutcome,
    RotatePages, Rotation, SaveOptions,
};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let input = PathBuf::from(args.next().ok_or("usage: organise_pages in.pdf [out-dir]")?);
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let organised = organise(std::fs::read(input)?)?;
    std::fs::write(out.join("organise_pages.pdf"), organised)?;
    Ok(())
}

/// The document with its third page deleted, its last page moved to the front, a blank Letter
/// page added at the end and its first two pages turned a quarter clockwise.
pub(crate) fn organise(bytes: Vec<u8>) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;
    let ids: Vec<_> = doc.page_infos()?.into_iter().map(|page| page.id).collect();
    let (Some(third), Some(last)) = (ids.get(2), ids.last()) else {
        return Err("the document needs at least three pages".into());
    };
    let edited = doc.apply(vec![
        Op::from(DeletePages::new(vec![third.clone()])),
        Op::from(MovePages::new(vec![last.clone()], PagePosition::Start)),
        Op::from(InsertBlankPages::new(PagePosition::End, 1, PageSize::LETTER)),
        Op::from(RotatePages::turn(
            ids.iter().take(2).cloned().collect(),
            Rotation::Clockwise90,
        )),
    ])?;
    for reference in &edited.references {
        if let ReferenceOutcome::Retargeted { to } = &reference.outcome {
            eprintln!("{:?} now points to page {to}", reference.kind);
        }
    }
    Ok(doc.save(&SaveOptions::new())?.bytes)
}
````

**Result:** [organise_pages.pdf](https://docs.laminapdf.com/examples/pdf/organise_pages.pdf)

The result has five pages: the old last page, then the old first and second pages turned on their side, then the old fourth page, then a blank Letter page. The bookmarks, links and named destinations that went to the deleted page now go to the old fourth page, and the program prints each one.

## How it works

- `doc.pages` lists the pages in order, each with its `id`. A page keeps its ID when it moves, and a new blank page gets a new one.
- `deletePages` (`DeletePages` in Rust) deletes pages by ID. The bookmarks, links, named destinations and opening view that pointed at a deleted page now point at the nearest page left (the next one, or the one before if none follows), shown whole. The edit's `references` lists each of them with where it now goes.
- `movePages` (`MovePages`) moves pages to `"start"`, `"end"`, or before or after another page, in the order you list them.
- `insertBlankPages` (`InsertBlankPages`) adds pages of a size in points: 612 × 792 is US Letter. In a document with page labels, such as roman numerals for the front matter, the new pages continue the numbering where they land, and every other page keeps its label.
- `rotatePages` (`RotatePages`) with `mode: "turn"` adds the angle to each page's own rotation, as rotate left and rotate right do. Without it, each page shows at the angle you give.
- `apply` runs the steps in order as one edit: if one fails, none of them apply, and one `undo()` reverts them all. The edit's `pageOrder` lists the page IDs in their new order.

## What to change

- **Remove what pointed at a deleted page:** pass `references: "remove"` to `deletePages`. A bookmark keeps its title but goes nowhere, and a link to the page stays on its own page but goes nowhere.
- **Reverse the order:** `reversePages` with every page's ID reverses the document. With some of them, those pages reverse among the places they hold, and the others stay put.
- **One step at a time:** `doc.deletePages(ids)`, `doc.movePages(ids, "start")` and the others each apply one step as its own edit, and `doc.transact(tx => …)` collects several of them into one.
- **Delete every page:** you can't. A PDF needs at least one page, so the edit refuses with [`no-pages-left`](../../errors/no-pages-left.md) and changes nothing.
- **Delete pages for good:** an [incremental save](../../pages/reference/glossary.md#incremental-save) keeps the deleted pages in the file's earlier revisions. Save with `mode: "fullRewrite"` to drop them.

Next: [save a PDF](../../pages/guides/save-a-pdf.md), for what a signed file allows.
