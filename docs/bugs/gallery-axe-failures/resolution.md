# Resolution: the gallery's accessibility tests fail on three views

The find results' page headings are now `h2`, and the gallery checks repeated landmarks inside each sample and colours its sample link. All 21 failures pass in Chromium, Firefox and WebKit. The causes are in [detail.md](detail.md).

## The fixes

- **Find results.** `ResultGroup` in `packages/ui/src/reader/find-results.tsx` heads each page's group with an `h2`, and "No matches" takes `EmptyState`'s default level, 2. The rail's Results tab now reads like Recent: the groups are its top level.
- **Repeated landmarks.** `apps/gallery/e2e/a11y.spec.ts` turns off `landmark-unique` for a page whose grid has more than one sample, then runs that rule inside each sample and on the rest of the page. A component that repeats a landmark within itself still fails; the gallery's side-by-side samples no longer do.
- **The sample link.** `apps/gallery/src/gallery.css` gives links in the sample text `--paper-accent-strong`, which `@laminapdf/tokens`' contrast table checks as text on the ground in every theme.

## Tests

- `packages/ui/test/browser/find-results.browser.test.tsx` now finds each group's heading and the "No matches" title as `h2`.
- `apps/gallery/e2e/a11y.spec.ts`, run with `cargo xtask gallery --test`, passes on every view in all three browsers.

## Left for later

The gallery's tests still run only by hand, in Docker, so a failure like these can sit unseen again. A CI job for `cargo xtask gallery --test` would catch them in the pull request that causes them.
