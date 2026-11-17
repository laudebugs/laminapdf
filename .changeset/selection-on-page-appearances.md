---
"@laminapdf/viewer": patch
---

Selections and find matches now show on dark pages, and a found word no longer disappears under its highlight. The text layer is no longer multiplied over the page, which turned a selection near black on a dark page; an app's `::selection` colour now needs to be translucent, such as the accent at 22%, and a column selection takes it from `--lamina-selection` (`SELECTION_PROPERTY`). Find highlights are blended with the page as a group, so the matched words show through them, with `--lamina-find-blend` (`multiply` by default, `screen` for a dark page); their outline and a new 1 px bottom edge (`--lamina-find-match-edge`) are drawn unblended, so the current match stays visible over a picture.
