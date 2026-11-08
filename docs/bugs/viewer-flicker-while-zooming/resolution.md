# Resolution: the page flickers while zooming

[Diagnosis](detail.md).

## The fix

When `#paintLow` repaints a page whose canvas already shows it in the same orientation at another zoom, it first copies the canvas with `createImageBitmap`, scaled down to the new canvas's size when that is smaller. After resizing, it draws that copy at the new size in place of the low-resolution pass. The canvas then looks exactly as the stretched canvas did a frame earlier, and the sharp tiles for the new zoom replace it a tile at a time. A rotated view or an empty canvas still starts from the low pass, since the old pixels don't fit it, and so does a copy the browser can't make (out of memory, say).

## How it was checked

- **Regression test** (`packages/viewer/test/browser/viewer.browser.test.ts`, "keeps the page's sharp pixels, scaled, when it repaints for a new zoom"): the page is sharp at 100 %, then the zoom goes to 150 %. At the moment the viewer reports the repainted page, the test reads the edge of the page's filled box from the canvas. With the fix the edge stays hard. Without it the edge measures 128 against a sharp page's 510, and the test fails.
- **The reproduction again.** The same ctrl+wheel bursts on the same paper: no frame falls under half the previous frame's sharpness (ten did before), and the score stays between 25 and 47 through the whole zoom.
- **The suites.** `@laminapdf/viewer`'s Node tests (31) and browser tests in Chromium, Firefox and WebKit (40 each).

## Left for later

- **The detail canvas** over the part in view is re-placed at a new zoom and cleared, so above the page canvas's pixel cap the view shows the backdrop (the old pixels at the backdrop's resolution) until the new detail tiles arrive. Carrying the detail canvas's own pixels over the same way would keep it fully sharp.
- **The copy costs memory for a moment:** one more bitmap, no larger than the new page canvas (at most 8 million pixels), closed as soon as it is drawn.
