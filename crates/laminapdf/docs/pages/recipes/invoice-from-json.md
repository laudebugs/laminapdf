---
title: "Fill an invoice from JSON"
description: "An invoice template written as JSON and filled from data, its line items repeated into a table that runs across pages, in Rust and TypeScript."
version: "0.3.0"
url: "https://docs.laminapdf.com/recipes/invoice-from-json/"
---

# Fill an invoice from JSON

An invoice template written as JSON and filled from data, its line items repeated into a table that runs across pages, in Rust and TypeScript.

You'll make an invoice from two JSON files: a template with the heading, text and a table, and the data with the number, line items and total. A program or a language model can write either, and the schema checks them before anything is drawn.

The template is a document with placeholders that point into the data. This table row is repeated once per line item, and each cell takes a value from the current item:

```json
"rows": [{
  "$each": "/items",
  "do": { "cells": [
    { "text": { "$item": "/item" } },
    { "text": { "$item": "/description" } },
    { "text": { "$item": "/quantity" }, "align": "end" },
    { "text": { "$item": "/amount" }, "align": "end" }
  ] }
}]
```

Each example fills the template from the data, gives it its two fonts by the names it uses, and returns the PDF's bytes. Both write the same bytes:

**TypeScript**

`packages/sdk/examples/invoice-from-json.ts`:

```ts
// An invoice from a JSON template and its data: the template lays out the heading, the text and
// a table, and the data fills in the number, the line items and the total.
//
// The table runs onto a second page with its header row repeated, and the total stays with the
// last rows. The template names its two fonts, `regular` and `bold`; their bytes come beside it.
// The output is checked against PDF/UA-1, which the template asks for.
import { Builder, type BuilderOptions, type FlowTemplate, Font } from "@laminapdf/sdk";

/**
 * The invoice's bytes. Needs `init({ text: true })`.
 *
 * @param template - the invoice template, a `laminapdf.document/1` document with placeholders.
 * @param data - the values its placeholders read.
 * @param regular - the font the template calls `regular`.
 * @param bold - a collection whose second face (index 1) is the one it calls `bold`.
 * @param options - can fix the file ID, the producer and the creation date.
 */
export function invoiceFromJson(
  template: FlowTemplate,
  data: unknown,
  regular: Uint8Array,
  bold: Uint8Array,
  options: BuilderOptions = {},
): Uint8Array<ArrayBuffer> {
  // The builder checks the profiles it is given: the ones the template asks for.
  using b = new Builder({ ...options, profiles: template.profiles ?? [] });
  b.flow(template, {
    data,
    fonts: { regular: Font.fromBytes(regular), bold: Font.fromCollection(bold, 1) },
  });
  using doc = b.finish();
  return doc.save();
}
```

**Rust**

`crates/laminapdf/examples/invoice_from_json.rs`:

```rust
//! An invoice from a JSON template and its data: the template lays out the heading, the text and
//! a table, and the data fills in the number, the line items and the total.
//!
//! The table runs onto a second page with its header row repeated, and the total stays with the
//! last rows. The template names its two fonts, `regular` and `bold`; their bytes come beside it.
//! The output is checked against PDF/UA-1, which the template asks for. Run it with `cargo run -p
//! laminapdf --example invoice_from_json -- invoice.json invoice-data.json regular.ttf bold.ttc`.

use std::error::Error;

use laminapdf::flow::document::{Resources, fill};
use laminapdf::{BuildOptions, Builder, Font, SaveOptions};

fn main() -> Result<(), Box<dyn Error>> {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    let [template, data, regular, bold] = paths.as_slice() else {
        return Err("arguments: <invoice.json> <invoice-data.json> <regular.ttf> <bold.ttc>".into());
    };
    let read = |path: &String| std::fs::read(path);
    let pdf = build(
        BuildOptions::new(),
        &read(template)?,
        &read(data)?,
        read(regular)?,
        read(bold)?,
    )?;
    std::fs::create_dir_all("target/examples-out")?;
    Ok(std::fs::write("target/examples-out/invoice_from_json.pdf", pdf)?)
}

/// The invoice's bytes. `options` can fix the file ID, the producer and the creation date.
pub(crate) fn build(
    options: BuildOptions,
    template: &[u8],
    data: &[u8],
    regular: Vec<u8>,
    bold: Vec<u8>,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let document = fill(&serde_json::from_slice(template)?, &serde_json::from_slice(data)?)?;
    let options = document
        .profiles()
        .into_iter()
        .fold(options, BuildOptions::with_profile);
    let mut b = Builder::new(options);
    let resources = Resources::new()
        .with_font("regular", &b.font(Font::from_bytes(regular)?)?)
        .with_font("bold", &b.font(Font::from_collection(bold, 1)?)?);
    b.document(&document, &resources)?;
    Ok(b.finish()?.save(&SaveOptions::new())?.bytes)
}
```

**Result:** [invoice_from_json.pdf](https://docs.laminapdf.com/examples/pdf/invoice_from_json.pdf)

The result is two A4 pages that meet PDF/UA-1. The table's header row repeats at the top of the second page, and the total stays with the last rows. The Result tab shows both pages, with a link to the PDF.

## How it works

- `fill` (in TypeScript, `builder.flow` given `data`) replaces each placeholder with the value it points to, then reads the result as a document. A pointer is a path into the data ([RFC 6901](https://www.rfc-editor.org/rfc/rfc6901)): `/invoice/number` is the `number` inside `invoice`, and `/items/0` the first line item.
- `{ "$data": "/total" }` puts a value from the data where text belongs, and `{ "$item": "/amount" }` puts one from the current item of an `$each`.
- `{ "$each": "/items", "do": … }` repeats a block or a table row once per item of a list. `{ "$if": "/invoice/paid", "do": … }` keeps its block only when that value is there and isn't `false` or `null`. This invoice's data says it isn't paid, so the "Paid in full" line is left out.
- The template names its text styles once, such as `body` and `strong`, and each style names a font. The fonts' bytes come beside the template, through `Resources` in Rust and `fonts` in TypeScript, so the template holds no files.
- The template asks for PDF/UA-1 in its `profiles`, so each example passes them to the builder, which checks the rules for accessible PDFs before it writes anything.

## What to change

- **The line items:** edit the data. The template doesn't change, and the table runs onto as many pages as the items need.
- **Totals, dates and currency:** put them in the data already formatted, such as `"total": "4,001.10 €"`. Templates have no arithmetic or formatting by design: whatever writes the data does the sums.
- **A misspelt key:** reading the document fails with `invalid-document`. Its `data.pointer` names the value, such as `/blocks/2/spaceAftr`, and its `didYouMean` the closest key the format knows. In TypeScript, `tsc` refuses a misspelt key in a template you write in your code as a `FlowTemplate`; a template read from a file is checked when `flow` reads it.
- **A value the data lacks:** filling fails with `template-data-missing` and the pointer the template used, so a gap never reaches the PDF.
- **Check what a model writes:** validate the document against `laminapdf.document/1`'s schema. The `laminapdf` crate and `@laminapdf/sdk` both ship it as `docs/schemas/document.schema.json`.

## Build it without code

The command-line tool fills the template and lays it out in one command, giving each font by the name the template uses:

```sh
laminapdf build invoice.json --data invoice-data.json --font regular=NotoSans-Regular.ttf --font bold=NotoSans-RegularBold.ttc#1 -o invoice.pdf
```

`#1` takes the bold face of a font collection. [Make and change PDFs](../../pages/cli/build-and-edit.md#a-pdf-from-json) has the rest.

Next: [Pages that flow](../../pages/guides/flowing-pages.md), with every block a document can hold.
