# laminapdf-render

Render PDF pages to RGBA pixels in Rust, a tile at a time, behind one renderer interface with limits on output size and a way to cancel. A pure-Rust backend, hayro, comes behind the `hayro` feature; [`laminapdf-render-pdfium`](../laminapdf-render-pdfium/README.md) adds PDFium.

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

## Page appearance

`RenderRequest::with_appearance` draws a page in Sepia, Dark or High contrast instead of its own colours. Original is the default. The UI theme is separate.

Neither engine can recolour as it paints, so each adapter draws the tile, then recolours it with `recolour_tile`. Three things stay as drawn:

- **Opaque images** keep every pixel. Each adapter finds them with its own parser: hayro through a device that only records where images land, PDFium through its page objects, forms included.
- **See-through images** (a soft mask, a mask, a blend mode) keep their samples, and the paper around them is recoloured with the page.
- **Annotations** keep their colours. The tile is drawn again with them, and every pixel they change comes from that drawing. An annotation that changes most of its rectangle, such as a highlight, keeps the whole rectangle, so the text under it keeps its colour too. The adapter makes this choice once per page with `mostly_changed`, from a small drawing of the whole page, so two tiles that share an annotation agree.

The colour maps use integers only. Dark keeps a colour's hue and inverts its lightness, and High contrast pushes greys toward the ink. `@laminapdf/render` ports them, and the shared vectors in `fixtures/appearance/colours.json` check that both give the same bytes.

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
