# Resolution: the page flickers while zooming

[Diagnosis](detail.md).

## The fix

When `#paintLow` repaints a page whose canvas already shows it in the same orientation at another zoom, it first copies the canvas (`createImageBitmap`). After resizing, it draws the low-resolution pass and then that copy over it, scaled to the new size. The canvas then looks exactly as the stretched canvas did a frame earlier, and the sharp tiles for the new zoom replace it a tile at a time. A rotated view or an empty canvas still starts from the low pass, since the old pixels don't fit it.

## How it was checked

- **Regression test** (`packages/viewer/test/browser/viewer.browser.test.ts`, "keeps the page's sharp pixels, scaled, when it repaints for a new zoom"): at 100 % the page is sharp, then the zoom goes to 150 %, and the test reads the edge of the page's filled box from the canvas at the moment the viewer reports the repainted page. With the fix the edge stays hard; without it the edge is 128 against a sharp page's 510, and the test fails.
- **The reproduction again.** The same ctrl+wheel bursts on the same paper: no frame falls under half the previous frame's sharpness (ten did before), and the score stays between 25 and 47 through the whole zoom.
- **The suites.** `@laminapdf/viewer`'s Node tests (29) and browser tests in Chromium and Firefox (35 each). WebKit in this container runs out of memory partway through the suite, with or without the fix; it passed on the branch with the smaller page canvases.

## Left for later

- **The detail canvas** (PR 317, not merged yet) is re-placed at a new zoom and cleared, so above the page canvas's pixel cap the view shows the backdrop (the old pixels at the backdrop's resolution) until the new detail tiles arrive. Carrying the detail canvas's own pixels over the same way would keep it fully sharp.
- **The copy costs memory for a moment:** one more bitmap the size of the page canvas, closed as soon as it is drawn.
