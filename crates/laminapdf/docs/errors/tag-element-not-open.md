---
title: "tag-element-not-open"
version: "0.3.0"
url: "https://docs.laminapdf.com/reference/codes/tag-element-not-open/"
---

# tag-element-not-open

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

`Surface::resume` or `Builder::close_element` was given a structure element that isn't open. When `resume` names one, the page's `PageBuilder::finish` fails.

## Why

An element that continues across pages is open from `Surface::open_element` until `Builder::close_element` writes it, and only then. The element named was one of these:

- closed already;
- opened by another builder;
- opened inside an artifact, so it is part of the artifact (ISO 32000-2 §14.8.2.2);
- opened on a page added to an existing document, which is never tagged;
- opened on a page whose `PageBuilder::finish` failed, so it was never written.

## How to fix

Open the element on a page of the same builder, outside every artifact, and close it once, after its last page is finished.

## Example

```rust
let paragraph = page.surface().open_element(TagKind::P);
page.finish()?;
builder.close_element(paragraph)?;
builder.close_element(paragraph)?; // tag-element-not-open: closed already
```

## Data fields

None.
