# Resolution: a right-to-left mark lands on the wrong letter

[Diagnosis](detail.md).

## The fix

`by_position` now records, for each mark, the next letter in content order. A nonspacing mark (every character of its text in general category Mn or Me) whose origin lies within a tenth of the size (`MARK_LEAD`) of that letter's origin goes with that letter. This is where a shaper puts a mark it draws just before its base. Every other mark keeps the old rules: the letter whose advance holds its origin, else the nearest one. So a mark drawn after its letter, over the letter's middle, still stays on it.

## How it was checked

- **Regression tests** in `crates/laminapdf-text/tests/extract_marks.rs`. The first reads the golden's Arabic paragraph and asserts the exact source string. The second builds a small Hebrew file with a qamats drawn before its letter, 0.4 pt inside the letter drawn before it. Both fail without the fix. The third draws the qamats after its letter, over its middle, and passes before and after the fix.
- **The builder's test** `the_arabic_sentence_reads_back_as_written` (`crates/laminapdf-build/tests/tagging_across_pages.rs`) now compares the text exactly. Before, it compared the marks as an unordered set.
- **The suites.** `laminapdf-text`'s tests, `cargo xtask differential --text`, `crates/laminapdf/tests/reading_order.rs`, and the tests of `laminapdf`, `laminapdf-extract` and `laminapdf-build`.

## Left for later

- **A mark at the exact edge between two letters** is still a guess. A producer that draws a mark after its letter, at the letter's end with its ink to the left, puts the origin at the next letter's origin, and the rule now gives the mark to the next letter. Only the mark's ink box, read from the font program, would settle that case.
