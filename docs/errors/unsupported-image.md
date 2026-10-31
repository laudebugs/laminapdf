# `unsupported-image`

- Type: error
- Kind: `unsupported` (valid input that lamina doesn't implement)

## What happened

The image is valid but uses a feature the builder can't embed.

## Why

PDF's `DCTDecode` filter (ISO 32000-2 §7.4.8) takes baseline and progressive JPEG with 8-bit samples; lossless, hierarchical and arithmetic-coded JPEG and 12-bit samples are refused, as are JPEGs with 2 or more than 4 components.

## How to fix

Re-encode the image as a baseline or progressive 8-bit JPEG, or as PNG.

## Example

```rust
let img = Image::jpeg(lossless_jpeg_bytes)?; // unsupported-image: data.feature says which
```

## Data fields

- `format`: the image's format.
- `feature`: the feature that is not supported (fixed text).
