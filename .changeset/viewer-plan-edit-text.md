---
"@laminapdf/viewer": minor
---

`client.planEditText(index, run, text, fonts)` says what an `editText` would do without doing it: the characters the run's font can't draw, the font that would draw each and how much wider the line would grow. An editor shows it before it commits, so a font never changes silently.
