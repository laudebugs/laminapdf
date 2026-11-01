# `invalid-structure-nesting`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

Under PDF/UA-1, a table or list element sits in a parent its type doesn't allow. Examples: a `TD` or `TH` outside a `TR`, a `TR` outside a `Table`, an `LI` outside an `L`, or a paragraph directly inside a `Table`, `TR`, `L` or `LI`.

## Why

ISO 32000-1 Tables 336 and 337 define how tables and lists are built, and assistive technology relies on that shape to announce rows, columns and list items. PDF/UA-1, the standard for accessible PDF (ISO 14289-1), in §7.2 requires it (Matterhorn Protocol 09-004 and 09-005; the protocol is the PDF Association's checklist of PDF/UA failures).

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
