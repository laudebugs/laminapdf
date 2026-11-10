# Settings are cut off on a phone

On a phone, the Settings dialog cut its options off at the right edge, so a reader saw half of each choice and had to scroll to find the rest. The dialog kept its desktop layout, a list of sections beside the chosen section's fields, inside a sheet as narrow as the phone. The fix is in [resolution.md](resolution.md).

## What the user saw

In the web reader on a phone, Settings opened as a bottom sheet with "Appearance and accessibility" and "Storage and privacy" in a column on the left, taking half the width. The fields beside them didn't fit: the section's heading read "Appearance and acces", the theme choices "Dar" and "Mat", the motion choice "Rec mo", and the help lines and the switch reached past the edge.

## How to reproduce it

1. Open the web reader in a window 390 px wide, or on a phone.
2. Open Settings from the More menu.

At this width every dialog opens as a bottom sheet (`Dialog` in `packages/ui/src/overlays/dialog.tsx` switches below the phone breakpoint, 599 px).

## The measurement

A browser test opened Settings at 360 × 740 with the reader's two sections and listed every label, button, heading and paragraph whose right edge passed the dialog's. Seven did: "Appearance and accessibility" (the heading), "Dark", "Match system", the second and third accent swatches, "Reduce motion" and "Single-key shortcuts". The section buttons sat in two rows beside the fields, and the fields filled less than half the sheet's width.

## The cause

`.paper-settings` in `packages/ui/src/styles/settings.css` lays out as a grid of two columns: the section list at `--paper-rail-width-min` (196 px) and the fields in the rest. Nothing changed it for a phone. Other dialogs adapt inside a sheet with `.paper-sheet …` rules (the Properties dialog's labels move above their values), but Settings had none. On a 360 px phone the fields' column was about 120 px wide, and the theme and motion choices, two to a row, are wider than that.
