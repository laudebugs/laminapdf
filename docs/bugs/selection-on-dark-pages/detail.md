# Selected text disappears on a Dark page

With the page appearance set to Dark, selecting text in the reader turned the selected lines into a near-black box with dim text in it, so a reader couldn't see what they had selected. Sepia showed the selection darker than designed, and on every page a find match hid the word it matched. The fix is in [resolution.md](resolution.md).

## What the user saw

The owner set a report's page appearance to Dark and selected its first paragraph. The paragraph sat on a box darker than the page, and its text dropped from the page's light ink to a dim grey-green. The selection toolbar showed, so the selection had worked; only its colours were wrong.

## How to reproduce it

1. Build the reader (`cargo xtask web build`) and serve `apps/web/dist/`.
2. Open `fixtures/reading-order/header-footer-ordered.pdf`.
3. In View settings, choose Dark.
4. Drag the mouse across a few lines.

The lines show a near-black box, and their text is dim.

## What was measured

In the built reader in Chromium, a mouse drag across four lines, then the most common colour of a selected line (its background) and the colour furthest from it (its ink), at a device pixel ratio of 2, with the Teal accent:

| Page | Selection as drawn | Designed `selection-tint` | Ink under the selection | Ink against the selection |
| --- | --- | --- | --- | --- |
| Original | `#CEDBDD` | `#CEDBDD` | `#000000` | 14.80:1 |
| Sepia | `#BDBDA1` | `#C5CCBE` | `#2E2618` | 7.79:1 |
| Dark | `#090B0B` | `#425858` | `#3C4E4B` | 2.24:1, under the 4.5:1 that text needs |

The same arithmetic on every accent gives the same picture: on a Dark page, the selection lands between `#090B0B` and `#0C0A0C` and the ink at 2.17–2.24:1.

## The cause

The text layer, which holds the selection, is drawn over the page with `mix-blend-mode: multiply` (`packages/viewer/src/layers/text-layer.ts`). That was the fix for [selection-hides-text](../selection-hides-text/resolution.md): multiplied over the page, a tint shows where the page is paper and leaves the ink as dark as drawn.

A multiplied fill shows as paper × fill. On the Original page the paper is white, so that is the fill itself. On the Dark page the paper is `#22211F`, so the tint is multiplied down to near black; and the ink, light on a Dark page, is multiplied down with it. That fix's own notes left this open: "Multiply suits a light page. … When the reader draws pages in those appearances, the text layer needs `screen` instead." The reader has since gained page appearances ([U2c](../../todos/done/U2c-page-appearance-modes.md)).

The desktop app has no `::selection` rule and doesn't set the find colours, so it shows the system's selection colour and the viewer's built-in find colours, multiplied the same way.

## Find matches hid the word they matched, on every page

Checking the find marks turned up a second fault with the same root. The find marks were blended one by one, inside the highlight layer's box. That box is transformed to the page's size, which makes it a group of its own, so each mark blended with the empty group and was then drawn over the page as it was: an opaque fill over the word it matched. In the built reader, searching for "basket" showed an amber box with no word in it on the Original page and a brown one on the Dark page. The text layer never had this fault, since its blend is on the transformed box itself.

## What the fix had to weigh

One blend chosen with the paper (`multiply` on a light paper, `screen` on a dark one), with a fill solved so the result lands on the designed colour, fixes the page but not the pictures on it: pictures and annotations keep their own colours in every appearance, and a screened selection over a white scan on a Dark page measures 1.0:1 against it, invisible.

The design language defines the tint as the accent at 22% over the paper (the dark accent at 35% on a Dark page). Drawn translucently with no blend, that lands on the tint exactly and shows over any picture:

| Page | Ink against the selection | Selection over a white picture | Over a black picture |
| --- | --- | --- | --- |
| Original | 9.58–9.69:1 | 1.41–1.45:1 | 1.10–1.12:1 |
| Sepia | 7.17–7.25:1 | 1.41–1.45:1 | 1.10–1.12:1 |
| Dark | 4.91–5.02:1 | 1.27–1.30:1 | 1.92–1.97:1 |

The find colours are a strong amber and need the other way: translucent, the colour that lands on `find-current` tints the ink with it, to 2.97:1 on Original and 2.41:1 on Sepia. Blended with a solved fill, the ink stays as drawn, at 5.3–12.8:1.
