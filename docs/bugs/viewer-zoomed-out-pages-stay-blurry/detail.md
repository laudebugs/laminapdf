# Zoomed-out pages stay blurry

## What happened

When you zoomed out far enough that more than three pages showed, the reader drew the first three pages sharp and left the rest blurry for good. The viewer's performance harness showed it first: its zoom scenario zooms the 10-page file out to 10 % with Ctrl+wheel, then waits for every page in view to be sharp, and it never was. The harness stopped there, after 20 seconds, so `cargo xtask viewer-perf` reported nothing at all.

## How to reproduce it

1. Open a file of more than four pages in the reader at fit width.
2. Zoom out with Ctrl+wheel until seven pages show (10 % on A4 in a 1280 × 800 window).
3. Wait. Pages 1 to 3 sharpen; pages 4 to 7 keep the blurry first pass, however long you wait.

The harness's zoom scenario does exactly this, in Chromium.

## Slow or stuck

Stuck. Sampled after 20 seconds, the pages in view were still `low`, the blurry pass, with no render under way and none queued. Pages 1 to 3 held all three detail canvases (the viewer's second pool, which draws the part in view of a page too large for its own canvas).

## Measurements

The viewer's state for each page in view at 10 % (`scale` is the page canvas's device pixels per point, `full` the scale a sharp page needs):

| Page | State | `scale` | Needs a detail canvas | Has one | Waiting |
| --- | --- | --- | --- | --- | --- |
| 1–3 | sharp | 0.132773 | yes | yes | no |
| 4–7 | low | 0.132773 | yes | no | yes |

`full` was 0.133333. Every page's canvas was a hair below the full scale, so every page counted as too large for its canvas and asked for a detail canvas. No page needs one at 10 %, where a page is 79 device pixels wide.

## Cause

Two parts of the viewer measured the page canvas in different units:

- **The canvas pool** fits a canvas in whole pixels. Asked for 79.33 pixels (the harness's 595-point page at 10 %, 96 dpi), it rounds to 79.
- **`sharpScale`** worked out the canvas's scale as 79, what the pool gave, over 79.33, the fractional size the viewer asked for. It read that rounding as a pool cap, and lowered the scale from 0.133333 to 0.132773.

The lower scale made `#needsDetail` true for every page, so each page wanted a detail canvas for its part in view. The pool holds three. Pages 1 to 3 took them and turned sharp; pages 4 to 7 waited for one to come free. A sharp page never gives its detail canvas back, so they waited for good.

A sweep covered three page sizes, every zoom from 10 to 800 %, six pixel ratios and two rotations: 28,476 cases. The old rule went below the full scale in 18,742 of them. The drop usually goes unseen: with one to three pages in view, the three detail canvases cover them, and they turn sharp. Only when more than three pages show do the others wait for good.
