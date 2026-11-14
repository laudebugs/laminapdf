---
title: "render-too-large"
version: "0.4.0"
url: "https://docs.laminapdf.com/reference/codes/render-too-large/"
---

# render-too-large

- Type: error
- Kind: `limit` (a configured limit was exceeded)

## What happened

A render backend (`laminapdf-render`) refused a request that is too large. The tile has more than `max_render_pixels` pixels or more than `max_render_side` pixels on a side (the document's `RenderLimits`), or more than 65,535 pixels on a side for hayro. Or, for hayro, which reads the whole file, the input is longer than `max_input_bytes`.

## Why

Page sizes and zoom levels come from files and users, so a render could otherwise ask for a bitmap of any size. The limits are checked before anything is allocated. The defaults are 16,777,216 pixels per tile (4096 × 4096, iOS Safari's per-canvas cap), 16,384 pixels per side and 1 GiB of input.

## How to fix

At high zoom, render the visible part of the page as tiles of 256 to 512 pixels (`RenderRequest::with_clip`) instead of the whole page. Raise the limits with `RenderOpenOptions::with_limits` only when the larger bitmap is intended.

## Example

```rust
let req = RenderRequest::page(0, 1000.0); // a letter page at 1000×: 612,000 × 792,000 pixels
doc.render(&req, &mut out, &CancelToken::new())?; // render-too-large
```

## Data fields

- `width`, `height`: the tile's size (tile limits).
- `limit`: the limit exceeded.
- `unit`: `pixels`, `pixels per side` or `pixels per side (hayro)` (tile limits).
- `len`: the input's length in bytes (the input limit, with `limit`).
