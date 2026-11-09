# Resolution: in Firefox, the reader stops selecting text once a text field has had the focus

[Diagnosis](detail.md).

## The fix

No ancestor of the spans is `user-select: none` any more. The viewer's root keeps the browser's default, and the elements with nothing to select set `none` on themselves instead:

- the page canvases and the detail canvases (`view/viewer.ts`);
- every layer slot except the text layer's, which holds the spans (`view/viewer.ts`);
- the hidden structure tree (`layers/structure-layer.ts`).

The spans set `text`, as before. One helper, `setUserSelect` in `view/user-select.ts`, writes the property with WebKit's prefix for all of them, and its comment says why no ancestor of the spans may be `none`.

Without the root's `none`, a drag can run on past the viewer into the app's own text, as it already could in Chromium before the fix. The text layer used to drop such a selection whole: copy fell back to the browser's, which took in the app's text, the pages it touched were no longer kept mounted, and `onSelection` reported nothing. The text layer now cuts such a selection at the first or last mounted span (`TextLayer.#range`), so copy writes only the page text, the pages it touches stay mounted, and `onSelection` reports them, whichever engine runs. A selection from one viewer into another on the same page is left to the browser, since neither viewer could tell the whole of it.

## How it was checked

- **Regression tests** (`packages/viewer/test/browser/text-layer.browser.test.ts`, in Chromium, Firefox and WebKit): a field takes the focus and two key presses, then it keeps the focus, gets Escape, loses the focus or is left with Tab; each time a real mouse drag across a word selects it. With the root's `none` put back, all four fail in Firefox and pass in the others.
- **Selections past the viewer**: one that ends after the viewer, one that starts before it and one that runs from before it to after it copy only page text; the one that starts before it keeps its pages mounted when the view scrolls far away. A selection from one viewer into another writes nothing to the clipboard. Without the cut, the first fails, and with the cut at the viewer's edge instead of a span, the second loses its pages.
- **Updated test**: "selects only the spans' text when the whole viewer is selected" checked the root's `user-select`; it now checks a canvas's, and still checks the selection and the copied text.
- **The reproduction again**: in the built reader in Firefox, each row of the diagnosis's first table now selects the word ("Page 1", or "Page 2" after going there). A drag from "Page 1" up into the top bar selects the top bar's text in Chromium, Firefox and WebKit alike. Before the fix, Chromium did the same, and Firefox selected one letter.
- **The suites**: the viewer's browser tests (text layer, viewer, navigation) in the three engines, `tsc` and Biome.

## Left for later

- **The app's own text.** A drag that leaves the viewer still highlights the top bar's and status bar's text, though copy leaves it out. Setting `user-select: none` on those bars (siblings of the viewer, not ancestors) in `@laminapdf/ui` would keep the highlight to the document.
- **Reporting it upstream.** The Firefox behaviour deserves a minimal test case on Bugzilla, with the trigger narrowed beyond the diagnosis's table.
