# The nightly text differential failed on three DIN 91379 pages

[Resolution](resolution.md).

The [14 November nightly job](https://github.com/laudebugs/laminapdf/actions/runs/43603944743/job/132474418223) reported three unexcused text pages, all from PDFBox's DIN 91379 `/ActualText` fixtures. Lamina PDF opened every file. The downloaded `differential-report` artifact identified the three pages and the full text from each reader.

Each page ends its `/ActualText` with byte `0x9F`, which PDFDocEncoding leaves undefined. Lamina PDF preserves that byte as U+009F under [ADR 0003 §1](../../decisions/0003-reader-tolerance-and-limits.md); pdftotext displays U+FFFD. After the differential removes whitespace, that single substitution is the only difference from pdftotext on all three pages.

pdf.js also differs on those pages. The two AWT files map `j` to `ȷ` and `µ` to `μ`; the FOP file has additional glyph and ordering differences. These pdf.js differences belong in the existing [text triage issue #110](https://github.com/laudebugs/laminapdf/issues/110). The 14 November reader change made the three pages fail the nightly comparison because every tool then disagreed with Lamina PDF.
