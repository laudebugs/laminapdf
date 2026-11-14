---
"@laminapdf/sdk": patch
"@laminapdf/wasm": patch
---

A stream encoded with `ASCIIHexDecode` or `ASCII85Decode` that holds characters outside the filter's alphabet now decodes: Lamina PDF skips those characters, as other PDF readers do, and raises the `filter-characters-skipped` warning. Before, the whole stream failed, so a page drawn from it had no text.

A text string's byte that PDFDocEncoding leaves undefined, such as 0x11 in an outline title, now reads as that control character (U+0011), as pdf.js, qpdf and pypdf read it, instead of U+FFFD.

A dictionary with something other than a name where a key belongs, such as a stray number or `)`, now reads: Lamina PDF skips that token, as pdf.js and PDFium do, and raises the `dictionary-key-skipped` warning. Before, the object failed to parse, and a damaged `/Encrypt` dictionary or page could stop the whole file from opening. An AES-256 file whose `/OE` or `/UE` is damaged now opens with a password that doesn't need it.
