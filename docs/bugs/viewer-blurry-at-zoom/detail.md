# The viewer stays blurry at most zoom levels

At most zoom levels the renderer refused one tile per page, and the viewer then stopped drawing that page's sharp tiles. The viewer and the renderer each computed the page's size in pixels, and their two rules differed by one pixel: the viewer asked for a tile one pixel wider than the renderer's page. The fix is in [resolution.md](resolution.md).

## What the user saw

The user opened a PDF in the web reader on staging, on a laptop with a 2× screen. Parts of the page loaded blurry, and zooming in or out left parts blurry again. At 100 % the page looked fine; at other zoom levels it didn't. The blur didn't clear with time: a band of the page, usually on the right, stayed soft until the next zoom.

## How to reproduce it

1. Build the reader (`cargo xtask web build`) and serve `apps/web/dist/`.
2. Open a real paper, for example arXiv 1706.03762, in Chromium at a device pixel ratio of 2.
3. Press Zoom in until the zoom reads 153 %, 175 % or 250 %, and wait 6 s.

A band of the page stays at low resolution. At 100 % and 200 % the page sharpens within a second. The viewer's synthetic perf fixtures didn't show it, and neither did one `setZoom` call in a test harness: the arXiv paper and the toolbar's buttons did.

## Slow or stuck

Screenshots at 300 ms, 1 s and 6 s after each zoom showed the same blurry band at 6 s as at 1 s. The viewer's `renderstats` event ended with the page in state `error`, not `sharp`. So some work stopped; nothing was merely slow.

## What was ruled out

We timed each stage between the zoom and the paint on its own:

| Suspect | Measurement | Verdict |
| --- | --- | --- |
| PDFium parses the page again for every tile | load, parse and close of a page: about 1 ms (Node, `@laminapdf/render-pdfium`) | ruled out |
| Rendering is too slow at 2× | a whole sharp text page at 2×: about 60 ms; the same page as 12 tiles: 65–70 ms | ruled out |
| The low-resolution pass is too small | it is replaced by tiles within a second at 100 % and 200 % | ruled out |
| Tiles are requested one at a time | 12 tiles at 6 ms each can't explain a blur that never clears | ruled out |

None of these could explain a blur that lasted seconds, so the search moved to failures.

## The failure

Logging every request from the viewer to the document worker, with its outcome, showed one refused tile per page at the failing zooms: the right-hand tile of a row, refused with `invalid render request: the tile extends past the page`.

The viewer's pass caught the error, marked the page `error` and stopped. It never asked for the tiles after the refused one, so the low-resolution canvas stayed on screen until the next zoom started a new pass.

## The cause: two definitions of the page's size in pixels

The viewer and the renderer each computed the page's width and height in device pixels:

- **The viewer** sized the canvas as the page's CSS width × the pixel ratio, rounded to whole pixels by the canvas pool (`Math.round`). The tiles then used the canvas's own ratio (canvas width ÷ CSS width). So the tile grid covered the canvas exactly, and each tile's request scale was the layout's scale × that ratio.
- **The renderer** (`planTile` in `@laminapdf/render`) uses `max(1, floor(points × scale))` in 32-bit floats, to agree with the Rust planner.

For a US Letter page (612 pt wide, 816 CSS px at 100 %) on a 2× screen:

| Zoom | Viewer's canvas width (px) | Renderer's width (px) | Right-hand tile |
| --- | --- | --- | --- |
| 100 % | 1632 | 1632 | accepted |
| 153 % | 2497 | 2496 | refused |
| 175 % | 2856 | 2855 | refused |
| 200 % | 3264 | 3264 | accepted |
| 250 % | 4080 | 4079 | refused |

The renderer's scale is a 32-bit float, and whether it rounds up or down decides the floor. At 100 % and 200 % it rounds up (2.6666667 becomes 2.66666675), so 612 × scale lands just above 1632 or 3264 and floors to the viewer's width. At 175 % and 250 % it rounds down, so 612 × scale lands just below 2856 or 4080, a whole number the viewer reaches exactly, and floors one pixel short. At 153 % the viewer rounds 2496.96 up while the renderer floors it down. Either way, the right-hand tile reaches one pixel past the renderer's page. At 300 % and 400 % the canvas pool's 64 MP cap lowers the pixel ratio to a fraction, and the same mismatch follows.

Run over zooms from 10 % to 800 %, eight pixel ratios, four pool caps, two page sizes and quarter turns, the old rule planned a tile the renderer refused in 4,365 of 14,464 cases. That is why 100 % and 200 % looked fine: they are among the zooms where the two rules happen to agree.
