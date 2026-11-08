# A right-to-left mark lands on the wrong letter

Lamina PDF's text extraction read the Arabic paragraph of `fixtures/golden/tagged-across-pages.pdf` with three diacritics on the neighbouring letter instead of their own: `تظُهر` for `تُظهر`, `يشُكَّل` for `يُشكَّل` and `ويرُتبَّ` for `ويُرتَّب`. pdf.js 6.4.299 reads the same page exactly. The fix is in [resolution.md](resolution.md).

## How to reproduce it

Extract page 3 of `fixtures/golden/tagged-across-pages.pdf` with `TextExtractor::page` and compare the Arabic runs' text with the `ARABIC` constant in `crates/laminapdf-build/tests/common/across_pages.rs`. Lamina PDF's own builder writes the file: HarfRust shapes the paragraph in Noto Sans Arabic at 14 pt, and the builder draws each line in visual order, left to right.

## The measurement

The glyphs around the first wrong mark, in content order, with their origins along the baseline:

| Glyph | Origin x (pt) | Advance (pt) | Box (pt) |
| --- | --- | --- | --- |
| ظ | 507.87 | 9.94 | 507.87 → 517.81 |
| damma (U+064F) | 517.37 | 0 | |
| ت's dot (no text) | 517.88 | 0 | |
| ت | 517.81 | 4.19 | 517.81 → 522.00 |

The damma belongs to ت, the letter drawn after it. Its origin sits 0.44 pt inside ظ, the letter drawn before it. The other two wrong marks look the same: the damma on ي is 0.38 pt inside ر, and the shadda and fatha on ت are 0.52 pt inside ب. Each offset is under 4% of the size. Each base is a narrow letter, so its anchor sits left of the mark's anchor and the move goes left.

## The cause

`by_position` in `crates/laminapdf-text/src/extract/runs.rs` gave each zero-advance mark to the letter whose advance holds its origin. Shaping engines (HarfBuzz, and HarfRust in our builder) reverse a right-to-left cluster into visual order, so a cluster's marks come before its base. The shaper draws each mark at its base's origin, moved by the difference between the two anchors. When that move is to the left, the origin falls just inside the letter drawn before. The position then points at the wrong letter.

Content order can't settle the case on its own either. The damma has a letter on each side of it in the content stream, so "the letter it follows" (ظ) is just as wrong. The distance from the mark's origin to each letter's origin tells the two apart: 0.44 pt to ت's origin, 9.5 pt to ظ's.
