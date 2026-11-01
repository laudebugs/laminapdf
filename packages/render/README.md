# @laminapdf/render

The interface lamina's web renderers share, the TypeScript twin of the Rust `lamina-render` crate. Two packages implement it: [`@laminapdf/render-pdfium`](../render-pdfium/README.md), the default, and [`@laminapdf/render-hayro`](../render-hayro/README.md), a second engine to compare against and debug with. Both run inside the viewer's document Web Worker. This package is private and internal, and isn't published.

```ts
const backend = await initPdfium(wasmUrl);            // or initHayro()
using doc = backend.open(bytesSource(bytes), { password });
const plan = doc.plan({ page: 0, scale: 2, clip: { x: 0, y: 0, width: 512, height: 512 } });
const { width, height, pixels } = doc.render({ page: 0, scale: 2, clip: plan.tile }, { signal });
self.postMessage(await createImageBitmap(new ImageData(pixels, width, height)));
```

- **`RenderSource`:** `{ size, readSync(offset, length) }`, read synchronously, because PDFium's file access can't await. A read returns `undefined` when its bytes haven't arrived; the call then throws `data-pending` with `ranges` to fetch before retrying. `bytesSource(bytes)` wraps bytes already in memory.
- **`RenderRequest`:**
  - `page` (0-based) and `scale` (pixels per point, device pixel ratio applied);
  - `clip?`: a tile, in the whole page's pixels;
  - `rotation?`: 0, 90, 180 or 270;
  - `flags?`: `annotations`, `formWidgets` (drawn only with annotations), `print`, `grayscale`;
  - `background?`: white by default, `null` for transparent.
- **Output:** straight-alpha RGBA8 rows of `width × 4` bytes.
- **`planTile`:** the port of Rust's `TilePlan::new`, so both adapters agree on sizes. The page is `max(1, floor(points × scale))` pixels per side after the view rotation, and `RenderLimits` (16.7 MP and 16,384 px per tile, 1 GiB of input) are checked before anything is allocated. `pageToTile` is Rust's `TilePlan::page_to_tile`.
- **Errors:** `RenderError` (`code`, `kind`, `ranges`), with lamina's error codes; `isRenderError(e, code)` checks structurally, so errors survive the trip out of a Web Worker. An aborted `signal` throws a `DOMException` named `AbortError`, and a closed document throws `disposed`.

## For contributors

The contract is [ADR 0004](../../docs/decisions/0004-render-backend.md); the renderers run in the document worker of [ADR 0009](../../docs/decisions/0009-web-client-architecture.md) §2, and errors follow [ADR 0026](../../docs/decisions/0026-error-and-diagnostic-contract.md).

### Tests

- **Unit tests:** `pnpm --filter @laminapdf/render test`, for the planner and the error, in Node.
- **The contract suite:** `test/contract.ts` isn't run here. Both adapters run it, in Node and in Chromium, Firefox and WebKit. It checks:
  - every corpus file's page count and size, against lamina's facts;
  - a blank tile is white;
  - four tiles stitch back into the whole page;
  - a 90° view equals the upright render rotated, with and without widgets;
  - typed errors (`render-failed`, `password-required`, `wrong-password`, `page-index-out-of-range`, `render-too-large`, `data-pending` with ranges);
  - `AbortError` and `disposed`;
  - 50 renders don't grow WebAssembly memory beyond 4 MiB;
  - in browsers, page 1 against the render goldens of [C2b](../../docs/todos/done/C2b-lamina-render-crate.md) (SSIM).
- **Browser helpers:** `test/browser-harness.ts` fetches fixtures as Vite assets and decodes the PNGs with the browser's own decoder.
