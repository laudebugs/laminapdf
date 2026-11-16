# Resolution: a page put before page 1 repeated its label

[Diagnosis](detail.md).

## The decision

New pages ahead of the first labelled page join the start of its range, as Paper's O11 asks. They take the numbers below that range's first label while there are any, so a range that starts at 5 gives one new page the label 4 and nothing else moves. When the range starts too low for that, it starts again at 1 on the first new page, and the range's own pages are numbered up by the shortfall. Later ranges keep their labels. A contents page put before "i" is now "i", the old "i" becomes "ii", and the body and appendix stay as they were.

The rule changes only the start of the list. Every other page keeps its label through moves, deletes and inserts, as E2j decided, so a page inserted inside a range still continues the page before it, and the pages after it keep theirs. A prefix-only range (no numbering style) has no numbers to move, so new pages before it repeat its prefix.

## The fix

`PageLabels::remap` finds the new pages right before the first labelled page in the new list. Before it numbers any page, it raises the values of that page's old range by the shortfall, so new pages inside that range continue the raised numbers. It then gives the leading new pages the numbers from the range's new start. `remap`'s doc comment states the rule.

## How it was checked

- `crates/laminapdf-model/tests/page_tree.rs`: a page before "i" in `labels.pdf` gives i–iv, 1–4, A-1, A-2, Back cover; O11's file (1–3, A-1–A-3) gives 1–4 and keeps A-1–A-3; a range from 5 gives the new page 4; three pages before a range from 2 give 1–5; a prefix-only first range repeats its prefix.
- The property test of random moves, deletes and inserts on `labels.pdf` still requires every other page to keep its label. It now lets the first range's pages change when new pages lead, and requires the first old page's label to differ from every new page's.
- `crates/laminapdf-ops/tests/pages.rs`: `InsertBlankPages` at the start of `labels.pdf` gives i–iv and keeps the rest.
- The model, ops, save and façade crates' tests pass.

## Left for later

The contents page listing (E3l) reads its previews' labels through the same `remap`, so it needs no change of its own; its tests, not yet on `main`, should expect the contents page at "i" and the title page at "ii".
