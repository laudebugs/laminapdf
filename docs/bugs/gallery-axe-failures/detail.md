# The gallery's accessibility tests fail on three views

The Paper gallery's axe tests failed on the reader, reader-mode and link-preview views, in all three browsers. Only one failure is in a shipped component: the find results' page headings sit one level too deep. The other two come from how the gallery shows its samples. Nobody saw them sooner because `cargo xtask gallery --test` couldn't start at all, so the tests hadn't run since several of these components were added. The fixes are in [resolution.md](resolution.md).

## What failed

`cargo xtask gallery --update` on the branch of [laudebugs/laminapdf#418](https://github.com/laudebugs/laminapdf/pull/418), once that branch let the gallery build again, reported 21 failures in `e2e/a11y.spec.ts`: seven views, each in Chromium, Firefox and WebKit.

| View | Themes | axe rule | Element |
| --- | --- | --- | --- |
| `reader` | light, dark, high contrast | `heading-order` | the first find results group's heading |
| `reader-mode` | light, dark, high contrast | `landmark-unique` | the first sample's `<section aria-label="Reader mode">` |
| `link-preview-popover` | dark | `color-contrast` | the sample text's `[12]` citation link |

Two Firefox runs on `reader` also hit the test's 30 s limit inside axe. They passed when run again.

## How to reproduce it

1. Build the gallery's packages: `pnpm --filter @laminapdf/tokens --filter @laminapdf/icons --filter @laminapdf/fonts run build`.
2. In `apps/gallery`, run `pnpm exec playwright test a11y.spec.ts --project=chromium -g "reader|link-preview"`.

Outside the pinned Docker image the screenshots differ, but the axe results are the same.

## The causes

**The find results' headings.** `FindResults` in `packages/ui/src/reader/find-results.tsx` heads each page's group with an `h3`, and its "No matches" state passes `headingLevel={3}`. The rail's Results tab has no heading of its own, so these headings are the tab's top level, like Recent's `h2` groups. On the gallery's page they came straight after the `h1`, which axe reports as a skipped level.

**Reader mode's landmark.** `ReaderView` is a `<section aria-label="Reader mode">`, and the app shows one at a time. The gallery's `reader-mode` route lays three of its states side by side, so three landmarks shared one name. The component is correct; the gallery's page is not something the app ever shows.

**The citation's colour.** The link preview's sample text in `apps/gallery/src/overlays.tsx` stands in for a page of a PDF. Its `[12]` link had no colour of its own, so it kept the browser's default blue, `#0000EE`, which fails contrast on the dark theme's ground.
