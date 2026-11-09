---
"@laminapdf/viewer": minor
---

Copying from the viewer now follows the page's reading order. A selection across two columns copies the left column, then the right. A paragraph's lines join into one line, hyphenated words join again, and a blank line separates paragraphs. HTML with a paragraph per block goes to the clipboard alongside the plain text.

`viewer.copySelection("exact")` keeps the layout's line breaks instead. An Alt/Option-drag, or a plain drag while `viewer.columnSelect` is on, selects the text inside a rectangle, such as one column. Selections also come as the viewer's `selectionchange` event, whose `settled` flag says when the pointer is up and a toolbar may show.
