# The viewer stays blurry at most zoom levels

## What the user saw

In the web reader (staging, a laptop with a 2× screen), a PDF loaded with some parts blurry, and zooming in or out left parts blurry again. At 100 % the page looked fine; at other zoom levels it didn't. The blur didn't clear with time: a band of the page, usually on the right, stayed soft until the next zoom.

## How to reproduce it

1. Build the reader (`cargo xtask web build`) and serve `apps/web/dist/`.
2. Open a real paper, for example arXiv 1706.03762, in Chromium at a device pixel ratio of 2.
3. Press Zoom in until the zoom reads 153 %, 175 % or 250 %, and wait 6 s.

A band of the page stays at low resolution. At 100 % and 200 % the page sharpens within a second. The viewer's synthetic perf fixtures didn't show it, and neither did one `setZoom` call in a test harness: the arXiv paper and the toolbar's buttons did.

## Slow or stuck

Screenshots at 300 ms, 1 s and 6 s after each zoom showed the same blurry band at 6 s as at 1 s. The viewer's `renderstats` event ended with the page in state `error`, not `sharp`. So some work stopped; nothing was merely slow.

## What was ruled out

Each stage between the zoom and the paint was timed on its own:

| Suspect | Measurement | Verdict |
| --- | --- | --- |
| PDFium parses the page again for every tile | load, parse and close of a page: about 1 ms (Node, `@laminapdf/render-pdfium`) | ruled out |
| Rendering is too slow at 2× | a whole sharp text page at 2×: about 60 ms; the same page as 12 tiles: 65–70 ms | ruled out |
| The low-resolution pass is too small | it is replaced by tiles within a second at 100 % and 200 % | ruled out |
| Tiles are requested one at a time | 12 tiles at 6 ms each can't explain a blur that never clears | ruled out |

None of these could explain a blur that lasted seconds, so the search moved to failures.

## The failure

Logging every request from the viewer to the document worker, with its outcome, showed one refused tile per page at the failing zooms: the right-hand tile of a row, refused with `invalid render request: the tile extends past the page`.

The viewer's pass caught the error, marked the page `error` and stopped. The tiles after the refused one were never asked for, and the low-resolution canvas stayed on screen until the next zoom made a new pass.

## The cause: two definitions of the page's size in pixels

The viewer and the renderer each computed the page's width and height in device pixels:

- **The viewer** sized the canvas and the tile grid from the page's CSS width × the pixel ratio, in 64-bit floats, rounded by the canvas pool (`Math.round`).
- **The renderer** (`planTile` in `@laminapdf/render`) uses `max(1, floor(points × scale))` in 32-bit floats, to agree with the Rust planner.

For a US Letter page (612 pt wide, 816 CSS px at 100 %) on a 2× screen:

| Zoom | Viewer's width (px) | Renderer's width (px) | Right-hand tile |
| --- | --- | --- | --- |
| 100 % | 1632 | 1632 | accepted |
| 153 % | 2497 | 2496 | refused |
| 175 % | 2856 | 2855 | refused |
| 200 % | 3264 | 3264 | accepted |
| 250 % | 4080 | 4079 | refused |

At 100 % and 200 % the product is a whole number, so both rules agree. At most other zooms the viewer's rounding lands one pixel above the renderer's floor, and the right-hand tile reaches one pixel past the page. At 300 % and 400 % the canvas pool's 64 MP cap lowers the pixel ratio to a fraction, and the same mismatch follows.

Run over zooms from 10 % to 800 %, eleven pixel ratios, two page sizes and quarter turns, the old rule planned a tile the renderer refused in 1,521 of 2,486 cases. The user's hunch was right: 100 % is one of the few zooms where the two rules agree.
