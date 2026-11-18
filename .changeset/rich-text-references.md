---
"@laminapdf/sdk": patch
"@laminapdf/wasm": patch
---

pdf.js now shows bold and italic in text boxes whose text goes beyond ASCII, such as Arabic or accented words. Lamina PDF writes each such character in the box's rich text (`/RC`) as an XML character reference, such as `&#x645;`. It used to write UTF-16, which the standard allows but pdf.js can't read.
