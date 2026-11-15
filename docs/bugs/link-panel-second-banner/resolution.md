# Resolution: the link editor's panel added a second banner to the page

[Detail](detail.md).

## Fix

The panel's title row is a `<div>` with the same class, so it adds no landmark. A comment beside it
says why it isn't a `<header>`.

## Tests

- `packages/ui/test/browser/overlays.browser.test.tsx`: "adds no second banner to a page whose top
  bar is its banner" renders a page `<header>` and an open `LinkPanel` and runs axe's
  `landmark-no-duplicate-banner` and `landmark-unique` rules. Before the fix it reported both.
- The gallery's `e2e/a11y.spec.ts` on the four link editor states in three themes passes in Chromium
  (12 tests), run against the gallery's Vite preview.
- `@laminapdf/ui`'s browser suite passes in Chromium (1,109 tests).

## Left for later

Nothing.
