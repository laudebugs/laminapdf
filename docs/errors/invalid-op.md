# `invalid-op`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

An edit carried a value Lamina PDF can't use, so it was refused and the document was left unchanged. Inserting blank pages, for example, needs a page count from 1 to 10,000 and a size whose width and height are above 0 and at most 14,400 points.

## Why

The value came from the caller, often from an op log that a script built or that came from elsewhere. It was out of range: a count of 0 or millions of pages, or a size that is zero, negative or not a number.

## How to fix

Check the values before you apply the edit. Insert blank pages in batches of at most 10,000, and give each page a width and height in points, such as 612 by 792 for US Letter.

## Example

```rust
let op = InsertBlankPages::new(PagePosition::End, 0, PageSize::LETTER);
doc.apply(vec![op.into()])?; // invalid-op: insert at least one page
```

## Data fields

None.

## For contributors

The decisions and todos behind this code:

- [E2b](../todos/done/E2b-page-structure-ops.md): the page-structure ops
