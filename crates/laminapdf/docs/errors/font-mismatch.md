---
title: "font-mismatch"
version: "0.3.0"
url: "https://docs.laminapdf.com/reference/codes/font-mismatch/"
---

# font-mismatch

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

`pdf_run` was given a shaped run and the key of a font other than the one the run was shaped with. Nothing was recorded in the registry.

## Why

Glyph IDs, advances and units per em only mean something in the font that produced them; drawing them with another font would show the wrong glyphs and wrong text.

## How to fix

Register every font of a layout and pass the key of the run's own font: each `LineRun::font` is an index into the font list given to `break_lines`.

## Example

```rust
let run = shaper.shape(&noto, text, &options)?;
pdf_run(&run, text, &mut registry, roboto_key)?; // font-mismatch: shaped with Noto
```

## Data fields

- `font`: the registered font's PostScript name (**untrusted**: read from the font).
- `shaped_with`: the PostScript name of the font the run was shaped with (**untrusted**: read from the font).
