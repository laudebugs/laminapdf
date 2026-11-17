# Resolution: a cloud has at most 4,096 bumps

[Detail](detail.md).

## The fix

`cloud()` now works out its bump count in `bump_count()`: whole bumps of the intensity's width, at least three and at most `MAX_BUMPS`, 4,096. Past that, the bumps grow instead of multiplying, so a cloud's appearance stays a bounded size however long its outline. An outline keeps the bumps it had before up to 16,384 points round at the smallest bumps, and up to about 49,000 points at Acrobat's intensity 1: past A0's 11,508 points, though not past every page PDF allows.

An outline whose length overflows `f64` has no spacing for bumps, so it is drawn plain: the same closed path without bumps, which the shape still fills and strokes.

The fix changes the rule in the one place that spaces the bumps. It doesn't reject large annotations, which are valid PDF.

## Tests

- `a_cloud_has_at_most_max_bumps_however_long_its_outline` (`crates/laminapdf-build/src/annot/appearance/drawing/tests.rs`): cloudy squares, circles and polygons from 1 point to 10^15 points across, at intensities 0.5, 1 and 2, each with between 3 and 4,096 bumps. On the squares, the old rule asked for more than 4,096 bumps in 34 of those 48 cases.
- `a_cloud_of_an_ordinary_size_keeps_its_bumps`: a 48-point square at intensity 1 still has its 16 bumps.
- `a_cloud_whose_outline_overflows_is_drawn_plain`: a closed path of straight lines, with no curves.
- The fuzz input, saved in `fixtures/regressions/ops_json/`, which `laminapdf-save`'s `every_saved_crash_input_replays_without_a_panic` replays.

## Left for later

A search of the annotation appearance code (`crates/laminapdf-build/src/annot/`) for counts or buffers sized from an annotation's geometry found only `cloud()`. Dashes and ellipses are drawn by the PDF's own operators, so their content doesn't grow with the annotation's size.
