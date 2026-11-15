---
title: "Choose how a PDF opens"
description: "Set the page and zoom a PDF opens at, the panel beside it and the title in the window, for Lamina PDF and every other PDF reader."
version: "0.4.0"
url: "https://docs.laminapdf.com/recipes/set-initial-view/"
---

# Choose how a PDF opens

Set the page and zoom a PDF opens at, the panel beside it and the title in the window, for Lamina PDF and every other PDF reader.

You'll make a PDF open on a chosen page at the page's width, with its bookmarks panel showing and its title in the window. You need this for a report that opens on its summary, or a manual that shows its contents first.

A PDF keeps these choices in its catalog, and every reader that follows the PDF standard reads them when it opens the file. One edit sets them:

**TypeScript**

`packages/sdk/examples/set-initial-view.ts`:

```ts
// Choose how a PDF opens: on a given page at the page's width, with its bookmarks shown and its title in the window.
// The edit changes only what the file asks a reader to do when it opens it. A file whose open
// action runs a script keeps that action: the edit leaves the opening page alone and changes the
// rest.
import { Document, type InitialViewPatch } from "@laminapdf/sdk";

/**
 * The document set to open on page `index` (0-based) at the page's width, with the bookmarks
 * panel and the document's title in the window, saved incrementally. Needs
 * `init({ extract: true })`.
 */
export function openOn(bytes: Uint8Array, index: number): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);

  const view: InitialViewPatch = { pageMode: "outlines", preferences: { displayDocTitle: true } };
  // An open action that runs a script or another action is never replaced, so leave the page out.
  const open = doc.initialView().open;
  if (open.kind === "action") {
    console.warn(`the file runs a ${open.action} action when it opens; keeping it`);
  } else {
    view.open = { page: doc.page(index).id, zoom: "fitWidth" };
  }
  doc.setInitialView(view);
  return doc.save({ mode: "incremental" });
}
```

**Rust**

`crates/laminapdf/examples/set_initial_view.rs`:

````rust
//! Choose how a PDF opens: on a given page at the page's width, with its bookmarks shown and its title in the window.
//!
//! The edit changes only what the file asks a reader to do when it opens it. A file whose open
//! action runs a script keeps that action: the edit leaves the opening page alone and changes
//! the rest.
//!
//! ```sh
//! cargo run -p laminapdf --example set_initial_view -- in.pdf <page-number> [out-dir]
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{
    Document, Op, OpenAt, OpenOptions, OpenTarget, OpenZoom, PageMode, SaveOptions, SetInitialView,
    WindowPreferencesPatch,
};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let usage = "usage: set_initial_view in.pdf <page-number> [out-dir]";
    let input = PathBuf::from(args.next().ok_or(usage)?);
    let number: u32 = args.next().ok_or(usage)?.parse()?;
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let index = number.checked_sub(1).ok_or("page numbers start at 1")?;
    let edited = open_on(std::fs::read(input)?, index)?;
    std::fs::write(out.join("set_initial_view.pdf"), edited)?;
    Ok(())
}

/// `bytes` set to open on page `index` (0-based) at the page's width, with the bookmarks panel
/// and the document's title in the window, saved incrementally.
pub(crate) fn open_on(bytes: Vec<u8>, index: u32) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;
    let page = doc.page_info(index)?.id;

    let mut view = SetInitialView::default()
        .with_page_mode(Some(PageMode::Outlines))
        .with_preferences(WindowPreferencesPatch::default().with_display_doc_title(true));
    // An open action that runs a script or another action is never replaced, so leave the page out.
    if let OpenTarget::Action(kind) = doc.initial_view()?.open {
        eprintln!("the file runs a {kind} action when it opens; keeping it");
    } else {
        view = view.with_open(Some(OpenAt::new(page, OpenZoom::FitWidth)));
    }
    doc.apply(vec![Op::from(view)])?;
    Ok(doc.save(&SaveOptions::new())?.bytes)
}
````

**Result:** [set_initial_view.pdf](https://docs.laminapdf.com/examples/pdf/set_initial_view.pdf)

The result is the original bytes, unchanged, followed by one revision. The file opens on the page you chose, fitted to the window's width, with the bookmarks panel open and the document's title in the window's title bar.

## How it works

- `initialView()` reads how the file asks to open. Its `open` is `"default"` when the file leaves the first page and zoom to the reader, `"page"` with a target in [page space](../../pages/reference/glossary.md#page-space), or `"action"` when the file runs an action, such as a script, instead.
- `setInitialView` (`SetInitialView` in Rust) takes a patch: a value sets a field, `null` removes it, and a field you leave out stays as the file has it.
- `open` names the page by its ID and a `zoom`: `"fitWidth"`, `"fitPage"`, `"default"` (the reader's own zoom) or a percentage from 1 to 6400.
- `pageMode` is the panel beside the page: `"outlines"` for bookmarks, `"thumbnails"`, `"attachments"`, `"optionalContent"` for layers, `"fullScreen"` or `"none"`.
- `preferences` holds the window's requests, such as `displayDocTitle` for the title in the title bar. `true` sets one; `false` removes it, which is its default.
- An open action that runs anything besides opening a page (a script, a named action, a page change followed by further actions) is never replaced: an edit that names the page fails with [`open-action-not-destination`](../../errors/open-action-not-destination.md). The example reads the file's view first and leaves the page out of its edit then. Lamina PDF never runs the action.
- When a value needs a newer PDF version than the file declares, such as the attachments panel (PDF 1.6), the edit raises the file's version to it.

## What to change

- **Another page layout:** add `pageLayout`, such as `"twoPageRight"` for two pages side by side with the cover alone.
- **The reader's defaults again:** pass `null` for each field you set, `open` included.
- **The article threads:** `threads()` lists a file's article threads, each with its parts as rectangles in page space, in reading order.

Next: [save a PDF](../../pages/guides/save-a-pdf.md), for what a signed or repaired file allows.
