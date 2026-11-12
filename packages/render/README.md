# @laminapdf/render

The interface Lamina PDF's web renderers share, the TypeScript twin of the Rust `laminapdf-render` crate. Two packages implement it: [`@laminapdf/render-pdfium`](../render-pdfium/README.md), the default, and [`@laminapdf/render-hayro`](../render-hayro/README.md), a second engine to compare against and debug with. Both run inside the viewer's document Web Worker. It's on npm as `@laminapdf/render`; most apps reach it through `@laminapdf/viewer`.

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
  - `background?`: white by default, `null` for transparent;
  - `appearance?`: `"original"` (the default), `"sepia"`, `"dark"` or `{ kind: "high-contrast", ink, paper }`.
- **Page appearance:** each adapter draws the tile, then recolours it with `recolourTile`. Opaque images keep every pixel, and see-through images keep their samples while the paper around them is recoloured. An annotation keeps what it draws, and an annotation that changes most of its rectangle, such as a highlight, keeps the whole rectangle, so the text under it keeps its colour. `mostlyChanged` makes that choice once per page, so tiles that share an annotation agree. The colour maps are the Rust crate's, and the shared vectors in `fixtures/appearance/colours.json` check that both give the same bytes.
- **Output:** straight-alpha RGBA8 rows of `width × 4` bytes.
- **`planTile`:** the port of Rust's `TilePlan::new`, so both adapters agree on sizes. The page is `max(1, floor(points × scale))` pixels per side after the view rotation, and `RenderLimits` (16.7 MP and 16,384 px per tile, 1 GiB of input) are checked before anything is allocated. `pageToTile` is Rust's `TilePlan::page_to_tile`.
- **Errors:** `RenderError` (`code`, `kind`, `ranges`), with Lamina PDF's error codes. `isRenderError(e, code)` checks structurally, so it works across copies of the package. `toDiagnostic()` returns the same error as the plain object every Lamina PDF surface uses. `@laminapdf/sdk`'s Web Worker posts that object, and the page receives a `LaminaPdfError` with the same code, message and ranges. An aborted `signal` throws a `DOMException` named `AbortError`, and a closed document throws `disposed`.

## For contributors

The contract is [ADR 0004](../../docs/decisions/0004-render-backend.md); the renderers run in the document worker of [ADR 0009](../../docs/decisions/0009-web-client-architecture.md) §2, and errors follow [ADR 0026](../../docs/decisions/0026-error-and-diagnostic-contract.md).

### Tests

- **Unit tests:** `pnpm --filter @laminapdf/render test`, for the planner and the error, in Node.
- **The contract suite** that both adapters run, in Node and in Chromium, Firefox and WebKit, lives in [`@laminapdf/test-utils`](../test-utils/README.md), with the browser harness and config it uses.
