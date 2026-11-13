---
title: "font-not-shapable"
version: "0.2.0"
url: "https://docs.laminapdf.com/reference/codes/font-not-shapable/"
---

# font-not-shapable

- Type: error
- Kind: `unsupported` (valid input that Lamina PDF doesn't implement)

## What happened

A standard 14 font (`Font::standard`) was given to the shaper, `itemize`, `break_lines` or `pdf_run`.

## Why

The standard 14 fonts are referred to by name and never embedded. Lamina PDF has their metrics (Adobe's AFM files) but no font program, so there is nothing for HarfRust to shape with and no `cmap` for itemizing. Nor are there CIDs for a shaped run: a standard 14 font is written as a simple font with one-byte codes.

## How to fix

Draw standard 14 text with `FontRegistry::encode` (one glyph per character, kerning not applied), or load a real font with `Font::from_bytes` and shape with it.

## Example

```rust
let helvetica = Font::standard(Standard14::Helvetica);
shaper.shape(&helvetica, "Hello", &options)?; // font-not-shapable
```

## Data fields

- `font`: the standard font's PostScript name (fixed text: one of the fourteen).
