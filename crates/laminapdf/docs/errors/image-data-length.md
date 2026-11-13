---
title: "image-data-length"
version: "0.2.0"
url: "https://docs.laminapdf.com/reference/codes/image-data-length/"
---

# image-data-length

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

`Image::rgba(width, height, bytes)` was given a number of bytes other than `width × height × 4`.

## Why

Raw RGBA samples are 4 bytes per pixel, row by row from the top, with no padding.

## How to fix

Pass exactly 4 bytes per pixel.

## Example

```rust
let img = Image::rgba(2, 2, &[0; 12])?; // image-data-length: 16 bytes expected
```

## Data fields

- `expected`: the length the dimensions call for.
- `actual`: the length given.
