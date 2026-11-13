# `no-pages-left`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

A page deletion named every page of the document. Lamina PDF refused it and changed nothing, because a PDF needs at least one page for a reader to show.

## Why

The pages given to the deletion cover the whole document. That can happen when a selection of all pages is deleted, or when a script deletes pages one batch at a time and the last batch takes the rest.

## How to fix

Keep at least one page. Delete all the pages but one, or insert a blank page first and then delete the others.

## Example

```rust
let pages: Vec<_> = doc.page_infos()?.into_iter().map(|p| p.id).collect();
doc.apply(vec![DeletePages::new(pages).into()])?; // no-pages-left
```

## Data fields

None.

## For contributors

The decisions and todos behind this code:

- [E2b](../todos/done/E2b-page-structure-ops.md): the page-structure ops
- [E2 plan §11](../plans/editor/e2-page-organisation.md#11-decisions-on-the-open-questions): deleting every page is refused
