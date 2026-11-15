# A long file name pushed the phone's top bar off screen

[Resolution](resolution.md).

## What we saw

On a phone (iPhone, Safari, `app.laminapdf.com`), a PDF with a long file name opened with its name
running past the right edge of the screen, and no ellipsis. The More button, at the end of the top
bar, was out of sight, so the bar's menu couldn't be reached. Back to files, at the start of the bar,
still showed.

## How to reproduce

Open a PDF whose name is wider than the screen, such as
`Laurence_Bugasu_Ininda_Consent_Form_Signed_Copy_For_The_Program_2026_Final_Version.pdf`, in a
window narrower than the phone breakpoint. The browser test "cuts a long file name short with an
ellipsis, so every button of the top bar stays on screen" in
`packages/ui/test/browser/phone.browser.test.tsx` renders the reader's shell and top bar with that
name at four widths. Before the fix, Chromium gave:

| Width | Buttons off screen | Name cut short |
| ---: | --- | --- |
| 320 | More | no |
| 390 | More | no |
| 768 | none | yes |
| 1280 | none | yes |

Only the phone's bar broke. The wider bar wraps its groups onto a second line, so it never had to
fit the name and its buttons on one line.

## Cause

The name already had an ellipsis (`overflow: hidden`, `text-overflow: ellipsis`), but an ellipsis
cuts text only when its box is narrower than the text. That box was never narrower.

`.paper-reader-shell` is a grid with rows and no columns, so its one column was an implicit `auto`
track. An `auto` track is at least as wide as its content's minimum width. The phone's bar keeps its
parts on one line (`flex-wrap: nowrap`), and a line of text that doesn't wrap is as wide as the whole
text at its minimum. So the column grew to the file name's full width, the bar grew with it, and the
shell's `overflow-x: clip` hid everything past the window's edge, More included. The name's box was
as wide as the name, so the ellipsis had nothing to cut.

## Two more problems in the same name

The owner reported these alongside the bug:

- **The name's last letter was cut off.** The end of ".pdf" lost part of its "f". The ellipsis needs
  `overflow: hidden`, which clips at the box's edge, and the box ends where the last letter's advance
  width ends. Newsreader's upright "f" draws 2.58 px past its advance at 16 px (2.74 at the phone's
  17 px), measured with the canvas `measureText` on the font files in `@laminapdf/fonts`. So the
  clip took the "f"'s hook.
- **The × sat too close to the name.** R19 drew it 2 px after the name, which the owner found
  cramped.
- **Buttons shrank beside a long name.** Checking the first fix in the built reader showed that,
  once the name could shrink, the buttons beside it shrank too. Flex items shrink by default, so a
  long name took width from them as well. With a long name, Back to files measured 27 px wide on a
  390 px phone, against 44 px with a short name, below the 44 px touch target. On the desktop, the
  Sidebar toggle went from 38 to 34 px and the editing area's Link button from 68 to 56 px.
