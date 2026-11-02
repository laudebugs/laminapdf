# laminapdf-render

The render backend contract of [ADR 0004](../../docs/decisions/0004-render-backend.md). Every Lamina PDF surface draws pages through two object-safe traits:

- **`RenderBackend`** opens documents from the same `ByteSource` Lamina PDF reads.
- **`RenderDocument`** reports page sizes and renders **tiles**: a rectangle of a page at a scale, with a view rotation, flags and a background. View rotations and a page's own `/Rotate` share one type, `Rotation`, re-exported from `laminapdf-syntax`. The output goes into a caller-supplied buffer in straight-alpha RGBA8, rows packed at `width × 4`.

A renderer only produces pixels. Text, glyph quads, structure, annotation and form geometry, links and outlines come from Lamina PDF's model, never from a renderer.

```rust
use std::sync::Arc;
use laminapdf_render::{CancelToken, HayroBackend, RenderBackend, RenderOpenOptions, RenderRequest, PixelRect};

let doc = HayroBackend::new().open(Arc::new(bytes), &RenderOpenOptions::new())?;
// The top-left 512 × 512 tile of page 1 at 2× (144 DPI), on white.
let req = RenderRequest::page(0, 2.0).with_clip(PixelRect { x: 0, y: 0, width: 512, height: 512 });
let plan = doc.plan(&req)?;                  // validated geometry and the byte count
let mut out = vec![0; plan.byte_len()];
doc.render(&req, &mut out, &CancelToken::new())?;
```

## Requests and limits

- **`TilePlan::new`** validates every request once, so all backends agree on sizes.
  - **Page size:** the whole page is `max(1, floor(points × scale))` pixels per side, after the view rotation.
  - **Tiles:** the tile must lie inside the page.
- **`RenderLimits`** are checked before anything is allocated (`render-too-large`):
  - `max_render_pixels` per tile, default 16,777,216 (4096 × 4096);
  - `max_render_side`, default 16,384;
  - `max_input_bytes`, default 1 GiB, for backends that read the whole file (hayro).
- **Errors** are `RenderError`, in the ADR 0026 contract:
  - **new codes** under `renderErrors` in `fixtures/codes.json`: `render-too-large`, `render-failed` and `render-backend-unavailable`;
  - **reused codes:** `page-index-out-of-range`, `password-required`, `wrong-password`, `data-pending`, `source`, `invalid-argument` and `cancelled`.
- **Cancellation:** a `CancelToken` (the façade's, which now lives in `laminapdf-syntax`) is checked before and after the engine call. A running render can't be interrupted (ADR 0004 §1).
- **Determinism:** the same backend version, input and request give the same bytes.

## Backends

| Feature / crate | Backend | Notes |
| --- | --- | --- |
| `hayro` (off by default) | `HayroBackend`: hayro 0.8.0, pure Rust | ADR 0004's CI oracle and debug choice. Reads the whole source into memory (`data-pending` lists every missing range in one error). Draws tiles with `render_into` under an offset transform. It has no print intent, and draws widgets with the other annotations. |
| [`laminapdf-render-pdfium`](../laminapdf-render-pdfium/README.md) (edge crate) | `PdfiumBackend`: native PDFium | The default on every viewer surface. |

This crate is `core`: no `unsafe`, no I/O, and it builds for wasm32, iOS and Android.

## Tests

- **Fast lane:** `cargo test -p laminapdf-render --features hayro` runs:
  - request validation and the source reader;
  - the error codes against `codes.json`;
  - hayro's error cases and tile geometry (four tiles stitch back into the whole page, and a 90° view equals the upright render rotated).
- **Goldens:** the render goldens are `laminapdf-render-pdfium`'s.
