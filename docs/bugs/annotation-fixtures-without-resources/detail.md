# Annotation fixtures without /Resources fail qpdf's check

[Resolution](resolution.md).

## What we saw

`an_edit_and_a_delete_save_in_both_modes_and_reopen` (`crates/laminapdf-save/tests/annotations.rs`) failed on 2026-11-13 on a Mac with qpdf 12.4.2, the version `doctor.rs` pins, on `main` and on every branch. Its first assertion, that `qpdf --check` accepts the incrementally saved file, failed with exit code 3: qpdf's warnings.

## How to reproduce

With qpdf on `PATH`, `cargo test -p laminapdf-save --all-features --test annotations`. Or check the fixture alone: `qpdf --check fixtures/annotations/threads.pdf` exits 3 with "kid 0 (from 0) Resources is missing or invalid; repairing" for each page. The other three files in `fixtures/annotations/` do the same.

## Cause

The fixtures, not the save. `tools/fixtures/annotations.py` writes every page without `/Resources`, which ISO 32000-2 §7.7.3.3 requires of a page (its own or inherited, an empty dictionary when it uses none). `tagged.pdf`'s page also draws text in a font `/F1` that no resource named. qpdf warns about such a page whatever Lamina PDF does to the file, so the test's check could never pass where qpdf is installed.

CI didn't catch it because `qpdf_accepts` counts a missing qpdf as acceptance, and the jobs that run the Rust tests don't install qpdf.
