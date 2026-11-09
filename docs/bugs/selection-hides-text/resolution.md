# Resolution: selecting text in the reader hides it

[Diagnosis](detail.md).

## The fix

The text layer's box (`.lamina-text-layer`, `layers/text-layer.ts`) is `mix-blend-mode: multiply`, as the find highlights are. Its spans are transparent, so the only thing it paints is the selection, and multiplied over the canvas that tint shows where the page is paper while the ink stays as dark as it was drawn. On the Original page this gives exactly the design language's `selection-tint` under the ink.

The viewer owns the rule, so any app's `::selection` colour lands under the text, not only the reader's.

## How it was checked

- **Regression test** (`packages/viewer/test/browser/text-layer.browser.test.ts`, "selection tint", in Chromium, Firefox and WebKit): with an opaque tint as the selection colour, a selected word keeps at least 80 % of its ink pixels in a real screenshot (the `screenshotOf` command in `vitest.browser.config.ts`). Without the fix it keeps none in every engine.
- **The reproduction again**: in the built reader, a mouse drag across five lines of `header-footer-ordered.pdf` leaves every selected word readable on the tint, in Chromium and WebKit.
- **The suites**: the viewer's Node and browser tests, `tsc` and Biome.

## Left for later

- **Overlapping line boxes.** Each span's box is one font size tall, which can exceed the gap between lines, so neighbouring lines' selections overlap by a pixel or two. Multiplied, the overlap shows as a slightly darker strip, most visibly in WebKit. Spans sized to their line pitch would remove it.
- **Dark page appearances.** Multiply suits a light page. When the reader draws pages in a dark appearance, the selection over light ink needs `screen` instead, chosen with the appearance.
