# Resolution: Settings are cut off on a phone

Inside a sheet, Settings now lays out in one column: the sections are a row of buttons at the top, and the chosen section's fields fill the sheet's width below them. The cause is in [detail.md](detail.md).

## The fix

New `.paper-sheet` rules in `packages/ui/src/styles/settings.css`, the way the Properties dialog adapts to a sheet:

- **One column.** `.paper-settings` gets a single column, so the fields take the sheet's full width and the theme and motion choices fit two to a row.
- **The sections in a row.** The section list becomes one row of equal buttons. A long name, such as "Appearance and accessibility", wraps onto a second line instead of pushing the row past the edge; scrolling the row sideways was tried first and left "Storage and privacy" half off the screen.
- **No repeated heading.** The heading that repeated the chosen section's name is hidden, since the row's highlighted button already says it. The section keeps its name for assistive technology through its `aria-label`.
- **Tighter gaps.** The gaps between fields drop from 20 px to 16 px, and between the row and the fields to 12 px, so more of a section fits above the fold. At 390 × 844 the whole of "Appearance and accessibility" fits without scrolling.

The desktop dialog is unchanged.

## Tests

`packages/ui/test/browser/settings.browser.test.tsx` opens Settings at 360 × 740 at touch density with the reader's two sections and checks:

- that it opens as a sheet in which no label, button, heading or paragraph reaches past the dialog's edge;
- that the section buttons share one row above the fields, and that the fields take the sheet's full width.

Both failed before the fix (seven elements past the edge; the buttons in two rows, the fields beside them) and pass after it. `cargo xtask pdf-runtimes <browser>` runs them in Chromium, Firefox and WebKit with the package's other browser tests.

## Left for later

Nothing.
