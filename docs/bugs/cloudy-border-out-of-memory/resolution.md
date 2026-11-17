# Resolution: a cloud has at most 4,096 bumps

[Detail](detail.md).

## The fix

`cloud()` now works out its bump count in `bump_count()`: whole bumps of the intensity's width, at least three and at most `MAX_BUMPS`, 4,096. Past that, the bumps grow instead of multiplying, so a cloud's appearance stays a bounded size however long its outline. An outline keeps the bumps it had before up to 16,384 points round at the smallest bumps, and up to about 49,000 points at Acrobat's intensity 1. Both are past A0's perimeter of about 11,500 points, but short of PDF's largest page, 57,600 points round.

An outline whose length overflows `f64` has no spacing for bumps, so it is drawn plain through its points; for a circle, those are the 48 points its cloud follows. Such an outline's coordinates are around 10^307, past the largest number a PDF can hold (about 3.4 × 10^38, ISO 32000-2 Annex C), and the pen writes every coordinate out of that range as 0. The appearance is valid content, a path collapsed to one point, but draws nothing visible. Before the fix, the same input panicked.

The fix changes the rule in the one place that spaces the bumps. It doesn't reject large annotations, which are valid PDF.

## Tests

- `a_cloud_has_at_most_max_bumps_however_long_its_outline` (`crates/laminapdf-build/src/annot/appearance/drawing/tests.rs`): cloudy squares, circles and polygons from 1 point to 10^15 points across, at intensities 0.5, 1 and 2, each with between 3 and 4,096 bumps.
- `the_old_rule_asked_for_more_than_max_bumps_in_34_of_the_48_squares`: the same sweep under the old rule.
- `a_cloud_of_an_ordinary_size_keeps_its_bumps`: a 48-point square at intensity 1 still has its 16 bumps.
- `a_cloud_whose_outline_overflows_is_drawn_plain_and_collapsed`: a square and a circle, each a closed path of straight lines with no curves, written at 0.
- The fuzz input, saved in `fixtures/regressions/ops_json/`, which `laminapdf-save`'s `every_saved_crash_input_replays_without_a_panic` replays.

## Left for later

Nothing for the bumps. A search of the annotation appearance code (`crates/laminapdf-build/src/annot/`) for counts or buffers sized from an annotation's geometry found only `cloud()`. Dashes and ellipses are drawn by the PDF's own operators, so their content doesn't grow with the annotation's size.

One existing rule this bug brought to light: the pen writes any coordinate past the PDF number limit as 0, for every annotation, not only clouds. An annotation that large can't be drawn faithfully in any PDF, so that stays as it is.
