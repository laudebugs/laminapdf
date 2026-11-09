---
"@laminapdf/viewer": minor
---

Find text across a whole document. `viewer.find` searches from the current page forward and wraps round, in the document Web Worker at background priority, so scrolling never waits. Results stream in by page, and the first match shows as soon as it's found. `next()` and `previous()` move between matches and scroll each into view, and the matches on the page are highlighted. Options cover case, whole words and diacritics. Match counts agree with pdf.js's find. `ViewerClient.find(query, options, pages)` is the call underneath.
