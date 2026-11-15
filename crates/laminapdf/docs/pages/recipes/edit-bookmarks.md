---
title: "Edit the bookmarks"
description: "Add, move and style a PDF's bookmarks, name a place links can jump to, and rename a destination together with everything that uses it."
version: "0.4.0"
url: "https://docs.laminapdf.com/recipes/edit-bookmarks/"
---

# Edit the bookmarks

Add, move and style a PDF's bookmarks, name a place links can jump to, and rename a destination together with everything that uses it.

You'll add a bookmark to the top of an outline, move another under a chapter, name places links jump to, and rename one with its uses. You need this for a bookmark panel, a script that fixes a converter's outlines, or a pipeline that adds a summary to every report.

A bookmark goes to a page and a view of it. Viewers report where they are in page space: points from the page's top-left corner, as the rest of Lamina PDF's API uses them. `destinationAt` turns that place into the destination the file stores, in the page's [user space](../../pages/reference/glossary.md#user-space). The example adds a bold "Summary" bookmark at the top, moves "Website" under "Chapters", names the last page `summary` and renames `chapter-2` to `methods`:

**TypeScript**

`packages/sdk/examples/edit-bookmarks.ts`:

```ts
// Edit a document's bookmarks and named destinations: add a bookmark, move one, and name a place links can jump to.
// A new "Summary" bookmark goes first, in bold, showing the top of the last page. The place comes
// from page space, as a viewer reports it, and becomes the destination the bookmark stores. The
// same place gets the name `summary`, the destination `chapter-2` is renamed with every bookmark
// that uses it, and the "Website" bookmark moves to the end of the "Chapters" bookmark, when the
// file has them.
import { Document } from "@laminapdf/sdk";

/** The document with its bookmarks edited, saved incrementally. Needs `init({ extract: true })`. */
export function editBookmarks(bytes: Uint8Array): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);

  // The top of the last page, in page space (points from the page's top-left).
  const last = Math.max(doc.pageCount - 1, 0);
  const summary = doc.destinationAt(last, { kind: "at", x: 0, y: 0 });

  doc.apply([
    {
      op: "addOutlineItem",
      args: {
        parent: null,
        at: "first",
        title: "Summary",
        target: { destination: summary },
        style: { bold: true },
      },
    },
    { op: "setNamedDestination", args: { name: "summary", destination: summary } },
  ]);

  // Rename a destination, if the file has it: bookmarks and links that use it follow.
  if (doc.namedDestinations().some((d) => d.name === "chapter-2")) {
    doc.apply([{ op: "renameNamedDestination", args: { from: "chapter-2", to: "methods" } }]);
  }

  const outline = doc.outline();
  const find = (title: string) => outline.find((node) => node.title === title)?.id;
  const website = find("Website");
  const chapters = find("Chapters");
  if (website !== undefined && chapters !== undefined) {
    doc.apply([{ op: "moveOutlineItem", args: { item: website, parent: chapters, at: "last" } }]);
  }

  return doc.save({ mode: "incremental" });
}
```

**Rust**

`crates/laminapdf/examples/edit_bookmarks.rs`:

````rust
//! Edit a document's bookmarks and named destinations: add a bookmark, move one, and name a place links can jump to.
//!
//! A new "Summary" bookmark goes first, in bold, showing the top of the last page. The place
//! comes from page space, as a viewer reports it, and becomes the destination the bookmark
//! stores. The same place gets the name `summary`, the destination `chapter-2` is renamed with
//! every bookmark that uses it, and the "Website" bookmark moves to the end of the "Chapters"
//! bookmark, when the file has them.
//!
//! ```sh
//! cargo run -p laminapdf --example edit_bookmarks -- in.pdf [out-dir]
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{
    AddOutlineItem, Document, MoveOutlineItem, Op, OpenOptions, OutlineStyle, OutlineTarget, PageTargetView,
    RenameNamedDestination, SaveMode, SaveOptions, SetNamedDestination, SiblingPosition,
};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let input = PathBuf::from(args.next().ok_or("usage: edit_bookmarks in.pdf [out-dir]")?);
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let edited = edit(std::fs::read(input)?)?;
    std::fs::write(out.join("edit_bookmarks.pdf"), edited)?;
    Ok(())
}

/// The document with its bookmarks edited, saved incrementally.
pub(crate) fn edit(bytes: Vec<u8>) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;

    // The top of the last page, in page space (points from the page's top-left).
    let last = doc.page_count()?.saturating_sub(1);
    let top = PageTargetView::At {
        x: Some(0.0),
        y: Some(0.0),
        zoom: None,
    };
    let summary = doc.destination_at(last, top)?;

    let mut add = AddOutlineItem::new(
        None,
        SiblingPosition::First,
        "Summary",
        OutlineTarget::Destination(summary.clone()),
    );
    add.style = OutlineStyle::new(None, true, false);
    doc.apply(vec![
        Op::from(add),
        Op::from(SetNamedDestination::new("summary", summary)),
    ])?;

    // Rename a destination, if the file has it: bookmarks and links that use it follow.
    if doc.named_destinations()?.iter().any(|d| d.name == "chapter-2") {
        doc.apply(vec![Op::from(RenameNamedDestination::new("chapter-2", "methods"))])?;
    }

    let outline = doc.outline()?;
    let find = |title: &str| outline.iter().find(|n| n.title == title).map(|n| n.id.clone());
    if let (Some(website), Some(chapters)) = (find("Website"), find("Chapters")) {
        doc.apply(vec![Op::from(MoveOutlineItem::new(
            website,
            Some(chapters),
            SiblingPosition::Last,
        ))])?;
    }

    let options = SaveOptions::new().with_mode(SaveMode::Incremental);
    Ok(doc.save(&options)?.bytes)
}
````

**Result:** [edit_bookmarks.pdf](https://docs.laminapdf.com/examples/pdf/edit_bookmarks.pdf)

The result is the original bytes, unchanged, followed by one revision. Its outline starts with "Summary" in bold, the "Website" bookmark sits at the end of "Chapters", and the name `summary` goes to the top of the last page. The bookmark that went to `chapter-2` now goes to `methods`, the same place under its new name.

## How it works

- `outline()` lists every bookmark with its `id`, which the bookmark edits take, its title, where it goes, its colour and style, and whether its children show.
- `addOutlineItem` (`AddOutlineItem` in Rust) adds a bookmark under a `parent` (`null` for the top level) at a position: `"first"`, `"last"`, `{ before: id }` or `{ after: id }`. Its target is a page (`{ destination }`), a named destination (`{ named }`) or a web address (`{ uri }`). The edit's result lists the new bookmark's ID in `createdOutlineItems`.
- `updateOutlineItem`, `moveOutlineItem` and `deleteOutlineItem` change, move and delete a bookmark by its ID. A moved bookmark takes its children with it and keeps its ID. A deleted one takes its children with it, or leaves them in its place with `children: "promote"`.
- `setNamedDestination` gives a place a name, or points an existing name elsewhere. `renameNamedDestination` renames it, together with every bookmark, link and opening view of the file that uses it. Pass `links: "keep"` to leave those alone.
- Lamina PDF keeps each bookmark's place in the tree, and the counts of open bookmarks, consistent after every edit. When a file's outline was broken, such as a loop or a wrong last item, the first edit rewrites it whole and says so in `outlineRepaired`.

## What to change

- **A colour or style:** pass `style: { color: "#c00000", bold: true, italic: false }`. Leave `color` out for the viewer's default black.
- **A web address:** pass `target: { uri: "https://example.com" }`. Only absolute `http`, `https` and `mailto` addresses are allowed; anything else, `javascript:` first, fails with [`link-uri-refused`](../../errors/link-uri-refused.md).
- **The current view:** pass what your viewer shows to `destinationAt`, such as `{ kind: "at", x, y, zoom }` for a point at the top-left, or `{ kind: "fit-width", y }` to fit the width.
- **Deleting a name:** `deleteNamedDestination` leaves the bookmarks and links that use it alone, since removing them could lose your content. The edit's `destinationUses` lists them, so you can retarget or delete them next.
- **A signed file:** a bookmark edit changes the document's structure, so a [certified](../../pages/reference/glossary.md#docmdp) file refuses the save. On a file with approval signatures only, the save appends a revision and the signatures stay valid.

Next: [save a PDF](../../pages/guides/save-a-pdf.md), for what a signed or repaired file allows.
