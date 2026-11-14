---
title: "Duplicate pages"
description: "Copy a page within a PDF so the copy has form fields of its own, and see what the copy renamed and left out."
version: "0.3.0"
url: "https://docs.laminapdf.com/recipes/duplicate-pages/"
---

# Duplicate pages

Copy a page within a PDF so the copy has form fields of its own, and see what the copy renamed and left out.

You'll add a copy of a form's first page at the end, so a second person can fill in the copy without changing the original. Use it for a sign-up sheet that needs another page, a template page repeated in a report, or a form filled in twice.

A copy shows the same content as its page, but its form fields are new: each gets the original's name with `#2` added, or the next free number. A copy of a copy continues the series: `name#3`. The program prints the name of each field the copy got:

**TypeScript**

`packages/sdk/examples/duplicate-pages.ts`:

```ts
// Duplicate a form page, so a second copy can be filled in on its own.
//
// A copy shares its page's content but gets new annotations, so filling in a field on one page
// leaves the other alone: each copied field gets a name of its own, `<name>#2` (or the next free
// number). The edit lists each one, and each signature field it left out.
import { Document } from "@laminapdf/sdk";

/** The document with a copy of its first page added at the end. Needs `init()`. */
export function duplicatePages(bytes: Uint8Array): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);
  const first = doc.pages[0]?.id;
  if (first === undefined) {
    throw new Error("the document has no pages");
  }
  const edited = doc.duplicatePages([first], "end");
  for (const field of edited.fields ?? []) {
    if (field.outcome === "renamed") {
      console.warn(`the copy's field is ${field.name}`);
    } else {
      console.warn(`field ${field.field} was not copied: it is a signature`);
    }
  }
  return doc.save();
}
```

**Rust**

`crates/laminapdf/examples/duplicate_pages.rs`:

````rust
//! Duplicate a form page, so a second copy can be filled in on its own.
//!
//! A copy shares its page's content but gets new annotations, so filling in a field on one
//! page leaves the other alone: each copied field gets a name of its own, `<name>#2` (or the
//! next free number). The edit lists each one, and a signature's widget, which isn't copied.
//!
//! ```sh
//! cargo run -p laminapdf --example duplicate_pages -- form.pdf [out-dir]
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{Document, DuplicatePages, FieldOutcome, OpenOptions, PagePosition, SaveOptions};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let input = PathBuf::from(args.next().ok_or("usage: duplicate_pages form.pdf [out-dir]")?);
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let duplicated = duplicate(std::fs::read(input)?)?;
    std::fs::write(out.join("duplicate_pages.pdf"), duplicated)?;
    Ok(())
}

/// The document with a copy of its first page added at the end.
pub(crate) fn duplicate(bytes: Vec<u8>) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;
    let first = doc.page_info(0)?.id;
    let edited = doc.apply(vec![DuplicatePages::new(vec![first], PagePosition::End).into()])?;
    for field in &edited.fields {
        match &field.outcome {
            FieldOutcome::Renamed { name, .. } => eprintln!("the copy's field is {name}"),
            _ => eprintln!("field {} was not copied: it is a signature", field.field),
        }
    }
    Ok(doc.save(&SaveOptions::new())?.bytes)
}
````

**Result:** [duplicate_pages.pdf](https://docs.laminapdf.com/examples/pdf/duplicate_pages.pdf)

The result has three pages, and the third is a copy of the first. Its text field `name#2` starts with the value the original had, `Ada`, and its checkbox `agree#2` starts unticked. Typing into `name#2` leaves `name` as it was.

## How it works

- `duplicatePages` (`DuplicatePages` in Rust) copies pages by ID to `"start"`, `"end"`, or before or after another page, in the order you list them. Each copy gets a new ID, listed in the edit's `pageOrder`.
- A copy shares its page's content and resources, so the file grows by a few objects, not by the page's images and fonts.
- The copy's links, notes and other annotations are new objects, so a viewer can select, edit or delete them on one page alone.
- A form field shown on the copied page gets a new field with its own value. The edit's `fields` lists each one with its new name.
- In a document with page labels, a copy continues the numbering where it lands, and every other page keeps its label.

## What to change

- **Copy several pages:** list their IDs in the order you want the copies, and they land together at the place you give.
- **Signed forms:** a signature's field isn't copied, since a signature signs one place in one file. The edit's `fields` lists it as `notCopied`, and the copy shows no signature box.
- **Tagged documents:** the copy gets tags of its own, so a screen reader reads it as it reads the original. Content of an element that runs on to another page, such as a paragraph that continues over the page break, stays untagged on the copy. The edit's `tagsIncomplete` lists that copy. On a file that claims PDF/UA-1, saving then warns with [`duplicate-tags-incomplete`](../../errors/duplicate-tags-incomplete.md).
- **Undo:** one `undo()` removes every copy the edit made, with its fields.

Next: [organise pages](../../pages/recipes/organise-pages.md), to delete, move and rotate them too.
