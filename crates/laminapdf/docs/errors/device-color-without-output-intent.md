---
title: "device-color-without-output-intent"
version: "0.2.0-alpha.6"
url: "https://docs.laminapdf.com/reference/codes/device-color-without-output-intent/"
---

# device-color-without-output-intent

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

Under a PDF/A profile, a page draws with `DeviceCMYK` (CMYK given as plain numbers, with no profile to say which inks they mean): a `Color::cmyk` colour or a CMYK JPEG.

## Why

The PDF/A standard for archival PDF (ISO 19005) gives device colour a defined meaning through the document's output intent, which Lamina PDF writes as sRGB. That sRGB intent defines device RGB and gray, but not device CMYK, so ISO 19005-2 §6.2.4.3 forbids CMYK under it.

## How to fix

Use `Color::rgb` or `Color::gray`, convert CMYK images to RGB, or give CMYK colours an ICC profile with `Color::icc` (an ICC-based colour carries its own meaning).

## Example

```rust
surface.fill_color(Color::cmyk(0.0, 0.5, 1.0, 0.0));
builder.finish()?; // device-color-without-output-intent under Profile::PdfA2b
```

## Data fields

- `page`: the index (0-based) of the first page that uses `DeviceCMYK`.
