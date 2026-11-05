---
id: tagged-pdf-ua
title: A tagged PDF/UA-1 document
checker: tagged-pdf-ua.mjs
inputs: [fixtures/fonts/NotoSans-Regular.ttf => NotoSans-Regular.ttf]
output: notice.pdf
---
Create `notice.pdf`, an accessible document that conforms to PDF/UA-1: one A4 page with the level 1 heading "Accessible notice" and, below it, the paragraph "This document is tagged for screen readers." Its title is "Accessible Notice" and its language is British English (`en-GB`). Tag the heading as a heading and the paragraph as a paragraph. Use the font in `NotoSans-Regular.ttf` for all text.
