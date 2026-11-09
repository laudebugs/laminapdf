# The thumbnail render-order test fails now and then in WebKit on CI

The viewer's browser test "renders behind the main view's visible pages" failed now and then in WebKit on CI, on branches that didn't touch the viewer, and passed when run again. A test that fails at random gets run again until it passes, so it stops guarding what it tests. The fix is in [resolution.md](resolution.md).

## What was seen

The full lane's "@laminapdf/sdk in WebKit" job failed with:

```
renders behind the main view's visible pages
→ expected 0.41830065359477125 to be 0.777
```

It failed this way on two unrelated branches on 2026-11-09 (`ci/nightly-fuzz-matrix-format`, run 42605200235, and `feat/c6b-links-and-history`, run 42617250916) and passed on reruns and on other branches. Locally, on a fast machine, it never failed.

## What the test checks

The thumbnail strip renders at `background` priority, so a page the main view shows must render first even when thumbnails queued before it. The test:

1. holds the document worker busy for 1.5 s (`__busy`);
2. scrolls the strip, so new thumbnails mount and queue their renders behind the hold;
3. waits, frame by frame, until some thumbnails are mounted without a canvas;
4. sends one `visible` render at scale 0.777;
5. reads the worker's log of renders and expects the 0.777 one first.

## The cause

The hold was a fixed time, and steps 2 to 4 run on the page's own frames. On a slow runner those frames can take longer than 1.5 s. The worker then lets go before the visible render is sent, so it starts the first thumbnail already queued. The log's first entry, 0.418, is a thumbnail's scale, far below the visible render's 0.777. Nothing in the viewer was wrong; the test's setup lost a race, and it couldn't tell that from a real failure.
