---
"@laminapdf/sdk": patch
"@laminapdf/wasm": patch
---

`builder.markdown` and `builder.flow` now lay a tab in a code block out as spaces to the next multiple of four columns, so the extracted text lines up as drawn. A tab anywhere else in the text becomes one space. Before, a font with CFF outlines (an `.otf` font) raised a `tounicode-conflict` warning for each code block line with a tab.
