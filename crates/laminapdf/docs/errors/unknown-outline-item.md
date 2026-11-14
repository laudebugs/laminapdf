---
title: "unknown-outline-item"
version: "0.4.0"
url: "https://docs.laminapdf.com/reference/codes/unknown-outline-item/"
---

# unknown-outline-item

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

An outline op names a bookmark by an ID that the document's outline doesn't hold.

## Why

The ID came from another document or from an older read of this one. IDs belong to one file, and a deleted bookmark's ID is gone.

## How to fix

Read the outline again and take the item's ID from it. A bookmark an op adds reports its ID in the edit's result.

## Example

```rust
doc.apply(vec![Op::from(DeleteOutlineItem::new("o999.0".parse()?, ChildItems::Delete))])?; // unknown-outline-item
```

## Data fields

None.
