# `heading-level-skipped`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

Under PDF/UA-1, the document's headings skip a level. Numbered headings are `H1`, `H2` and so on, as opposed to the unnumbered `H`. Either the first numbered heading is not `H1`, or a heading is more than one level below the heading before it (an `H3` straight after an `H1`).

## Why

Screen-reader users move through a document by its headings, and a skipped level suggests a missing section. The PDF/UA-1 standard for accessible PDF (ISO 14289-1 §7.4.2) requires numbered headings to start at `H1` and to go down one level at a time. These are conditions 14-002 and 14-003 of the Matterhorn Protocol, the PDF Association's checklist of PDF/UA failures. Going back up any number of levels is fine.

## How to fix

Use the next level down for a subsection (`H2` under `H1`, `H3` under `H2`). If a heading should only look smaller, change its font size, not its level.

## Example

```rust
s.text("Results", &font, Pt::new(18.0), at)?.tag(TagKind::H1);
s.text("Method", &font, Pt::new(12.0), at2)?.tag(TagKind::H3);
builder.finish()?; // heading-level-skipped: the second heading should be an H2
```

## Data fields

- `page`: the page's index (0-based).
- `level`: the heading's level.
- `previous`: the level of the heading before it (0 when there is none).
