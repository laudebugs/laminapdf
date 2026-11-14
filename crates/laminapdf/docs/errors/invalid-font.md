---
title: "invalid-font"
version: "0.3.0"
url: "https://docs.laminapdf.com/reference/codes/invalid-font/"
---

# invalid-font

- Type: error
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

Font bytes passed to `Font::from_bytes` or `Font::from_collection` (`laminapdf-text`) could not be read as a font.

## Why

The bytes are not an OpenType/TrueType font or collection (WOFF/WOFF2 must be decompressed first), or a table every font needs (`head`, `hhea`, `hmtx`, `maxp`) is missing or malformed.

## How to fix

Pass the bytes of a `.ttf`, `.otf`, `.ttc` or `.otc` file; check that the file is complete.

## Example

```rust
let err = laminapdf::Font::from_bytes(std::fs::read("font.woff2")?).unwrap_err(); // invalid-font
```

## Data fields

- `index`: the collection member that was asked for (0 for a single font).
