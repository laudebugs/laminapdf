---
title: "unknown-page"
version: "0.2.0"
url: "https://docs.laminapdf.com/reference/codes/unknown-page/"
---

# unknown-page

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

An op names a page the document doesn't have.

## Why

The page ID is stale or from another document.

## How to fix

Use a `PageId` from this document.

## Example

```rust
let other = Document::open_bytes(other_bytes, OpenOptions::new())?;
let id = other.page_info(0)?.id;
doc.apply(vec![Op::from(RotatePage::new(id, Rotation::Clockwise90))]); // unknown-page
```

## Data fields

None.
