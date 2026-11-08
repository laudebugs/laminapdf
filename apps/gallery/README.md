# Paper gallery

The gallery shows every Paper token, icon and `@laminapdf/ui` component in each state, in every UI theme, and in every page appearance where a component shows page content. Its screenshot tests turn any change to how a component looks into an image diff in the pull request, and its axe tests check every page for accessibility problems. It is a development tool: it never ships to users and is never deployed.

## Run it

```sh
cargo xtask gallery
```

The command builds the token, icon and font packages, then serves the gallery with Vite's dev server. The index page lists every route. The bar at the top switches the UI theme, the page appearance, the density, the text direction and the motion setting, and each choice goes into the URL:

```text
http://localhost:5173/?route=buttons&theme=dark&density=touch&dir=rtl
```

Routes with overlays (dialogs, sheets, menus, the palette) open one at a time; `state` chooses which, such as `?route=dialogs&state=password`.

The states each route shows come from `@laminapdf/ui/testing`, the same list the package's own tests use, so a new state written there appears here too. A new component needs a route, or a place on an existing one, in `src/routes.ts`: `test/coverage.test.ts` fails until every component the package exports appears somewhere.

## Test it

```sh
cargo xtask gallery --test
```

The tests run in Docker, in the Playwright image pinned in `package.json` (`config.playwrightImage`, kept equal to the installed `@playwright/test` version by a test). Fonts, browsers and their rasterisation are then the same on every machine, so screenshots match pixel for pixel. There are three specs:

- `e2e/visual.spec.ts` screenshots every route in Chromium. It covers each UI theme, the touch density at a 390 px phone width, forced colours and right to left, plus every page appearance for page-related routes.
- `e2e/a11y.spec.ts` runs axe on every route in every theme, in Chromium, Firefox and WebKit, and expects no violations.
- `e2e/fonts.spec.ts` checks that no request leaves the gallery's origin.

When a test fails, `target/gallery/results/` holds the expected, actual and diff images, and Playwright's HTML report.

## Update the screenshots

```sh
cargo xtask gallery --update
```

The command reruns the tests in the same image, rewrites `e2e/__screenshots__/` and lists the changed files. It is the only way to change a screenshot: a test run never writes one. Commit the changed images with the change that caused them.

Without Docker, run the **Update gallery screenshots** workflow from the repository's Actions tab on your branch. It uploads the new images as an artifact; copy them into `e2e/__screenshots__/` and commit them.

## Review screenshot changes

GitHub shows a changed PNG as an image diff: open the file in the pull request's **Files changed** tab and switch between **2-up**, **Swipe** and **Onion skin**. Check that every changed image follows from the pull request's change, in every theme it touches. A change that affects only one theme, or images on routes the pull request didn't mean to touch, is a bug to fix before merging.

## When a screenshot is flaky

A screenshot that differs between two runs of the same commit is a flaky test, and [ADR 0018 §4](../../docs/decisions/0018-test-strategy.md#4-flaky-tests) treats it as a bug. Quarantine it within a day: mark its test with `test.fixme()` and a comment that links the issue, then fix or delete it within two weeks. Don't raise `maxDiffPixels` to hide the difference. Every comparison starts at zero pixels, and any change to that number needs a recorded reason.
