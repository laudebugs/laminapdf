# `summary-page-not-drawn`

- Type: warning
- Kind: `unsupported` (valid input that Lamina PDF can't handle yet)

## What happened

`Document::comment_summary` listed a page's comments without drawing the page beside them. The rest of the summary is as asked: every comment on that page is still listed, with its number.

## Why

A comment summary is a PDF/UA-1 document, and PDF/UA-1 needs every font a page uses to be embedded in the file. This page uses a font the file only names, such as Helvetica, so a copy of the page would break the summary's conformance. Lamina PDF leaves the drawing out instead.

## How to fix

Nothing, if the list of comments is enough. To see the page beside its comments, embed its fonts first, for example by printing the document to PDF again, then make the summary. The list layout, `SummaryLayout::List`, never draws pages and never raises this warning.

## Example

```rust
let summary = doc.comment_summary(&SummaryOptions::new(font))?;
let mut warnings = summary.warnings();
for warning in warnings.next_batch(100) {
    if warning.code() == "summary-page-not-drawn" {
        println!("{}", warning.hint().unwrap_or_default());
    }
}
```

## Data fields

- `page`: the page of the summarised document whose drawing was left out, from 0.
