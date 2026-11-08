---
"@laminapdf/sdk": minor
"@laminapdf/wasm": minor
---

Text runs and text boxes now say where the next element can start: `bottom` on what `surface.text()` and `surface.textBox()` return (and `TextRun::bottom()` and `TextLayout::bottom()` in Rust) is the bottom of the last line that draws something, where the next text box's top can go. It is left out when nothing is drawn, such as empty text, text of only white space, or a box too short for one line, so an empty section adds no gap. A clipped box gives its last drawn line; check `overflow !== undefined` before treating the text as placed.
