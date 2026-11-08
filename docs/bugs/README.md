# Bugs

One folder per bug we diagnosed, named after its symptom. Each holds `detail.md` (what the user saw, how to reproduce it, the measurements and the cause) and `resolution.md` (the fix, the tests that prove it and what was left for later). How we work a bug from report to fix: [bug triage](../guidelines/bug-triage.md).

| Bug | Symptom | Cause | Fixed in |
| --- | --- | --- | --- |
| [`viewer-blurry-at-zoom`](viewer-blurry-at-zoom/detail.md) | In the reader, part of a page stayed blurry at most zoom levels; 100 % and 200 % looked fine. | The viewer and the renderer each computed a page's size in pixels, and they differed by one pixel, so the renderer refused the right-hand tile and the page's sharp pass stopped. | `@laminapdf/viewer`, `@laminapdf/render` ([resolution](viewer-blurry-at-zoom/resolution.md)) |
