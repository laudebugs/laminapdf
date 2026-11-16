# Resolution: selected text disappears on a Dark page

[Diagnosis](detail.md). Todo [U2d](../../todos/done/U2d-selection-and-find-on-every-page-appearance.md).

## The fix

The selection and the find marks follow different rules, because they need different things.

**The selection is translucent and unblended.** The design language defines the selection tint as the accent at 22% over the paper (the dark accent at 35% on a Dark page). The token generator solves that opacity from each appearance's paper, accent and tint (`packages/tokens/scripts/page-marks.ts`) and writes the overlay as `--paper-page-selection-overlay`, such as `rgb(125 189 194 / 35%)` for Teal on a Dark page. The text layer's box is no longer multiplied, so the overlay lands on the tint on every appearance and shows over pictures, which keep their own colours. A column selection takes the same colour from `--lamina-selection`.

**The find marks are blended, as a group.** Find colours are a strong amber, which a translucent fill would mix into the ink under it. They stay blended, with the blend chosen from the paper (`multiply` for Original and Sepia, `screen` for Dark) and a fill solved so that paper and fill give the designed colour: `--paper-page-find-blend`, `--paper-page-find-match-fill`, `--paper-page-find-current-fill`. The blend is now on the highlight layer's box, so it reaches the page, and the words under a match show. The outline and the 1 px edge the design language asks for are drawn unblended in a second box, so the current match shows over a picture too.

`@laminapdf/ui`'s `page-marks.css` maps those tokens onto the viewer's `--lamina-*` properties and the text layer's `::selection`, on the root where `data-page-appearance` is set. The web reader's own rules moved there, and the desktop app, which had none, gets the same.

## How it was checked

- **Token tests** (`packages/tokens/test/page-marks.test.ts`, 53 checks): for Original, Sepia and Dark with every accent, the selection lands within 1/255 of its tint and the find colours within 8/255 of theirs; the ink under each measures at least 4.5:1; the selection measures at least 1.1:1 over white and black pictures.
- **Viewer browser tests** (`text-layer.browser.test.ts`, "selection tint", in Chromium, Firefox and WebKit): on a Dark page, a selected word's background lands on `#425858` with its ink at 4.5:1 or more, and a word selected over a white picture still shows; the current find match lands on `find-current` with its outline drawn unblended; and on an Original page a found word keeps its ink under an opaque find colour. Run against the multiplied text layer, the Dark test failed with the selection drawn as `#1C1E1C`.
- **The reproduction again**, in the built reader in Chromium, with a mouse drag across four lines:

| Page | Selection as drawn | Ink against it |
| --- | --- | --- |
| Original | `#CEDBDD` | 13.21:1 |
| Sepia | `#C5CCBF` | 7.18:1 |
| Dark | `#425758` | 4.98:1 |

  A search for "basket" shows the word through its mark on the Original and the Dark page.
- **The suites:** the viewer's Node and Chromium browser tests, the `@laminapdf/ui` and `@laminapdf/tokens` tests, `tsc` and Biome.

## Left for later

- **High contrast.** Its selection is the system's `Highlight` at 40%, and its find marks are screened, since the design language draws High contrast on black paper. The renderer draws it on the system's `Canvas`, which can be white, and the design asks for the text under those fills in `HighlightText` and `MarkText`. That needs its own look.
- **Overlapping line boxes**, carried over from [selection-hides-text](../selection-hides-text/resolution.md): a span is one font size tall, which can be more than the gap between lines, so where two lines' selections overlap the overlay shows twice, as a lighter band on a Dark page.
- **The reader in WebKit and the desktop app** weren't driven by hand here: the reader's home screen didn't come up in Playwright's WebKit on this machine. The viewer's browser tests cover WebKit, and the desktop app loads the same stylesheet.
