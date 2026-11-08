# Resolution: focus mode's status-bar item fails axe

[Diagnosis](detail.md).

## The fix

`FocusModeItem` now takes its name from its content, as the top bar's Commands button does. Its `aria-label` is gone, so the name is "Focus mode Ctrl \" (or "Focus mode ⌘\"): all the visible text, the keys included. A space between the label and the chip keeps the two apart in the name. It takes no room on screen, because a space-only text node in the item's `inline-flex` box is not laid out; the flex gap still spaces the chip.

The `aria-keyshortcuts` code is removed with its `ariaKeys` helper. React Aria's `ToggleButton` dropped the attribute, so the code never ran. The keys now reach assistive technology through the name.

The six `shell-regions` screenshots are written by `cargo xtask gallery --update`. They show the three regions and the status bar in each theme, mirrored right to left, and at 390 px the rail as a drawer over a scrim.

## How it was checked

- **Regression test** (`packages/ui/test/browser/shell.browser.test.tsx`, "names the focus mode item by all its visible text, its shortcut included"): it turns focus mode on, finds the item by its place in the status bar rather than its name, and runs axe's `label-content-name-mismatch` on it. Under the old code it fails on that rule; with the fix it passes. It also checks the space in the name. The shell's other tests now find the item by its full name.
- **The reproduction again.** `cargo xtask gallery --test`: 428 passed, the nine axe tests in all three browsers included.
- **The suites.** `@laminapdf/ui`'s browser tests in Chromium (689 passed), its typecheck and Biome. Firefox and WebKit ran through the gallery's tests in Docker. The sandbox lacks Playwright's own Firefox and WebKit builds, so the UI's browser suite ran in Chromium only.

## Left for later

- **The gallery's tests outside CI.** No workflow runs `cargo xtask gallery --test`, which is how this bug reached `main`. Running it in the full lane, at least when `cargo xtask changes` reports `gallery=true`, would catch the next one.
- **`aria-keyshortcuts` on React Aria controls.** Any control that wants it needs a way past `filterDOMProps`. None does today.
- **An encoder-only screenshot change.** `--update` also rewrote `palette-results--high-contrast.png` with identical pixels, a change of PNG encoding only. That file is left as it was.
