---
"@laminapdf/sdk": patch
---

Text extraction now avoids a quadratic scan for right-to-left runs whose glyphs have zero advance, preventing long stalls on PDFs with missing font widths. The extracted text stays the same.
