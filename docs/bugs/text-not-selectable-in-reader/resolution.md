# Resolution: the reader's text can't be selected with the mouse

[Diagnosis](detail.md).

## The fix

Every layer slot but `text` now lets the pointer through. `Viewer.#slot` sets `pointer-events: none` on the built-in `annot` and `highlight` slots and on any slot a layer adds on top. Presses reach the spans. An element a layer wants clicked, such as a link, sets `pointer-events: auto` on itself; `PageLayer`'s comment and the viewer's README say so.

The rule lives where the slots are made, so a layer added later can't cover the text again by forgetting it.

## How it was checked

- **Regression tests** (`packages/viewer/test/browser/text-layer.browser.test.ts`, "pointer", in Chromium, Firefox and WebKit): a real mouse drag across a word in view, through Playwright's mouse (the `dragAcross` command in `vitest.browser.config.ts`), selects the start of that word's text, and the element at the word's centre is its span. A second test adds a layer on top with a child that covers the whole page, and the drag still selects the word. Without the fix both fail: the element at the centre is a slot, and nothing is selected.
- **The reproduction again**: in the built reader, the drag across "Widgets, blue (W-100)" selects it in Chromium and WebKit.
- **The suites**: the viewer's Node tests and browser tests, `tsc` and Biome.

## Left for later

- **Links.** When links mount in the `annot` slot, each link element must set `pointer-events: auto`, and a press on a link should still let a drag that starts beside it select text.
