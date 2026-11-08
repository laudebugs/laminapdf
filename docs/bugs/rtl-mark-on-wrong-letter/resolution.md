# Resolution: a right-to-left mark lands on the wrong letter

[Diagnosis](detail.md).

## The fix

`by_position` now records, for each mark, the next letter in content order. A nonspacing mark (every character of its text in general category Mn or Me) whose origin lies before that letter's origin, by at most a tenth of the size (`MARK_LEAD`), goes with that letter. A shaper draws a mark just before its base, inside that window. Every other mark keeps the old rules: the letter whose advance holds its origin, else the nearest one. So a mark drawn after its letter stays on it, whether over the letter's middle or at the pen, where the next letter starts.

## How it was checked

- **Regression tests** in `crates/laminapdf-text/tests/extract_marks.rs`. The first reads the golden's Arabic paragraph and asserts the exact source string. The second builds a small Hebrew file with a qamats drawn before its letter, 0.4 pt inside the letter drawn before it. Both fail without the fix. Two more draw the qamats after its letter, over its middle and at the pen, and pass before and after the fix.
- **The builder's test** `the_arabic_sentence_reads_back_as_written` (`crates/laminapdf-build/tests/tagging_across_pages.rs`) now compares the text exactly. Before, it compared the marks as an unordered set.
- **The suites.** `laminapdf-text`'s tests, `cargo xtask differential --text`, `crates/laminapdf/tests/reading_order.rs`, and the tests of `laminapdf`, `laminapdf-extract` and `laminapdf-build`.

## Left for later

- **A mark drawn before its base but far from the base's origin** still goes by position alone. A font whose mark anchor puts the origin more than a tenth of the size before the base would show the bug again. Only the mark's ink box, read from the font program, would settle every case.
