---
title: "appearance-needs-text"
version: "0.6.0"
url: "https://docs.laminapdf.com/reference/codes/appearance-needs-text/"
---

# appearance-needs-text

- Type: error
- Kind: `unsupported` (valid input that Lamina PDF does not support yet)

## What happened

An edit adds a text box or callout, or changes one, but this build of Lamina PDF can't lay out its text. The box uses a font given by its bytes, or fallback fonts, and the layout those need was left out of the build.

## Why

Laying out text in any font takes shaping, line breaking and font subsetting, about 470 KB of WebAssembly once compressed. The base and extract modules leave it out to stay small. The extract module, which the reader uses, lays out boxes in Helvetica, Times and Courier with their built-in metrics. The base module lays out no text boxes at all. The text module and the native crates lay out every box.

## How to fix

In the extract module, use Helvetica, Times or Courier for the box. Otherwise, open the document in the text module: call `init({ text: true })` before `Document.open`. In Rust, the `laminapdf` crate with its default features lays out every box.

## Example

```json
{ "op": "addAnnot", "args": { "page": "p3.0", "spec": { "type": "freeText", "rect": { "x0": 72, "y0": 700, "x1": 272, "y1": 730 }, "text": [[{ "text": "مرحبا" }]], "font": "5f6c…" } } }
```

Applied to a document opened in the extract module.

## Data fields

None.
