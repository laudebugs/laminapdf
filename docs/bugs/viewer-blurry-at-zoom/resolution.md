# Resolution: the viewer stays blurry at most zoom levels

[Diagnosis](detail.md).

## The fix

The page's size in pixels now has one home. `@laminapdf/render` exports `pagePixels(page, scale, rotation)`, the rule `planTile` checks tiles against: `max(1, floor(points × scale))` in 32-bit floats, sides swapped on a quarter turn. The viewer uses it for everything it sizes:

- **The sharp scale.** Each page view keeps `scale`, the device pixels per point its canvas was painted at. `sharpScale` in `tiles.ts` computes it: the layout's scale × the pixel ratio, lowered when the canvas pool's pixel cap applies.
- **The canvas.** Its width and height are `pagePixels` at that scale, not CSS size × ratio.
- **The tiles.** `tilesFor` takes the page's size in device pixels from `pagePixels` and clamps every tile to it, and each tile's request carries the same scale.

The viewer can no longer plan a tile the renderer refuses: both read the page's size from the same function. No guard, retry or tolerance was added; the failing tile isn't planned any more.

## How it was checked

- **Regression test** (`packages/viewer/src/render/tiles.test.ts`): a sweep over zooms from 10 % to 800 % in steps of 7, pixel ratios from 1 to 3, the canvas pool fitting all the pixels or only 96 %, 88 % or 66 % of them, US Letter and A4, and quarter turns. Each case sizes the canvas as the viewer does (`sharpScale`, then `pagePixels`). The canvas must stay within what the pool fitted, every planned tile must pass `planTile`, and the tiles must reach the page's right and bottom edges. Under the old rule, 4,365 of those 14,464 cases fail.
- **The reproduction again.** In the built reader at a pixel ratio of 2 on arXiv 1706.03762, every zoom from 153 % to 758 % ends with no failed render request and the page in state `sharp`; 400 % is sharp within 1 s.
- **The suites.** `@laminapdf/viewer`'s Node tests (29) and Chromium browser tests (34), `@laminapdf/render`'s tests (17), `tsc` and Biome.

## Left for later

- **Resolution above about 300 %.** The canvas pool's 64 MP cap lowers the pixel ratio at high zoom (at 400 % the visible page gets about 1.92×, its neighbours about 0.76×). The page is sharp at that resolution but softer than the screen. [C3d](../../todos/ongoing/C3d-viewer-detail-canvas.md) adds a detail canvas over the part in view, at the full ratio at every zoom.
- **One failed tile stops the page's pass.** A tile that fails for another reason, such as a crashed worker, still leaves the rest of the page at low resolution until the next zoom. Drawing the remaining tiles, or retrying, would make that case recover.
