# The page flickers while zooming

The page went blurry for a moment at every pause in a pinch or ctrl+wheel zoom, then sharpened again. Each pause repainted the page for its new zoom by filling the canvas with the low-resolution pass, which threw away the sharp pixels the canvas already showed. The fix is in [resolution.md](resolution.md).

## What the user saw

In the web reader on a Mac, the page flickered while the user zoomed a paper in and out with the trackpad: it flashed blurry, then sharp, several times a second. Once the fingers left the trackpad, the page settled sharp. The screen recording at 120 frames a second shows one- to three-frame blurry flashes at the pauses within the pinch, and some frames where part of the page is sharp and the rest blurry.

## How to reproduce it

1. Build the reader and serve `apps/web/dist/`.
2. Open arXiv 1706.03762 in Chromium at a device pixel ratio of 2.
3. Hold ctrl and scroll the wheel in short bursts: five ticks 16 ms apart, then a 250 ms pause, six times.

A trackpad pinch sends wheel events the same way: bursts with gaps. Any gap longer than the viewer's settle delay (150 ms) ends the "gesture" as far as the viewer can tell.

## The measurement

A script logged every animation frame of that zoom: the page canvas's size in pixels and a sharpness score over the part of the canvas in view. The score is the share of pixels whose neighbour differs by more than 120 in the sum of the colour channels, which a text edge does and a smear doesn't.

| Frame (ms) | Canvas width (px) | Width on screen (CSS px) | Sharpness |
| --- | --- | --- | --- |
| 24 | 2495 | 1248 | 30.3 |
| 1029 | 3368 | 1685 | 0.01 |
| 1312 | 3368 | 1899 | 0 |
| 2013 | 4548 | 2274 | 0.12 |
| 3124 | 4548 | 3070 | 4.1 |
| 6256 | 2899 | 1450 | 18.7 |

Between bursts, the canvas still holds the previous zoom's sharp pixels, stretched by CSS, and scores about 30. As soon as a pause passes the settle delay, the canvas is resized for the new zoom and scores 0: it now shows the low-resolution pass. When the next burst starts before the sharp tiles arrive, those tiles are cancelled and the page stays blurry until the gesture ends. During the zoom, the score fell below half of the previous frame's score ten times.

## The cause

`#paintLow` in `packages/viewer/src/view/viewer.ts` repaints a page for a new geometry (zoom or rotation). It resizes the canvas, which clears it, then draws the low-resolution pass (at most 256 pixels wide) stretched over the whole page. That pass is meant for a page with nothing on it yet. On a re-zoom, the canvas already showed the same page sharp, so the repaint traded those pixels for a much blurrier picture, until the sharp tiles for the new zoom replaced it a tile at a time.
