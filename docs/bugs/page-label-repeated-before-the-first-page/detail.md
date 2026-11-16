# A page put before page 1 repeated its label

[Resolution](resolution.md).

## What we saw

Opening `fixtures/golden/links-and-outlines.pdf` in the web reader and creating a contents page before page "i" (todo [E3l](../../todos/ready/E3l-contents-page.md)) gave both the contents page and the title page the label "i". Inserting blank pages, inserting pages from another file or merging documents at the start of any labelled file did the same whenever its first range started at 1: "i", "1" or "A-1".

Paper's design for the contents page dialog ([O11](../../design/paper/pages-navigation-extraction.md)) expects the opposite: a contents page put before page 1 takes label 1, every later page in that range goes up by one, and the appendix keeps A-1 to A-3.

## How to reproduce

Insert one blank page at the start of `fixtures/pages/labels.pdf` (labels i–iii, 1–4, A-1, A-2, Back cover) and read the labels back: before the fix they were i, i, ii, iii, 1, ….

## Cause

Every page op rebuilds the labels with `PageLabels::remap` (`crates/laminapdf-model/src/labels/remap.rs`, from [E2j](../../todos/done/E2j-page-tree-and-tree-writers.md)). Its rule keeps each old page's label and lets new pages at the start "lead into the first page's range, numbered down to 1". Because label numbers start at 1 (ISO 32000-2 §12.4.2, `/St`), a range that already starts at 1 leaves no number below it, so the code clamped every new page to 1 and two pages shared it. No ADR fixes the rule: E2j and the E2 plan (§ Page labels) state it only as "each surviving page keeps its label".
