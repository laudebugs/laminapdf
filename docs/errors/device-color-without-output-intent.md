# `device-color-without-output-intent`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

Under a PDF/A profile, a page draws with `DeviceCMYK`: a `Color::cmyk` colour or a CMYK JPEG.

## Why

PDF/A (ISO 19005, the standard for archival PDF) gives device colour a defined meaning through the document's output intent, which lamina writes as sRGB. Device RGB and gray are covered by it; device CMYK is not, so ISO 19005-2 §6.2.4.3 forbids it under an RGB output intent.

## How to fix

Use `Color::rgb` or `Color::gray`, convert CMYK images to RGB, or give CMYK colours an ICC profile with `Color::icc` (an ICC-based colour carries its own meaning).

## Example

```rust
surface.fill_color(Color::cmyk(0.0, 0.5, 1.0, 0.0));
builder.finish()?; // device-color-without-output-intent under Profile::PdfA2b
```

## Data fields

- `page`: the index (0-based) of the first page that uses `DeviceCMYK`.
