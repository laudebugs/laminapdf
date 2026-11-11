---
title: "image-too-large"
version: "0.2.0-alpha.6"
url: "https://docs.laminapdf.com/reference/codes/image-too-large/"
---

# image-too-large

- Type: error
- Kind: `limit` (a `Limits` value or another resource bound was hit)

## What happened

Decoding a PNG would need more than `MAX_DECODED_IMAGE_BYTES` (256 MiB), or the dimensions given to `Image::rgba` describe more bytes than fit in memory's address space.

## Why

The builder decodes PNGs to split colour and alpha; a bound on the decoded size keeps a small file with huge dimensions from exhausting memory.

## How to fix

Scale the image down before adding it, or use a JPEG, which is embedded without decoding.

## Example

```rust
let img = Image::png(&png_of_30000_by_30000_pixels)?; // image-too-large: over 256 MiB decoded
```

## Data fields

- `limit`: the limit in decoded bytes.
