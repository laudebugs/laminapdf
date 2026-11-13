---
title: "invalid-image"
version: "0.3.0"
url: "https://docs.laminapdf.com/reference/codes/invalid-image/"
---

# invalid-image

- Type: error
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

`Image::jpeg`, `Image::png` or `Image::png_with` (`laminapdf-build`) could not read the bytes as an image of that format.

## Why

A JPEG must start with a start-of-image marker and have a frame header (SOF) before its first scan, with every marker segment inside the data; a PNG must decode without errors (signature, chunk CRCs, zlib data, sizes).

## How to fix

Pass the complete file. Check that the bytes are the format you called (a PNG passed to `Image::jpeg` fails here).

## Example

```rust
let img = Image::jpeg(std::fs::read("photo.png")?)?; // invalid-image: PNG bytes given as JPEG
```

## Data fields

- `format`: `Jpeg`, `Png` or `Rgba`.
- `reason`: what was wrong (fixed text).
