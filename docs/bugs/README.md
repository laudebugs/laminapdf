# Bugs

One folder per bug we diagnosed, named after its symptom. Each holds `detail.md` (what the user saw, how to reproduce it, the measurements and the cause) and `resolution.md` (the fix, the tests that prove it and what was left for later). How we work a bug from report to fix: [bug triage](../guidelines/bug-triage.md).

| Bug | Symptom | Cause | Fixed in |
| --- | --- | --- | --- |
| [`viewer-blurry-at-zoom`](viewer-blurry-at-zoom/detail.md) | In the reader, part of a page stayed blurry at most zoom levels; 100 % and 200 % looked fine. | The viewer and the renderer sized the page differently, one pixel apart, so the renderer refused the right-hand tile. | `@laminapdf/viewer`, `@laminapdf/render` ([resolution](viewer-blurry-at-zoom/resolution.md)) |
| [`viewer-flicker-while-zooming`](viewer-flicker-while-zooming/detail.md) | The page flashed blurry at every pause in a pinch or ctrl+wheel zoom. | Each repaint for the new zoom filled the canvas with the low-resolution pass, throwing away the sharp pixels it showed. | `@laminapdf/viewer` ([resolution](viewer-flicker-while-zooming/resolution.md)) |
| [`rtl-mark-on-wrong-letter`](rtl-mark-on-wrong-letter/detail.md) | Lamina PDF's text of an Arabic paragraph put three diacritics on the neighbouring letter instead of their own; pdf.js read it exactly. | A shaper draws a mark just before its base, and its origin can fall just inside the letter drawn before, which the position test chose. | `laminapdf-text` ([resolution](rtl-mark-on-wrong-letter/resolution.md)) |
