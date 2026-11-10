---
title: "invalid-geometry"
version: "0.2.0-alpha.5"
url: "https://docs.laminapdf.com/reference/codes/invalid-geometry/"
---

# invalid-geometry

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

A page size, page box, `/UserUnit`, drawing value or image size given to the builder is not usable. Drawing values are checked as the `Surface` receives them and the first bad one is reported by `PageBuilder::finish`.

## Why

Page sizes and `/UserUnit` must be finite and positive, and page boxes, path coordinates, matrices and image rectangles finite. Line widths and dash lengths must be finite and not negative, and a dash pattern not all zeros (ISO 32000-2 §8.4.3.6). Miter limits must be at least 1, images at least one pixel wide and tall, font sizes and line heights positive, text boxes wider than 0, and destination coordinates finite.

## How to fix

Check the values passed to `Builder::page`, `PageBuilder::user_unit`, the box setters, the `Surface` (paths, `transform`, `StrokeStyle`, `image`, `text` and `text_box` sizes, positions, boxes and line heights, `link` rectangles), destinations (`Fit` coordinates) and `Image::rgba`.

## Example

```rust
let page = builder.page(PageSize::new(Pt::new(0.0), Pt::new(842.0))); // invalid-geometry at finish
```

## Data fields

- `what`: which value was invalid (fixed text).
