# `invalid-geometry`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

A page size, page box, `/UserUnit`, drawing value or image size given to the builder is not usable. Drawing values are checked as the `Surface` receives them and the first bad one is reported by `PageBuilder::finish`.

## Why

Page sizes and `/UserUnit` must be finite and positive, page boxes, path coordinates, matrices and image rectangles finite, line widths and dash lengths finite and not negative (and a dash pattern not all zeros, ISO 32000-2 §8.4.3.6), miter limits at least 1, images at least one pixel wide and tall, font sizes and line heights positive, text boxes wider than 0, and destination coordinates finite.

## How to fix

Check the values passed to `Builder::page`, `PageBuilder::user_unit`, the box setters, the `Surface` (paths, `transform`, `StrokeStyle`, `image`, `text` and `text_box` sizes, positions, boxes and line heights, `link` rectangles), destinations (`Fit` coordinates) and `Image::rgba`.

## Example

To be added ([L1b](../todos/ongoing/L1b-error-contract-codegen.md)).

## Data fields

- `what`: which value was invalid (fixed text).
