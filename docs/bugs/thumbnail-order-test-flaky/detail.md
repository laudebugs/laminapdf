# The thumbnail render-order test fails now and then in WebKit on CI

The viewer's browser test "renders behind the main view's visible pages" failed now and then in WebKit on CI, on branches that didn't touch the viewer, and passed when run again. A test that fails at random gets run again until it passes, so it stops guarding what it tests. The fix is in [resolution.md](resolution.md).

## What was seen

The full lane's "@laminapdf/sdk in WebKit" job failed with:

```
renders behind the main view's visible pages
→ expected 0.41830065359477125 to be 0.777
```

On 2026-11-09 it failed this way on two unrelated branches and passed on reruns and on other branches:

- `ci/nightly-fuzz-matrix-format`, run 42605200235;
- `feat/c6b-links-and-history`, run 42617250916.

Locally, on a fast machine, it never failed.

## What the test checks

The thumbnail strip renders at `background` priority, so a page the main view shows must render first even when thumbnails queued before it. The test:

1. holds the worker busy for 1.5 s (`__busy`): the hold;
2. scrolls the strip, so new thumbnails mount and queue their renders behind the hold;
3. waits, frame by frame, until some thumbnails are mounted without a canvas;
4. sends one `visible` render at scale 0.777;
5. reads the worker's log of renders and expects the 0.777 one first.

## The cause

The test had two wrong assumptions; nothing in the viewer was wrong.

First, the hold was a fixed time, and steps 2 to 4 run on the page's own frames. On a slow runner those frames can take longer than 1.5 s. The hold then ends before the test sends the visible render, and the worker starts the first queued thumbnail.

Second, step 5 expected the test's render to be the first in the log, but the main view renders at `visible` priority too. A thumbnail renders at 0.196 (120 px over a 612 pt page). The failing log's first entry, 0.418, is the main view's own render: the 256 px wide test view's fit-width scale. On a slow runner the main view still had a render queued when the hold began. It ran first, as its priority says it should. The first fix took 0.418 for a thumbnail, so it addressed only the timing, and CI failed again with the timing check passing.
