# The DIN 91379 text pages have precise differential classifications

[Diagnosis](detail.md).

The text differential now recognizes one deliberate substitution: pdftotext's U+FFFD in place of Lamina PDF's U+009F. The rule applies only to the three PDFBox DIN 91379 `/ActualText` fixtures and only when every other character agrees. Their remaining pdf.js differences are tracked under [issue #110](https://github.com/laudebugs/laminapdf/issues/110), for pdf.js alone. A different character, an extra character, or a disagreement on another file still fails the nightly comparison.

The regression tests check the substitution's exact match and rejection cases, plus a page where this rule and the existing known issue cover separate tools. The downloaded nightly report supplies the three real page comparisons; `cargo test -p xtask` and `cargo xtask ci` validate the change locally.

The extractor's PDFDocEncoding behavior remains as [ADR 0003 §1](../../decisions/0003-reader-tolerance-and-limits.md) specifies. Issue #110 remains the place to resolve the separate pdf.js text differences.
