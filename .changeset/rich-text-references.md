---
"@laminapdf/sdk": patch
"@laminapdf/wasm": patch
---

Text boxes with bold or italic text in a script beyond ASCII, such as Arabic or a word with an accent, now keep their rich text in pdf.js. The box's rich text (`/RC`) writes each character past ASCII as an XML character reference such as `&#x645;`. Before, it was written as UTF-16, which the standard allows but pdf.js can't read, so pdf.js showed such a box's text without its styles.
