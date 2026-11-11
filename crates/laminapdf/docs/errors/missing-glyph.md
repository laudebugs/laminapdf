---
title: "missing-glyph"
version: "0.2.0-alpha.6"
url: "https://docs.laminapdf.com/reference/codes/missing-glyph/"
---

# missing-glyph

- Type: error
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

Text contained a character the font has no glyph for. Lamina PDF never draws `.notdef` (an empty box) silently.

## Why

The font's `cmap` does not map the character (for example a CJK character with a Latin font).

## How to fix

Use a font that covers the character (check with `Font::glyph_for`), or split the text across fonts.

## Example

```rust
registry.encode(latin_font, "東京")?; // missing-glyph: the font has no CJK glyphs
```

## Data fields

- `ch`: the character (**untrusted**: caller or document text).
- `font`: the font's PostScript name (**untrusted**: read from the font).
