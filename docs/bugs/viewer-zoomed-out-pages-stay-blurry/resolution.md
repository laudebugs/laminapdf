# Zoomed-out pages stay blurry: resolution

## The fix

The page canvas's size now has one measure: whole pixels, as the renderer sizes the page (`pagePixels`).

- **The viewer asks the pool for whole pixels.** It asks for `pagePixels` at the full scale, 79 pixels in the case above, not the fractional 79.33.
- **`sharpScale` compares whole pixels with whole pixels.** It takes that wanted size too. When the pool's fit covers it, the scale stays the full one. Only a real cap, a canvas the pool couldn't fit whole, lowers it.

A page then needs a detail canvas only when its canvas truly can't hold the page at the full pixel ratio, as before the pool's rounding got in the way. At 10 % no page does, so every page turns sharp from its own canvas.

## The tests

- **The regression sweep** (`packages/viewer/src/render/tiles.test.ts`, "keeps the full scale whenever the pool fits the whole page"). It covers three page sizes (letter, A4 and letter turned), every zoom from 10 to 800 %, pixel ratios 1, 1.25, 1.5, 2, 2.5 and 3, and both rotations: 28,476 cases. Whenever the pool grants the whole page, the scale is exactly the full one. The old rule fails 18,742 of the same cases, and the test pins that count.
- **The existing sweep** ("plans only tiles the renderer accepts") now sizes the pool's request in whole pixels, as the viewer does. Every tile still passes the renderer's own plan and reaches the page's edges.
- **The reproduction again.** `pnpm --filter @laminapdf/viewer perf -- --browsers chromium --quick` now completes. The zoom bursts reach sharp pages in at most 533 ms, where the harness had hung.
- **The harness runs again.** It imported `src/view/budgets.ts`, whose `.js` imports Node can't resolve, so it hadn't started since the phone layout's PR. It now imports the built module.

## Left for later

- **A page that truly needs a detail canvas can still wait for good.** If more than three pages in view need one, the pages without one wait, and a sharp page never gives its detail canvas back. Today no layout reaches that: a detail canvas is needed only for pages larger than the backdrop's cap, and three of those never fit in view at once. A sharp page could give its detail canvas back to a waiting page, but that is a change of its own.
- **The cold first page misses its target.** On Fast 4G in Chromium it took 5.1 s against the 3 s target. That is a separate measurement, unrelated to this bug.
