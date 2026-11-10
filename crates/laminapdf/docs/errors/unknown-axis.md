---
title: "unknown-axis"
version: "0.2.0-alpha.5"
url: "https://docs.laminapdf.com/reference/codes/unknown-axis/"
---

# unknown-axis

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

`Font::instance` was given a variation axis the font does not have.

## Why

A static font has no axes; a variable font has the axes listed in `FontInfo::axes` (`fvar`).

## How to fix

Check `font.info().axes` before choosing coordinates, or use a variable font.

## Example

```rust
let bold = static_font.instance(&[(OpenTypeTag::WEIGHT, 700.0)])?; // unknown-axis: no wght axis
```

## Data fields

- `font`: the font's PostScript name (**untrusted**: read from the font).
- `tag`: the axis tag that was asked for.
