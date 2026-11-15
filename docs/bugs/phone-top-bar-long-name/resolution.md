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

The two other problems in the name are fixed in the same file:

- The name has a 4 px end padding (`--paper-space-4`), so a letter's overhang stays inside the box
  that clips it.
- The × follows the name 8 px after its text: the 4 px padding and 4 px more. R19 in
  `docs/design/paper/reader-a.md` now says 8 px.
- Everything in the file area but the name keeps its size (`flex-shrink: 0`), as does the phone
  bar's More (`phone.css`). The name is the one part that gives way.

## Tests

- `packages/ui/test/browser/phone.browser.test.tsx`: "cuts a long file name short with an ellipsis,
  so every button of the top bar stays on screen" renders the shell with a long name at 320, 390,
  768 and 1280 wide. At each width it checks that every button of the top bar is inside the window
  and that the name is cut short. Before the fix it failed at 320 and 390 (More off screen, name not
  cut).
- `packages/ui/test/browser/reader.browser.test.tsx`: "keeps the whole of the name's last letter and
  leaves 8 px before the × (R19)" loads Newsreader and opens `brief.pdf`. It checks that the ink of
  the name's last letter ends inside the name's box, measured with the canvas, and that the × starts
  8 px after the text. Before the fix it found the ink outside the box and a 2 px gap.
- The phone test also renders at touch density and counts a button narrower than 44 px as
  failing. On the old CSS it reports Back to files and More at both phone widths.
- `@laminapdf/ui`'s browser suite passes in Chromium (1,108 tests).
- The built reader, through `wrangler dev`, in Chromium. An iPhone 13 profile (390 × 844) and a
  1280 × 800 window each opened a PDF with a long name and `brief.pdf`. On the phone the long name
  ends in an ellipsis, Back to files and More stay 44 px wide and on screen, and the page doesn't
  scroll sideways. On the desktop, `brief.pdf` shows its whole "f" with 8 px before the ×. Every
  button has the same size with either name.

## Left for later

Nothing.
