# Resolution: shortcut chips run their keys together

[Diagnosis](detail.md).

## The fix

`ShortcutChip` now writes each key in its own span (`.paper-shortcut-chip-key`) and marks the chip with its platform (`data-platform`). Its text is unchanged: `⌘V` on macOS, `Ctrl V` elsewhere, so screen readers and the existing tests read the same.

- **The gap.** On macOS, where the keys run together, each key after the first gets `--paper-space-4` (4 px) before it. The owner chose 4 px from renders at 2 and 4 px. Windows and Linux chips keep their spaces and get no gap.
- **The lift.** ⌘ and the arrows carry `data-low` and move up by 0.07 em, the font's own offset, so they line up with the capitals beside them at any size. It is `position: relative`, so the chip's size doesn't change.

## How it was checked

- **Regression test** (`packages/ui/test/browser/display.browser.test.tsx`, "keeps every macOS key 4 px from the next and lifts ⌘ and the arrows to the letters"): every combination of Ctrl, Alt, Shift and Mod with V, an arrow and Escape, 45 bindings. In each macOS chip, adjacent keys must be at least 4 px apart, ⌘ and ↑ must sit at least 0.5 px above the letters beside them, and only those two glyphs may be lifted. Under the old code, 24 of the 45 bindings fail: every one with ⌘. The old chip had no spans, so its gaps couldn't be measured; the ⌘ check is the one that fails.
- **The reproduction again.** The same chips, rendered at 4× in the dark and light themes: the keys stand apart and ⌘ lines up with the letters. The owner approved the render.
- **The suites.** `@laminapdf/ui`'s Node tests (217) and Chromium browser tests (673), `tsc` and Biome.

## Left for later

- **Glyphs missing from the font.** Lamina Sans' subset has no ⌃, ↩, ⌦ or ⇥, and `fonts.css`'s `unicode-range` doesn't claim them, so the browser draws them from another font, with other metrics. Adding them to the subset (`tools/fonts/subset.py`), then measuring whether they need the lift, is its own change.
