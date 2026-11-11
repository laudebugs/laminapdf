---
title: "invalid-structure-nesting"
version: "0.2.0-alpha.6"
url: "https://docs.laminapdf.com/reference/codes/invalid-structure-nesting/"
---

# invalid-structure-nesting

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

Under PDF/UA-1, a table or list element sits in a parent its type doesn't allow. A table (`Table`) holds rows (`TR`), each holding header cells (`TH`) and data cells (`TD`); a list (`L`) holds list items (`LI`), each holding a label (`Lbl`, its bullet or number) and a body (`LBody`). Examples of a wrong parent: a `TD` or `TH` outside a `TR`, a `TR` outside a `Table`, an `LI` outside an `L`, or a paragraph directly inside a `Table`, `TR`, `L` or `LI`.

## Why

ISO 32000-1 Tables 336 and 337 define how tables and lists are built, and assistive technology relies on that shape to announce rows, columns and list items. The PDF/UA-1 standard for accessible PDF (ISO 14289-1 §7.2) requires tables and lists to have that shape. These are conditions 09-004 and 09-005 of the Matterhorn Protocol, the PDF Association's checklist of PDF/UA failures.

## How to fix

Nest tables as `Table` > `TR` > `TH` or `TD` (optionally with `THead`, `TBody` and `TFoot` around the rows, and a `Caption`), and lists as `L` > `LI` > `Lbl` and `LBody`. Put paragraphs and other content inside the cells and list bodies.

## Example

```rust
s.tag(TagKind::Table, |s| {
    s.text("Total", &font, Pt::new(11.0), at)?.tag(TagKind::TD); // a TD straight in the Table
    Ok(())
})?;
builder.finish()?; // invalid-structure-nesting: wrap the cell in a TR
```

## Data fields

- `page`: the page's index (0-based).
- `kind`: the element's standard type, such as `TD`.
- `parent`: its parent's standard type (`Document` at the top level).
