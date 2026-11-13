# Resolution: annotation fixtures without /Resources fail qpdf's check

[Diagnosis](detail.md).

## The fix

`tools/fixtures/annotations.py` gives every page `/Resources`: empty, except on `tagged.pdf`'s page, whose text names `/F1` (Helvetica, a direct dictionary). The four files in `fixtures/annotations/` are written again from it, and copied again to `fuzz/seeds/annots/`, which holds the same files. Every object keeps its number, so `pdfjs.json` and the tests that name annotations by object (`a10.0`, `a16.0`) still hold.

The test stays as it was. It checks the right thing; its fixture was what broke the rule.

## How it was checked

- `qpdf --check` exits 0 on all four fixtures.
- With qpdf installed, the four Rust test files that read them pass: `laminapdf-model`'s, `laminapdf-ops`'s, `laminapdf-save`'s (the one that failed) and the façade's `annotations.rs`.
