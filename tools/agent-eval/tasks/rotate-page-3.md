---
id: rotate-page-3
title: Rotate page 3 and save incrementally
checker: rotate-page-3.mjs
inputs: [fixtures/golden/links-and-outlines.pdf => report.pdf]
output: rotated.pdf
---
`report.pdf` has four pages. Rotate its third page by 90 degrees clockwise, leave the other pages as they are, and save the result to `rotated.pdf` as an incremental update: the bytes of `report.pdf` must stay unchanged at the start of `rotated.pdf`, with the change appended after them.
