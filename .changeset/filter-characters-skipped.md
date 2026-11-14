---
"@laminapdf/sdk": patch
"@laminapdf/wasm": patch
---

A stream encoded with `ASCIIHexDecode` or `ASCII85Decode` that holds characters outside the filter's alphabet now decodes: Lamina PDF skips those characters, as other PDF readers do, and raises the `filter-characters-skipped` warning. Before, the whole stream failed, so a page drawn from it had no text.

When a text string, such as an outline title, holds a byte that PDFDocEncoding (PDF's default text encoding) leaves undefined, such as 0x11, Lamina PDF now reads it as that control character (U+0011), as pdf.js, qpdf and pypdf do, instead of U+FFFD.

Lamina PDF now reads a dictionary that has a stray token, such as a number or `)`, where a key belongs. It skips the token, as pdf.js and PDFium do, and raises the `dictionary-key-skipped` warning. Before, the object failed to parse, and a damaged `/Encrypt` dictionary or page could stop the whole file from opening.

An AES-256 file with a damaged `/OE` or `/UE` entry now opens with the password that doesn't use that entry: the user password when `/OE` is damaged, the owner password when `/UE` is.
