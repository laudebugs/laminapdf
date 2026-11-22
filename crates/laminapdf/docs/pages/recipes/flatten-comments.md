---
title: "Flatten comments"
description: "Draw a PDF's comments and marks into its pages, so a copy prints and shows the same in every reader and the marks can't be edited."
version: "0.7.0"
url: "https://docs.laminapdf.com/recipes/flatten-comments/"
---

# Flatten comments

Draw a PDF's comments and marks into its pages, so a copy prints and shows the same in every reader and the marks can't be edited.

You'll flatten the comments on a marked-up report: its highlight, strike-out, note and drawings become part of their page. You need this before you send a reviewed copy to someone who should see the marks but not change them, or to a printer that leaves comments out.

`flattenAnnotations` takes the annotations to flatten and whether the copy is for the screen or for printing. Each annotation's picture is drawn into its page where it was, and the annotation goes, with its pop-up and its replies. Here it flattens the report from [Mark up a page](../../pages/recipes/mark-up.md) for printing:

**TypeScript**

`packages/sdk/examples/flatten-comments.ts`:

```ts
// Flatten a marked-up report for printing: every comment and mark becomes part of its page, so
// the copy prints and shows the same everywhere and the marks can no longer be selected, edited
// or removed as comments.
// The annotations themselves go, with their pop-ups and replies. Saving as a full rewrite leaves no earlier
// revision that still holds them.
import { Document } from "@laminapdf/sdk";

/**
 * The document with every comment drawn into its page, as a printer would show it. Needs
 * `init({ extract: true })`.
 */
export function flattenComments(bytes: Uint8Array): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);
  const edited = doc.flattenAnnotations("all", "print");
  // Marks with no picture to keep, such as a file attachment, stay as they are.
  for (const warning of edited.warnings ?? []) {
    console.warn(warning.code);
  }
  // A full rewrite keeps only the flattened pages, not the revision with the comments.
  return doc.save({ mode: "fullRewrite" });
}
```

**Rust**

`crates/laminapdf/examples/flatten_comments.rs`:

````rust
//! Flatten a marked-up report for printing: every comment and mark becomes part of its page, so
//! the copy prints and shows the same everywhere and the marks can no longer be selected, edited
//! or removed as comments.
//!
//! The annotations themselves go, with their pop-ups and replies. Saving as a full rewrite leaves no earlier
//! revision that still holds them.
//!
//! ```sh
//! cargo run -p laminapdf --features extract --example flatten_comments -- marked.pdf [out-dir]
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{Document, FlattenIntent, FlattenScope, OpenOptions, SaveMode, SaveOptions};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let usage = "usage: flatten_comments marked.pdf [out-dir]";
    let input = PathBuf::from(args.next().ok_or(usage)?);
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let flat = flatten(std::fs::read(input)?)?;
    std::fs::write(out.join("flatten_comments.pdf"), flat)?;
    Ok(())
}

/// The document with every comment drawn into its page, as a printer would show it.
pub(crate) fn flatten(bytes: Vec<u8>) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;
    let edited = doc.flatten_annotations(&FlattenScope::All, FlattenIntent::Print)?;
    // Marks with no picture to keep, such as a file attachment, stay as they are.
    for warning in &edited.warnings {
        eprintln!("{}", warning.code());
    }
    // A full rewrite keeps only the flattened pages, not the revision with the comments.
    let options = SaveOptions::new().with_mode(SaveMode::FullRewrite);
    Ok(doc.save(&options)?.bytes)
}
````

**Result:** [flatten_comments.pdf](https://docs.laminapdf.com/examples/pdf/flatten_comments.pdf)

The pages look the same as before. Open the result in a reader with a comments panel, and the panel is empty: the marks are now part of the page, like its text.

## How it works

- Each annotation keeps the picture a reader shows for it, its appearance. Flattening draws that picture at the same place and size, then removes the annotation, so the marks can no longer be selected, edited or removed as comments.
- `"print"` keeps what a printer shows, and `"view"` what a screen shows. A mark meant only for the screen stays an annotation when you flatten for printing, and a hidden one always stays.
- Some marks have no picture of their own. Lamina PDF draws one first for notes, carets, text markup, ink, shapes, lines, polygons and text boxes. A file attachment or a sound has none to keep, so it stays, and the edit's warnings list it as `flatten-no-appearance`.
- In a tagged file, the drawings are marked as decoration, outside the structure that screen readers follow, and the comments leave the structure tree. A file that claims PDF/UA-1 then gets the `flatten-untagged` warning when you save: the comments' text is no longer read out.
- Form fields are never flattened this way: naming one fails with `flatten-widget`. [Flatten a filled form](../../pages/recipes/flatten-form.md) does that. A locked annotation can't be removed, so naming one, or a comment whose reply is locked, fails with `annotation-locked`. `"all"` and `{ pages }` leave both out, with links.
- Saving as a [full rewrite](../../pages/reference/glossary.md#full-rewrite) drops the old revision. An [incremental save](../../pages/reference/glossary.md#incremental-save) keeps it, comments included, at the end of the file.

## What to change

- **Some pages only:** pass `{ pages: [0, 2] }` (0-based) instead of `"all"`.
- **A selection:** pass `{ annotations: ids }` with IDs from `pageAnnotations` or `commentThreads`, or apply the `flattenAnnots` op with them, which works in every module.
- **For the screen:** pass `"view"`, and marks that print but don't show on screen stay as they are.
- **A signed file:** flattening changes what a page draws, so a certified file can't save it, whatever its permissions. A file with only approval signatures can save it incrementally (`mode: "auto"`), not as the full rewrite this example asks for, which would break the signatures. Each signature stays valid for the revision it signed, and the flattened pages come after it.

Next: [answer a review](../../pages/recipes/review-comments.md) to reply to comments before you flatten them.
