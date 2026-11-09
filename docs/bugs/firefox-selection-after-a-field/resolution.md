# Resolution: in Firefox, the reader stops selecting text once a text field has had the focus

[Diagnosis](detail.md).

## The fix

No ancestor of the spans is `user-select: none` any more. The viewer's root keeps the browser's default, and the elements with nothing to select set `none` on themselves instead:

- the page canvases and the detail canvases (`view/viewer.ts`);
- every layer slot but `text`, which already let the pointer through;
- the hidden structure tree (`layers/structure-layer.ts`).

The spans set `text`, as before. One helper, `selectable` in `layers/box.ts`, writes the property with WebKit's prefix for all of them.

A selection still never takes in a canvas or a gap. A drag that leaves the viewer can now take in the app's own text, such as the top bar's, in Firefox too. Chromium and WebKit already did that before the fix, so the root's `none` had only held the selection inside the viewer in Firefox. Copy still writes only the page text, since the text layer builds it from its spans.

## How it was checked

- **Regression test** (`packages/viewer/test/browser/text-layer.browser.test.ts`, "selects a word under a mouse drag after a text field on the page has had the focus", in Chromium, Firefox and WebKit): a field takes the focus and two keys, loses the focus, then a real mouse drag across a word selects it. With the root's `none` put back, the test fails in Firefox and passes in the others.
- **Updated test**: "selects only the spans' text when the whole viewer is selected" checked the root's `user-select`; it now checks a canvas's, and still checks that the selection and the copied text are only the spans' text.
- **The reproduction again**: in the built reader in Firefox, every case of the table in the diagnosis now selects the word.
- **The suites**: the viewer's browser tests (text layer, viewer, navigation) in the three engines, `tsc` and Biome.

## Left for later

- **The app's own text.** A drag that starts on the page and leaves the viewer selects the top bar's and status bar's text in every engine. Setting `user-select: none` on those bars (siblings of the viewer, not ancestors) in `@laminapdf/ui` would keep a selection to the document.
- **Reporting it upstream.** The Firefox behaviour deserves a minimal test case on Bugzilla.
