---
title: "unnumbered-heading-misused"
version: "0.4.0"
url: "https://docs.laminapdf.com/reference/codes/unnumbered-heading-misused/"
---

# unnumbered-heading-misused

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

Under PDF/UA-1, the document misuses the unnumbered heading `H`, a heading whose level comes from the section it sits in. Numbered headings are `H1`, `H2` and so on. Either two `H` elements share one parent, or the document uses `H` and numbered headings together.

## Why

A screen reader works out an `H` heading's level from the sections around it. A section therefore has one `H` at most, and a document that mixes the two kinds gives no consistent outline. The PDF/UA-1 standard for accessible PDF (ISO 14289-1 §7.4.4) requires one `H` per section and only one kind of heading per document. These are conditions 14-006 and 14-007 of the Matterhorn Protocol, the PDF Association's checklist of PDF/UA failures.

## How to fix

Use numbered headings (`TagKind::H1` to `TagKind::H6`) throughout; most documents need nothing else. If you use `H` (`Tag::custom("H", …)`), give each section (`Sect`) its own and leave the numbered ones out.

## Example

```rust
s.text("Results", &font, Pt::new(18.0), at)?.tag(TagKind::H1);
s.text("Method", &font, Pt::new(14.0), at2)?.tag(Tag::custom("H", TagKind::H2));
builder.finish()?; // unnumbered-heading-misused: use TagKind::H2 for the second heading
```

## Data fields

- `page`: the page's index (0-based).
