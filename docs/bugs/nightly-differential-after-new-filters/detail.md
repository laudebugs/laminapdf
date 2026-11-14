# The nightly differential failed on files the new filters decode

[Resolution](resolution.md).

## What happened

The nightly run on `8d01dc83a` (14 November 2026, issue #190) failed in its differential job, on two pdf.js test files.

- **Text.** On `pdfjs/test/pdfs/poppler-90-0-fuzzed.pdf`, a fuzzed copy of a poppler test file, Lamina PDF's text disagreed with both pdf.js and pdftotext on pages 1, 9 and 13. On page 9 Lamina PDF had no text, and its extraction failed with `invalid-structure`: "ASCIIHexDecode: not a hex digit".
- **XMP.** On `pdfjs/test/pdfs/issue18529.pdf`, Lamina PDF read five XMP fields (dates, creator tool, document and instance IDs) where ExifTool read none.

## Cause

#565, merged that day, taught Lamina PDF the `LZWDecode`, `ASCIIHexDecode`, `ASCII85Decode` and `RunLengthDecode` filters. Both files use them.

- The XMP packet of `issue18529.pdf` is `ASCIIHexDecode`-encoded. Lamina PDF now reads it correctly, and ExifTool doesn't decode that filter, so it reads nothing. The difference is ExifTool's.
- The fuzzed file's ASCIIHex and ASCII85 streams hold bytes outside their alphabets. ISO 32000-2 §7.4.2 and §7.4.3 call such a byte an error, and the new decoders refused the whole stream at the first one. pdf.js and poppler skip it and decode the rest, so they had text where Lamina PDF had none.

Issue #37, still open, listed five older disagreements, and three came from the same strictness. A dictionary key that isn't a name made the parser refuse the object, so two mutants of `enc.pdf` failed to open and pdf.js's `issue22011.pdf` lost a page. And an undefined PDFDocEncoding byte in a title became U+FFFD where pdf.js, qpdf and pypdf keep it.
