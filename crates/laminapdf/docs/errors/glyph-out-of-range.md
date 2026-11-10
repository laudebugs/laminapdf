---
title: "glyph-out-of-range"
version: "0.2.0-alpha.5"
url: "https://docs.laminapdf.com/reference/codes/glyph-out-of-range/"
---

# glyph-out-of-range

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

`FontRegistry::use_glyph` was given a glyph ID at or past the font's glyph count.

## Why

Glyph IDs must come from the same font (its `cmap` or a shaper run on it).

## How to fix

Pass glyph IDs from the font you registered.

## Example

```rust
registry.use_glyph(key, GlyphId::new(60_000), "x")?; // the font has 1,200 glyphs
```

## Data fields

- `gid`: the glyph ID.
- `count`: the font's glyph count.
- `font`: the font's PostScript name (**untrusted**: read from the font).
