# Resolution: shortcut chips run their keys together

[Diagnosis](detail.md).

## The fix

`ShortcutChip` now writes each key in its own span (`.paper-shortcut-chip-key`) and marks the chip with its platform (`data-platform`). Its text is unchanged: `⌘V` on macOS, `Ctrl V` elsewhere, so screen readers and the existing tests read the same.

- **The gap.** On macOS, where the keys run together, each key after the first gets `--paper-space-4` (4 px) before it. The owner chose 4 px from renders at 2 and 4 px. Windows and Linux chips keep their spaces and get no gap.
- **The lift.** ⌘ and the arrows carry `data-low` and move up by `--paper-shortcut-chip-glyph-lift`, a new token of 0.75 px. That is ⌘'s own offset, 0.068 em at the chip's 11 px; the arrows' 0.077 em is only 0.1 px more. It is `position: relative`, so the chip's size doesn't change. The token is the one home for the amount; the component only lists which glyphs to lift.

## How it was checked

- **Regression test** (`packages/ui/test/browser/display.browser.test.tsx`, "keeps every macOS key 4 px from the next and lifts ⌘ and the arrows to the letters"): every combination of Ctrl, Alt, Shift and Mod with V, an arrow and Escape, 45 bindings. In each macOS chip, adjacent keys must be at least 4 px apart, and only ⌘ and ↑ may be lifted. Each must sit 0.5 to 1.2 px above every key beside it that isn't. A second test checks that Windows and Linux chips put no gap between their keys, which their text already spaces. Under the old code, 24 of the 45 bindings fail: every one with ⌘. The old chip had no spans, so its gaps couldn't be measured; the ⌘ check is the one that fails.
- **The reproduction again.** The same chips, rendered at 4× in the dark and light themes: the keys stand apart and ⌘ lines up with the letters. The owner approved the render.
- **The suites.** `@laminapdf/ui`'s Node tests (217) and Chromium browser tests (674), `@laminapdf/tokens`' tests (21), the gallery's tests, `tsc` and Biome.

## Left for later

- **The lift while the font loads.** For the moment before Lamina Sans arrives, a fallback face draws ⌘, and the 0.75 px lift is tuned to Lamina Sans. It is too short-lived to need its own handling.

- **Glyphs missing from the font.** Lamina Sans' subset has no ⌃, ↩, ⌦ or ⇥, and `fonts.css`'s `unicode-range` doesn't claim them, so the browser draws them from another font, with other metrics. Adding them to the subset (`tools/fonts/subset.py`), then measuring whether they need the lift, is its own change.
