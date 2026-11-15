# Resolution: a long file name pushed the phone's top bar off screen

[Detail](detail.md).

## Fix

`.paper-reader-shell` (`packages/ui/src/styles/reader.css`) now declares its column:
`grid-template-columns: minmax(0, 1fr)`. The column is as wide as the window, whatever its content
asks for. The top bar then has the window's width, the name's box shrinks to the room left between
Back to files and More, and the ellipsis cuts the name there.

This fixes the cause, the column's width, not the symptom. The name, the bar and the buttons keep
their own rules, and anything else in the shell that doesn't wrap can no longer widen the column
either.

## Tests

- `packages/ui/test/browser/phone.browser.test.tsx`: "cuts a long file name short with an ellipsis,
  so every button of the top bar stays on screen" renders the shell with a long name at 320, 390,
  768 and 1280 wide. At each width it checks that every button of the top bar is inside the window
  and that the name is cut short. Before the fix it failed at 320 and 390 (More off screen, name not
  cut).
- `@laminapdf/ui`'s browser suite passes in Chromium (1,107 tests).

## Left for later

Nothing.
