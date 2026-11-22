---
title: "Flatten a filled form"
description: "Draw a filled PDF form's answers into its pages and remove the fields, so a copy shows the same everywhere and the answers can't be changed."
version: "0.7.0"
url: "https://docs.laminapdf.com/recipes/flatten-form/"
---

# Flatten a filled form

Draw a filled PDF form's answers into its pages and remove the fields, so a copy shows the same everywhere and the answers can't be changed.

You'll flatten a filled invoice form: its answers become part of the page, like its printed labels. You need this before you send a filled form to someone who should read the answers but not change them, or to an archive that keeps no form fields.

`flattenForm` takes the fields to flatten, or, when you leave them out, every field that isn't a signature or locked by one, and whether the copy is for the screen or for printing. Lamina PDF draws each field's picture (its appearance) into the page, answer included, and removes the field from the form. Here it fills the invoice from [Fill a form from JSON](../../pages/recipes/fill-form-from-json.md), then flattens it for printing:

**TypeScript**

`packages/sdk/examples/flatten-form.ts`:

```ts
// Flatten a filled invoice form for sending: every answer becomes part of its page, so the copy
// shows the same everywhere and the answers can no longer be changed.
// The fields go from the form; signature fields, and fields a signature locks, stay. Saving
// as a full rewrite leaves no earlier revision that still holds the fields.
import { Document, type FormDataJson } from "@laminapdf/sdk";

/**
 * The form filled from `answers`, then every field drawn into its page, as a printer would show
 * it. Needs `init({ extract: true })`.
 */
export function flattenForm(form: Uint8Array, answers: FormDataJson): Uint8Array<ArrayBuffer> {
  using doc = Document.open(form);
  doc.fillForm(answers);
  const edited = doc.flattenForm(undefined, "print");
  // A push button or radio button with no appearance of its own stays.
  for (const warning of edited.warnings ?? []) {
    console.warn(warning.code);
  }
  // A full rewrite keeps only the flattened pages, not the revisions with the fields.
  return doc.save({ mode: "fullRewrite" });
}
```

**Rust**

`crates/laminapdf/examples/flatten_form.rs`:

````rust
//! Flatten a filled invoice form for sending: every answer becomes part of its page, so the copy
//! shows the same everywhere and the answers can no longer be changed.
//!
//! The fields go from the form; signature fields, and fields a signature locks, stay. Saving
//! as a full rewrite leaves no earlier revision that still holds the fields.
//!
//! ```sh
//! cargo run -p laminapdf --example flatten_form -- invoice.pdf invoice.json [out-dir]
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{Document, FillOptions, FlattenIntent, OpenOptions, SaveMode, SaveOptions};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let usage = "usage: flatten_form form.pdf answers.json [out-dir]";
    let form = PathBuf::from(args.next().ok_or(usage)?);
    let answers = std::fs::read_to_string(args.next().ok_or(usage)?)?;
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let flat = flatten(std::fs::read(form)?, &answers)?;
    std::fs::write(out.join("flatten_form.pdf"), flat)?;
    Ok(())
}

/// The form filled from `answers`, then every field drawn into its page, as a printer would
/// show it.
pub(crate) fn flatten(bytes: Vec<u8>, answers: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;
    doc.fill_form(answers, FillOptions::default())?;
    let edited = doc.flatten_form(None, FlattenIntent::Print)?;
    // A push button or radio button with no appearance of its own stays.
    for warning in &edited.warnings {
        eprintln!("{}", warning.code());
    }
    // A full rewrite keeps only the flattened pages, not the revisions with the fields.
    let options = SaveOptions::new().with_mode(SaveMode::FullRewrite);
    Ok(doc.save(&options)?.bytes)
}
````

**Result:** [flatten_form.pdf](https://docs.laminapdf.com/examples/pdf/flatten_form.pdf)

The page looks the same as the filled form. Click a box in a PDF reader, and nothing happens: the answers are now part of the page.

## How it works

- Each field keeps the picture a reader shows for it. Flattening draws that picture at the same place and size, then removes the field. Lamina PDF first draws a picture with the answer for a field that has none, and for every field of a form that asks readers to draw its fields.
- `"print"` keeps what a printer shows, and `"view"` what a screen shows. A field hidden for the copy you make is removed without being drawn.
- Signature fields are never flattened: a signature drawn into a page would look present when nobody can check it. Naming one fails with `field-is-signature`. Leaving the fields out flattens every other field, except those a signature locks.
- A push button or radio button with no picture of its own, which each reader draws its own way, stays a field, and the edit's warnings list it as `flatten-no-appearance`.
- Some forms made in Adobe's tools carry a second, XFA version of their fields. Flattening removes it, since Adobe's apps would otherwise show the flattened fields again. When fields other than signatures are left, the warnings say so with `xfa-removed`.
- In a tagged file, the drawings are marked as decoration, outside the structure that screen readers follow, so a screen reader no longer reads the answers out. A file that claims PDF/UA-1 then gets the `flatten-untagged` warning when you save.
- Saving as a [full rewrite](../../pages/reference/glossary.md#full-rewrite) drops the old revisions. An [incremental save](../../pages/reference/glossary.md#incremental-save) keeps them, fields and answers included, at the end of the file.

## What to change

- **Some fields only:** pass their IDs, as `formFields()` lists them, instead of `undefined`. The other fields stay fillable. Naming a field a signature locks makes the save fail.
- **For the screen:** pass `"view"`, and fields that print but don't show on screen are removed without being drawn.
- **A signed form:** flattening changes what a page draws, so saving a flattened certified form fails, whatever its permissions. With only approval signatures, you can save it incrementally (`mode: "auto"`), not as the full rewrite this example uses. Each signature stays valid for the revision it signed, and the fields its lock covers stay fillable.

Next: [flatten comments](../../pages/recipes/flatten-comments.md) to do the same for a review's marks.
